//! Cache warming for application startup.
//!
//! Pre-populates the cache with critical data before traffic arrives, so the
//! first requests hit a warm cache instead of cold-loading under load. A
//! [`CacheWarmer`] collects (partition, key, loader) tasks and runs them all
//! against a cache using single-flight stampede protection.

use crate::caching::cache::Cache;
use crate::caching::partition::{Partition, PartitionTtls};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Status report for cache warming.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WarmStatus {
    /// Total keys configured for warming.
    pub total_keys: usize,
    /// Keys successfully warmed so far.
    pub keys_warmed: usize,
    /// Keys currently being warmed.
    pub keys_in_progress: usize,
    /// Estimated time of completion in seconds (if warming in progress).
    pub eta_seconds: Option<f64>,
}

/// Shared handle for querying warmer progress and awaiting keys.
#[derive(Clone)]
pub struct WarmerHandle {
    pub(crate) scheduled_keys: Arc<Mutex<HashSet<String>>>,
    pub(crate) warming_in_progress: Arc<Mutex<HashSet<String>>>,
    pub(crate) warmed_keys: Arc<Mutex<HashSet<String>>>,
    pub(crate) notifiers: Arc<Mutex<HashMap<String, Arc<tokio::sync::Notify>>>>,
    pub(crate) total_keys: Arc<AtomicUsize>,
    pub(crate) keys_warmed: Arc<AtomicUsize>,
    pub(crate) start_time: Arc<Mutex<Option<Instant>>>,
    pub(crate) warm_timeout: Duration,
}

impl Default for WarmerHandle {
    fn default() -> Self {
        Self {
            scheduled_keys: Arc::new(Mutex::new(HashSet::new())),
            warming_in_progress: Arc::new(Mutex::new(HashSet::new())),
            warmed_keys: Arc::new(Mutex::new(HashSet::new())),
            notifiers: Arc::new(Mutex::new(HashMap::new())),
            total_keys: Arc::new(AtomicUsize::new(0)),
            keys_warmed: Arc::new(AtomicUsize::new(0)),
            start_time: Arc::new(Mutex::new(None)),
            warm_timeout: Duration::from_secs(30),
        }
    }
}

impl WarmerHandle {
    /// Returns current warming status snapshot.
    pub fn status(&self) -> WarmStatus {
        let total = self.total_keys.load(Ordering::Relaxed);
        let warmed = self.keys_warmed.load(Ordering::Relaxed);
        let in_progress = self.warming_in_progress.lock().unwrap().len();

        let eta_seconds = if let Some(start) = *self.start_time.lock().unwrap() {
            let elapsed = start.elapsed().as_secs_f64();
            if warmed > 0 && elapsed > 0.0 && warmed < total {
                let remaining = total.saturating_sub(warmed);
                let rate = warmed as f64 / elapsed;
                Some((remaining as f64 / rate).max(0.0))
            } else if warmed >= total && total > 0 {
                Some(0.0)
            } else {
                None
            }
        } else {
            None
        };

        WarmStatus {
            total_keys: total,
            keys_warmed: warmed,
            keys_in_progress: in_progress,
            eta_seconds,
        }
    }

    /// Checks if a key is currently being warmed.
    pub fn is_warming(&self, partition: Partition, key: &str) -> bool {
        let composite = PartitionTtls::composite_key(partition, key);
        self.warming_in_progress.lock().unwrap().contains(&composite)
    }

    /// Checks if a key is currently being warmed or scheduled to be warmed.
    pub fn is_warming_or_scheduled(&self, partition: Partition, key: &str) -> bool {
        let composite = PartitionTtls::composite_key(partition, key);
        self.warming_in_progress.lock().unwrap().contains(&composite)
            || self.scheduled_keys.lock().unwrap().contains(&composite)
    }

    /// Checks if a key was warmed or is currently in-progress or scheduled.
    pub fn was_warmed_or_warming(&self, partition: Partition, key: &str) -> bool {
        let composite = PartitionTtls::composite_key(partition, key);
        self.warming_in_progress.lock().unwrap().contains(&composite)
            || self.scheduled_keys.lock().unwrap().contains(&composite)
            || self.warmed_keys.lock().unwrap().contains(&composite)
    }

