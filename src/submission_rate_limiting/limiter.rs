//! The core submission rate limiter.
//!
//! Ties together tiered limits, adaptive scaling, bypass/exemption,
//! challenge issuance and monitoring into a single [`SubmissionRateLimiter::check`]
//! entry point used by the vulnerability-submission and file-upload endpoints.

use crate::submission_rate_limiting::adaptive::{AdaptiveController, SystemHealth};
use crate::submission_rate_limiting::bypass::ResearcherRegistry;
use crate::submission_rate_limiting::challenge::{Challenge, ChallengePolicy};
use crate::submission_rate_limiting::config::{SubmissionRateLimitConfig, Tier};
use crate::submission_rate_limiting::headers::RateLimitHeaders;
use crate::submission_rate_limiting::monitoring::{Monitor, Violation};
use crate::submission_rate_limiting::store::{InMemoryStore, RateLimitStore};
use chrono::{DateTime, Duration, Utc};
use std::net::IpAddr;
use std::sync::Mutex;
use uuid::Uuid;

/// The enforcement window for all hourly limits.
fn window() -> Duration {
    Duration::hours(1)
}

/// A request to be evaluated by the limiter.
#[derive(Debug, Clone)]
pub struct SubmissionRequest {
    /// Unique id for correlation (used when issuing challenges).
    pub id: Uuid,
    /// Caller tier.
    pub tier: Tier,
    /// Authenticated user id, if any.
    pub user_id: Option<Uuid>,
    /// Source IP address.
    pub ip: IpAddr,
    /// Target endpoint path (for logging/monitoring).
    pub endpoint: String,
    /// Optional researcher authorization token.
    pub researcher_token: Option<String>,
    /// Size of an uploaded file in bytes, if this is an upload request.
    pub upload_bytes: Option<u64>,
}

impl SubmissionRequest {
    /// Builds a plain submission request (no upload) with a fresh id.
    pub fn submission(
        tier: Tier,
        user_id: Option<Uuid>,
        ip: IpAddr,
        endpoint: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            tier,
            user_id,
            ip,
            endpoint: endpoint.into(),
            researcher_token: None,
            upload_bytes: None,
        }
    }

    /// Builds a file-upload request with a fresh id.
    pub fn upload(
        tier: Tier,
        user_id: Option<Uuid>,
        ip: IpAddr,
        endpoint: impl Into<String>,
        bytes: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            tier,
            user_id,
            ip,
            endpoint: endpoint.into(),
            researcher_token: None,
            upload_bytes: Some(bytes),
        }
    }

    /// Attaches a researcher authorization token.
    pub fn with_researcher_token(mut self, token: impl Into<String>) -> Self {
        self.researcher_token = Some(token.into());
        self
    }
}

/// The outcome of a rate-limit check.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// Request may proceed; `challenge` is set when the caller looks
    /// suspicious and must solve a CAPTCHA before the request is honoured.
    Allowed {
        /// Headers describing the most-constraining scope.
        headers: RateLimitHeaders,
        /// Optional CAPTCHA challenge.
        challenge: Option<Challenge>,
    },
    /// Request bypassed enforcement (full-bypass researcher or exempt admin).
    Bypassed {
        /// Why enforcement was skipped.
        reason: String,
    },
    /// Request is rejected.
    Blocked {
        /// The scope that was exceeded.
        scope: String,
        /// Human-readable reason.
        reason: String,
        /// Headers, including `Retry-After`.
        headers: RateLimitHeaders,
    },
}

impl Decision {
    /// Convenience: whether the request is permitted to proceed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, Decision::Allowed { .. } | Decision::Bypassed { .. })
    }
}

/// Internal per-scope evaluation.
struct ScopeState {
    name: &'static str,
    key: String,
    limit: u64,
    count: u64,
}

impl ScopeState {
    fn remaining(&self) -> u64 {
        self.limit.saturating_sub(self.count)
    }

    fn exceeded(&self) -> bool {
        self.count >= self.limit
    }
}

/// Tiered, adaptive, distributed-ready rate limiter for submissions and uploads.
pub struct SubmissionRateLimiter {
    config: SubmissionRateLimitConfig,
    store: Box<dyn RateLimitStore>,
    adaptive: AdaptiveController,
    challenge_policy: ChallengePolicy,
    researchers: ResearcherRegistry,
    monitor: Monitor,
    /// Serializes the count→record critical section so concurrent callers
    /// against the same store can never both observe sub-limit counts and
    /// over-admit (avoids the check-then-act race). A distributed store
    /// enforces the same invariant server-side (e.g. a Redis Lua script).
    gate: Mutex<()>,
}

