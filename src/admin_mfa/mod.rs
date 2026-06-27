//! Multi-factor authentication for administrative access (issue #328).
//!
//! A self-contained MFA subsystem layered on top of the existing JWT auth. It
//! adds TOTP, FIDO2/WebAuthn and SMS factors, single-use backup codes,
//! enrollment policy (mandatory for admins), session anomaly detection with
//! automatic step-up challenges, sensitive-operation re-authentication, and a
//! multi-admin emergency bypass procedure.
//!
//! # Acceptance-criteria mapping
//!
//! | Requirement | Where |
//! |-------------|-------|
//! | TOTP MFA (RFC 6238) | [`totp::TotpConfig`] |
//! | Hardware keys (FIDO2/WebAuthn) | [`webauthn::WebAuthnRegistry`] |
//! | SMS fallback | [`sms::SmsChallenge`] |
//! | MFA enforced for admins, optional for users | [`policy::MfaPolicy`] |
//! | Setup & recovery with backup codes | [`backup_codes::BackupCodeSet`] |
//! | Session anomaly detection (IP/device/location/time) | [`anomaly::AnomalyDetector`] |
//! | Automatic MFA challenge for suspicious logins | [`manager::MfaManager::begin_login`] |
//! | Forced re-auth for sensitive operations | [`session::AdminSession`] |
//! | Emergency bypass with multi-admin approval | [`emergency::BypassRequest`] |
//! | Comprehensive security testing | per-module `#[cfg(test)]` + [`tests`] |
//!
//! # Example: admin TOTP enrollment + login
//!
//! ```
//! use soroban_security_scanner::admin_mfa::*;
//! use uuid::Uuid;
//!
//! let mut mgr = MfaManager::new(MfaPolicy::default());
//! let admin = Uuid::new_v4();
//! mgr.add_account(admin, Role::Admin);
//!
//! // Admins cannot clear policy until a strong factor is enrolled.
//! let ctx = LoginContext {
//!     ip: "203.0.113.1".into(),
//!     device_fingerprint: "yubi-laptop".into(),
//!     location: "US-CA".into(),
//!     hour_utc: 9,
//! };
//! let decision = mgr.begin_login(admin, ctx, 1_700_000_000).unwrap();
//! assert_eq!(decision.policy, PolicyDecision::AdminMfaRequired);
//! ```

pub mod anomaly;
pub mod backup_codes;
pub mod base32;
pub mod emergency;
pub mod manager;
pub mod policy;
pub mod session;
pub mod sms;
pub mod totp;
pub mod webauthn;

#[cfg(test)]
mod tests;

pub use anomaly::{AnomalyAssessment, AnomalyConfig, AnomalyDetector, LoginContext, RiskLevel};
pub use backup_codes::{BackupCodeSet, GeneratedBackupCodes};
pub use emergency::{BypassRequest, BypassStatus, EmergencyConfig, EmergencyError};
pub use manager::{LoginDecision, MfaManager, TotpSetup};
pub use policy::{MfaEnrollment, MfaMethod, MfaPolicy, PolicyDecision, Role};
pub use session::{AccessDenied, AdminSession, SessionConfig, Sensitivity};
pub use sms::{SmsChallenge, SmsSender, SmsVerifyResult};
pub use totp::TotpConfig;
pub use webauthn::{
    AcceptingVerifier, AssertionResponse, AssertionVerifier, RegisteredCredential, WebAuthnChallenge,
    WebAuthnError, WebAuthnRegistry,
};
