//! Query result caching for frequently-accessed data.
//!
//! A TTL cache keyed by the normalized query plus its bound parameters, so
//! repeated reads of the same data skip the database. Tracks hit/miss for the
//! performance dashboard.

use crate::query_optimization::normalize::normalize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A cached result with an expiry.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CachedResult {
    rows: Vec<Vec<String>>,
    expires_at: i64,
}

/// Hit/miss statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// Cache hits.
    pub hits: u64,
    /// Cache misses.
    pub misses: u64,
}

impl CacheStats {
    /// Hit rate in `[0.0, 1.0]` (0 when no lookups).
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// A TTL query-result cache.
#[derive(Debug, Clone, Default)]
pub struct QueryResultCache {
    entries: HashMap<String, CachedResult>,
    stats: CacheStats,
}

impl QueryResultCache {
    /// Creates an empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Cache key from the normalized query and its bound parameters.
    fn key(sql: &str, params: &[String]) -> String {
        format!("{}|{}", normalize(sql), params.join(","))
    }

    /// Looks up cached rows, honoring TTL. Records hit/miss.
    pub fn get(&mut self, sql: &str, params: &[String], now: i64) -> Option<Vec<Vec<String>>> {
        let key = Self::key(sql, params);
        match self.entries.get(&key) {
            Some(entry) if now < entry.expires_at => {
                self.stats.hits += 1;
                Some(entry.rows.clone())
            }
            Some(_) => {
                // Expired: evict and count as a miss.
                self.entries.remove(&key);
                self.stats.misses += 1;
                None
            }
            None => {
                self.stats.misses += 1;
                None
            }
        }
    }

    /// Stores rows for a query with a TTL (seconds).
    pub fn put(
        &mut self,
        sql: &str,
        params: &[String],
        rows: Vec<Vec<String>>,
        now: i64,
        ttl_secs: i64,
    ) {
        self.entries.insert(
            Self::key(sql, params),
            CachedResult {
                rows,
                expires_at: now + ttl_secs,
            },
        );
    }

    /// Invalidates a specific cached query.
    pub fn invalidate(&mut self, sql: &str, params: &[String]) -> bool {
        self.entries.remove(&Self::key(sql, params)).is_some()
    }

    /// Current stats.
    pub fn stats(&self) -> CacheStats {
        self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<Vec<String>> {
        vec![vec!["alice".to_string(), "1".to_string()]]
    }

    #[test]
    fn miss_then_hit() {
        let mut c = QueryResultCache::new();
        let sql = "SELECT * FROM users WHERE id = ?";
        let params = vec!["1".to_string()];
        assert!(c.get(sql, &params, 1000).is_none());
        c.put(sql, &params, rows(), 1000, 60);
        assert_eq!(c.get(sql, &params, 1010), Some(rows()));
        let s = c.stats();
        assert_eq!(s.hits, 1);
        assert_eq!(s.misses, 1);
    }

    #[test]
    fn expiry_is_a_miss() {
        let mut c = QueryResultCache::new();
        c.put("q", &[], rows(), 1000, 30);
        assert!(c.get("q", &[], 1031).is_none()); // expired
    }

    #[test]
    fn different_params_are_distinct_entries() {
        let mut c = QueryResultCache::new();
        let sql = "SELECT * FROM users WHERE id = ?";
        c.put(sql, &["1".to_string()], rows(), 1000, 60);
        assert!(c.get(sql, &["2".to_string()], 1000).is_none());
        assert!(c.get(sql, &["1".to_string()], 1000).is_some());
    }

    #[test]
    fn invalidate_removes_entry() {
        let mut c = QueryResultCache::new();
        c.put("q", &[], rows(), 1000, 60);
        assert!(c.invalidate("q", &[]));
        assert!(c.get("q", &[], 1000).is_none());
    }

    #[test]
    fn hit_rate() {
        let mut c = QueryResultCache::new();
        c.put("q", &[], rows(), 1000, 60);
        for _ in 0..9 {
            c.get("q", &[], 1000);
        }
        c.get("other", &[], 1000); // miss
        assert!((c.stats().hit_rate() - 0.9).abs() < 1e-9);
    }
}
