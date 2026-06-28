//! End-to-end integration tests spanning the MFA subsystem: full admin
//! lifecycle, step-up re-auth, anomaly-driven challenges and emergency bypass.

use super::*;
use uuid::Uuid;

fn ctx(ip: &str, device: &str, location: &str, hour: u8) -> LoginContext {
    LoginContext {
        ip: ip.to_string(),
        device_fingerprint: device.to_string(),
        location: location.to_string(),
        hour_utc: hour,
    }
}

#[test]
fn full_admin_mfa_lifecycle() {
    let mut mgr = MfaManager::new(MfaPolicy::default());
    let admin = Uuid::new_v4();
    mgr.add_account(admin, Role::Admin);
    let mut rng = rand::rngs::OsRng;

    // 1. Before enrollment, policy blocks the admin.
    let pre = mgr
        .begin_login(admin, ctx("203.0.113.1", "dev-A", "US-CA", 9), 1000)
        .unwrap();
    assert_eq!(pre.policy, PolicyDecision::AdminMfaRequired);

    // 2. Enroll TOTP and confirm.
    let setup = mgr
        .setup_totp(admin, "Soroban", "admin@example.com", &mut rng)
        .unwrap();
    let totp = TotpConfig::from_secret(base32::decode(&setup.secret_base32).unwrap());
    let t = 1_700_000_000u64;
    assert!(mgr.confirm_totp(admin, &totp.code_at(t), t));

    // 3. Issue recovery backup codes.
    let codes = mgr.generate_backup_codes(admin, 10, &mut rng).unwrap();
    assert_eq!(codes.len(), 10);

    // 4. Policy now clears; MFA is still required (admins always step up).
    let post = mgr
        .begin_login(admin, ctx("203.0.113.1", "dev-A", "US-CA", 9), t as i64)
        .unwrap();
    assert_eq!(post.policy, PolicyDecision::Allow);
    assert!(post.mfa_required);

    // 5. Verify the second factor.
    let later = t + 60;
    assert!(mgr.verify_totp(admin, &totp.code_at(later), later));

    // 6. Recovery path: a backup code works when the device is lost.
    assert!(mgr.redeem_backup_code(admin, &codes[0]));
    assert_eq!(mgr.backup_codes_remaining(admin), 9);
}

#[test]
fn step_up_required_for_sensitive_admin_operation() {
    let user = Uuid::new_v4();
    let cfg = SessionConfig::default();
    let mut session = AdminSession::start(user, 1000);

    // Routine work is fine.
    assert!(session
        .authorize(Sensitivity::Normal, &cfg, 1000 + 60)
        .is_ok());

    // A sensitive action after the freshness window demands re-auth.
    let stale = 1000 + cfg.step_up_freshness_secs + 10;
    session.touch(stale - 1);
    assert_eq!(
        session.authorize(Sensitivity::Sensitive, &cfg, stale),
        Err(AccessDenied::StepUpRequired)
    );

    // Re-authenticate (MFA), then the action proceeds.
    session.mark_mfa(stale);
    assert!(session
        .authorize(Sensitivity::Sensitive, &cfg, stale)
        .is_ok());
}

#[test]
fn suspicious_login_triggers_challenge_then_webauthn_clears_it() {
    let mut mgr = MfaManager::new(MfaPolicy::default());
    let admin = Uuid::new_v4();
    mgr.add_account(admin, Role::Admin);

    // Register a hardware key.
    let credential = RegisteredCredential {
        credential_id: vec![1, 2, 3],
        public_key: vec![9],
        sign_count: 1,
        label: "YubiKey".to_string(),
    };
    assert!(mgr.register_webauthn(admin, credential));

    // Baseline a normal login, then attempt from a new country/device.
    mgr.begin_login(admin, ctx("203.0.113.1", "dev-A", "US-CA", 9), 1000);
    let decision = mgr
        .begin_login(admin, ctx("8.8.8.8", "dev-Z", "RU-MOW", 3), 2000)
        .unwrap();
    assert_eq!(decision.anomaly.level, RiskLevel::High);
    assert!(decision.mfa_required);

    // Satisfy the challenge with the hardware key.
    let challenge = mgr.webauthn_mut().issue_challenge(vec![5; 32], 2000, 300);
    let response = AssertionResponse {
        credential_id: vec![1, 2, 3],
        sign_count: 2,
        signature: vec![0xaa],
        challenge: vec![5; 32],
    };
    assert!(mgr
        .verify_webauthn(admin, challenge.id, &response, &AcceptingVerifier, 2010)
        .is_ok());
}

#[test]
fn emergency_bypass_requires_multiple_admins() {
    let target = Uuid::new_v4();
    let admin_a = Uuid::new_v4();
    let admin_b = Uuid::new_v4();
    let cfg = EmergencyConfig::default(); // quorum 2

    let mut req = BypassRequest::open(target, admin_a, 1000, cfg).unwrap();
    assert_eq!(req.status, BypassStatus::Pending);
    assert!(!req.is_bypass_active(1000));

    // A single second admin reaches quorum and grants a time-boxed bypass.
    req.approve(admin_b, 1000).unwrap();
    assert!(req.is_bypass_active(1000));

    // The grant is one-time and expires.
    assert!(req.consume(1000));
    assert!(!req.is_bypass_active(1000));
}

#[test]
fn regular_user_can_login_without_mfa_until_enrolled() {
    let mut mgr = MfaManager::new(MfaPolicy::default());
    let user = Uuid::new_v4();
    mgr.add_account(user, Role::User);

    // First login establishes baseline (elevated), second is clear.
    mgr.begin_login(user, ctx("203.0.113.5", "dev-U", "US-NY", 14), 1000);
    let decision = mgr
        .begin_login(user, ctx("203.0.113.5", "dev-U", "US-NY", 14), 1100)
        .unwrap();
    assert_eq!(decision.policy, PolicyDecision::Allow);
    assert!(!decision.mfa_required);
    assert!(decision.is_clear());
}
