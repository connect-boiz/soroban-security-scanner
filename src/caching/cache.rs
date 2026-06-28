//! The multi-level cache engine.
//!
//! Ties the pieces together: partitioned keys with per-type TTLs, multiple
//! cache levels (L1 application → L2 Redis → L3 CDN) with read-through and
//! promotion, single-flight stampede protection on misses, change-driven and
//! TTL-based invalidation, and hit/miss monitoring.

use crate::caching::backend::CacheBackend;
use crate::caching::entry::{CacheEntry, Clock};
use crate::caching::monitoring::{CacheMonitor, CacheStats};
use crate::caching::partition::{Partition, PartitionTtls};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A multi-level, partitioned cache over value type `V`.
pub struct Cache<V> {
    /// Ordered cache levels, fastest (L1) first.
    levels: Vec<Box<dyn CacheBackend<V>>>,
    ttls: PartitionTtls,
    monitor: Arc<CacheMonitor>,
    clock: Clock,
    /// Per-key locks providing single-flight loads (stampede protection).
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl<V: Clone + Send + Sync + 'static> Cache<V> {
    /// Builds a cache from an ordered list of levels (index 0 = L1, fastest).
    /// Panics if `levels` is empty.
    pub fn new(levels: Vec<Box<dyn CacheBackend<V>>>, ttls: PartitionTtls, clock: Clock) -> Self {
        assert!(!levels.is_empty(), "a cache needs at least one level");
        Self {
            levels,
            ttls,
            monitor: Arc::new(CacheMonitor::new()),
            clock,
            locks: Mutex::new(HashMap::new()),
        }
    }

    /// Convenience constructor with a single in-memory L1 level and default TTLs.
    pub fn in_memory(clock: Clock) -> Self
    where
        V: 'static,
    {
        Self::new(
            vec![Box::new(crate::caching::backend::InMemoryBackend::new())],
            PartitionTtls::new(),
            clock,
        )
    }

    /// The metrics handle.
    pub fn monitor(&self) -> &Arc<CacheMonitor> {
        &self.monitor
    }

    /// Snapshot of cache statistics.
    pub fn stats(&self) -> CacheStats {
        self.monitor.stats()
    }

    /// Looks up a value, searching each level in order and promoting a hit to
    /// all faster levels. Expired entries are evicted and treated as misses.
    pub fn get(&self, partition: Partition, key: &str) -> Option<V> {
        let now = self.clock.now();
        let composite = PartitionTtls::composite_key(partition, key);

        for idx in 0..self.levels.len() {
            if let Some(entry) = self.levels[idx].get(&composite) {
                if entry.is_expired(now) {
                    self.levels[idx].remove(&composite);
                    continue;
                }
                // Promote to all faster levels.
                for faster in &self.levels[..idx] {
                    faster.set(&composite, entry.clone());
                }
                self.monitor.record_hit();
                return Some(entry.value);
            }
        }
        self.monitor.record_miss();
        None
    }

    /// Stores a value in every level using the partition's TTL and a version
    /// stamp (write-through).
    pub fn put(&self, partition: Partition, key: &str, value: V, version: u64) {
        let now = self.clock.now();
        let ttl = self.ttls.ttl(partition);
        let composite = PartitionTtls::composite_key(partition, key);
        let entry = CacheEntry::new(value, now, ttl, version);
        for level in &self.levels {
            level.set(&composite, entry.clone());
        }
    }

    /// Read-through with stampede protection: returns the cached value, or runs
    /// `loader` exactly once across concurrent callers and caches the result.
    pub fn get_or_load<F>(&self, partition: Partition, key: &str, version: u64, loader: F) -> V
    where
        F: FnOnce() -> V,
    {
        // Fast path.
        if let Some(v) = self.get(partition, key) {
            return v;
        }

        // Single-flight: serialize concurrent misses for this key.
        let composite = PartitionTtls::composite_key(partition, key);
        let key_lock = {
            let mut locks = self.locks.lock().expect("cache locks poisoned");
            locks.entry(composite.clone()).or_default().clone()
        };
        let _guard = key_lock.lock().expect("key lock poisoned");

        // Double-check: a peer may have populated the cache while we waited.
        if let Some(v) = self.get(partition, key) {
            self.monitor.record_stampede_avoided();
            return v;
        }

        let value = loader();
        self.monitor.record_load();
        self.put(partition, key, value.clone(), version);

        // Best-effort cleanup of the per-key lock entry.
        if let Ok(mut locks) = self.locks.lock() {
            if Arc::strong_count(&key_lock) <= 2 {
                locks.remove(&composite);
            }
        }
        value
    }

    /// Invalidates a single key across all levels (change-driven invalidation).
    pub fn invalidate(&self, partition: Partition, key: &str) -> bool {
        let composite = PartitionTtls::composite_key(partition, key);
        let mut removed = false;
        for level in &self.levels {
            removed |= level.remove(&composite);
        }
        if removed {
            self.monitor.record_invalidation(1);
        }
        removed
    }