impl SubmissionRateLimiter {
    /// Creates a limiter with the default in-memory store.
    pub fn new(config: SubmissionRateLimitConfig) -> Self {
        Self::with_store(config, Box::new(InMemoryStore::new()))
    }

    /// Creates a limiter backed by a custom store (e.g. a Redis-backed,
    /// distributed implementation of [`RateLimitStore`]).
    pub fn with_store(config: SubmissionRateLimitConfig, store: Box<dyn RateLimitStore>) -> Self {
        let adaptive = AdaptiveController::new(config.adaptive);
        Self {
            config,
            store,
            adaptive,
            challenge_policy: ChallengePolicy::default(),
            researchers: ResearcherRegistry::new(),
            monitor: Monitor::default(),
            gate: Mutex::new(()),
        }
    }

    /// Mutable access to the researcher registry for authorizing tokens.
    pub fn researchers_mut(&mut self) -> &mut ResearcherRegistry {
        &mut self.researchers
    }

    /// Overrides the challenge policy.
    pub fn set_challenge_policy(&mut self, policy: ChallengePolicy) {
        self.challenge_policy = policy;
    }

    /// Read-only access to the monitor (stats, violations, alerts).
    pub fn monitor(&self) -> &Monitor {
        &self.monitor
    }

    /// Evaluates a request at the current time, assuming a healthy system.
    pub fn check(&self, req: &SubmissionRequest) -> Decision {
        self.check_at(req, Utc::now(), SystemHealth::healthy())
    }

    /// Evaluates a request at an explicit time and system-health reading.
    ///
    /// Exposing `now` and `health` keeps the limiter fully deterministic and
    /// testable, and lets callers feed real load metrics into the adaptive
    /// controller.
    pub fn check_at(
        &self,
        req: &SubmissionRequest,
        now: DateTime<Utc>,
        health: SystemHealth,
    ) -> Decision {
        // 1. Full bypass for verified researchers holding a bypass token.
        if let Some(token) = &req.researcher_token {
            if self.researchers.grants_full_bypass(token) {
                self.monitor.record_bypassed();
                return Decision::Bypassed {
                    reason: "verified security researcher (full bypass)".to_string(),
                };
            }
        }

        // 2. Admin exemption from trusted IP ranges.
        if req.tier == Tier::Admin && self.config.is_trusted_admin_ip(req.ip) {
            self.monitor.record_bypassed();
            return Decision::Bypassed {
                reason: "admin from trusted IP range".to_string(),
            };
        }

        // Researchers with a (non-bypass) authorized token get the elevated tier.
        let effective_tier = match &req.researcher_token {
            Some(t) if self.researchers.is_authorized(t) => Tier::Researcher,
            _ => req.tier,
        };
        let tier_mult = self.config.tier_multipliers.for_tier(effective_tier);

        // 3. Upload size hard cap (independent of windows).
        if let Some(bytes) = req.upload_bytes {
            if bytes > self.config.uploads.max_bytes {
                let reason = format!(
                    "upload of {} bytes exceeds maximum of {} bytes",
                    bytes, self.config.uploads.max_bytes
                );
                let headers = RateLimitHeaders::blocked(
                    self.config.uploads.max_bytes,
                    now.timestamp(),
                    0,
                    "upload_size",
                );
                self.record_violation(req, "upload_size", &reason, now);
                return Decision::Blocked {
                    scope: "upload_size".to_string(),
                    reason,
                    headers,
                };
            }
        }

        // Hold the gate across counting and recording so concurrent callers
        // cannot both read a sub-limit count and then both record.
        let _gate = self.gate.lock().expect("rate-limit gate poisoned");

        // 4. Build the list of windowed scopes to enforce.
        let scopes = self.build_scopes(req, effective_tier, tier_mult, health, now);

        // 5. Reject on the first (most specific) exceeded scope.
        if let Some(scope) = scopes.iter().find(|s| s.exceeded()) {
            let retry_after = self.retry_after(&scope.key, now);
            let reset = now.timestamp() + retry_after;
            let reason = format!("{} hourly limit of {} exceeded", scope.name, scope.limit);
            let headers = RateLimitHeaders::blocked(scope.limit, reset, retry_after, scope.name);
            self.record_violation(req, scope.name, &reason, now);
            return Decision::Blocked {
                scope: scope.name.to_string(),
                reason,
                headers,
            };
        }

        // 6. Allowed — record the event in every relevant scope.
        for scope in &scopes {
            self.store.record(&scope.key, now);
        }

        // 7. Headers reflect the most-constraining scope (smallest remaining
        //    *after* recording this request).
        let headers = self.constraining_headers(&scopes, now);

        // 8. Issue a challenge if the caller's usage looks suspicious.
        let challenge = self.maybe_challenge(req, &scopes);
        if challenge.is_some() {
            self.monitor.record_challenged();
        } else {
            self.monitor.record_allowed();
        }

        Decision::Allowed { headers, challenge }
    }

