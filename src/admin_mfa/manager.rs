//! Top-level MFA orchestration.
//!
//! [`MfaManager`] holds per-account factor state and ties the pieces together
//! into the login decision flow: it applies the enrollment policy, scores the
//! attempt for anomalies, and verifies individual factors (TOTP with replay
//! protection, SMS, backup codes, WebAuthn).

use crate::admin_mfa::anomaly::{AnomalyAssessment, AnomalyConfig, AnomalyDetector, LoginContext};
use crate::admin_mfa::backup_codes::BackupCodeSet;
use crate::admin_mfa::policy::{MfaEnrollment, MfaMethod, MfaPolicy, PolicyDecision, Role};
use crate::admin_mfa::totp::TotpConfig;
use crate::admin_mfa::webauthn::{
    AssertionResponse, AssertionVerifier, RegisteredCredential, WebAuthnError, WebAuthnRegistry,
};
use std::collections::HashMap;
use uuid::Uuid;

/// Provisioning data returned when setting up TOTP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TotpSetup {
    /// Base32 secret for manual entry.
    pub secret_base32: String,
    /// `otpauth://` URI for QR-code enrollment.
    pub uri: String,
}

/// The decision produced at the start of a login.
#[derive(Debug, Clone, PartialEq)]
pub struct LoginDecision {
    /// Whether enrollment policy is satisfied.
    pub policy: PolicyDecision,
    /// Anomaly scoring of the attempt.
    pub anomaly: AnomalyAssessment,
    /// Whether a second factor must be presented before access is granted.
    pub mfa_required: bool,
}

impl LoginDecision {
    /// Whether the login may proceed to issue a session without further steps.
    pub fn is_clear(&self) -> bool {
        self.policy == PolicyDecision::Allow && !self.mfa_required
    }
}

/// Per-account MFA state.
struct AccountMfa {
    role: Role,
    enrollment: MfaEnrollment,
    totp: Option<TotpConfig>,
    last_totp_counter: Option<u64>,
    sms_phone: Option<String>,
    backup: BackupCodeSet,
    anomaly: AnomalyDetector,
}

impl AccountMfa {
    fn new(role: Role) -> Self {
        Self {
            role,
            enrollment: MfaEnrollment::none(),
            totp: None,
            last_totp_counter: None,
            sms_phone: None,
            backup: BackupCodeSet::default(),
            anomaly: AnomalyDetector::new(AnomalyConfig::default()),
        }
    }
}

/// Orchestrates MFA across all accounts.
pub struct MfaManager {
    policy: MfaPolicy,
    accounts: HashMap<Uuid, AccountMfa>,
    webauthn: WebAuthnRegistry,
}

impl MfaManager {
    /// Creates a manager with the given policy.
    pub fn new(policy: MfaPolicy) -> Self {
        Self {
            policy,
            accounts: HashMap::new(),
            webauthn: WebAuthnRegistry::new(),
        }
    }

    /// Registers an account with a role (idempotent: re-registering resets role).
    pub fn add_account(&mut self, user: Uuid, role: Role) {
        self.accounts
            .entry(user)
            .or_insert_with(|| AccountMfa::new(role))
            .role = role;
    }

    /// Returns the account's enrollment snapshot, if registered.
    pub fn enrollment(&self, user: Uuid) -> Option<&MfaEnrollment> {
        self.accounts.get(&user).map(|a| &a.enrollment)
    }

    /// Read-only access to the WebAuthn registry.
    pub fn webauthn(&self) -> &WebAuthnRegistry {
        &self.webauthn
    }

    /// Mutable access to the WebAuthn registry (challenge issuance, etc.).
    pub fn webauthn_mut(&mut self) -> &mut WebAuthnRegistry {
        &mut self.webauthn
    }

    // --- Enrollment -------------------------------------------------------

    /// Begins TOTP setup, returning provisioning data. The factor is not
    /// marked enrolled until [`confirm_totp`](Self::confirm_totp) succeeds.
    pub fn setup_totp(
        &mut self,
        user: Uuid,
        issuer: &str,
        account_label: &str,
        rng: &mut impl rand::RngCore,
    ) -> Option<TotpSetup> {
        let acct = self.accounts.get_mut(&user)?;
        let cfg = TotpConfig::generate(rng);
        let setup = TotpSetup {
            secret_base32: cfg.secret_base32(),
            uri: cfg.provisioning_uri(issuer, account_label),
        };
        acct.totp = Some(cfg);
        acct.last_totp_counter = None;
        Some(setup)
    }

