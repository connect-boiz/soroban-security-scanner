//! Cache monitoring: hit/miss tracking and hit-rate alerting.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// Target hit rate for frequently-accessed data (issue target: 90%+).
pub const TARGET_HIT_RATE: f64 = 0.90;

/// A snapshot of cache statistics.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CacheStats {
    /// Cache hits.
    pub hits: u64,
    /// Cache misses.
    pub misses: u64,
    /// Loader invocations (backfills on miss).
    pub loads: u64,
    /// Misses that were absorbed by stampede protection (waited for an in-flight load).
    pub stampede_avoided: u64,
    /// Invalidations performed.
    pub invalidations: u64,
}

impl CacheStats {
    /// Total lookups (hits + misses).
    pub fn lookups(&self) -> u64 {
        self.hits + self.misses
    }

    /// Hit rate in `[0.0, 1.0]` (0 when no lookups).
    pub fn hit_rate(&self) -> f64 {
        let total = self.lookups();
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Thread-safe cache metrics collector.
#[derive(Default)]
pub struct CacheMonitor {
    hits: AtomicU64,
    misses: AtomicU64,
    loads: AtomicU64,
    stampede_avoided: AtomicU64,
    invalidations: AtomicU64,
}

impl CacheMonitor {
    /// Creates a monitor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a cache hit.
    pub fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a cache miss.
    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a loader invocation.
    pub fn record_load(&self) {
        self.loads.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a stampede that was avoided (waiter served from a peer's load).
    pub fn record_stampede_avoided(&self) {
        self.stampede_avoided.fetch_add(1, Ordering::Relaxed);
    }

    /// Records an invalidation.
    pub fn record_invalidation(&self, count: u64) {
        self.invalidations.fetch_add(count, Ordering::Relaxed);
    }

    /// Current snapshot.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            loads: self.loads.load(Ordering::Relaxed),
            stampede_avoided: self.stampede_avoided.load(Ordering::Relaxed),
            invalidations: self.invalidations.load(Ordering::Relaxed),
        }
    }

    /// Whether the hit rate is below `target` after at least `min_samples`
    /// lookups (returns false on cold start to avoid noise).
    pub fn below_target(&self, target: f64, min_samples: u64) -> bool {
        let stats = self.stats();
        stats.lookups() >= min_samples && stats.hit_rate() < target
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_rate_computation() {
        let m = CacheMonitor::new();
        for _ in 0..9 {
            m.record_hit();
        }
        m.record_miss();
        assert!((m.stats().hit_rate() - 0.9).abs() < 1e-9);
    }

    #[test]
    fn empty_monitor_is_zero() {
        let m = CacheMonitor::new();
        assert_eq!(m.stats().hit_rate(), 0.0);
        assert_eq!(m.stats().lookups(), 0);
    }

    #[test]
    fn alert_below_target_after_min_samples() {
        let m = CacheMonitor::new();
        // 80% hit rate over 10 lookups.
        for _ in 0..8 {
            m.record_hit();
        }
        for _ in 0..2 {
            m.record_miss();
        }
        assert!(m.below_target(TARGET_HIT_RATE, 10));
        // Not enough samples → no alert.
        let m2 = CacheMonitor::new();
        m2.record_miss();
        assert!(!m2.below_target(TARGET_HIT_RATE, 10));
    }

    #[test]
    fn counters_accumulate() {
        let m = CacheMonitor::new();
        m.record_load();
        m.record_stampede_avoided();
        m.record_invalidation(3);
        let s = m.stats();
        assert_eq!(s.loads, 1);
        assert_eq!(s.stampede_avoided, 1);
        assert_eq!(s.invalidations, 3);
    }
}