    /// Constructs the ordered scope checks for a request (most specific first).
    fn build_scopes(
        &self,
        req: &SubmissionRequest,
        _tier: Tier,
        tier_mult: f64,
        health: SystemHealth,
        now: DateTime<Utc>,
    ) -> Vec<ScopeState> {
        let mut scopes = Vec::new();

        if req.upload_bytes.is_some() {
            // Upload-specific per-user (or per-IP when anonymous) limit.
            let limit =
                self.effective_limit(self.config.uploads.per_user_hourly, tier_mult, health);
            let key = match req.user_id {
                Some(uid) => format!("up:user:{uid}"),
                None => format!("up:ip:{}", req.ip),
            };
            let count = self.store.count(&key, now, window());
            scopes.push(ScopeState {
                name: "upload",
                key,
                limit,
                count,
            });
        } else if let Some(uid) = req.user_id {
            // Per-user submission limit.
            let limit =
                self.effective_limit(self.config.submissions.per_user_hourly, tier_mult, health);
            let key = format!("sub:user:{uid}");
            let count = self.store.count(&key, now, window());
            scopes.push(ScopeState {
                name: "user",
                key,
                limit,
                count,
            });
        }

        // Per-IP limit (applies to everyone).
        let ip_limit =
            self.effective_limit(self.config.submissions.per_ip_hourly, tier_mult, health);
        let ip_key = format!("sub:ip:{}", req.ip);
        let ip_count = self.store.count(&ip_key, now, window());
        scopes.push(ScopeState {
            name: "ip",
            key: ip_key,
            limit: ip_limit,
            count: ip_count,
        });

        // Global limit (adaptive only — tier multiplier does not relax the
        // platform-wide ceiling).
        let global_limit = self
            .adaptive
            .apply(self.config.submissions.global_hourly, health);
        let global_key = "sub:global".to_string();
        let global_count = self.store.count(&global_key, now, window());
        scopes.push(ScopeState {
            name: "global",
            key: global_key,
            limit: global_limit,
            count: global_count,
        });

        scopes
    }

    /// Applies tier and adaptive multipliers to a base limit (never below 1).
    fn effective_limit(&self, base: u64, tier_mult: f64, health: SystemHealth) -> u64 {
        let tiered = ((base as f64) * tier_mult).round() as u64;
        self.adaptive.apply(tiered.max(1), health)
    }

    /// Seconds until the given scope's window frees a slot.
    fn retry_after(&self, key: &str, now: DateTime<Utc>) -> i64 {
        match self.store.earliest(key, now, window()) {
            Some(earliest) => {
                let frees_at = earliest + window();
                (frees_at - now).num_seconds().max(0)
            }
            None => 0,
        }
    }

    /// Builds headers for the scope with the least remaining capacity.
    fn constraining_headers(&self, scopes: &[ScopeState], now: DateTime<Utc>) -> RateLimitHeaders {
        let scope = scopes
            .iter()
            .min_by_key(|s| s.remaining())
            .expect("at least one scope is always present");
        // Remaining was computed before recording; subtract the just-recorded one.
        let remaining = scope.remaining().saturating_sub(1);
        let reset = now.timestamp() + window().num_seconds();
        RateLimitHeaders::allowed(scope.limit, remaining, reset, scope.name)
    }

    /// Issues a challenge when the constraining scope is past the suspicion
    /// threshold (counting this request).
    fn maybe_challenge(&self, req: &SubmissionRequest, scopes: &[ScopeState]) -> Option<Challenge> {
        let suspicious = scopes.iter().any(|s| {
            // +1 to count this just-recorded request.
            self.challenge_policy.is_suspicious(s.count + 1, s.limit)
        });
        if suspicious {
            Some(self.challenge_policy.issue(req.id))
        } else {
            None
        }
    }

