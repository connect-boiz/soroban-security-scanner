//! Multi-level caching strategy for frequently-accessed and expensive data
//! (issue #336).
//!
//! A self-contained caching layer: partitioned by data type with per-type TTLs,
//! multiple cache levels (application → Redis → CDN) with read-through and
//! promotion, single-flight stampede protection, change-driven and TTL-based
//! invalidation, consistency verification, startup warming, and hit/miss
//! monitoring with hit-rate alerting.
//!
//! # Acceptance-criteria mapping
//!
//! | Requirement | Where |
//! |-------------|-------|
//! | Caching for frequently-accessed data (patterns, scans, profiles) | [`partition::Partition`], [`cache::Cache`] |
//! | Invalidation on data changes + TTL policies | [`cache::Cache::invalidate`], [`partition::PartitionTtls`] |
//! | Cache warming at startup | [`warming::CacheWarmer`] |
//! | Monitoring with hit/miss ratio + alerting | [`monitoring::CacheMonitor`] |
//! | Multi-level caching (app / Redis / CDN) | [`cache::Cache`] over [`backend::CacheBackend`] levels |
//! | Cache stampede protection | [`cache::Cache::get_or_load`] (single-flight) |
//! | Cache consistency verification | [`consistency::verify`], [`consistency::VersionRegistry`] |
//! | Caching expensive operations (scan results) | [`partition::Partition::ScanResults`] + `get_or_load` |
//! | Partitioning with appropriate TTLs | [`partition`] |
//! | 90%+ hit-rate target | [`monitoring::TARGET_HIT_RATE`] |
//! | Comprehensive performance testing | per-module tests + [`tests`] |
//!
//! # Example
//!
//! ```
//! use soroban_security_scanner::caching::*;
//!
//! let cache = Cache::<String>::in_memory(Clock::fixed(1_700_000_000));
//! // Expensive scan result is computed once, then served from cache.
//! let result = cache.get_or_load(Partition::ScanResults, "contract-abc", 1, || {
//!     "expensive-analysis-result".to_string()
//! });
//! assert_eq!(result, "expensive-analysis-result");
//! assert_eq!(cache.get(Partition::ScanResults, "contract-abc"), Some(result));
//! ```

pub mod backend;
pub mod cache;
pub mod consistency;
pub mod entry;
pub mod monitoring;
pub mod partition;
pub mod warming;

#[cfg(test)]
mod tests;

pub use backend::{CacheBackend, InMemoryBackend};
pub use cache::Cache;
pub use consistency::{verify, Consistency, VersionRegistry};
pub use entry::{CacheEntry, Clock};
pub use monitoring::{CacheMonitor, CacheStats, TARGET_HIT_RATE};
pub use partition::{Partition, PartitionTtls};
pub use warming::{CacheWarmer, WarmTask};
