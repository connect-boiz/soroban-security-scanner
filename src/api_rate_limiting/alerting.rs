//! Rate-limit threshold-breach alerting.
//!
//! Watches the rolling throttle rate and raises an alert when too large a
//! fraction of requests are being rejected — a sign of an attack or a
//! misconfigured limit — after a minimum sample size to avoid cold-start noise.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Alert configuration.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Throttle-rate (0–1) over the window above which an alert fires.
    pub throttle_rate_threshold: f64,
    /// Rolling window size (recent decisions considered).
    pub window: usize,
    /// Minimum samples before alerting.
    pub min_samples: usize,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            throttle_rate_threshold: 0.5,
            window: 100,
            min_samples: 20,
        }
    }
}

/// A raised alert.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThrottleAlert {
    /// Observed throttle rate.
    pub throttle_rate: f64,
    /// Samples in the window.
    pub samples: usize,
    /// Message.
    pub message: String,
}

/// Tracks the rolling throttle rate and emits alerts.
pub struct ThrottleAlerter {
    config: AlertConfig,
    recent: VecDeque<bool>, // true = throttled
}

impl ThrottleAlerter {
    /// Creates an alerter.
    pub fn new(config: AlertConfig) -> Self {
        Self {
            config,
            recent: VecDeque::new(),
        }
    }

    /// Observes a decision (`throttled = true` if the request was rejected).
    /// Returns an alert if the rolling throttle rate exceeds the threshold.
    pub fn observe(&mut self, throttled: bool) -> Option<ThrottleAlert> {
        self.recent.push_back(throttled);
        while self.recent.len() > self.config.window {
            self.recent.pop_front();
        }
        if self.recent.len() < self.config.min_samples {
            return None;
        }
        let throttled_count = self.recent.iter().filter(|t| **t).count();
        let rate = throttled_count as f64 / self.recent.len() as f64;
        if rate > self.config.throttle_rate_threshold {
            Some(ThrottleAlert {
                throttle_rate: rate,
                samples: self.recent.len(),
                message: format!(
                    "throttle rate {:.0}% over last {} requests exceeds {:.0}%",
                    rate * 100.0,
                    self.recent.len(),
                    self.config.throttle_rate_threshold * 100.0
                ),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_alert_before_min_samples() {
        let mut a = ThrottleAlerter::new(AlertConfig::default());
        for _ in 0..10 {
            assert!(a.observe(true).is_none());
        }
    }

    #[test]
    fn alert_when_throttle_rate_high() {
        let cfg = AlertConfig {
            throttle_rate_threshold: 0.5,
            window: 100,
            min_samples: 4,
        };
        let mut a = ThrottleAlerter::new(cfg);
        a.observe(false);
        a.observe(true);
        a.observe(true);
        let alert = a.observe(true); // 3/4 = 75% > 50%
        assert!(alert.is_some());
    }

    #[test]
    fn no_alert_when_mostly_allowed() {
        let cfg = AlertConfig {
            throttle_rate_threshold: 0.5,
            window: 100,
            min_samples: 4,
        };
        let mut a = ThrottleAlerter::new(cfg);
        for _ in 0..10 {
            assert!(a.observe(false).is_none());
        }
    }
}
