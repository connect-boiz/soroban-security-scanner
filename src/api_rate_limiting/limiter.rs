//! The API rate limiter.
//!
//! Ties together per-endpoint policies, tier-based allowances, adaptive
//! tightening, verified-user bypass, CAPTCHA challenges, analytics and
//! threshold alerting into a single `check` call. Limits are enforced per
//! (endpoint, user) and per (endpoint, IP) with an accurate sliding window; a
//! gate serializes the count→record section so concurrent callers cannot
//! over-admit.

use crate::api_rate_limiting::adaptive::{AdaptiveController, SystemHealth};
use crate::api_rate_limiting::alerting::{AlertConfig, ThrottleAlert, ThrottleAlerter};
use crate::api_rate_limiting::analytics::{Analytics, Outcome};
use crate::api_rate_limiting::bypass::BypassRegistry;
use crate::api_rate_limiting::challenge::{Challenge, ChallengePolicy};
use crate::api_rate_limiting::config::{RateLimitConfig, UserTier};
use crate::api_rate_limiting::store::{InMemoryStore, RateLimitStore};
use chrono::{DateTime, Duration, Utc};
use std::sync::Mutex;
use uuid::Uuid;

/// An incoming API request to evaluate.
#[derive(Debug, Clone)]
pub struct ApiRequest {
    /// Correlation id (used for challenges).
    pub id: Uuid,
    /// Endpoint path / route key.
    pub endpoint: String,
    /// Authenticated user id, if any.
    pub user_id: Option<String>,
    /// Source IP.
    pub ip: String,
    /// User tier.
    pub tier: UserTier,
    /// Optional verified-user bypass token.
    pub bypass_token: Option<String>,
}

impl ApiRequest {
    /// Builds a request with a fresh id.
    pub fn new(endpoint: impl Into<String>, tier: UserTier, ip: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            endpoint: endpoint.into(),
            user_id: None,
            ip: ip.into(),
            tier,
            bypass_token: None,
        }
    }

    /// Sets the user id.
    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Sets the bypass token.
    pub fn with_bypass(mut self, token: impl Into<String>) -> Self {
        self.bypass_token = Some(token.into());
        self
    }

    /// The user identity for keying/analytics (user id, else IP).
    fn identity(&self) -> String {
        self.user_id
            .clone()
            .unwrap_or_else(|| format!("ip:{}", self.ip))
    }
}

/// Rate-limit headers for a response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitHeaders {
    /// Effective limit.
    pub limit: u64,
    /// Remaining in the window.
    pub remaining: u64,
    /// Unix timestamp the window resets.
    pub reset: i64,
    /// Seconds to wait before retrying (when throttled).
    pub retry_after: Option<i64>,
}

/// The decision for a request.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// Allowed; carries headers and an optional CAPTCHA challenge.
    Allowed {
        /// Rate-limit headers.
        headers: RateLimitHeaders,
        /// Challenge to solve, if the caller looks suspicious.
        challenge: Option<Challenge>,
    },
    /// Bypassed (verified user).
    Bypassed,
    /// Throttled; carries headers (with `Retry-After`) and the exceeded scope.
    Throttled {
        /// Scope that was exceeded ("user" or "ip").
        scope: String,
        /// Rate-limit headers.
        headers: RateLimitHeaders,
    },
}

impl Decision {
    /// Whether the request may proceed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, Decision::Allowed { .. } | Decision::Bypassed)
    }
}

struct Scope {
    name: &'static str,
    key: String,
    count: u64,
}

/// The API rate limiter.
pub struct ApiRateLimiter {
    config: RateLimitConfig,
    store: Box<dyn RateLimitStore>,
    adaptive: AdaptiveController,
    challenge: ChallengePolicy,
    bypass: BypassRegistry,
    analytics: Mutex<Analytics>,
    alerter: Mutex<ThrottleAlerter>,
    alerts: Mutex<Vec<ThrottleAlert>>,
    gate: Mutex<()>,
}

impl ApiRateLimiter {
    /// Creates a limiter with the in-memory store.
    pub fn new(config: RateLimitConfig) -> Self {
        Self::with_store(config, Box::new(InMemoryStore::new()))
    }

    /// Creates a limiter with a custom (e.g. Redis-backed) store.
    pub fn with_store(config: RateLimitConfig, store: Box<dyn RateLimitStore>) -> Self {
        let adaptive = AdaptiveController::new(config.adaptive);
        Self {
            config,
            store,
            adaptive,
            challenge: ChallengePolicy::default(),
            bypass: BypassRegistry::new(),
            analytics: Mutex::new(Analytics::new()),
            alerter: Mutex::new(ThrottleAlerter::new(AlertConfig::default())),
            alerts: Mutex::new(Vec::new()),
            gate: Mutex::new(()),
        }
    }