    /// Confirms TOTP enrollment by verifying a freshly generated code.
    pub fn confirm_totp(&mut self, user: Uuid, code: &str, now_secs: u64) -> bool {
        let Some(acct) = self.accounts.get_mut(&user) else {
            return false;
        };
        let Some(cfg) = &acct.totp else { return false };
        if let Some(counter) = cfg.verify_at(code, now_secs) {
            acct.last_totp_counter = Some(counter);
            acct.enrollment.enroll(MfaMethod::Totp);
            true
        } else {
            false
        }
    }

    /// Enrolls an SMS phone number as a fallback factor.
    pub fn setup_sms(&mut self, user: Uuid, phone: impl Into<String>) -> bool {
        let Some(acct) = self.accounts.get_mut(&user) else {
            return false;
        };
        acct.sms_phone = Some(phone.into());
        acct.enrollment.enroll(MfaMethod::Sms);
        true
    }

    /// The enrolled SMS phone, if any.
    pub fn sms_phone(&self, user: Uuid) -> Option<&str> {
        self.accounts
            .get(&user)
            .and_then(|a| a.sms_phone.as_deref())
    }

    /// Registers a WebAuthn credential and marks the factor enrolled.
    pub fn register_webauthn(&mut self, user: Uuid, credential: RegisteredCredential) -> bool {
        let Some(acct) = self.accounts.get_mut(&user) else {
            return false;
        };
        self.webauthn.register(user, credential);
        acct.enrollment.enroll(MfaMethod::WebAuthn);
        true
    }

    /// Generates and stores backup codes, returning the plaintext to show once.
    pub fn generate_backup_codes(
        &mut self,
        user: Uuid,
        count: usize,
        rng: &mut impl rand::RngCore,
    ) -> Option<Vec<String>> {
        let acct = self.accounts.get_mut(&user)?;
        let generated = BackupCodeSet::generate(count, rng);
        acct.backup = generated.store;
        acct.enrollment.enroll(MfaMethod::BackupCode);
        Some(generated.plaintext)
    }

    // --- Login flow -------------------------------------------------------

    /// Scores a login attempt and decides what is required, recording the
    /// context into the anomaly baseline.
    pub fn begin_login(
        &mut self,
        user: Uuid,
        ctx: LoginContext,
        now_secs: i64,
    ) -> Option<LoginDecision> {
        let acct = self.accounts.get_mut(&user)?;
        let policy = self.policy.evaluate(acct.role, &acct.enrollment);
        let anomaly = acct.anomaly.assess_and_record(ctx);
        let mfa_required =
            self.policy.mfa_enforced(acct.role, &acct.enrollment) || anomaly.requires_challenge();
        let _ = now_secs; // reserved for future time-based scoring
        Some(LoginDecision {
            policy,
            anomaly,
            mfa_required,
        })
    }

    // --- Factor verification ---------------------------------------------

    /// Verifies a TOTP code at login, enforcing replay protection (a counter
    /// may not be reused).
    pub fn verify_totp(&mut self, user: Uuid, code: &str, now_secs: u64) -> bool {
        let Some(acct) = self.accounts.get_mut(&user) else {
            return false;
        };
        if !acct.enrollment.has(MfaMethod::Totp) {
            return false;
        }
        let Some(cfg) = &acct.totp else { return false };
        match cfg.verify_at(code, now_secs) {
            Some(counter) => {
                if acct.last_totp_counter.is_some_and(|last| counter <= last) {
                    // Replay of an already-used (or older) step.
                    false
                } else {
                    acct.last_totp_counter = Some(counter);
                    true
                }
            }
            None => false,
        }
    }

    /// Redeems a single-use backup code.
    pub fn redeem_backup_code(&mut self, user: Uuid, code: &str) -> bool {
        let Some(acct) = self.accounts.get_mut(&user) else {
            return false;
        };
        acct.backup.redeem(code)
    }

    /// Remaining unused backup codes.
    pub fn backup_codes_remaining(&self, user: Uuid) -> usize {
        self.accounts
            .get(&user)
            .map(|a| a.backup.remaining())
            .unwrap_or(0)
    }

