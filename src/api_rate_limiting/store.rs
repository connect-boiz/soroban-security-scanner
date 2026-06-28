//! Pluggable counter storage for the rate limiter.
//!
//! The [`RateLimitStore`] trait abstracts the windowed counters so the same
//! limiter runs against a single-node in-memory store or a shared Redis backend
//! (for distributed, multi-instance consistency). The default store is an
//! accurate sliding-window log.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Mutex;

/// Windowed counter storage. Must be thread-safe.
pub trait RateLimitStore: Send + Sync {
    /// Events recorded for `key` within `window` of `now`.
    fn count(&self, key: &str, now: DateTime<Utc>, window: Duration) -> u64;
    /// Oldest in-window event for `key` (for accurate `Retry-After`).
    fn earliest(&self, key: &str, now: DateTime<Utc>, window: Duration) -> Option<DateTime<Utc>>;
    /// Records one event for `key` at `now`.
    fn record(&self, key: &str, now: DateTime<Utc>);
    /// Drops events older than `window` across all keys.
    fn cleanup(&self, now: DateTime<Utc>, window: Duration);
}

/// In-memory sliding-window store (single-node / local fallback).
#[derive(Default)]
pub struct InMemoryStore {
    events: Mutex<HashMap<String, Vec<DateTime<Utc>>>>,
}

impl InMemoryStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Distinct keys currently tracked.
    pub fn tracked_keys(&self) -> usize {
        self.events.lock().expect("store poisoned").len()
    }
}

impl RateLimitStore for InMemoryStore {
    fn count(&self, key: &str, now: DateTime<Utc>, window: Duration) -> u64 {
        let cutoff = now - window;
        self.events
            .lock()
            .expect("store poisoned")
            .get(key)
            .map(|times| times.iter().filter(|t| **t > cutoff).count() as u64)
            .unwrap_or(0)
    }

    fn earliest(&self, key: &str, now: DateTime<Utc>, window: Duration) -> Option<DateTime<Utc>> {
        let cutoff = now - window;
        self.events
            .lock()
            .expect("store poisoned")
            .get(key)
            .and_then(|times| times.iter().filter(|t| **t > cutoff).min().copied())
    }

    fn record(&self, key: &str, now: DateTime<Utc>) {
        self.events
            .lock()
            .expect("store poisoned")
            .entry(key.to_string())
            .or_default()
            .push(now);
    }

    fn cleanup(&self, now: DateTime<Utc>, window: Duration) {
        let cutoff = now - window;
        let mut map = self.events.lock().expect("store poisoned");
        map.retain(|_, times| {
            times.retain(|t| *t > cutoff);
            !times.is_empty()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000 + secs, 0).unwrap()
    }

    #[test]
    fn counts_within_window() {
        let s = InMemoryStore::new();
        let w = Duration::seconds(60);
        s.record("k", t(0));
        s.record("k", t(30));
        assert_eq!(s.count("k", t(30), w), 2);
        // At t(80): cutoff is 20, so t(0) has aged out but t(30) remains.
        assert_eq!(s.count("k", t(80), w), 1);
    }

    #[test]
    fn earliest_for_retry_after() {
        let s = InMemoryStore::new();
        let w = Duration::seconds(60);
        s.record("k", t(10));
        s.record("k", t(20));
        assert_eq!(s.earliest("k", t(20), w), Some(t(10)));
    }

    #[test]
    fn cleanup_prunes() {
        let s = InMemoryStore::new();
        let w = Duration::seconds(60);
        s.record("a", t(0));
        s.cleanup(t(120), w);
        assert_eq!(s.tracked_keys(), 0);
    }
}