    /// Records a violation in the monitor.
    fn record_violation(
        &self,
        req: &SubmissionRequest,
        scope: &str,
        reason: &str,
        now: DateTime<Utc>,
    ) {
        self.monitor.record_blocked(Violation {
            at: now,
            ip: req.ip,
            tier: req.tier,
            scope: scope.to_string(),
            reason: reason.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::submission_rate_limiting::config::SubmissionRateLimitConfig;

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn ip() -> IpAddr {
        "203.0.113.10".parse().unwrap()
    }

    fn limiter() -> SubmissionRateLimiter {
        SubmissionRateLimiter::new(SubmissionRateLimitConfig::default())
    }

    #[test]
    fn allows_up_to_user_limit_then_blocks() {
        let l = limiter();
        let uid = Uuid::new_v4();
        let req =
            SubmissionRequest::submission(Tier::User, Some(uid), ip(), "/api/v1/vulnerabilities");

        for i in 0..10 {
            let d = l.check_at(&req, now(), SystemHealth::healthy());
            assert!(d.is_allowed(), "request {i} should be allowed");
        }
        let d = l.check_at(&req, now(), SystemHealth::healthy());
        match d {
            Decision::Blocked { scope, .. } => assert_eq!(scope, "user"),
            other => panic!("expected block, got {other:?}"),
        }
    }

    #[test]
    fn ip_limit_blocks_across_users() {
        let l = limiter();
        // 100/hour per IP; exhaust via distinct users from the same IP.
        for _ in 0..100 {
            let req = SubmissionRequest::submission(
                Tier::User,
                Some(Uuid::new_v4()),
                ip(),
                "/api/v1/vulnerabilities",
            );
            assert!(l
                .check_at(&req, now(), SystemHealth::healthy())
                .is_allowed());
        }
        let req = SubmissionRequest::submission(
            Tier::User,
            Some(Uuid::new_v4()),
            ip(),
            "/api/v1/vulnerabilities",
        );
        match l.check_at(&req, now(), SystemHealth::healthy()) {
            Decision::Blocked { scope, .. } => assert_eq!(scope, "ip"),
            other => panic!("expected ip block, got {other:?}"),
        }
    }

    #[test]
    fn researcher_bypass_skips_enforcement() {
        let mut l = limiter();
        l.researchers_mut().authorize_full_bypass("vip-token");
        let uid = Uuid::new_v4();
        let req = SubmissionRequest::submission(
            Tier::Researcher,
            Some(uid),
            ip(),
            "/api/v1/vulnerabilities",
        )
        .with_researcher_token("vip-token");
        // Far beyond any limit.
        for _ in 0..50 {
            assert!(matches!(
                l.check_at(&req, now(), SystemHealth::healthy()),
                Decision::Bypassed { .. }
            ));
        }
    }

    #[test]
    fn authorized_researcher_gets_elevated_allowance() {
        let mut l = limiter();
        l.researchers_mut().authorize("res-token"); // elevated, not full bypass
        let uid = Uuid::new_v4();
        let req =
            SubmissionRequest::submission(Tier::User, Some(uid), ip(), "/api/v1/vulnerabilities")
                .with_researcher_token("res-token");
        // 10 base * 10x researcher multiplier = 100 allowed before user scope.
        for i in 0..100 {
            assert!(
                l.check_at(&req, now(), SystemHealth::healthy())
                    .is_allowed(),
                "req {i}"
            );
        }
    }

    #[test]
    fn admin_from_trusted_ip_is_exempt() {
        let mut cfg = SubmissionRateLimitConfig::default();
        cfg.trusted_admin_ranges.push("10.0.0.0/8".to_string());
        let l = SubmissionRateLimiter::with_store(cfg, Box::new(InMemoryStore::new()));
        let req = SubmissionRequest::submission(
            Tier::Admin,
            Some(Uuid::new_v4()),
            "10.1.2.3".parse().unwrap(),
            "/api/v1/vulnerabilities",
        );
        match l.check_at(&req, now(), SystemHealth::healthy()) {
            Decision::Bypassed { .. } => {}
            other => panic!("expected bypass, got {other:?}"),
        }
    }

    #[test]
    fn upload_size_cap_is_enforced() {
        let l = limiter();
        let uid = Uuid::new_v4();
        let too_big = SubmissionRequest::upload(
            Tier::User,
            Some(uid),
            ip(),
            "/api/v1/upload",
            60 * 1024 * 1024,
        );
        match l.check_at(&too_big, now(), SystemHealth::healthy()) {
            Decision::Blocked { scope, .. } => assert_eq!(scope, "upload_size"),
            other => panic!("expected size block, got {other:?}"),
        }
    }

    #[test]
    fn upload_count_limit_is_separate_from_submissions() {
        let l = limiter();
        let uid = Uuid::new_v4();
        // 5 uploads/hour per user.
        for _ in 0..5 {
            let up = SubmissionRequest::upload(Tier::User, Some(uid), ip(), "/api/v1/upload", 1024);
            assert!(l.check_at(&up, now(), SystemHealth::healthy()).is_allowed());
        }
        let up = SubmissionRequest::upload(Tier::User, Some(uid), ip(), "/api/v1/upload", 1024);
        match l.check_at(&up, now(), SystemHealth::healthy()) {
            Decision::Blocked { scope, .. } => assert_eq!(scope, "upload"),
            other => panic!("expected upload block, got {other:?}"),
        }
        // A normal submission by the same user is still allowed.
        let sub =
            SubmissionRequest::submission(Tier::User, Some(uid), ip(), "/api/v1/vulnerabilities");
        assert!(l
            .check_at(&sub, now(), SystemHealth::healthy())
            .is_allowed());
    }

    #[test]
    fn headers_report_remaining() {
        let l = limiter();
        let uid = Uuid::new_v4();
        let req =
            SubmissionRequest::submission(Tier::User, Some(uid), ip(), "/api/v1/vulnerabilities");
        if let Decision::Allowed { headers, .. } = l.check_at(&req, now(), SystemHealth::healthy())
        {
            assert_eq!(headers.scope, "user");
            assert_eq!(headers.limit, 10);
            assert_eq!(headers.remaining, 9);
        } else {
            panic!("expected allowed");
        }
    }

    #[test]
    fn blocked_response_has_retry_after() {
        let l = limiter();
        let uid = Uuid::new_v4();
        let req =
            SubmissionRequest::submission(Tier::User, Some(uid), ip(), "/api/v1/vulnerabilities");
        for _ in 0..10 {
            l.check_at(&req, now(), SystemHealth::healthy());
        }
        if let Decision::Blocked { headers, .. } = l.check_at(&req, now(), SystemHealth::healthy())
        {
            // First event was at `now`, so the window frees in ~3600s.
            assert_eq!(headers.retry_after, Some(3600));
        } else {
            panic!("expected blocked");
        }
    }

    #[test]
    fn adaptive_load_tightens_limits() {
        let l = limiter();
        let uid = Uuid::new_v4();
        let req =
            SubmissionRequest::submission(Tier::User, Some(uid), ip(), "/api/v1/vulnerabilities");
        // Under full load the per-user limit (10) is scaled to the min (25% => 2).
        let stressed = SystemHealth::new(1.0, 0.0);
        assert!(l.check_at(&req, now(), stressed).is_allowed());
        assert!(l.check_at(&req, now(), stressed).is_allowed());
        match l.check_at(&req, now(), stressed) {
            Decision::Blocked { scope, .. } => assert_eq!(scope, "user"),
            other => panic!("expected block under load, got {other:?}"),
        }
    }

    #[test]
    fn challenge_issued_when_suspicious() {
        let l = limiter();
        let uid = Uuid::new_v4();
        let req =
            SubmissionRequest::submission(Tier::User, Some(uid), ip(), "/api/v1/vulnerabilities");
        // Threshold 0.8 of 10 => challenge from the 8th request onward.
        let mut challenged = false;
        for _ in 0..9 {
            if let Decision::Allowed { challenge, .. } =
                l.check_at(&req, now(), SystemHealth::healthy())
            {
                if challenge.is_some() {
                    challenged = true;
                }
            }
        }
        assert!(challenged, "expected a challenge near the limit");
    }

    #[test]
    fn monitor_tracks_decisions() {
        let l = limiter();
        let uid = Uuid::new_v4();
        let req =
            SubmissionRequest::submission(Tier::User, Some(uid), ip(), "/api/v1/vulnerabilities");
        for _ in 0..12 {
            l.check_at(&req, now(), SystemHealth::healthy());
        }
        let stats = l.monitor().stats();
        assert_eq!(stats.total_requests, 12);
        assert_eq!(stats.blocked, 2); // 10 allowed/challenged, 2 blocked
        assert!(!l.monitor().recent_violations(10).is_empty());
    }
}
