//! Multi-layer caching strategy.
//!
//! Layer 1: In-memory LRU cache (hot data, sub-millisecond)
//! Layer 2: Redis cache (shared across instances, seconds TTL)
//!
//! Cache keys are namespaced by data type to prevent collisions.
//! All writes update both layers; reads prefer L1, fall back to L2.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Canonical TTLs for different data types.
pub mod ttl {
    use std::time::Duration;
    /// Vulnerability patterns: stable, cache aggressively.
    pub const VULN_PATTERNS:  Duration = Duration::from_secs(3600);
    /// Scan results: immutable once complete.
    pub const SCAN_RESULTS:   Duration = Duration::from_secs(1800);
    /// Contract metadata: changes rarely.
    pub const CONTRACT_META:  Duration = Duration::from_secs(600);
    /// User sessions: short-lived.
    pub const SESSION:        Duration = Duration::from_secs(300);
    /// Rate limit counters: very short.
    pub const RATE_LIMIT:     Duration = Duration::from_secs(60);
}

/// Cache namespace prefixes.
pub mod ns {
    pub const VULN:    &str = "vuln";
    pub const SCAN:    &str = "scan";
    pub const SESSION: &str = "sess";
    pub const RATE:    &str = "rate";
}

/// Build a namespaced cache key.
pub fn cache_key(namespace: &str, id: &str) -> String {
    format!("sss:{}:{}", namespace, id)
}

/// Cache strategy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub l1_max_entries: usize,
    pub l1_ttl:         Duration,
    pub l2_ttl:         Duration,
    pub l2_enabled:     bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            l1_max_entries: 1_000,
            l1_ttl:         Duration::from_secs(30),
            l2_ttl:         Duration::from_secs(300),
            l2_enabled:     true,
        }
    }
}

/// Cache operation result with hit/miss metadata.
#[derive(Debug)]
pub struct CacheResult<T> {
    pub value:  T,
    pub hit_l1: bool,
    pub hit_l2: bool,
}

/// Cache invalidation helper: derive all related keys to purge.
pub fn invalidation_keys(namespace: &str, id: &str) -> Vec<String> {
    vec![
        cache_key(namespace, id),
        cache_key(namespace, &format!("list:{}", &id[..id.len().min(8)])),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn key_is_namespaced()  { assert!(cache_key(ns::SCAN, "abc").starts_with("sss:scan:"));    }
    #[test] fn key_contains_id()    { assert!(cache_key(ns::VULN, "xyz").contains("xyz"));             }
    #[test] fn invalidation_keys_count() { assert_eq!(invalidation_keys(ns::SCAN, "12345678").len(), 2); }
    #[test] fn default_l2_enabled() { assert!(CacheConfig::default().l2_enabled);                      }
    #[test] fn vuln_ttl_is_longest() {
        assert!(ttl::VULN_PATTERNS > ttl::SESSION);
        assert!(ttl::VULN_PATTERNS > ttl::RATE_LIMIT);
    }
}