    /// Verifies a WebAuthn assertion for a user.
    pub fn verify_webauthn(
        &mut self,
        user: Uuid,
        challenge_id: Uuid,
        response: &AssertionResponse,
        verifier: &dyn AssertionVerifier,
        now_secs: i64,
    ) -> Result<(), WebAuthnError> {
        self.webauthn
            .verify_assertion(user, challenge_id, response, verifier, now_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn login_ctx() -> LoginContext {
        LoginContext {
            ip: "203.0.113.1".to_string(),
            device_fingerprint: "device-A".to_string(),
            location: "US-CA".to_string(),
            hour_utc: 9,
        }
    }

    #[test]
    fn admin_must_enroll_before_clearing_policy() {
        let mut mgr = MfaManager::new(MfaPolicy::default());
        let admin = Uuid::new_v4();
        mgr.add_account(admin, Role::Admin);
        let decision = mgr.begin_login(admin, login_ctx(), 1000).unwrap();
        assert_eq!(decision.policy, PolicyDecision::AdminMfaRequired);
        assert!(decision.mfa_required);
    }

    #[test]
    fn totp_enrollment_and_login() {
        let mut mgr = MfaManager::new(MfaPolicy::default());
        let admin = Uuid::new_v4();
        mgr.add_account(admin, Role::Admin);

        let mut rng = rand::rngs::OsRng;
        let setup = mgr
            .setup_totp(admin, "Soroban", "admin@example.com", &mut rng)
            .unwrap();
        assert!(setup.uri.starts_with("otpauth://"));

        // Confirm with a current code derived from the stored secret.
        let now = 1_700_000_100u64;
        let cfg = TotpConfig::from_secret(
            crate::admin_mfa::base32::decode(&setup.secret_base32).unwrap(),
        );
        let code = cfg.code_at(now);
        assert!(mgr.confirm_totp(admin, &code, now));
        assert!(mgr.enrollment(admin).unwrap().has(MfaMethod::Totp));

        // Policy is now satisfied.
        let decision = mgr.begin_login(admin, login_ctx(), now as i64).unwrap();
        assert_eq!(decision.policy, PolicyDecision::Allow);

        // Login-time verification works once...
        let later = now + 60;
        let code2 = cfg.code_at(later);
        assert!(mgr.verify_totp(admin, &code2, later));
        // ...and the same code is rejected on replay.
        assert!(!mgr.verify_totp(admin, &code2, later));
    }

    #[test]
    fn backup_codes_are_single_use() {
        let mut mgr = MfaManager::new(MfaPolicy::default());
        let user = Uuid::new_v4();
        mgr.add_account(user, Role::User);
        let mut rng = rand::rngs::OsRng;
        let codes = mgr.generate_backup_codes(user, 5, &mut rng).unwrap();
        assert_eq!(mgr.backup_codes_remaining(user), 5);
        assert!(mgr.redeem_backup_code(user, &codes[0]));
        assert!(!mgr.redeem_backup_code(user, &codes[0]));
        assert_eq!(mgr.backup_codes_remaining(user), 4);
    }

    #[test]
    fn anomalous_login_forces_mfa_even_for_user_without_enrollment() {
        let mut mgr = MfaManager::new(MfaPolicy::default());
        let user = Uuid::new_v4();
        mgr.add_account(user, Role::User);
        // Establish a baseline.
        mgr.begin_login(user, login_ctx(), 1000);
        // Now a wildly different context.
        let suspicious = LoginContext {
            ip: "8.8.8.8".to_string(),
            device_fingerprint: "device-Z".to_string(),
            location: "RU-MOW".to_string(),
            hour_utc: 3,
        };
        let decision = mgr.begin_login(user, suspicious, 2000).unwrap();
        assert!(decision.anomaly.requires_challenge());
        assert!(decision.mfa_required);
    }

    #[test]
    fn sms_enrollment_records_phone() {
        let mut mgr = MfaManager::new(MfaPolicy::default());
        let user = Uuid::new_v4();
        mgr.add_account(user, Role::User);
        assert!(mgr.setup_sms(user, "+15551234567"));
        assert_eq!(mgr.sms_phone(user), Some("+15551234567"));
        assert!(mgr.enrollment(user).unwrap().has(MfaMethod::Sms));
    }

    #[test]
    fn unknown_account_operations_fail_gracefully() {
        let mut mgr = MfaManager::new(MfaPolicy::default());
        let ghost = Uuid::new_v4();
        assert!(mgr.begin_login(ghost, login_ctx(), 1000).is_none());
        assert!(!mgr.verify_totp(ghost, "000000", 0));
        assert!(!mgr.setup_sms(ghost, "+1"));
    }
}
