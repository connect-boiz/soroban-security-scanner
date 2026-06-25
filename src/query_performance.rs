//! Database query optimisation utilities.
//!
//! Provides:
//! - `QueryTimer`: records slow queries above a configurable threshold
//! - `PaginatedQuery`: typed pagination helper preventing unbounded queries
//! - `QueryCache`: in-memory TTL cache for expensive read queries

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

// ---------------------------------------------------------------------------
// Query timing
// ---------------------------------------------------------------------------

/// Records the duration of a database query and logs slow ones.
#[must_use = "drop the guard to stop the timer"]
pub struct QueryTimer {
    label:     String,
    started:   Instant,
    threshold: Duration,
}

impl QueryTimer {
    /// Start a timer.  Queries exceeding `threshold` are logged as warnings.
    pub fn start(label: impl Into<String>, threshold: Duration) -> Self {
        Self { label: label.into(), started: Instant::now(), threshold }
    }
}

impl Drop for QueryTimer {
    fn drop(&mut self) {
        let elapsed = self.started.elapsed();
        if elapsed > self.threshold {
            tracing::warn!(
                query = %self.label,
                duration_ms = elapsed.as_millis(),
                threshold_ms = self.threshold.as_millis(),
                "slow query detected"
            );
        } else {
            tracing::debug!(
                query = %self.label,
                duration_ms = elapsed.as_millis(),
                "query ok"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Pagination
// ---------------------------------------------------------------------------

/// Validated pagination parameters that prevent unbounded table scans.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PaginatedQuery {
    /// 1-based page number.
    pub page:     u32,
    /// Page size; capped at `MAX_PAGE_SIZE`.
    pub per_page: u32,
}

impl PaginatedQuery {
    pub const MAX_PAGE_SIZE: u32 = 100;
    pub const DEFAULT_PAGE_SIZE: u32 = 20;

    pub fn new(page: u32, per_page: u32) -> Self {
        Self {
            page:     page.max(1),
            per_page: per_page.clamp(1, Self::MAX_PAGE_SIZE),
        }
    }

    /// SQL LIMIT value.
    pub fn limit(&self) -> u32 { self.per_page }

    /// SQL OFFSET value.
    pub fn offset(&self) -> u32 { (self.page - 1) * self.per_page }
}

impl Default for PaginatedQuery {
    fn default() -> Self {
        Self::new(1, Self::DEFAULT_PAGE_SIZE)
    }
}

// ---------------------------------------------------------------------------
// Query cache
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct CacheEntry<V> {
    value:      V,
    expires_at: Instant,
}

/// Thread-safe in-memory TTL cache for expensive read queries.
#[derive(Clone)]
pub struct QueryCache<K, V> {
    inner: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    ttl:   Duration,
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> QueryCache<K, V> {
    pub fn new(ttl: Duration) -> Self {
        Self { inner: Arc::new(RwLock::new(HashMap::new())), ttl }
    }

    /// Insert or update a cache entry.
    pub async fn set(&self, key: K, value: V) {
        let entry = CacheEntry { value, expires_at: Instant::now() + self.ttl };
        self.inner.write().await.insert(key, entry);
    }

    /// Retrieve a value if it exists and has not expired.
    pub async fn get(&self, key: &K) -> Option<V> {
        let map = self.inner.read().await;
        map.get(key).and_then(|e| {
            if Instant::now() < e.expires_at { Some(e.value.clone()) } else { None }
        })
    }

    /// Remove expired entries (call periodically to reclaim memory).
    pub async fn evict_expired(&self) {
        let now = Instant::now();
        self.inner.write().await.retain(|_, e| e.expires_at > now);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_offset_correct() {
        let p = PaginatedQuery::new(3, 20);
        assert_eq!(p.offset(), 40);
        assert_eq!(p.limit(), 20);
    }

    #[test]
    fn pagination_caps_per_page() {
        let p = PaginatedQuery::new(1, 9999);
        assert_eq!(p.per_page, PaginatedQuery::MAX_PAGE_SIZE);
    }

    #[test]
    fn pagination_minimum_page_is_1() {
        let p = PaginatedQuery::new(0, 10);
        assert_eq!(p.page, 1);
        assert_eq!(p.offset(), 0);
    }

    #[tokio::test]
    async fn cache_hit_within_ttl() {
        let cache: QueryCache<String, u64> = QueryCache::new(Duration::from_secs(60));
        cache.set("k".into(), 42).await;
        assert_eq!(cache.get(&"k".to_owned()).await, Some(42));
    }

    #[tokio::test]
    async fn cache_miss_for_unknown_key() {
        let cache: QueryCache<String, u64> = QueryCache::new(Duration::from_secs(60));
        assert_eq!(cache.get(&"missing".to_owned()).await, None);
    }
}