    /// Mutable access to the bypass registry.
    pub fn bypass_mut(&mut self) -> &mut BypassRegistry {
        &mut self.bypass
    }

    /// Overrides the challenge policy.
    pub fn set_challenge_policy(&mut self, policy: ChallengePolicy) {
        self.challenge = policy;
    }

    /// An analytics dashboard snapshot.
    pub fn analytics_snapshot(
        &self,
        top_n: usize,
    ) -> crate::api_rate_limiting::analytics::AnalyticsSnapshot {
        self.analytics
            .lock()
            .expect("analytics poisoned")
            .snapshot(top_n)
    }

    /// Evaluates a request at the current time, healthy system.
    pub fn check(&self, req: &ApiRequest) -> Decision {
        self.check_at(req, Utc::now(), SystemHealth::healthy())
    }

    /// Evaluates a request at an explicit time + health reading.
    pub fn check_at(&self, req: &ApiRequest, now: DateTime<Utc>, health: SystemHealth) -> Decision {
        // 1. Bypass.
        if let Some(token) = &req.bypass_token {
            if self.bypass.is_authorized(token) {
                self.record_outcome(req, Outcome::Bypassed, false);
                return Decision::Bypassed;
            }
        }

        let policy = self.config.policy_for(&req.endpoint);
        let window = Duration::seconds(policy.window_secs);
        // Effective limit: base * tier multiplier, then adaptive scaling.
        let tiered = ((policy.base_limit as f64) * req.tier.multiplier()).round() as u64;
        let limit = self.adaptive.apply(tiered.max(1), health);

        let identity = req.identity();
        let user_key = format!("u:{}:{}", req.endpoint, identity);
        let ip_key = format!("i:{}:{}", req.endpoint, req.ip);

        // Hold the gate across count→record so concurrent callers can't both
        // observe a sub-limit count and over-admit.
        let _gate = self.gate.lock().expect("gate poisoned");

        let user_count = self.store.count(&user_key, now, window);
        let ip_count = self.store.count(&ip_key, now, window);
        let scopes = [
            Scope {
                name: "user",
                key: user_key,
                count: user_count,
            },
            Scope {
                name: "ip",
                key: ip_key,
                count: ip_count,
            },
        ];

        if let Some(scope) = scopes.iter().find(|s| s.count >= limit) {
            let retry_after = self.retry_after(&scope.key, now, window);
            let headers = RateLimitHeaders {
                limit,
                remaining: 0,
                reset: now.timestamp() + retry_after,
                retry_after: Some(retry_after),
            };
            drop(_gate);
            self.record_outcome(req, Outcome::Throttled, true);
            return Decision::Throttled {
                scope: scope.name.to_string(),
                headers,
            };
        }

        // Allowed: record in all scopes.
        for scope in &scopes {
            self.store.record(&scope.key, now);
        }
        let min_remaining = scopes
            .iter()
            .map(|s| limit.saturating_sub(s.count + 1))
            .min()
            .unwrap_or(limit);
        let headers = RateLimitHeaders {
            limit,
            remaining: min_remaining,
            reset: now.timestamp() + window.num_seconds(),
            retry_after: None,
        };

        // Challenge if the caller's usage looks suspicious.
        let challenge = if scopes
            .iter()
            .any(|s| self.challenge.is_suspicious(s.count + 1, limit))
        {
            Some(self.challenge.issue(req.id))
        } else {
            None
        };
        drop(_gate);

        let outcome = if challenge.is_some() {
            Outcome::Challenged
        } else {
            Outcome::Allowed
        };
        self.record_outcome(req, outcome, false);
        Decision::Allowed { headers, challenge }
    }

    fn retry_after(&self, key: &str, now: DateTime<Utc>, window: Duration) -> i64 {
        match self.store.earliest(key, now, window) {
            Some(earliest) => ((earliest + window) - now).num_seconds().max(0),
            None => 0,
        }
    }

    fn record_outcome(&self, req: &ApiRequest, outcome: Outcome, throttled: bool) {
        self.analytics.lock().expect("analytics poisoned").record(
            &req.endpoint,
            &req.identity(),
            outcome,
        );
        if let Some(alert) = self
            .alerter
            .lock()
            .expect("alerter poisoned")
            .observe(throttled)
        {
            self.alerts.lock().expect("alerts poisoned").push(alert);
        }
    }

