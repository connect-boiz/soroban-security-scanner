//! Redis-backed rate limiting with in-memory fallback.
//!
//! Implements a sliding-window rate limiter using Redis `INCR` + `EXPIRE`.
//! Falls back transparently to an in-memory `HashMap` protected by
//! `tokio::sync::RwLock` when Redis is unavailable.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;

/// Per-key rate-limit configuration.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests allowed in the window.
    pub max_requests: u64,
    /// Length of the sliding window.
    pub window: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self { max_requests: 100, window: Duration::from_secs(60) }
    }
}

/// Result of a rate-limit check.
#[derive(Debug, PartialEq, Eq)]
pub enum RateLimitResult {
    /// Request is allowed; `remaining` slots left in the window.
    Allowed { remaining: u64 },
    /// Request is denied; retry after `retry_after` seconds.
    Denied { retry_after: u64 },
}

/// In-memory window state (used as fallback).
#[derive(Debug)]
struct WindowState {
    count:      u64,
    window_end: u64, // UNIX seconds
}

/// Redis-backed rate limiter with in-memory fallback.
#[derive(Clone)]
pub struct RateLimiter {
    config:   RateLimitConfig,
    /// `None` when Redis is unavailable.
    redis:    Option<redis::aio::MultiplexedConnection>,
    fallback: Arc<RwLock<HashMap<String, WindowState>>>,
}

impl RateLimiter {
    /// Creates a limiter.  Pass `None` for `redis` to use in-memory only.
    pub fn new(config: RateLimitConfig, redis: Option<redis::aio::MultiplexedConnection>) -> Self {
        Self {
            config,
            redis,
            fallback: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check and increment the counter for `key`.
    pub async fn check(&self, key: &str) -> RateLimitResult {
        match &self.redis {
            Some(_) => self.check_redis(key).await.unwrap_or_else(|_| self.check_memory(key)),
            None    => self.check_memory(key),
        }
    }

    // ------------------------------------------------------------------
    // Redis path
    // ------------------------------------------------------------------

    async fn check_redis(&self, key: &str) -> anyhow::Result<RateLimitResult> {
        use redis::AsyncCommands;
        let mut conn = self.redis.as_ref().unwrap().clone();
        let redis_key = format!("rl:{}", key);
        let window_secs = self.config.window.as_secs();

        // INCR atomically increments; EXPIRE sets TTL only on first call.
        let count: u64 = conn.incr(&redis_key, 1u64).await?;
        if count == 1 {
            conn.expire(&redis_key, window_secs as usize).await?;
        }

        if count > self.config.max_requests {
            let ttl: u64 = conn.ttl(&redis_key).await.unwrap_or(window_secs);
            Ok(RateLimitResult::Denied { retry_after: ttl })
        } else {
            Ok(RateLimitResult::Allowed {
                remaining: self.config.max_requests.saturating_sub(count),
            })
        }
    }

    // ------------------------------------------------------------------
    // In-memory fallback path
    // ------------------------------------------------------------------

    fn now_secs() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
    }

    fn check_memory(&self, key: &str) -> RateLimitResult {
        // We must block here because this is called from a sync context.
        // In async callers use `.await` on `check()` instead.
        let rt = tokio::runtime::Handle::try_current();
        if let Ok(handle) = rt {
            handle.block_on(self.check_memory_async(key))
        } else {
            // Last-resort: allow (fail-open) if we have no runtime.
            RateLimitResult::Allowed { remaining: self.config.max_requests }
        }
    }

    async fn check_memory_async(&self, key: &str) -> RateLimitResult {
        let now = Self::now_secs();
        let window_secs = self.config.window.as_secs();
        let mut map = self.fallback.write().await;

        let state = map.entry(key.to_owned()).or_insert(WindowState {
            count:      0,
            window_end: now + window_secs,
        });

        // Reset window if expired.
        if now >= state.window_end {
            state.count      = 0;
            state.window_end = now + window_secs;
        }

        state.count += 1;

        if state.count > self.config.max_requests {
            RateLimitResult::Denied {
                retry_after: state.window_end.saturating_sub(now),
            }
        } else {
            RateLimitResult::Allowed {
                remaining: self.config.max_requests.saturating_sub(state.count),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn limiter(max: u64, secs: u64) -> RateLimiter {
        RateLimiter::new(
            RateLimitConfig { max_requests: max, window: Duration::from_secs(secs) },
            None,
        )
    }

    #[tokio::test]
    async fn allows_up_to_limit() {
        let rl = limiter(3, 60);
        for i in 0..3 {
            let r = rl.check("user1").await;
            assert!(matches!(r, RateLimitResult::Allowed { .. }), "failed at i={i}");
        }
    }

    #[tokio::test]
    async fn denies_over_limit() {
        let rl = limiter(2, 60);
        rl.check("user2").await;
        rl.check("user2").await;
        let r = rl.check("user2").await;
        assert!(matches!(r, RateLimitResult::Denied { .. }));
    }

    #[tokio::test]
    async fn different_keys_are_independent() {
        let rl = limiter(1, 60);
        let r1 = rl.check("alice").await;
        let r2 = rl.check("bob").await;
        assert!(matches!(r1, RateLimitResult::Allowed { .. }));
        assert!(matches!(r2, RateLimitResult::Allowed { .. }));
    }
}
