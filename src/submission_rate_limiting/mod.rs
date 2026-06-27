//! Tiered rate limiting for vulnerability-submission and file-upload endpoints.
//!
//! Implements issue #327. The limiter enforces three submission scopes and a
//! dedicated upload scope, scales limits with system load, exempts trusted
//! callers, and feeds a monitoring/alerting surface — all behind a single
//! [`SubmissionRateLimiter::check`] call.
//!
//! # Acceptance criteria mapping
//!
//! | Requirement | Where |
//! |-------------|-------|
//! | Tiered limits (10/user·h, 100/IP·h, 1000/global·h) | [`config::SubmissionLimits`], [`limiter`] |
//! | File-upload limits (5/user·h, 50 MB max) | [`config::UploadLimits`], [`limiter`] |
//! | Adaptive limiting by load & latency | [`adaptive::AdaptiveController`] |
//! | Rate-limit headers (`X-RateLimit-*`, `Retry-After`) | [`headers::RateLimitHeaders`] |
//! | Researcher bypass | [`bypass::ResearcherRegistry`] |
//! | Admin exemption from trusted IP ranges | [`bypass::cidr_contains`], [`config::SubmissionRateLimitConfig::is_trusted_admin_ip`] |
//! | Distributed enforcement (Redis-ready) | [`store::RateLimitStore`] trait |
//! | CAPTCHA challenge for suspicious patterns | [`challenge::ChallengePolicy`] |
//! | Monitoring dashboard data & alerts | [`monitoring::Monitor`] |
//!
//! # Example
//!
//! ```
//! use soroban_security_scanner::submission_rate_limiting::*;
//! use uuid::Uuid;
//!
//! let mut limiter = SubmissionRateLimiter::new(SubmissionRateLimitConfig::default());
//! limiter.researchers_mut().authorize_full_bypass("trusted-researcher-token");
//!
//! let req = SubmissionRequest::submission(
//!     Tier::User,
//!     Some(Uuid::new_v4()),
//!     "203.0.113.10".parse().unwrap(),
//!     "/api/v1/vulnerabilities",
//! );
//!
//! match limiter.check(&req) {
//!     Decision::Allowed { headers, .. } => {
//!         assert_eq!(headers.scope, "user");
//!     }
//!     other => panic!("unexpected: {other:?}"),
//! }
//! ```

pub mod adaptive;
pub mod bypass;
pub mod challenge;
pub mod config;
pub mod headers;
pub mod limiter;
pub mod monitoring;
pub mod store;

#[cfg(test)]
mod tests;

pub use adaptive::{AdaptiveController, SystemHealth};
pub use bypass::{cidr_contains, ResearcherRegistry};
pub use challenge::{Challenge, ChallengeKind, ChallengePolicy};
pub use config::{
    AdaptiveConfig, SubmissionLimits, SubmissionRateLimitConfig, Tier, TierMultipliers, UploadLimits,
};
pub use headers::RateLimitHeaders;
pub use limiter::{Decision, SubmissionRateLimiter, SubmissionRequest};
pub use monitoring::{Alert, Monitor, RateLimitStats, Violation};
pub use store::{InMemoryStore, RateLimitStore};
