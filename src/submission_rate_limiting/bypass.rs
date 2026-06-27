//! Bypass and exemption handling.
//!
//! Two distinct mechanisms are provided:
//! - **Researcher bypass** — verified security researchers present an
//!   authorization token that grants an elevated allowance (or a full bypass).
//! - **Admin exemption** — admins connecting from a configured trusted IP
//!   range skip rate limiting entirely.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::IpAddr;

/// Registry of authorized security-researcher bypass tokens.
///
/// Tokens are opaque strings issued out-of-band to verified researchers.
/// A present, registered token grants the `Researcher` allowance; a token
/// additionally listed in `full_bypass` skips enforcement entirely.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResearcherRegistry {
    authorized: HashSet<String>,
    full_bypass: HashSet<String>,
}

impl ResearcherRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a token granting the elevated researcher allowance.
    pub fn authorize(&mut self, token: impl Into<String>) {
        self.authorized.insert(token.into());
    }

    /// Registers a token that fully bypasses rate limiting.
    pub fn authorize_full_bypass(&mut self, token: impl Into<String>) {
        let token = token.into();
        self.authorized.insert(token.clone());
        self.full_bypass.insert(token);
    }

    /// Revokes a previously issued token.
    pub fn revoke(&mut self, token: &str) {
        self.authorized.remove(token);
        self.full_bypass.remove(token);
    }

    /// Returns true if the token is a recognised researcher token.
    pub fn is_authorized(&self, token: &str) -> bool {
        self.authorized.contains(token)
    }

    /// Returns true if the token grants a full bypass.
    pub fn grants_full_bypass(&self, token: &str) -> bool {
        self.full_bypass.contains(token)
    }
}

/// Parses an IPv4/IPv6 CIDR string and tests whether `ip` is contained.
///
/// A bare address (no `/prefix`) is treated as a `/32` (IPv4) or `/128`
/// (IPv6) — i.e. an exact match. Malformed input returns `false` rather
/// than panicking, so untrusted config can never crash the limiter.
pub fn cidr_contains(cidr: &str, ip: IpAddr) -> bool {
    let (addr_part, prefix_part) = match cidr.split_once('/') {
        Some((a, p)) => (a, Some(p)),
        None => (cidr, None),
    };

    let network: IpAddr = match addr_part.trim().parse() {
        Ok(addr) => addr,
        Err(_) => return false,
    };

    match (network, ip) {
        (IpAddr::V4(net), IpAddr::V4(ip)) => {
            let prefix = parse_prefix(prefix_part, 32);
            let prefix = match prefix {
                Some(p) => p,
                None => return false,
            };
            let mask = u32::MAX.checked_shl(32 - prefix).unwrap_or(0);
            (u32::from(net) & mask) == (u32::from(ip) & mask)
        }
        (IpAddr::V6(net), IpAddr::V6(ip)) => {
            let prefix = parse_prefix(prefix_part, 128);
            let prefix = match prefix {
                Some(p) => p,
                None => return false,
            };
            let mask = u128::MAX.checked_shl(128 - prefix).unwrap_or(0);
            (u128::from(net) & mask) == (u128::from(ip) & mask)
        }
        // Mixed address families never match.
        _ => false,
    }
}

/// Parses a CIDR prefix length, defaulting to `max` when absent. Returns
/// `None` for out-of-range or unparseable values.
fn parse_prefix(prefix: Option<&str>, max: u32) -> Option<u32> {
    match prefix {
        None => Some(max),
        Some(p) => match p.trim().parse::<u32>() {
            Ok(v) if v <= max => Some(v),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn researcher_authorization_lifecycle() {
        let mut reg = ResearcherRegistry::new();
        reg.authorize("tok-123");
        assert!(reg.is_authorized("tok-123"));
        assert!(!reg.grants_full_bypass("tok-123"));

        reg.authorize_full_bypass("tok-vip");
        assert!(reg.is_authorized("tok-vip"));
        assert!(reg.grants_full_bypass("tok-vip"));

        reg.revoke("tok-123");
        assert!(!reg.is_authorized("tok-123"));
    }

    #[test]
    fn cidr_ipv4_matching() {
        assert!(cidr_contains("10.0.0.0/8", "10.255.1.1".parse().unwrap()));
        assert!(cidr_contains(
            "192.168.1.0/24",
            "192.168.1.42".parse().unwrap()
        ));
        assert!(!cidr_contains(
            "192.168.1.0/24",
            "192.168.2.42".parse().unwrap()
        ));
    }

    #[test]
    fn cidr_exact_and_zero_prefix() {
        assert!(cidr_contains("203.0.113.5", "203.0.113.5".parse().unwrap()));
        assert!(!cidr_contains(
            "203.0.113.5",
            "203.0.113.6".parse().unwrap()
        ));
        // /0 matches everything.
        assert!(cidr_contains("0.0.0.0/0", "8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn cidr_ipv6_matching() {
        assert!(cidr_contains(
            "2001:db8::/32",
            "2001:db8:1234::1".parse().unwrap()
        ));
        assert!(!cidr_contains(
            "2001:db8::/32",
            "2001:dba::1".parse().unwrap()
        ));
    }

    #[test]
    fn malformed_cidr_is_rejected_safely() {
        assert!(!cidr_contains("not-an-ip", "10.0.0.1".parse().unwrap()));
        assert!(!cidr_contains("10.0.0.0/99", "10.0.0.1".parse().unwrap()));
        // Mixed families do not match.
        assert!(!cidr_contains("10.0.0.0/8", "2001:db8::1".parse().unwrap()));
    }
}
