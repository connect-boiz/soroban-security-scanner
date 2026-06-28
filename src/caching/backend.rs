//! Cache storage backends (one per cache *level*).
//!
//! Each level of the multi-level cache is a [`CacheBackend`]: the local
//! application cache (L1), a shared store such as Redis (L2), and a CDN/edge
//! cache (L3) all share this interface. The default [`InMemoryBackend`] is used
//! for L1 and as a test/fallback double for the others.

use crate::caching::entry::CacheEntry;
use std::collections::HashMap;
use std::sync::Mutex;

/// A single cache level. Implementations must be thread-safe.
pub trait CacheBackend<V>: Send + Sync {
    /// Fetches an entry by composite key.
    fn get(&self, key: &str) -> Option<CacheEntry<V>>;
    /// Stores an entry under a composite key.
    fn set(&self, key: &str, entry: CacheEntry<V>);
    /// Removes a key, returning whether it was present.
    fn remove(&self, key: &str) -> bool;
    /// Removes every key whose composite name starts with `prefix`.
    /// Returns the number removed.
    fn remove_prefix(&self, prefix: &str) -> usize;
    /// Clears the level.
    fn clear(&self);
    /// Number of entries currently stored.
    fn len(&self) -> usize;
    /// Whether the level is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// An in-memory cache level backed by a mutex-guarded map.
pub struct InMemoryBackend<V> {
    map: Mutex<HashMap<String, CacheEntry<V>>>,
}

impl<V> Default for InMemoryBackend<V> {
    fn default() -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
        }
    }
}

impl<V: Clone + Send + Sync> InMemoryBackend<V> {
    /// Creates an empty backend.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<V: Clone + Send + Sync> CacheBackend<V> for InMemoryBackend<V> {
    fn get(&self, key: &str) -> Option<CacheEntry<V>> {
        self.map
            .lock()
            .expect("cache backend poisoned")
            .get(key)
            .cloned()
    }

    fn set(&self, key: &str, entry: CacheEntry<V>) {
        self.map
            .lock()
            .expect("cache backend poisoned")
            .insert(key.to_string(), entry);
    }

    fn remove(&self, key: &str) -> bool {
        self.map
            .lock()
            .expect("cache backend poisoned")
            .remove(key)
            .is_some()
    }

    fn remove_prefix(&self, prefix: &str) -> usize {
        let mut map = self.map.lock().expect("cache backend poisoned");
        let before = map.len();
        map.retain(|k, _| !k.starts_with(prefix));
        before - map.len()
    }

    fn clear(&self) {
        self.map.lock().expect("cache backend poisoned").clear();
    }

    fn len(&self) -> usize {
        self.map.lock().expect("cache backend poisoned").len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(v: &str) -> CacheEntry<String> {
        CacheEntry::new(v.to_string(), 1000, 60, 1)
    }

    #[test]
    fn set_get_remove() {
        let b = InMemoryBackend::new();
        assert!(b.is_empty());
        b.set("k1", entry("a"));
        assert_eq!(b.get("k1").unwrap().value, "a");
        assert!(b.remove("k1"));
        assert!(!b.remove("k1"));
        assert!(b.get("k1").is_none());
    }

    #[test]
    fn remove_prefix_scopes_to_partition() {
        let b = InMemoryBackend::new();
        b.set("scan_results:a", entry("1"));
        b.set("scan_results:b", entry("2"));
        b.set("user_profiles:c", entry("3"));
        let removed = b.remove_prefix("scan_results:");
        assert_eq!(removed, 2);
        assert!(b.get("user_profiles:c").is_some());
    }

    #[test]
    fn clear_empties() {
        let b = InMemoryBackend::new();
        b.set("k", entry("v"));
        b.clear();
        assert_eq!(b.len(), 0);
    }
}
