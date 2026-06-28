//! Rate-limit bypass for verified high-volume users.
//!
//! Verified callers present an authorization token issued out-of-band; a
//! recognized token exempts the request from rate limiting entirely. Tokens can
//! be revoked.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Registry of authorized bypass tokens.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BypassRegistry {
    authorized: HashSet<String>,
}

impl BypassRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Authorizes a bypass token.
    pub fn authorize(&mut self, token: impl Into<String>) {
        self.authorized.insert(token.into());
    }

    /// Revokes a token.
    pub fn revoke(&mut self, token: &str) -> bool {
        self.authorized.remove(token)
    }

    /// Whether a token grants bypass.
    pub fn is_authorized(&self, token: &str) -> bool {
        self.authorized.contains(token)
    }

    /// Number of active tokens.
    pub fn len(&self) -> usize {
        self.authorized.len()
    }

    /// Whether there are no tokens.
    pub fn is_empty(&self) -> bool {
        self.authorized.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorize_and_check() {
        let mut r = BypassRegistry::new();
        r.authorize("partner-token");
        assert!(r.is_authorized("partner-token"));
        assert!(!r.is_authorized("other"));
    }

    #[test]
    fn revoke_removes() {
        let mut r = BypassRegistry::new();
        r.authorize("t");
        assert!(r.revoke("t"));
        assert!(!r.is_authorized("t"));
        assert!(!r.revoke("t"));
    }
}
