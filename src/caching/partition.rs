//! Cache partitioning with per-type TTL policies.
//!
//! Different data types have different freshness needs, so each lives in its own
//! partition with its own default TTL. Keys are namespaced by partition so an
//! invalidation of one type never touches another.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A logical cache partition for a category of data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Partition {
    /// Vulnerability detection patterns (rarely change).
    VulnerabilityPatterns,
    /// Results of scans / contract analysis (expensive to compute).
    ScanResults,
    /// User profile data.
    UserProfiles,
    /// Application configuration.
    Config,
}

impl Partition {
    /// Stable namespace prefix used in composite keys.
    pub fn prefix(&self) -> &'static str {
        match self {
            Partition::VulnerabilityPatterns => "vuln_patterns",
            Partition::ScanResults => "scan_results",
            Partition::UserProfiles => "user_profiles",
            Partition::Config => "config",
        }
    }

    /// The default TTL (seconds) appropriate for this data type.
    pub fn default_ttl_secs(&self) -> i64 {
        match self {
            // Patterns change rarely → long TTL.
            Partition::VulnerabilityPatterns => 3600,
            // Expensive results, but contracts can be re-scanned → medium TTL.
            Partition::ScanResults => 900,
            // Profiles change occasionally → short-ish TTL.
            Partition::UserProfiles => 300,
            // Config changes are deployment-driven → medium TTL.
            Partition::Config => 1800,
        }
    }

    /// All partitions (for warming / iteration).
    pub fn all() -> [Partition; 4] {
        [
            Partition::VulnerabilityPatterns,
            Partition::ScanResults,
            Partition::UserProfiles,
            Partition::Config,
        ]
    }
}

/// Resolves the TTL for each partition, allowing per-partition overrides.
#[derive(Debug, Clone, Default)]
pub struct PartitionTtls {
    overrides: HashMap<Partition, i64>,
}

impl PartitionTtls {
    /// Default policy (each partition uses its built-in default TTL).
    pub fn new() -> Self {
        Self::default()
    }

    /// Overrides the TTL for a partition.
    pub fn set(&mut self, partition: Partition, ttl_secs: i64) {
        self.overrides.insert(partition, ttl_secs);
    }

    /// The effective TTL for a partition.
    pub fn ttl(&self, partition: Partition) -> i64 {
        self.overrides
            .get(&partition)
            .copied()
            .unwrap_or_else(|| partition.default_ttl_secs())
    }

    /// Builds the composite cache key `prefix:key`.
    pub fn composite_key(partition: Partition, key: &str) -> String {
        format!("{}:{}", partition.prefix(), key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partitions_have_sensible_default_ttls() {
        assert!(
            Partition::VulnerabilityPatterns.default_ttl_secs()
                > Partition::UserProfiles.default_ttl_secs()
        );
        assert_eq!(Partition::ScanResults.default_ttl_secs(), 900);
    }

    #[test]
    fn overrides_take_effect() {
        let mut ttls = PartitionTtls::new();
        assert_eq!(ttls.ttl(Partition::UserProfiles), 300);
        ttls.set(Partition::UserProfiles, 60);
        assert_eq!(ttls.ttl(Partition::UserProfiles), 60);
        // Untouched partition keeps its default.
        assert_eq!(ttls.ttl(Partition::Config), 1800);
    }

    #[test]
    fn composite_key_is_namespaced() {
        let k = PartitionTtls::composite_key(Partition::ScanResults, "contract-abc");
        assert_eq!(k, "scan_results:contract-abc");
    }
}
