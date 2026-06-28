//! Adaptive rate limiting.
//!
//! Scales the base limits down when the platform is under stress. The
//! controller takes a system-load signal (0.0–1.0) and an observed
//! response-time, and produces a multiplier in `[min_multiplier, 1.0]`
//! that the limiter applies to every scope's allowance.

use crate::submission_rate_limiting::config::AdaptiveConfig;
use serde::{Deserialize, Serialize};

/// A point-in-time view of platform health used to drive adaptation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SystemHealth {
    /// Normalised system load, where 0.0 is idle and 1.0 is saturated.
    pub load: f64,
    /// Recent average response time in milliseconds.
    pub avg_latency_ms: f64,
}

impl SystemHealth {
    /// Convenience constructor.
    pub fn new(load: f64, avg_latency_ms: f64) -> Self {
        Self {
            load: load.clamp(0.0, 1.0),
            avg_latency_ms: avg_latency_ms.max(0.0),
        }
    }

    /// A nominal "all healthy" reading.
    pub fn healthy() -> Self {
        Self {
            load: 0.0,
            avg_latency_ms: 0.0,
        }
    }
}

/// Computes adaptive multipliers from health readings.
#[derive(Debug, Clone)]
pub struct AdaptiveController {
    config: AdaptiveConfig,
}

impl AdaptiveController {
    /// Creates a controller from configuration.
    pub fn new(config: AdaptiveConfig) -> Self {
        Self { config }
    }

    /// Returns a multiplier in `[min_multiplier, 1.0]` to apply to limits.
    ///
    /// The multiplier degrades linearly past the healthy thresholds: the
    /// worse of the load-based and latency-based pressures wins, so a spike
    /// in either dimension tightens limits.
    pub fn multiplier(&self, health: SystemHealth) -> f64 {
        if !self.config.enabled {
            return 1.0;
        }

        let load_pressure = self.pressure(health.load, self.config.healthy_load, 1.0);
        let latency_pressure = self.pressure(
            health.avg_latency_ms,
            self.config.healthy_latency_ms,
            // Treat 4x the healthy latency as full saturation.
            self.config.healthy_latency_ms * 4.0,
        );

        let pressure = load_pressure.max(latency_pressure);
        let multiplier = 1.0 - pressure * (1.0 - self.config.min_multiplier);
        multiplier.clamp(self.config.min_multiplier, 1.0)
    }

    /// Applies the multiplier to a base limit, never dropping below 1 so a
    /// scope is never accidentally reduced to "deny everything".
    pub fn apply(&self, base_limit: u64, health: SystemHealth) -> u64 {
        let scaled = (base_limit as f64 * self.multiplier(health)).floor() as u64;
        scaled.max(1)
    }

    /// Pressure in `[0.0, 1.0]`: 0 at/below `healthy`, ramping to 1 at `saturated`.
    fn pressure(&self, value: f64, healthy: f64, saturated: f64) -> f64 {
        if value <= healthy || saturated <= healthy {
            0.0
        } else {
            ((value - healthy) / (saturated - healthy)).clamp(0.0, 1.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn controller() -> AdaptiveController {
        AdaptiveController::new(AdaptiveConfig::default())
    }

    #[test]
    fn healthy_system_keeps_full_limits() {
        let c = controller();
        assert_eq!(c.multiplier(SystemHealth::healthy()), 1.0);
        assert_eq!(c.apply(100, SystemHealth::healthy()), 100);
    }

    #[test]
    fn high_load_reduces_limits() {
        let c = controller();
        let m = c.multiplier(SystemHealth::new(1.0, 0.0));
        assert!(m < 1.0);
        assert!(m >= AdaptiveConfig::default().min_multiplier);
        assert!(c.apply(100, SystemHealth::new(1.0, 0.0)) < 100);
    }

    #[test]
    fn high_latency_reduces_limits() {
        let c = controller();
        // 4x healthy latency => full pressure => min multiplier.
        let m = c.multiplier(SystemHealth::new(0.0, 2000.0));
        assert!((m - AdaptiveConfig::default().min_multiplier).abs() < 1e-9);
    }

    #[test]
    fn worst_dimension_dominates() {
        let c = controller();
        let load_only = c.multiplier(SystemHealth::new(0.85, 0.0));
        let both = c.multiplier(SystemHealth::new(0.85, 2000.0));
        assert!(both <= load_only);
    }

    #[test]
    fn disabled_controller_is_a_noop() {
        let cfg = AdaptiveConfig {
            enabled: false,
            ..AdaptiveConfig::default()
        };
        let c = AdaptiveController::new(cfg);
        assert_eq!(c.multiplier(SystemHealth::new(1.0, 9999.0)), 1.0);
    }

    #[test]
    fn apply_never_returns_zero() {
        let c = controller();
        assert!(c.apply(1, SystemHealth::new(1.0, 9999.0)) >= 1);
    }
}
