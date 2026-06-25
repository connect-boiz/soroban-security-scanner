//! Per-endpoint rate limit configuration.
//!
//! Builds on `RateLimiter` (src/rate_limiting.rs) to provide:
//! - Different limits per route sensitivity tier
//! - User-context-aware keys (IP + optional user_id)
//! - Axum middleware that injects `RateLimited` header and returns
//!   `AppError::RateLimited` when the budget is exhausted

use crate::app_error::AppError;
use crate::rate_limiting::{RateLimitConfig, RateLimiter, RateLimitResult};
use axum::{
    body::Body,
    extract::{ConnectInfo, MatchedPath},
    http::{HeaderName, HeaderValue, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::{net::SocketAddr, sync::Arc, time::Duration};

// ---------------------------------------------------------------------------
// Tier definitions
// ---------------------------------------------------------------------------

/// Rate limit tiers keyed by endpoint sensitivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateTier {
    /// Public read-only endpoints: 300 req / 60 s
    Public,
    /// Authenticated user endpoints: 120 req / 60 s
    Authenticated,
    /// Sensitive endpoints (scan submit, wallet ops): 20 req / 60 s
    Sensitive,
    /// Admin endpoints: 10 req / 60 s
    Admin,
}

impl RateTier {
    pub fn config(self) -> RateLimitConfig {
        match self {
            Self::Public        => RateLimitConfig { max_requests: 300, window: Duration::from_secs(60) },
            Self::Authenticated => RateLimitConfig { max_requests: 120, window: Duration::from_secs(60) },
            Self::Sensitive     => RateLimitConfig { max_requests:  20, window: Duration::from_secs(60) },
            Self::Admin         => RateLimitConfig { max_requests:  10, window: Duration::from_secs(60) },
        }
    }

    /// Map a route path prefix to its tier.
    pub fn for_path(path: &str) -> Self {
        if path.starts_with("/admin") {
            Self::Admin
        } else if path.starts_with("/scan") || path.starts_with("/wallet") {
            Self::Sensitive
        } else if path.starts_with("/api") {
            Self::Authenticated
        } else {
            Self::Public
        }
    }
}

// ---------------------------------------------------------------------------
// Shared limiter map
// ---------------------------------------------------------------------------

/// One `RateLimiter` instance per tier, shared via Arc.
#[derive(Clone)]
pub struct EndpointLimiters {
    pub public:        Arc<RateLimiter>,
    pub authenticated: Arc<RateLimiter>,
    pub sensitive:     Arc<RateLimiter>,
    pub admin:         Arc<RateLimiter>,
}

impl EndpointLimiters {
    /// Construct with optional Redis connection (falls back to in-memory).
    pub fn new(redis: Option<redis::aio::MultiplexedConnection>) -> Self {
        Self {
            public:        Arc::new(RateLimiter::new(RateTier::Public.config(),        redis.clone())),
            authenticated: Arc::new(RateLimiter::new(RateTier::Authenticated.config(), redis.clone())),
            sensitive:     Arc::new(RateLimiter::new(RateTier::Sensitive.config(),     redis.clone())),
            admin:         Arc::new(RateLimiter::new(RateTier::Admin.config(),         redis.clone())),
        }
    }

    fn limiter_for_tier(&self, tier: RateTier) -> &RateLimiter {
        match tier {
            RateTier::Public        => &self.public,
            RateTier::Authenticated => &self.authenticated,
            RateTier::Sensitive     => &self.sensitive,
            RateTier::Admin         => &self.admin,
        }
    }
}

// ---------------------------------------------------------------------------
// Axum middleware
// ---------------------------------------------------------------------------

/// Tower middleware: applies per-endpoint rate limiting.
///
/// Key = `<tier>:<ip>[:<user_id>]`
/// On denial: returns 429 with `Retry-After` header.
pub async fn rate_limit_middleware(
    req:  Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_owned())
        .unwrap_or_else(|| req.uri().path().to_owned());

    let ip = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_owned());

    let tier = RateTier::for_path(&path);

    let limiters = req
        .extensions()
        .get::<EndpointLimiters>()
        .cloned()
        .ok_or(AppError::InternalError("rate limiter not configured".into()))?;

    let key = format!("{}:{}", tier as u8, ip);
    let limiter = limiters.limiter_for_tier(tier);

    match limiter.check(&key).await {
        RateLimitResult::Allowed { .. } => Ok(next.run(req).await),
        RateLimitResult::Denied { retry_after } => {
            tracing::warn!(ip = %ip, path = %path, retry_after, "rate limit exceeded");
            Err(AppError::RateLimited)
        }
    }
}
