//! Rate-limit configuration: tiers, per-endpoint policies, and versioned
//! configuration management.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// User tier governing the allowance multiplier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UserTier {
    /// Free tier (baseline).
    Free,
    /// Paid tier.
    Paid,
    /// Enterprise tier.
    Enterprise,
}

impl UserTier {
    /// Allowance multiplier applied to the base per-endpoint limit.
    pub fn multiplier(&self) -> f64 {
        match self {
            UserTier::Free => 1.0,
            UserTier::Paid => 5.0,
            UserTier::Enterprise => 20.0,
        }
    }

    /// Stable label.
    pub fn as_str(&self) -> &'static str {
        match self {
            UserTier::Free => "free",
            UserTier::Paid => "paid",
            UserTier::Enterprise => "enterprise",
        }
    }
}

/// A per-endpoint rate-limit policy (base limit for the Free tier).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointPolicy {
    /// Base requests allowed per window for a Free-tier user.
    pub base_limit: u64,
    /// Window length in seconds.
    pub window_secs: i64,
}

impl EndpointPolicy {
    /// Builds a policy.
    pub fn new(base_limit: u64, window_secs: i64) -> Self {
        Self {
            base_limit,
            window_secs,
        }
    }
}

/// Adaptive-control configuration.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AdaptiveConfig {
    /// Whether adaptive tightening is enabled.
    pub enabled: bool,
    /// Load (0–1) below which limits are not reduced.
    pub healthy_load: f64,
    /// Latency (ms) below which limits are not reduced.
    pub healthy_latency_ms: f64,
    /// Floor multiplier under maximum stress.
    pub min_multiplier: f64,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            healthy_load: 0.7,
            healthy_latency_ms: 500.0,
            min_multiplier: 0.25,
        }
    }
}

/// A complete rate-limit configuration snapshot (one version).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Monotonic config version.
    pub version: u64,
    /// Per-endpoint policies.
    endpoints: HashMap<String, EndpointPolicy>,
    /// Default policy for endpoints without a specific entry.
    pub default_policy: EndpointPolicy,
    /// Adaptive control config.
    pub adaptive: AdaptiveConfig,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            version: 1,
            endpoints: HashMap::new(),
            default_policy: EndpointPolicy::new(100, 60),
            adaptive: AdaptiveConfig::default(),
        }
    }
}

impl RateLimitConfig {
    /// Sets a per-endpoint policy (builder style).
    pub fn with_endpoint(mut self, endpoint: impl Into<String>, policy: EndpointPolicy) -> Self {
        self.endpoints.insert(endpoint.into(), policy);
        self
    }

    /// The policy for an endpoint (falls back to the default).
    pub fn policy_for(&self, endpoint: &str) -> EndpointPolicy {
        self.endpoints
            .get(endpoint)
            .copied()
            .unwrap_or(self.default_policy)
    }

    /// Number of endpoint-specific policies.
    pub fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }
}

/// Manages the active configuration with versioning and rollback history.
#[derive(Debug, Clone)]
pub struct ConfigManager {
    current: RateLimitConfig,
    history: Vec<RateLimitConfig>,
}

impl ConfigManager {
    /// Creates a manager around an initial config.
    pub fn new(initial: RateLimitConfig) -> Self {
        Self {
            current: initial,
            history: Vec::new(),
        }
    }

    /// The active config.
    pub fn current(&self) -> &RateLimitConfig {
        &self.current
    }

    /// Replaces the active config, archiving the old one and bumping the
    /// version. Returns the new version number.
    pub fn update(&mut self, mut new: RateLimitConfig) -> u64 {
        new.version = self.current.version + 1;
        let old = std::mem::replace(&mut self.current, new);
        self.history.push(old);
        self.current.version
    }

    /// Rolls back to a previous version (if present in history), promoting it to
    /// a new version. Returns the new version number, or `None` if not found.
    pub fn rollback(&mut self, version: u64) -> Option<u64> {
        let snapshot = self.history.iter().find(|c| c.version == version)?.clone();
        Some(self.update(snapshot))
    }

    /// Number of archived versions.
    pub fn history_len(&self) -> usize {
        self.history.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_multipliers_increase() {
        assert!(UserTier::Enterprise.multiplier() > UserTier::Paid.multiplier());
        assert!(UserTier::Paid.multiplier() > UserTier::Free.multiplier());
    }

    #[test]
    fn endpoint_policy_lookup_falls_back_to_default() {
        let cfg =
            RateLimitConfig::default().with_endpoint("/api/login", EndpointPolicy::new(5, 60));
        assert_eq!(cfg.policy_for("/api/login").base_limit, 5);
        assert_eq!(cfg.policy_for("/api/unknown").base_limit, 100); // default
    }

    #[test]
    fn config_update_bumps_version_and_archives() {
        let mut mgr = ConfigManager::new(RateLimitConfig::default());
        assert_eq!(mgr.current().version, 1);
        let v =
            mgr.update(RateLimitConfig::default().with_endpoint("/x", EndpointPolicy::new(10, 60)));
        assert_eq!(v, 2);
        assert_eq!(mgr.current().version, 2);
        assert_eq!(mgr.history_len(), 1);
    }

    #[test]
    fn rollback_restores_previous_config() {
        let mut mgr = ConfigManager::new(RateLimitConfig::default()); // v1, no endpoints
        mgr.update(RateLimitConfig::default().with_endpoint("/x", EndpointPolicy::new(10, 60))); // v2
                                                                                                 // Roll back to v1 → becomes v3 with no endpoint-specific policies.
        let v = mgr.rollback(1).unwrap();
        assert_eq!(v, 3);
        assert_eq!(mgr.current().endpoint_count(), 0);
    }

    #[test]
    fn rollback_unknown_version_is_none() {
        let mut mgr = ConfigManager::new(RateLimitConfig::default());
        assert!(mgr.rollback(99).is_none());
    }
}
