//! Configuration for vulnerability-submission rate limiting.
//!
//! Defines the tiered limits required by issue #327:
//! - 10 submissions/hour per user
//! - 100 submissions/hour per IP
//! - 1000 submissions/hour globally
//! - 5 file uploads/hour per user, 50 MB max per upload

use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// Default per-user hourly submission allowance.
pub const DEFAULT_PER_USER_HOURLY: u64 = 10;
/// Default per-IP hourly submission allowance.
pub const DEFAULT_PER_IP_HOURLY: u64 = 100;
/// Default global hourly submission allowance.
pub const DEFAULT_GLOBAL_HOURLY: u64 = 1000;
/// Default per-user hourly file-upload allowance.
pub const DEFAULT_UPLOAD_PER_USER_HOURLY: u64 = 5;
/// Default maximum upload size in bytes (50 MB).
pub const DEFAULT_MAX_UPLOAD_BYTES: u64 = 50 * 1024 * 1024;

/// Caller classification used to select bypass / exemption behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tier {
    /// Unauthenticated caller — subject to IP and global limits only.
    Anonymous,
    /// Authenticated regular user — subject to all limits.
    User,
    /// Verified security researcher — eligible for an elevated/bypass allowance.
    Researcher,
    /// Administrator — eligible for exemption from a trusted IP range.
    Admin,
}

impl Tier {
    /// Human-readable label, also used in headers and logs.
    pub fn as_str(&self) -> &'static str {
        match self {
            Tier::Anonymous => "anonymous",
            Tier::User => "user",
            Tier::Researcher => "researcher",
            Tier::Admin => "admin",
        }
    }
}

/// Hourly submission limits across the three enforcement scopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmissionLimits {
    /// Maximum submissions per authenticated user per hour.
    pub per_user_hourly: u64,
    /// Maximum submissions per source IP per hour.
    pub per_ip_hourly: u64,
    /// Maximum submissions across the whole platform per hour.
    pub global_hourly: u64,
}

impl Default for SubmissionLimits {
    fn default() -> Self {
        Self {
            per_user_hourly: DEFAULT_PER_USER_HOURLY,
            per_ip_hourly: DEFAULT_PER_IP_HOURLY,
            global_hourly: DEFAULT_GLOBAL_HOURLY,
        }
    }
}

/// Limits applied specifically to contract-code file uploads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadLimits {
    /// Maximum uploads per authenticated user per hour.
    pub per_user_hourly: u64,
    /// Maximum size of a single upload, in bytes.
    pub max_bytes: u64,
}

impl Default for UploadLimits {
    fn default() -> Self {
        Self {
            per_user_hourly: DEFAULT_UPLOAD_PER_USER_HOURLY,
            max_bytes: DEFAULT_MAX_UPLOAD_BYTES,
        }
    }
}

/// Parameters controlling adaptive tightening of limits under load.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AdaptiveConfig {
    /// Whether adaptive adjustment is active.
    pub enabled: bool,
    /// Load (0.0–1.0) below which limits are not reduced.
    pub healthy_load: f64,
    /// Response-time (ms) below which limits are not reduced.
    pub healthy_latency_ms: f64,
    /// Smallest multiplier limits may be scaled down to (e.g. 0.25 = 25%).
    pub min_multiplier: f64,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            healthy_load: 0.70,
            healthy_latency_ms: 500.0,
            min_multiplier: 0.25,
        }
    }
}

/// Top-level configuration for the submission rate limiter.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubmissionRateLimitConfig {
    /// Submission scope limits.
    pub submissions: SubmissionLimits,
    /// File-upload limits.
    pub uploads: UploadLimits,
    /// Adaptive controller configuration.
    pub adaptive: AdaptiveConfig,
    /// Trusted IP ranges (CIDR) from which admins are fully exempt.
    pub trusted_admin_ranges: Vec<String>,
    /// Per-tier multipliers applied on top of the base limits.
    /// Researchers, for example, may receive a 10x allowance.
    pub tier_multipliers: TierMultipliers,
}

impl SubmissionRateLimitConfig {
    /// Returns true if `ip` falls inside any configured trusted admin range.
    pub fn is_trusted_admin_ip(&self, ip: IpAddr) -> bool {
        self.trusted_admin_ranges
            .iter()
            .any(|cidr| crate::submission_rate_limiting::bypass::cidr_contains(cidr, ip))
    }
}

/// Multipliers applied to the base limits for each tier.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TierMultipliers {
    /// Multiplier for anonymous callers.
    pub anonymous: f64,
    /// Multiplier for regular users.
    pub user: f64,
    /// Multiplier for verified researchers.
    pub researcher: f64,
    /// Multiplier for admins (when not fully exempt).
    pub admin: f64,
}

impl Default for TierMultipliers {
    fn default() -> Self {
        Self {
            anonymous: 1.0,
            user: 1.0,
            researcher: 10.0,
            admin: 50.0,
        }
    }
}

impl TierMultipliers {
    /// Returns the multiplier for the given tier.
    pub fn for_tier(&self, tier: Tier) -> f64 {
        match tier {
            Tier::Anonymous => self.anonymous,
            Tier::User => self.user,
            Tier::Researcher => self.researcher,
            Tier::Admin => self.admin,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_acceptance_criteria() {
        let cfg = SubmissionRateLimitConfig::default();
        assert_eq!(cfg.submissions.per_user_hourly, 10);
        assert_eq!(cfg.submissions.per_ip_hourly, 100);
        assert_eq!(cfg.submissions.global_hourly, 1000);
        assert_eq!(cfg.uploads.per_user_hourly, 5);
        assert_eq!(cfg.uploads.max_bytes, 50 * 1024 * 1024);
    }

    #[test]
    fn tier_multiplier_lookup() {
        let m = TierMultipliers::default();
        assert_eq!(m.for_tier(Tier::User), 1.0);
        assert!(m.for_tier(Tier::Researcher) > m.for_tier(Tier::User));
        assert!(m.for_tier(Tier::Admin) > m.for_tier(Tier::Researcher));
    }

    #[test]
    fn trusted_admin_range_detection() {
        let mut cfg = SubmissionRateLimitConfig::default();
        cfg.trusted_admin_ranges.push("10.0.0.0/8".to_string());
        assert!(cfg.is_trusted_admin_ip("10.4.5.6".parse().unwrap()));
        assert!(!cfg.is_trusted_admin_ip("192.168.1.1".parse().unwrap()));
    }
}
