//! Tier-based size limits and per-user / per-IP upload rate limiting.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

/// Account tier governing the maximum upload size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UploadTier {
    /// Free tier — 1 MB.
    Free,
    /// Paid tier — 10 MB.
    Paid,
    /// Enterprise tier — 100 MB.
    Enterprise,
}

impl UploadTier {
    /// Maximum upload size in bytes for this tier.
    pub fn max_bytes(&self) -> u64 {
        match self {
            UploadTier::Free => 1024 * 1024,
            UploadTier::Paid => 10 * 1024 * 1024,
            UploadTier::Enterprise => 100 * 1024 * 1024,
        }
    }

    /// Returns `Ok(())` if `size` is within the tier limit.
    pub fn check_size(&self, size: u64) -> Result<(), SizeError> {
        if size == 0 {
            return Err(SizeError::Empty);
        }
        let max = self.max_bytes();
        if size > max {
            Err(SizeError::TooLarge { size, max })
        } else {
            Ok(())
        }
    }
}

/// Why a size check failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeError {
    /// The upload had no content.
    Empty,
    /// The upload exceeded the tier limit.
    TooLarge {
        /// Actual size.
        size: u64,
        /// Allowed maximum.
        max: u64,
    },
}

/// Per-user and per-IP hourly upload-count limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadRateConfig {
    /// Max uploads per user per hour.
    pub per_user_hourly: u64,
    /// Max uploads per IP per hour.
    pub per_ip_hourly: u64,
}

impl Default for UploadRateConfig {
    fn default() -> Self {
        Self {
            per_user_hourly: 20,
            per_ip_hourly: 60,
        }
    }
}

/// The scope whose limit was hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateScope {
    /// Per-user limit.
    User,
    /// Per-IP limit.
    Ip,
}

/// A sliding-window upload rate limiter keyed by user and IP.
pub struct UploadRateLimiter {
    config: UploadRateConfig,
    events: Mutex<HashMap<String, Vec<DateTime<Utc>>>>,
}

impl UploadRateLimiter {
    /// Creates a limiter with the given configuration.
    pub fn new(config: UploadRateConfig) -> Self {
        Self {
            config,
            events: Mutex::new(HashMap::new()),
        }
    }

    /// Checks (and on success records) an upload from `user`/`ip` at `now`.
    ///
    /// Returns `Err(scope)` for the first exceeded scope without recording.
    pub fn check_and_record(
        &self,
        user: &str,
        ip: &str,
        now: DateTime<Utc>,
    ) -> Result<(), RateScope> {
        let window = Duration::hours(1);
        let cutoff = now - window;
        let user_key = format!("u:{user}");
        let ip_key = format!("i:{ip}");

        let mut map = self.events.lock().expect("rate limiter mutex poisoned");

        let user_count = count(&map, &user_key, cutoff);
        if user_count >= self.config.per_user_hourly {
            return Err(RateScope::User);
        }
        let ip_count = count(&map, &ip_key, cutoff);
        if ip_count >= self.config.per_ip_hourly {
            return Err(RateScope::Ip);
        }

        map.entry(user_key).or_default().push(now);
        map.entry(ip_key).or_default().push(now);
        Ok(())
    }
}

fn count(map: &HashMap<String, Vec<DateTime<Utc>>>, key: &str, cutoff: DateTime<Utc>) -> u64 {
    map.get(key)
        .map(|times| times.iter().filter(|t| **t > cutoff).count() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    #[test]
    fn tier_limits_match_acceptance_criteria() {
        assert_eq!(UploadTier::Free.max_bytes(), 1024 * 1024);
        assert_eq!(UploadTier::Paid.max_bytes(), 10 * 1024 * 1024);
        assert_eq!(UploadTier::Enterprise.max_bytes(), 100 * 1024 * 1024);
    }

    #[test]
    fn size_check_enforced() {
        assert_eq!(UploadTier::Free.check_size(0), Err(SizeError::Empty));
        assert!(UploadTier::Free.check_size(1024 * 1024).is_ok());
        assert_eq!(
            UploadTier::Free.check_size(2 * 1024 * 1024),
            Err(SizeError::TooLarge {
                size: 2 * 1024 * 1024,
                max: 1024 * 1024
            })
        );
        // A file too big for free fits in paid.
        assert!(UploadTier::Paid.check_size(2 * 1024 * 1024).is_ok());
    }

    #[test]
    fn per_user_limit_blocks() {
        let limiter = UploadRateLimiter::new(UploadRateConfig {
            per_user_hourly: 3,
            per_ip_hourly: 100,
        });
        for _ in 0..3 {
            assert!(limiter
                .check_and_record("user-1", "10.0.0.1", now())
                .is_ok());
        }
        assert_eq!(
            limiter.check_and_record("user-1", "10.0.0.1", now()),
            Err(RateScope::User)
        );
    }

    #[test]
    fn per_ip_limit_blocks_across_users() {
        let limiter = UploadRateLimiter::new(UploadRateConfig {
            per_user_hourly: 100,
            per_ip_hourly: 2,
        });
        assert!(limiter
            .check_and_record("user-a", "10.0.0.9", now())
            .is_ok());
        assert!(limiter
            .check_and_record("user-b", "10.0.0.9", now())
            .is_ok());
        assert_eq!(
            limiter.check_and_record("user-c", "10.0.0.9", now()),
            Err(RateScope::Ip)
        );
    }

    #[test]
    fn window_slides() {
        let limiter = UploadRateLimiter::new(UploadRateConfig {
            per_user_hourly: 1,
            per_ip_hourly: 100,
        });
        assert!(limiter.check_and_record("u", "1.1.1.1", now()).is_ok());
        assert_eq!(
            limiter.check_and_record("u", "1.1.1.1", now()),
            Err(RateScope::User)
        );
        let later = now() + Duration::seconds(3601);
        assert!(limiter.check_and_record("u", "1.1.1.1", later).is_ok());
    }
}
