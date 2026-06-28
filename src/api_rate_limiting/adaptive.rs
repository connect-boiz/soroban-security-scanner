//! Adaptive rate limiting driven by system load and latency.
//!
//! Produces a multiplier in `[min_multiplier, 1.0]` applied to every endpoint's
//! limit, tightening as load or response time degrade so the API sheds load
//! before it falls over.

use crate::api_rate_limiting::config::AdaptiveConfig;
use serde::{Deserialize, Serialize};

/// A point-in-time health reading.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SystemHealth {
    /// Normalized load, 0.0 (idle) – 1.0 (saturated).
    pub load: f64,
    /// Recent average latency in milliseconds.
    pub avg_latency_ms: f64,
}

impl SystemHealth {
    /// A nominal healthy reading.
    pub fn healthy() -> Self {
        Self {
            load: 0.0,
            avg_latency_ms: 0.0,
        }
    }

    /// Clamps inputs to sane ranges.
    pub fn new(load: f64, avg_latency_ms: f64) -> Self {
        Self {
            load: load.clamp(0.0, 1.0),
            avg_latency_ms: avg_latency_ms.max(0.0),
        }
    }
}

/// Computes adaptive multipliers from health.
#[derive(Debug, Clone, Copy)]
pub struct AdaptiveController {
    config: AdaptiveConfig,
}

impl AdaptiveController {
    /// Creates a controller.
    pub fn new(config: AdaptiveConfig) -> Self {
        Self { config }
    }

    /// Multiplier in `[min_multiplier, 1.0]`; the worse of load/latency wins.
    pub fn multiplier(&self, health: SystemHealth) -> f64 {
        if !self.config.enabled {
            return 1.0;
        }
        let load_p = pressure(health.load, self.config.healthy_load, 1.0);
        let lat_p = pressure(
            health.avg_latency_ms,
            self.config.healthy_latency_ms,
            self.config.healthy_latency_ms * 4.0,
        );
        let pressure = load_p.max(lat_p);
        (1.0 - pressure * (1.0 - self.config.min_multiplier)).clamp(self.config.min_multiplier, 1.0)
    }

    /// Applies the multiplier to a base limit (never below 1).
    pub fn apply(&self, base_limit: u64, health: SystemHealth) -> u64 {
        ((base_limit as f64 * self.multiplier(health)).floor() as u64).max(1)
    }
}

fn pressure(value: f64, healthy: f64, saturated: f64) -> f64 {
    if value <= healthy || saturated <= healthy {
        0.0
    } else {
        ((value - healthy) / (saturated - healthy)).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctrl() -> AdaptiveController {
        AdaptiveController::new(AdaptiveConfig::default())
    }

    #[test]
    fn healthy_keeps_full_limit() {
        assert_eq!(ctrl().multiplier(SystemHealth::healthy()), 1.0);
        assert_eq!(ctrl().apply(100, SystemHealth::healthy()), 100);
    }

    #[test]
    fn load_reduces_limit() {
        let m = ctrl().multiplier(SystemHealth::new(1.0, 0.0));
        assert!(m < 1.0 && m >= AdaptiveConfig::default().min_multiplier);
        assert!(ctrl().apply(100, SystemHealth::new(1.0, 0.0)) < 100);
    }

    #[test]
    fn latency_reduces_limit_to_floor() {
        let m = ctrl().multiplier(SystemHealth::new(0.0, 2000.0));
        assert!((m - AdaptiveConfig::default().min_multiplier).abs() < 1e-9);
    }

    #[test]
    fn disabled_is_noop() {
        let c = AdaptiveController::new(AdaptiveConfig {
            enabled: false,
            ..AdaptiveConfig::default()
        });
        assert_eq!(c.multiplier(SystemHealth::new(1.0, 9999.0)), 1.0);
    }

    #[test]
    fn apply_never_zero() {
        assert!(ctrl().apply(1, SystemHealth::new(1.0, 9999.0)) >= 1);
    }
}
