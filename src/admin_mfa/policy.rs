//! MFA enrollment state and enforcement policy.
//!
//! Tracks which factors each account has enrolled and decides, per the org
//! policy, whether a given account is allowed to authenticate. Admins must
//! have at least one strong factor; MFA is optional (but supported) for
//! regular users.

use serde::{Deserialize, Serialize};

/// Account role for policy decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// Regular platform user.
    User,
    /// Administrator (elevated privileges).
    Admin,
}

/// A second-factor method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MfaMethod {
    /// Authenticator-app TOTP (RFC 6238).
    Totp,
    /// FIDO2/WebAuthn hardware key.
    WebAuthn,
    /// SMS one-time code (fallback).
    Sms,
    /// Single-use recovery backup code.
    BackupCode,
}

impl MfaMethod {
    /// Whether this method is considered a *strong* factor. SMS and backup
    /// codes are fallbacks and do not by themselves satisfy admin policy.
    pub fn is_strong(&self) -> bool {
        matches!(self, MfaMethod::Totp | MfaMethod::WebAuthn)
    }
}

/// Per-account enrollment record.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MfaEnrollment {
    totp: bool,
    webauthn: bool,
    sms: bool,
    backup_codes: bool,
}

impl MfaEnrollment {
    /// An empty enrollment (nothing set up).
    pub fn none() -> Self {
        Self::default()
    }

    /// Marks a method as enrolled.
    pub fn enroll(&mut self, method: MfaMethod) {
        self.set(method, true);
    }

    /// Marks a method as removed.
    pub fn remove(&mut self, method: MfaMethod) {
        self.set(method, false);
    }

    fn set(&mut self, method: MfaMethod, value: bool) {
        match method {
            MfaMethod::Totp => self.totp = value,
            MfaMethod::WebAuthn => self.webauthn = value,
            MfaMethod::Sms => self.sms = value,
            MfaMethod::BackupCode => self.backup_codes = value,
        }
    }

    /// Whether a method is enrolled.
    pub fn has(&self, method: MfaMethod) -> bool {
        match method {
            MfaMethod::Totp => self.totp,
            MfaMethod::WebAuthn => self.webauthn,
            MfaMethod::Sms => self.sms,
            MfaMethod::BackupCode => self.backup_codes,
        }
    }

    /// Whether any second factor at all is enrolled.
    pub fn any(&self) -> bool {
        self.totp || self.webauthn || self.sms || self.backup_codes
    }

    /// Whether at least one *strong* factor is enrolled.
    pub fn has_strong(&self) -> bool {
        self.totp || self.webauthn
    }
}

/// Organization MFA policy.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MfaPolicy {
    /// Admins must enroll a strong factor (cannot authenticate otherwise).
    pub require_admin_mfa: bool,
    /// Regular users must enroll some factor.
    pub require_user_mfa: bool,
}

impl Default for MfaPolicy {
    fn default() -> Self {
        Self {
            require_admin_mfa: true,
            require_user_mfa: false,
        }
    }
}

/// Why a login is blocked by policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyDecision {
    /// Authentication may proceed (MFA may still be challenged separately).
    Allow,
    /// Admin lacks a required strong factor — must enroll before access.
    AdminMfaRequired,
    /// User lacks a required factor — must enroll before access.
    UserMfaRequired,
}

impl MfaPolicy {
    /// Decides whether an account with `enrollment` and `role` satisfies policy.
    pub fn evaluate(&self, role: Role, enrollment: &MfaEnrollment) -> PolicyDecision {
        match role {
            Role::Admin if self.require_admin_mfa && !enrollment.has_strong() => {
                PolicyDecision::AdminMfaRequired
            }
            Role::User if self.require_user_mfa && !enrollment.any() => {
                PolicyDecision::UserMfaRequired
            }
            _ => PolicyDecision::Allow,
        }
    }

    /// Whether MFA must be *enforced* (a factor verified) for this account.
    pub fn mfa_enforced(&self, role: Role, enrollment: &MfaEnrollment) -> bool {
        match role {
            Role::Admin => true,
            Role::User => enrollment.any(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_without_strong_factor_is_blocked() {
        let policy = MfaPolicy::default();
        let mut e = MfaEnrollment::none();
        assert_eq!(policy.evaluate(Role::Admin, &e), PolicyDecision::AdminMfaRequired);
        // SMS alone is not strong enough for admins.
        e.enroll(MfaMethod::Sms);
        assert_eq!(policy.evaluate(Role::Admin, &e), PolicyDecision::AdminMfaRequired);
        // TOTP satisfies the policy.
        e.enroll(MfaMethod::Totp);
        assert_eq!(policy.evaluate(Role::Admin, &e), PolicyDecision::Allow);
    }

    #[test]
    fn user_mfa_optional_by_default() {
        let policy = MfaPolicy::default();
        let e = MfaEnrollment::none();
        assert_eq!(policy.evaluate(Role::User, &e), PolicyDecision::Allow);
        assert!(!policy.mfa_enforced(Role::User, &e));
    }

    #[test]
    fn user_mfa_enforced_once_enrolled() {
        let policy = MfaPolicy::default();
        let mut e = MfaEnrollment::none();
        e.enroll(MfaMethod::Totp);
        assert!(policy.mfa_enforced(Role::User, &e));
    }

    #[test]
    fn admin_mfa_always_enforced() {
        let policy = MfaPolicy::default();
        assert!(policy.mfa_enforced(Role::Admin, &MfaEnrollment::none()));
    }

    #[test]
    fn required_user_policy_blocks_until_enrolled() {
        let policy = MfaPolicy {
            require_admin_mfa: true,
            require_user_mfa: true,
        };
        let mut e = MfaEnrollment::none();
        assert_eq!(policy.evaluate(Role::User, &e), PolicyDecision::UserMfaRequired);
        e.enroll(MfaMethod::Sms);
        assert_eq!(policy.evaluate(Role::User, &e), PolicyDecision::Allow);
    }

    #[test]
    fn enrollment_enroll_remove_roundtrip() {
        let mut e = MfaEnrollment::none();
        e.enroll(MfaMethod::WebAuthn);
        assert!(e.has(MfaMethod::WebAuthn));
        assert!(e.has_strong());
        e.remove(MfaMethod::WebAuthn);
        assert!(!e.has(MfaMethod::WebAuthn));
        assert!(!e.any());
    }
}
