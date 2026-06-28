//! CAPTCHA challenge for suspicious request patterns.
//!
//! Rather than hard-blocking borderline traffic, the limiter can demand a
//! CAPTCHA once a caller's usage crosses a suspicion threshold (a fraction of
//! the allowance), letting legitimate users continue while raising the cost of
//! automated abuse.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Challenge policy configuration.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ChallengePolicy {
    /// Whether challenges are issued.
    pub enabled: bool,
    /// Fraction of the allowance (0–1) above which a caller is challenged.
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
    /// Whether `used` of `limit` should trigger a challenge.
    pub fn is_suspicious(&self, used: u64, limit: u64) -> bool {
        self.enabled && limit > 0 && (used as f64) >= (limit as f64) * self.suspicion_ratio
    }

    /// Issues a CAPTCHA challenge for a request.
    pub fn issue(&self, request_id: Uuid) -> Challenge {
        Challenge {
            id: Uuid::new_v4(),
            request_id,
        }
    }
}

/// A CAPTCHA challenge the client must solve.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Challenge {
    /// Challenge id.
    pub id: Uuid,
    /// The request that triggered it.
    pub request_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suspicion_threshold() {
        let p = ChallengePolicy::default(); // 0.8
        assert!(!p.is_suspicious(7, 10));
        assert!(p.is_suspicious(8, 10));
    }

    #[test]
    fn disabled_never_suspicious() {
        let p = ChallengePolicy {
            enabled: false,
            suspicion_ratio: 0.1,
        };
        assert!(!p.is_suspicious(100, 10));
    }

    #[test]
    fn zero_limit_safe() {
        assert!(!ChallengePolicy::default().is_suspicious(5, 0));
    }

    #[test]
    fn issue_references_request() {
        let req = Uuid::new_v4();
        assert_eq!(ChallengePolicy::default().issue(req).request_id, req);
    }
}
