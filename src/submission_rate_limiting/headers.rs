//! Standard rate-limit response headers.
//!
//! Produces the `X-RateLimit-Limit`, `X-RateLimit-Remaining`,
//! `X-RateLimit-Reset` and (when blocked) `Retry-After` headers so clients
//! can self-throttle.

use serde::{Deserialize, Serialize};

/// A set of rate-limit headers describing the most-constraining scope for a
/// given request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimitHeaders {
    /// The effective limit for the constraining scope.
    pub limit: u64,
    /// Remaining requests in the current window (0 when blocked).
    pub remaining: u64,
    /// Unix timestamp (seconds) at which the window resets.
    pub reset: i64,
    /// Seconds to wait before retrying; `None` unless the request was blocked.
    pub retry_after: Option<i64>,
    /// The scope that produced these headers (e.g. "user", "ip", "global").
    pub scope: String,
}

impl RateLimitHeaders {
    /// Builds headers for an allowed request.
    pub fn allowed(limit: u64, remaining: u64, reset: i64, scope: impl Into<String>) -> Self {
        Self {
            limit,
            remaining,
            reset,
            retry_after: None,
            scope: scope.into(),
        }
    }

    /// Builds headers for a blocked request.
    pub fn blocked(limit: u64, reset: i64, retry_after: i64, scope: impl Into<String>) -> Self {
        Self {
            limit,
            remaining: 0,
            reset,
            retry_after: Some(retry_after.max(0)),
            scope: scope.into(),
        }
    }

    /// Renders the headers as `(name, value)` pairs ready to attach to an
    /// HTTP response.
    pub fn to_pairs(&self) -> Vec<(&'static str, String)> {
        let mut pairs = vec![
            ("X-RateLimit-Limit", self.limit.to_string()),
            ("X-RateLimit-Remaining", self.remaining.to_string()),
            ("X-RateLimit-Reset", self.reset.to_string()),
            ("X-RateLimit-Scope", self.scope.clone()),
        ];
        if let Some(retry) = self.retry_after {
            pairs.push(("Retry-After", retry.to_string()));
        }
        pairs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_headers_have_no_retry_after() {
        let h = RateLimitHeaders::allowed(10, 7, 1_700_003_600, "user");
        assert_eq!(h.remaining, 7);
        assert!(h.retry_after.is_none());
        let pairs = h.to_pairs();
        assert!(pairs.iter().any(|(k, v)| *k == "X-RateLimit-Limit" && v == "10"));
        assert!(!pairs.iter().any(|(k, _)| *k == "Retry-After"));
    }

    #[test]
    fn blocked_headers_carry_retry_after() {
        let h = RateLimitHeaders::blocked(10, 1_700_003_600, 120, "ip");
        assert_eq!(h.remaining, 0);
        assert_eq!(h.retry_after, Some(120));
        let pairs = h.to_pairs();
        assert!(pairs.iter().any(|(k, v)| *k == "Retry-After" && v == "120"));
    }

    #[test]
    fn negative_retry_after_is_clamped() {
        let h = RateLimitHeaders::blocked(10, 0, -5, "global");
        assert_eq!(h.retry_after, Some(0));
    }
}
