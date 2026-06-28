//! Cache entry and time source.
//!
//! A [`CacheEntry`] carries a value alongside its TTL bookkeeping. An
//! injectable [`Clock`] keeps expiry logic deterministic in tests.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// A cached value with TTL metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheEntry<V> {
    /// The cached value.
    pub value: V,
    /// When the entry was stored (unix seconds).
    pub stored_at: i64,
    /// When the entry expires (unix seconds).
    pub expires_at: i64,
    /// Monotonic version stamp of the source data (for consistency checks).
    pub version: u64,
}

impl<V> CacheEntry<V> {
    /// Builds an entry valid for `ttl_secs` from `now`.
    pub fn new(value: V, now: i64, ttl_secs: i64, version: u64) -> Self {
        Self {
            value,
            stored_at: now,
            expires_at: now + ttl_secs,
            version,
        }
    }

    /// Whether the entry has expired at `now`.
    pub fn is_expired(&self, now: i64) -> bool {
        now >= self.expires_at
    }

    /// Remaining lifetime in seconds (0 if expired).
    pub fn ttl_remaining(&self, now: i64) -> i64 {
        (self.expires_at - now).max(0)
    }
}

/// Source of timestamps (unix seconds). `Fixed` makes TTL testing deterministic.
#[derive(Clone)]
pub enum Clock {
    /// Wall-clock time.
    System,
    /// A manually-advanced test clock.
    Fixed(Arc<Mutex<i64>>),
}

impl Clock {
    /// Creates a fixed clock starting at `t`.
    pub fn fixed(t: i64) -> Self {
        Clock::Fixed(Arc::new(Mutex::new(t)))
    }

    /// Current time in unix seconds.
    pub fn now(&self) -> i64 {
        match self {
            Clock::System => chrono::Utc::now().timestamp(),
            Clock::Fixed(t) => *t.lock().expect("clock poisoned"),
        }
    }

    /// Advances a fixed clock by `secs` (no-op for the system clock).
    pub fn advance(&self, secs: i64) {
        if let Clock::Fixed(t) = self {
            *t.lock().expect("clock poisoned") += secs;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_expires_after_ttl() {
        let e = CacheEntry::new("v", 1000, 60, 1);
        assert!(!e.is_expired(1059));
        assert!(e.is_expired(1060)); // expiry is inclusive
        assert_eq!(e.ttl_remaining(1030), 30);
        assert_eq!(e.ttl_remaining(2000), 0);
    }

    #[test]
    fn fixed_clock_advances() {
        let c = Clock::fixed(1000);
        assert_eq!(c.now(), 1000);
        c.advance(50);
        assert_eq!(c.now(), 1050);
    }
}
