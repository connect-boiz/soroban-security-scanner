//! Pluggable counter storage backing the rate limiter.
//!
//! The [`RateLimitStore`] trait abstracts the windowed event counters so the
//! same limiter logic runs against either a single-node in-memory store or a
//! shared backend (e.g. Redis) for distributed, multi-instance consistency.
//!
//! The default [`InMemoryStore`] implements an accurate sliding-window log:
//! every event timestamp is retained for the configured window and pruned on
//! access, so counts never suffer the boundary burst of a fixed window.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Mutex;

/// Storage backend for windowed request counters.
///
/// Implementations must be safe to share across threads. A production Redis
/// implementation can satisfy this trait using sorted-set range counts,
/// giving the same semantics across backend instances.
pub trait RateLimitStore: Send + Sync {
    /// Returns the number of events recorded for `key` within `window` of `now`.
    fn count(&self, key: &str, now: DateTime<Utc>, window: Duration) -> u64;

    /// Returns the oldest event for `key` still inside `window`, if any.
    ///
    /// Used to compute an accurate `Retry-After`: the window has room again
    /// once this event ages out (`earliest + window`).
    fn earliest(&self, key: &str, now: DateTime<Utc>, window: Duration) -> Option<DateTime<Utc>>;

    /// Records a single event for `key` at `now`.
    fn record(&self, key: &str, now: DateTime<Utc>);

    /// Drops events older than `window` across all keys (housekeeping).
    fn cleanup(&self, now: DateTime<Utc>, window: Duration);

    /// Removes every counter (primarily for tests / administrative reset).
    fn reset(&self);
}

/// In-memory sliding-window store suitable for single-node deployments and
/// as a local fallback when a distributed backend is unavailable.
#[derive(Default)]
pub struct InMemoryStore {
    events: Mutex<HashMap<String, Vec<DateTime<Utc>>>>,
}

impl InMemoryStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of distinct keys currently tracked (for monitoring/tests).
    pub fn tracked_keys(&self) -> usize {
        self.events.lock().expect("store mutex poisoned").len()
    }
}

impl RateLimitStore for InMemoryStore {
    fn count(&self, key: &str, now: DateTime<Utc>, window: Duration) -> u64 {
        let cutoff = now - window;
        let map = self.events.lock().expect("store mutex poisoned");
        match map.get(key) {
            Some(times) => times.iter().filter(|t| **t > cutoff).count() as u64,
            None => 0,
        }
    }

    fn earliest(&self, key: &str, now: DateTime<Utc>, window: Duration) -> Option<DateTime<Utc>> {
        let cutoff = now - window;
        let map = self.events.lock().expect("store mutex poisoned");
        map.get(key)
            .and_then(|times| times.iter().filter(|t| **t > cutoff).min().copied())
    }

    fn record(&self, key: &str, now: DateTime<Utc>) {
        let mut map = self.events.lock().expect("store mutex poisoned");
        map.entry(key.to_string()).or_default().push(now);
    }

    fn cleanup(&self, now: DateTime<Utc>, window: Duration) {
        let cutoff = now - window;
        let mut map = self.events.lock().expect("store mutex poisoned");
        map.retain(|_, times| {
            times.retain(|t| *t > cutoff);
            !times.is_empty()
        });
    }

    fn reset(&self) {
        self.events.lock().expect("store mutex poisoned").clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000 + secs, 0).unwrap()
    }

    #[test]
    fn counts_only_within_window() {
        let store = InMemoryStore::new();
        let window = Duration::hours(1);
        store.record("k", t(0));
        store.record("k", t(1800)); // 30 min later
        assert_eq!(store.count("k", t(1800), window), 2);
        // ~83 min after the first event: it has aged out, only the second remains.
        assert_eq!(store.count("k", t(5000), window), 1);
        // Exactly one window after the second event, it too ages out (strict >).
        assert_eq!(store.count("k", t(1800 + 3600), window), 0);
    }

    #[test]
    fn cleanup_prunes_stale_keys() {
        let store = InMemoryStore::new();
        let window = Duration::hours(1);
        store.record("a", t(0));
        store.record("b", t(0));
        assert_eq!(store.tracked_keys(), 2);
        store.cleanup(t(7200), window); // 2h later, all stale
        assert_eq!(store.tracked_keys(), 0);
    }

    #[test]
    fn earliest_returns_oldest_in_window() {
        let store = InMemoryStore::new();
        let window = Duration::hours(1);
        store.record("k", t(100));
        store.record("k", t(200));
        assert_eq!(store.earliest("k", t(200), window), Some(t(100)));
        // After the first ages out, the second is earliest.
        assert_eq!(store.earliest("k", t(3700), window), Some(t(200)));
        assert_eq!(store.earliest("missing", t(0), window), None);
    }

    #[test]
    fn reset_clears_everything() {
        let store = InMemoryStore::new();
        store.record("a", t(0));
        store.reset();
        assert_eq!(store.tracked_keys(), 0);
    }
}