    /// Threshold-breach alerts raised so far.
    pub fn alerts(&self) -> Vec<ThrottleAlert> {
        self.alerts.lock().expect("alerts poisoned").clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_rate_limiting::config::EndpointPolicy;

    fn limiter() -> ApiRateLimiter {
        let config =
            RateLimitConfig::default().with_endpoint("/api/login", EndpointPolicy::new(3, 60));
        ApiRateLimiter::new(config)
    }

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    #[test]
    fn per_endpoint_limit_enforced() {
        let l = limiter();
        let req = ApiRequest::new("/api/login", UserTier::Free, "1.1.1.1").with_user("alice");
        for _ in 0..3 {
            assert!(l
                .check_at(&req, now(), SystemHealth::healthy())
                .is_allowed());
        }
        match l.check_at(&req, now(), SystemHealth::healthy()) {
            Decision::Throttled { scope, .. } => assert_eq!(scope, "user"),
            other => panic!("expected throttle, got {other:?}"),
        }
    }

    #[test]
    fn paid_tier_gets_higher_allowance() {
        let l = limiter();
        // Free base 3 * 5 (paid) = 15 allowed on /api/login.
        let req = ApiRequest::new("/api/login", UserTier::Paid, "2.2.2.2").with_user("bob");
        for i in 0..15 {
            assert!(
                l.check_at(&req, now(), SystemHealth::healthy())
                    .is_allowed(),
                "req {i}"
            );
        }
        assert!(!l
            .check_at(&req, now(), SystemHealth::healthy())
            .is_allowed());
    }

    #[test]
    fn default_policy_used_for_unconfigured_endpoint() {
        let l = limiter();
        // Default base limit is 100 for Free.
        let req = ApiRequest::new("/api/other", UserTier::Free, "3.3.3.3").with_user("c");
        for _ in 0..100 {
            assert!(l
                .check_at(&req, now(), SystemHealth::healthy())
                .is_allowed());
        }
        assert!(!l
            .check_at(&req, now(), SystemHealth::healthy())
            .is_allowed());
    }

    #[test]
    fn verified_user_bypasses() {
        let mut l = limiter();
        l.bypass_mut().authorize("vip");
        let req = ApiRequest::new("/api/login", UserTier::Free, "4.4.4.4")
            .with_user("d")
            .with_bypass("vip");
        for _ in 0..50 {
            assert_eq!(
                l.check_at(&req, now(), SystemHealth::healthy()),
                Decision::Bypassed
            );
        }
    }

    #[test]
    fn adaptive_load_tightens_limit() {
        let l = limiter();
        let req = ApiRequest::new("/api/login", UserTier::Free, "5.5.5.5").with_user("e");
        // Base 3 → under full load scaled to min 25% → floor(3*0.25)=0 → max(1)=1.
        let stressed = SystemHealth::new(1.0, 0.0);
        assert!(l.check_at(&req, now(), stressed).is_allowed());
        assert!(!l.check_at(&req, now(), stressed).is_allowed());
    }

    #[test]
    fn ip_scope_throttles_across_users() {
        let l = ApiRateLimiter::new(
            RateLimitConfig::default().with_endpoint("/api/x", EndpointPolicy::new(2, 60)),
        );
        // Same IP, different users (free tier base 2 each, but IP scope also 2).
        let a = ApiRequest::new("/api/x", UserTier::Free, "9.9.9.9").with_user("u1");
        let b = ApiRequest::new("/api/x", UserTier::Free, "9.9.9.9").with_user("u2");
        assert!(l.check_at(&a, now(), SystemHealth::healthy()).is_allowed());
        assert!(l.check_at(&b, now(), SystemHealth::healthy()).is_allowed());
        let c = ApiRequest::new("/api/x", UserTier::Free, "9.9.9.9").with_user("u3");
        match l.check_at(&c, now(), SystemHealth::healthy()) {
            Decision::Throttled { scope, .. } => assert_eq!(scope, "ip"),
            other => panic!("expected ip throttle, got {other:?}"),
        }
    }

    #[test]
    fn analytics_track_decisions() {
        let l = limiter();
        let req = ApiRequest::new("/api/login", UserTier::Free, "6.6.6.6").with_user("f");
        for _ in 0..5 {
            l.check_at(&req, now(), SystemHealth::healthy());
        }
        let snap = l.analytics_snapshot(5);
        assert_eq!(snap.total_requests, 5);
        assert_eq!(snap.total_throttled, 2); // 3 allowed, 2 throttled
    }
}