    /// Invalidates an entire partition across all levels.
    pub fn invalidate_partition(&self, partition: Partition) -> usize {
        let prefix = format!("{}:", partition.prefix());
        let mut total = 0;
        for level in &self.levels {
            total += level.remove_prefix(&prefix);
        }
        self.monitor.record_invalidation(total as u64);
        total
    }

    /// Total entries across the L1 level (for diagnostics).
    pub fn l1_len(&self) -> usize {
        self.levels[0].len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::caching::backend::InMemoryBackend;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    fn cache() -> Cache<String> {
        Cache::in_memory(Clock::fixed(1000))
    }

    fn two_level() -> (Cache<String>, ()) {
        let l1: Box<dyn CacheBackend<String>> = Box::new(InMemoryBackend::new());
        let l2: Box<dyn CacheBackend<String>> = Box::new(InMemoryBackend::new());
        (
            Cache::new(vec![l1, l2], PartitionTtls::new(), Clock::fixed(1000)),
            (),
        )
    }

    #[test]
    fn put_then_get_hits() {
        let c = cache();
        c.put(Partition::UserProfiles, "u1", "alice".to_string(), 1);
        assert_eq!(
            c.get(Partition::UserProfiles, "u1"),
            Some("alice".to_string())
        );
        assert_eq!(c.stats().hits, 1);
    }

    #[test]
    fn miss_on_unknown_key() {
        let c = cache();
        assert!(c.get(Partition::Config, "missing").is_none());
        assert_eq!(c.stats().misses, 1);
    }

    #[test]
    fn expired_entry_is_a_miss() {
        let clock = Clock::fixed(1000);
        let c = Cache::<String>::in_memory(clock.clone());
        c.put(Partition::UserProfiles, "u1", "v".to_string(), 1); // TTL 300
        clock.advance(301);
        assert!(c.get(Partition::UserProfiles, "u1").is_none());
    }

    #[test]
    fn partitions_isolate_keys() {
        let c = cache();
        c.put(Partition::UserProfiles, "x", "profile".to_string(), 1);
        c.put(Partition::Config, "x", "config".to_string(), 1);
        assert_eq!(
            c.get(Partition::UserProfiles, "x"),
            Some("profile".to_string())
        );
        assert_eq!(c.get(Partition::Config, "x"), Some("config".to_string()));
    }

    #[test]
    fn invalidate_removes_key() {
        let c = cache();
        c.put(Partition::ScanResults, "s1", "result".to_string(), 1);
        assert!(c.invalidate(Partition::ScanResults, "s1"));
        assert!(c.get(Partition::ScanResults, "s1").is_none());
        assert_eq!(c.stats().invalidations, 1);
    }

    #[test]
    fn invalidate_partition_scopes() {
        let c = cache();
        c.put(Partition::ScanResults, "a", "1".to_string(), 1);
        c.put(Partition::ScanResults, "b", "2".to_string(), 1);
        c.put(Partition::Config, "c", "3".to_string(), 1);
        assert_eq!(c.invalidate_partition(Partition::ScanResults), 2);
        assert!(c.get(Partition::Config, "c").is_some());
    }

    #[test]
    fn multilevel_promotes_on_l2_hit() {
        let (c, _) = two_level();
        // Seed only L2 by writing then clearing L1 — simulate L2-only presence.
        c.put(Partition::Config, "k", "v".to_string(), 1);
        c.levels[0].clear(); // drop from L1
        assert_eq!(c.levels[0].len(), 0);
        // A get finds it in L2 and promotes back to L1.
        assert_eq!(c.get(Partition::Config, "k"), Some("v".to_string()));
        assert_eq!(c.levels[0].len(), 1, "value promoted to L1");
    }

    #[test]
    fn get_or_load_loads_once_then_caches() {
        let c = cache();
        let calls = AtomicUsize::new(0);
        let load = || {
            calls.fetch_add(1, Ordering::Relaxed);
            "computed".to_string()
        };
        let v1 = c.get_or_load(Partition::ScanResults, "expensive", 1, load);
        assert_eq!(v1, "computed");
        // Second call hits cache; loader not invoked again.
        let v2 = c.get_or_load(Partition::ScanResults, "expensive", 1, || {
            calls.fetch_add(1, Ordering::Relaxed);
            "recomputed".to_string()
        });
        assert_eq!(v2, "computed");
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert_eq!(c.stats().loads, 1);
    }

    #[test]
    fn stampede_protection_single_flight_under_concurrency() {
        let cache = Arc::new(cache());
        let calls = Arc::new(AtomicUsize::new(0));

        let handles: Vec<_> = (0..16)
            .map(|_| {
                let cache = Arc::clone(&cache);
                let calls = Arc::clone(&calls);
                thread::spawn(move || {
                    cache.get_or_load(Partition::ScanResults, "hot", 1, || {
                        calls.fetch_add(1, Ordering::Relaxed);
                        // Simulate an expensive computation window.
                        for _ in 0..1000 {
                            std::hint::spin_loop();
                        }
                        "value".to_string()
                    })
                })
            })
            .collect();

        for h in handles {
            assert_eq!(h.join().unwrap(), "value");
        }
        // The expensive loader ran exactly once despite 16 concurrent misses.
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert!(cache.stats().stampede_avoided >= 1);
    }
}
