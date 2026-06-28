//! Cache warming for application startup.
//!
//! Pre-populates the cache with critical data before traffic arrives, so the
//! first requests hit a warm cache instead of cold-loading under load. A
//! [`CacheWarmer`] collects (partition, key, loader) tasks and runs them all
//! against a cache.

use crate::caching::cache::Cache;
use crate::caching::partition::Partition;

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
}

impl<V> Default for CacheWarmer<V> {
    fn default() -> Self {
        Self { tasks: Vec::new() }
    }
}

impl<V: Clone + Send + Sync + 'static> CacheWarmer<V> {
    /// Creates an empty warmer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a warm-up task.
    pub fn add<F>(&mut self, partition: Partition, key: impl Into<String>, version: u64, loader: F)
    where
        F: FnOnce() -> V + Send + 'static,
    {
        self.tasks.push(WarmTask {
            partition,
            key: key.into(),
            version,
            loader: Box::new(loader),
        });
    }

    /// Number of pending warm-up tasks.
    pub fn pending(&self) -> usize {
        self.tasks.len()
    }

    /// Runs every task against `cache`, populating it. Returns the number of
    /// entries warmed. Consumes the warmer.
    pub fn warm(self, cache: &Cache<V>) -> usize {
        let count = self.tasks.len();
        for task in self.tasks {
            let value = (task.loader)();
            cache.put(task.partition, &task.key, value, task.version);
        }
        count
    }
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
}
