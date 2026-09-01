use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub max_requests_per_window: u32,
    pub window_duration_seconds: u64,
    pub max_concurrent_operations: u32,
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests_per_window: 100,
            window_duration_seconds: 60,
            max_concurrent_operations: 5,
            burst_size: 20,
        }
    }
}

impl RateLimitConfig {
    pub fn strict() -> Self {
        Self {
            max_requests_per_window: 10,
            window_duration_seconds: 60,
            max_concurrent_operations: 2,
            burst_size: 5,
        }
    }

    pub fn moderate() -> Self {
        Self {
            max_requests_per_window: 50,
            window_duration_seconds: 60,
            max_concurrent_operations: 5,
            burst_size: 15,
        }
    }

    pub fn relaxed() -> Self {
        Self {
            max_requests_per_window: 200,
            window_duration_seconds: 60,
            max_concurrent_operations: 10,
            burst_size: 30,
        }
    }
}

#[derive(Debug, Clone)]
struct UserRateState {
    request_timestamps: Vec<Instant>,
    concurrent_operations: u32,
    burst_tokens: u32,
    last_burst_refill: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStatus {
    pub allowed: bool,
    pub remaining_requests: u32,
    pub retry_after_seconds: u64,
    pub current_concurrent: u32,
    pub max_concurrent: u32,
}

pub struct RateLimiter {
    config: RateLimitConfig,
    user_states: Arc<RwLock<HashMap<String, UserRateState>>>,
    cleanup_interval: Duration,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            user_states: Arc::new(RwLock::new(HashMap::new())),
            cleanup_interval: Duration::from_secs(300),
        }
    }

    pub async fn check_rate_limit(&self, user_id: &str) -> RateLimitStatus {
        let mut states = self.user_states.write().await;
        let now = Instant::now();
        let window = Duration::from_secs(self.config.window_duration_seconds);

        let state = states
            .entry(user_id.to_string())
            .or_insert_with(|| UserRateState {
                request_timestamps: Vec::new(),
                concurrent_operations: 0,
                burst_tokens: self.config.burst_size,
                last_burst_refill: now,
            });

        state.request_timestamps.retain(|t| now.duration_since(*t) < window);

        if state.last_burst_refill.elapsed() > Duration::from_secs(1) {
            let refill = (now.duration_since(state.last_burst_refill).as_secs() as u32)
                .min(self.config.burst_size - state.burst_tokens);
            state.burst_tokens = (state.burst_tokens + refill).min(self.config.burst_size);
            state.last_burst_refill = now;
        }

        let within_window = state.request_timestamps.len() < self.config.max_requests_per_window as usize;
        let has_burst = state.burst_tokens > 0;
        let within_concurrent = state.concurrent_operations < self.config.max_concurrent_operations;

        if within_window && has_burst && within_concurrent {
            state.request_timestamps.push(now);
            state.concurrent_operations += 1;
            state.burst_tokens -= 1;

            RateLimitStatus {
                allowed: true,
                remaining_requests: self.config.max_requests_per_window
                    - state.request_timestamps.len() as u32,
                retry_after_seconds: 0,
                current_concurrent: state.concurrent_operations,
                max_concurrent: self.config.max_concurrent_operations,
            }
        } else {
            let retry = if !within_window {
                let oldest = state.request_timestamps.first().copied().unwrap_or(now);
                window.checked_duration_since(now.duration_since(oldest))
                    .map(|d| d.as_secs())
                    .unwrap_or(0)
            } else {
                1
            };

            RateLimitStatus {
                allowed: false,
                remaining_requests: 0,
                retry_after_seconds: retry,
                current_concurrent: state.concurrent_operations,
                max_concurrent: self.config.max_concurrent_operations,
            }
        }
    }

    pub async fn release_concurrent(&self, user_id: &str) {
        let mut states = self.user_states.write().await;
        if let Some(state) = states.get_mut(user_id) {
            state.concurrent_operations = state.concurrent_operations.saturating_sub(1);
        }
    }

    pub async fn cleanup_stale_entries(&self) {
        let mut states = self.user_states.write().await;
        let now = Instant::now();
        let max_age = Duration::from_secs(self.config.window_duration_seconds * 2);

        states.retain(|_, state| {
            if let Some(latest) = state.request_timestamps.last() {
                now.duration_since(*latest) < max_age
            } else {
                false
            }
        });
    }

    pub async fn get_user_status(&self, user_id: &str) -> Option<RateLimitStatus> {
        let states = self.user_states.read().await;
        let now = Instant::now();
        let window = Duration::from_secs(self.config.window_duration_seconds);

        states.get(user_id).map(|state| {
            let recent_count = state
                .request_timestamps
                .iter()
                .filter(|t| now.duration_since(**t) < window)
                .count();

            RateLimitStatus {
                allowed: false,
                remaining_requests: self.config.max_requests_per_window - recent_count as u32,
                retry_after_seconds: 0,
                current_concurrent: state.concurrent_operations,
                max_concurrent: self.config.max_concurrent_operations,
            }
        })
    }

    pub fn config(&self) -> &RateLimitConfig {
        &self.config
    }
}
