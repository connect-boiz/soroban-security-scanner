//! Comprehensive API rate limiting and throttling (issue #345).
//!
//! A self-contained, distributed-ready rate limiter with per-endpoint policies,
//! tier-based allowances (free/paid/enterprise), adaptive tightening under
//! load, verified-user bypass, CAPTCHA challenges for suspicious traffic,
//! versioned configuration management, usage analytics and threshold alerting.
//!
//! # Acceptance-criteria mapping
//!
//! | Requirement | Where |
//! |-------------|-------|
//! | Per-endpoint rate limiting | [`config::EndpointPolicy`], [`limiter::ApiRateLimiter`] |
//! | Tier-based allowances (free/paid/enterprise) | [`config::UserTier`] |
//! | Adaptive limiting by load & latency | [`adaptive::AdaptiveController`] |
//! | Bypass for verified users | [`bypass::BypassRegistry`] |
//! | Rate-limit analytics dashboard | [`analytics::Analytics`] |
//! | Config management with versioning | [`config::ConfigManager`] |
//! | Threshold-breach alerting | [`alerting::ThrottleAlerter`] |
//! | CAPTCHA challenge for suspicious patterns | [`challenge::ChallengePolicy`] |
//! | Distributed rate limiting (Redis-ready) | [`store::RateLimitStore`] trait |
//! | High enforcement accuracy | sliding-window log + atomic critical section |
//! | Comprehensive testing | per-module tests + [`tests`] |
//!
//! # Example
//!
//! ```
//! use soroban_security_scanner::api_rate_limiting::*;
//!
//! let config = RateLimitConfig::default().with_endpoint("/api/login", EndpointPolicy::new(5, 60));
//! let limiter = ApiRateLimiter::new(config);
//! let req = ApiRequest::new("/api/login", UserTier::Free, "203.0.113.1").with_user("alice");
//! assert!(limiter.check(&req).is_allowed());
//! ```

pub mod adaptive;
pub mod alerting;
pub mod analytics;
pub mod bypass;
pub mod challenge;
pub mod config;
pub mod limiter;
pub mod store;

#[cfg(test)]
mod tests;

pub use adaptive::{AdaptiveController, SystemHealth};
pub use alerting::{AlertConfig, ThrottleAlert, ThrottleAlerter};
pub use analytics::{Analytics, AnalyticsSnapshot, Outcome, OutcomeCounts};
pub use bypass::BypassRegistry;
pub use challenge::{Challenge, ChallengePolicy};
pub use config::{AdaptiveConfig, ConfigManager, EndpointPolicy, RateLimitConfig, UserTier};
pub use limiter::{ApiRateLimiter, ApiRequest, Decision, RateLimitHeaders};
pub use store::{InMemoryStore, RateLimitStore};
