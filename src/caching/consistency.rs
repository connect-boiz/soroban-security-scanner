//! Cache consistency verification.
//!
//! Detects stale cache entries by comparing the version stamp carried on a
//! cached entry against the authoritative source version. Used by background
//! consistency sweeps and by change-driven invalidation.

use serde::{Deserialize, Serialize};

/// The outcome of a consistency check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Consistency {
    /// Cached version matches the source.
    Fresh,
    /// Cached version is behind the source — must be invalidated/refreshed.
    Stale {
        /// Version held in cache.
        cached: u64,
        /// Authoritative source version.
        source: u64,
    },
}

impl Consistency {
    /// Whether the cache is consistent with the source.
    pub fn is_fresh(&self) -> bool {
        matches!(self, Consistency::Fresh)
    }
}

/// Compares a cached version against the source version.
pub fn verify(cached_version: u64, source_version: u64) -> Consistency {
    if cached_version >= source_version {
        Consistency::Fresh
    } else {
        Consistency::Stale {
            cached: cached_version,
            source: source_version,
        }
    }
}

/// A monotonic version registry for source data, so the cache layer can learn
/// when a key's underlying data has changed (change-driven invalidation).
#[derive(Debug, Clone, Default)]
pub struct VersionRegistry {
    versions: std::collections::HashMap<String, u64>,
}

impl VersionRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// The current source version for a key (0 if never recorded).
    pub fn version(&self, key: &str) -> u64 {
        self.versions.get(key).copied().unwrap_or(0)
    }

    /// Bumps a key's version (call when the underlying data changes), returning
    /// the new version.
    pub fn bump(&mut self, key: &str) -> u64 {
        let v = self.versions.entry(key.to_string()).or_insert(0);
        *v += 1;
        *v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_or_newer_cache_is_fresh() {
        assert!(verify(5, 5).is_fresh());
        assert!(verify(6, 5).is_fresh());
    }

    #[test]
    fn older_cache_is_stale() {
        let c = verify(4, 5);
        assert!(!c.is_fresh());
        assert_eq!(
            c,
            Consistency::Stale {
                cached: 4,
                source: 5
            }
        );
    }

    #[test]
    fn version_registry_tracks_changes() {
        let mut reg = VersionRegistry::new();
        assert_eq!(reg.version("k"), 0);
        assert_eq!(reg.bump("k"), 1);
        assert_eq!(reg.bump("k"), 2);
        assert_eq!(reg.version("k"), 2);
        // A cache entry stamped at v1 is now stale against source v2.
        assert!(!verify(1, reg.version("k")).is_fresh());
    }
}
