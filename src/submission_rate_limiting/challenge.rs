//! Challenge (CAPTCHA) issuance for suspicious request patterns.
//!
//! Rather than hard-blocking borderline traffic, the limiter can demand a
//! CAPTCHA challenge once a caller's recent usage crosses a suspicion
//! threshold (a fraction of its allowance) — letting legitimate users
//! continue while raising the cost of automated abuse.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Configuration for the challenge policy.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ChallengePolicy {
    /// Whether challenges are issued at all.
    pub enabled: bool,
    /// Fraction of the allowance (0.0–1.0) above which a caller is considered
    /// suspicious and challenged on subsequent requests.
    pub suspicion_ratio: f64,
}

impl Default for ChallengePolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            suspicion_ratio: 0.8,
        }
    }
}

impl ChallengePolicy {
    /// Returns true if `used` out of `limit` requests should trigger a
    /// challenge on the next request.
    pub fn is_suspicious(&self, used: u64, limit: u64) -> bool {
        if !self.enabled || limit == 0 {
            return false;
        }
        (used as f64) >= (limit as f64) * self.suspicion_ratio
    }

    /// Issues a fresh challenge keyed to a request identifier.
    pub fn issue(&self, request_id: Uuid) -> Challenge {
        Challenge {
            id: Uuid::new_v4(),
            request_id,
            kind: ChallengeKind::Captcha,
        }
    }
}

/// The type of challenge demanded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChallengeKind {
    /// A standard interactive CAPTCHA.
    Captcha,
}

/// A challenge the client must satisfy before the request is honoured.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Challenge {
    /// Unique challenge identifier (echoed back on solution).
    pub id: Uuid,
    /// The request that triggered the challenge.
    pub request_id: Uuid,
    /// Kind of challenge.
    pub kind: ChallengeKind,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suspicion_triggers_at_threshold() {
        let policy = ChallengePolicy::default(); // 0.8
        assert!(!policy.is_suspicious(7, 10));
        assert!(policy.is_suspicious(8, 10));
        assert!(policy.is_suspicious(10, 10));
    }

    #[test]
    fn disabled_policy_never_challenges() {
        let policy = ChallengePolicy {
            enabled: false,
            suspicion_ratio: 0.1,
        };
        assert!(!policy.is_suspicious(100, 10));
    }

    #[test]
    fn zero_limit_is_not_suspicious() {
        let policy = ChallengePolicy::default();
        assert!(!policy.is_suspicious(5, 0));
    }

    #[test]
    fn issued_challenge_references_request() {
        let policy = ChallengePolicy::default();
        let req = Uuid::new_v4();
        let challenge = policy.issue(req);
        assert_eq!(challenge.request_id, req);
        assert_eq!(challenge.kind, ChallengeKind::Captcha);
    }
}