    /// Asynchronously awaits completion of warming for a key up to `warm_timeout`.
    /// Returns `true` if completed, or `false` if abandoned after timeout.
    pub async fn wait_for_key(&self, partition: Partition, key: &str) -> bool {
        let composite = PartitionTtls::composite_key(partition, key);
        let notifier = {
            let in_prog = self.warming_in_progress.lock().unwrap().contains(&composite);
            let sched = self.scheduled_keys.lock().unwrap().contains(&composite);
            if !in_prog && !sched {
                return true;
            }
            let mut notifs = self.notifiers.lock().unwrap();
            notifs.entry(composite).or_default().clone()
        };

        match tokio::time::timeout(self.warm_timeout, notifier.notified()).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Synchronously awaits completion of warming for a key up to `warm_timeout`.
    pub fn wait_for_key_sync(&self, partition: Partition, key: &str) -> bool {
        let composite = PartitionTtls::composite_key(partition, key);
        let start = Instant::now();
        loop {
            {
                let in_prog = self.warming_in_progress.lock().unwrap().contains(&composite);
                let sched = self.scheduled_keys.lock().unwrap().contains(&composite);
                if !in_prog && !sched {
                    return true;
                }
            }
            if start.elapsed() >= self.warm_timeout {
                return false;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

/// A single warming task: where to store and how to produce the value.
pub struct WarmTask<V> {
    /// Target partition.
    pub partition: Partition,
    /// Cache key.
    pub key: String,
    /// Source version stamp for the loaded value.
    pub version: u64,
    /// Loader producing the value.
    pub loader: Box<dyn FnOnce() -> V + Send>,
}

/// Collects warm-up tasks and applies them to a cache at startup.
pub struct CacheWarmer<V> {
    tasks: Vec<WarmTask<V>>,
    handle: WarmerHandle,
}

impl<V> Default for CacheWarmer<V> {
    fn default() -> Self {
        Self {
            tasks: Vec::new(),
            handle: WarmerHandle::default(),
        }
    }
}

impl<V: Clone + Send + Sync + 'static> CacheWarmer<V> {
    /// Creates an empty warmer with default 30-second timeout.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configures the warming timeout (default 30 seconds).
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.handle.warm_timeout = timeout;
        self
    }

    /// Configurable warm timeout.
    pub fn warm_timeout(&self) -> Duration {
        self.handle.warm_timeout
    }

    /// Sets warm timeout.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.handle.warm_timeout = timeout;
    }

    /// Returns a cloneable handle for monitoring and synchronization.
    pub fn handle(&self) -> WarmerHandle {
        self.handle.clone()
    }

    /// Registers a warm-up task.
    pub fn add<F>(&mut self, partition: Partition, key: impl Into<String>, version: u64, loader: F)
    where
        F: FnOnce() -> V + Send + 'static,
    {
        let key_str = key.into();
        let composite = PartitionTtls::composite_key(partition, &key_str);
        self.handle.scheduled_keys.lock().unwrap().insert(composite);
        self.handle.total_keys.fetch_add(1, Ordering::Relaxed);
        self.tasks.push(WarmTask {
            partition,
            key: key_str,
            version,
            loader: Box::new(loader),
        });
    }

    /// Number of pending warm-up tasks.
    pub fn pending(&self) -> usize {
        self.tasks.len()
    }

    /// Checks if a key is currently warming.
    pub fn is_warming(&self, partition: Partition, key: &str) -> bool {
        self.handle.is_warming(partition, key)
    }

    /// Current status snapshot.
    pub fn status(&self) -> WarmStatus {
        self.handle.status()
    }

    /// Warms a specific key using `get_or_load` semantics so concurrent requests
    /// are coalesced into a single computation.
    pub fn warm_key<F>(
        &self,
        cache: &Cache<V>,
        partition: Partition,
        key: &str,
        version: u64,
        loader: F,
    ) -> V
    where
        F: FnOnce() -> V,
    {
        cache.set_warmer_handle(self.handle.clone());
        let composite = PartitionTtls::composite_key(partition, key);
        let notifier = {
            self.handle.scheduled_keys.lock().unwrap().remove(&composite);
            let mut in_prog = self.handle.warming_in_progress.lock().unwrap();
            in_prog.insert(composite.clone());
            let mut notifs = self.handle.notifiers.lock().unwrap();
            notifs.entry(composite.clone()).or_default().clone()
        };

        // Single-flight lock on cache to coalesce with any active get_or_load calls
        let key_lock = {
            let mut locks = cache.locks.lock().expect("cache locks poisoned");
            locks.entry(composite.clone()).or_default().clone()
        };
        let _guard = key_lock.lock().expect("key lock poisoned");

        let value = if let Some(v) = cache.peek(partition, key) {
            v
        } else {
            let val = loader();
            cache.monitor.record_load();
            cache.put(partition, key, val.clone(), version);
            val
        };

        // Mark completed
        {
            let mut in_prog = self.handle.warming_in_progress.lock().unwrap();
            in_prog.remove(&composite);
            let mut warmed = self.handle.warmed_keys.lock().unwrap();
            warmed.insert(composite.clone());
            let mut notifs = self.handle.notifiers.lock().unwrap();
            notifs.remove(&composite);
        }
        self.handle.keys_warmed.fetch_add(1, Ordering::Relaxed);
        notifier.notify_waiters();

        value
    }

    /// Runs all queued warming tasks against `cache` using single-flight `get_or_load`.
    pub fn warm_all(&mut self, cache: &Cache<V>) -> usize {
        cache.set_warmer_handle(self.handle.clone());
        {
            let mut start = self.handle.start_time.lock().unwrap();
            if start.is_none() {
                *start = Some(Instant::now());
            }
        }

        let tasks = std::mem::take(&mut self.tasks);
        let count = tasks.len();
        for task in tasks {
            self.warm_key(cache, task.partition, &task.key, task.version, task.loader);
        }
        count
    }

    /// Backward-compatible `warm`: warms all tasks against `cache`.
    pub fn warm(mut self, cache: &Cache<V>) -> usize {
        self.warm_all(cache)
    }
}

/// Builds an Axum router serving `GET /api/v1/cache/warm-status`.
pub fn warm_status_router(handle: WarmerHandle) -> Router {
    Router::new().route(
        "/api/v1/cache/warm-status",
        get(move || {
            let h = handle.clone();
            async move { Json(h.status()) }
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::caching::entry::Clock;

    #[test]
    fn warming_populates_cache_before_traffic() {
        let cache = Cache::<String>::in_memory(Clock::fixed(1000));
        let mut warmer = CacheWarmer::new();
        warmer.add(Partition::VulnerabilityPatterns, "reentrancy", 1, || {
            "pattern-def".to_string()
        });
        warmer.add(Partition::Config, "limits", 1, || "config-blob".to_string());
        assert_eq!(warmer.pending(), 2);

        let warmed = warmer.warm(&cache);
        assert_eq!(warmed, 2);

        // The very first lookups are hits — no cold load.
        assert_eq!(
            cache.get(Partition::VulnerabilityPatterns, "reentrancy"),
            Some("pattern-def".to_string())
        );
        assert_eq!(
            cache.get(Partition::Config, "limits"),
            Some("config-blob".to_string())
        );
        assert_eq!(cache.stats().hits, 2);
        assert_eq!(cache.stats().misses, 0);
    }

    #[test]
    fn empty_warmer_warms_nothing() {
        let cache = Cache::<String>::in_memory(Clock::fixed(1000));
        let warmer = CacheWarmer::<String>::new();
        assert_eq!(warmer.warm(&cache), 0);
    }

    #[test]
    fn warm_status_reports_correctly() {
        let cache = Cache::<String>::in_memory(Clock::fixed(1000));
        let mut warmer = CacheWarmer::new().with_timeout(Duration::from_secs(15));
        assert_eq!(warmer.warm_timeout(), Duration::from_secs(15));

        warmer.add(Partition::Config, "k1", 1, || "v1".to_string());
        warmer.add(Partition::Config, "k2", 1, || "v2".to_string());

        let status = warmer.status();
        assert_eq!(status.total_keys, 2);
        assert_eq!(status.keys_warmed, 0);
        assert_eq!(status.keys_in_progress, 0);

        let warmed = warmer.warm(&cache);
        assert_eq!(warmed, 2);

        let post_status = cache.stats();
        assert_eq!(post_status.loads, 2);
    }
}
