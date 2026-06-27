//! Admin session management with step-up re-authentication.
//!
//! A session records when MFA was last satisfied. Sensitive operations
//! (verifying vulnerabilities, distributing bounties, changing security rules)
//! require a *fresh* MFA assertion: if the last verification is older than the
//! configured freshness window, the caller must re-authenticate before the
//! operation proceeds.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Operations classified by sensitivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sensitivity {
    /// Routine read/navigation — only an active session is needed.
    Normal,
    /// High-impact actions — require recent MFA (step-up).
    Sensitive,
}

/// Configuration for session lifetimes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Absolute session lifetime in seconds.
    pub max_age_secs: i64,
    /// Idle timeout in seconds.
    pub idle_timeout_secs: i64,
    /// How recent an MFA assertion must be to authorize a sensitive op.
    pub step_up_freshness_secs: i64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_age_secs: 8 * 3600,
            idle_timeout_secs: 30 * 60,
            step_up_freshness_secs: 5 * 60,
        }
    }
}

/// An authenticated admin session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminSession {
    /// Session identifier.
    pub id: Uuid,
    /// Owning account.
    pub user: Uuid,
    /// Creation timestamp (unix secs).
    pub created_at: i64,
    /// Last activity timestamp (unix secs).
    pub last_activity: i64,
    /// Last successful MFA timestamp (unix secs).
    pub last_mfa_at: i64,
}

/// Why an operation was denied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessDenied {
    /// Session exceeded its absolute lifetime.
    Expired,
    /// Session idle too long.
    IdleTimeout,
    /// Sensitive operation needs a fresh MFA assertion.
    StepUpRequired,
}

impl AdminSession {
    /// Starts a session for `user` at `now`, with MFA just satisfied.
    pub fn start(user: Uuid, now: i64) -> Self {
        Self {
            id: Uuid::new_v4(),
            user,
            created_at: now,
            last_activity: now,
            last_mfa_at: now,
        }
    }

    /// Records that MFA was satisfied again (e.g. after a step-up challenge).
    pub fn mark_mfa(&mut self, now: i64) {
        self.last_mfa_at = now;
        self.last_activity = now;
    }

    /// Records general activity, refreshing the idle timer.
    pub fn touch(&mut self, now: i64) {
        self.last_activity = now;
    }

    /// Authorizes an operation of the given sensitivity at `now`.
    ///
    /// On `Ok`, the idle timer is refreshed. On `Err`, the reason indicates
    /// whether to terminate the session or merely demand a step-up challenge.
    pub fn authorize(
        &mut self,
        sensitivity: Sensitivity,
        cfg: &SessionConfig,
        now: i64,
    ) -> Result<(), AccessDenied> {
        if now - self.created_at > cfg.max_age_secs {
            return Err(AccessDenied::Expired);
        }
        if now - self.last_activity > cfg.idle_timeout_secs {
            return Err(AccessDenied::IdleTimeout);
        }
        if sensitivity == Sensitivity::Sensitive
            && now - self.last_mfa_at > cfg.step_up_freshness_secs
        {
            return Err(AccessDenied::StepUpRequired);
        }
        self.last_activity = now;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> SessionConfig {
        SessionConfig::default()
    }

    #[test]
    fn normal_op_allowed_within_idle_window() {
        let mut s = AdminSession::start(Uuid::new_v4(), 1000);
        assert!(s.authorize(Sensitivity::Normal, &cfg(), 1000 + 60).is_ok());
    }

    #[test]
    fn sensitive_op_requires_fresh_mfa() {
        let mut s = AdminSession::start(Uuid::new_v4(), 1000);
        // Within freshness window: allowed.
        assert!(s.authorize(Sensitivity::Sensitive, &cfg(), 1000 + 60).is_ok());
        // Past freshness window (but still active): step-up required.
        let later = 1000 + cfg().step_up_freshness_secs + 1;
        // Keep the session active so it's not idle-timed-out.
        s.touch(later - 1);
        assert_eq!(
            s.authorize(Sensitivity::Sensitive, &cfg(), later),
            Err(AccessDenied::StepUpRequired)
        );
        // After re-authenticating, the sensitive op proceeds.
        s.mark_mfa(later);
        assert!(s.authorize(Sensitivity::Sensitive, &cfg(), later).is_ok());
    }

    #[test]
    fn idle_timeout_is_enforced() {
        let mut s = AdminSession::start(Uuid::new_v4(), 1000);
        let idle = 1000 + cfg().idle_timeout_secs + 1;
        assert_eq!(
            s.authorize(Sensitivity::Normal, &cfg(), idle),
            Err(AccessDenied::IdleTimeout)
        );
    }

    #[test]
    fn absolute_expiry_is_enforced() {
        let mut s = AdminSession::start(Uuid::new_v4(), 1000);
        let expired = 1000 + cfg().max_age_secs + 1;
        // Even with recent activity, absolute lifetime wins.
        s.touch(expired - 1);
        s.mark_mfa(expired - 1);
        assert_eq!(
            s.authorize(Sensitivity::Sensitive, &cfg(), expired),
            Err(AccessDenied::Expired)
        );
    }
}
