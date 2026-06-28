//! End-to-end integration tests: per-endpoint + tier limiting under concurrency,
//! adaptive tightening, bypass, challenges, config versioning, analytics and
//! alerting, plus distributed-store sharing.

use super::*;
use chrono::{DateTime, Duration, Utc};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn now() -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000, 0).unwrap()
}

fn config() -> RateLimitConfig {
    RateLimitConfig::default()
        .with_endpoint("/api/login", EndpointPolicy::new(5, 60))
        .with_endpoint("/api/bulk", EndpointPolicy::new(1000, 60))
}

#[test]
fn sensitive_endpoint_is_strict_bulk_is_permissive() {
    let limiter = ApiRateLimiter::new(config());

    // /api/login: free tier base 5.
    let login = ApiRequest::new("/api/login", UserTier::Free, "1.1.1.1").with_user("alice");
    for _ in 0..5 {
        assert!(limiter
            .check_at(&login, now(), SystemHealth::healthy())
            .is_allowed());
    }
    assert!(!limiter
        .check_at(&login, now(), SystemHealth::healthy())
        .is_allowed());

    // /api/bulk: base 1000 — comfortably allows a large burst for the same user.
    let bulk = ApiRequest::new("/api/bulk", UserTier::Free, "1.1.1.1").with_user("alice");
    for _ in 0..500 {
        assert!(limiter
            .check_at(&bulk, now(), SystemHealth::healthy())
            .is_allowed());
    }
}

#[test]
fn concurrent_requests_never_exceed_limit() {
    let limiter = Arc::new(ApiRateLimiter::new(config()));
    let allowed = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let limiter = Arc::clone(&limiter);
            let allowed = Arc::clone(&allowed);
            thread::spawn(move || {
                for _ in 0..20 {
                    let req =
                        ApiRequest::new("/api/login", UserTier::Free, "9.9.9.9").with_user("same");
                    if limiter
                        .check_at(&req, now(), SystemHealth::healthy())
                        .is_allowed()
                    {
                        allowed.fetch_add(1, Ordering::Relaxed);
                    }
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    // The per-user limit (5) is the binding constraint despite 160 attempts.
    assert_eq!(allowed.load(Ordering::Relaxed), 5);
}

#[test]
fn tiers_scale_allowance() {
    let limiter = ApiRateLimiter::new(config());
    // Enterprise: base 5 * 20 = 100 on /api/login.
    let req = ApiRequest::new("/api/login", UserTier::Enterprise, "2.2.2.2").with_user("ent");
    for i in 0..100 {
        assert!(
            limiter
                .check_at(&req, now(), SystemHealth::healthy())
                .is_allowed(),
            "req {i}"
        );
    }
    assert!(!limiter
        .check_at(&req, now(), SystemHealth::healthy())
        .is_allowed());
}

#[test]
fn window_slides_and_frees_capacity() {
    let limiter = ApiRateLimiter::new(config());
    let req = ApiRequest::new("/api/login", UserTier::Free, "3.3.3.3").with_user("c");
    for _ in 0..5 {
        limiter.check_at(&req, now(), SystemHealth::healthy());
    }
    assert!(!limiter
        .check_at(&req, now(), SystemHealth::healthy())
        .is_allowed());
    let later = now() + Duration::seconds(61);
    assert!(limiter
        .check_at(&req, later, SystemHealth::healthy())
        .is_allowed());
}

#[test]
fn distributed_store_shares_state_across_instances() {
    use std::sync::Arc as StdArc;
    struct Shared(StdArc<InMemoryStore>);
    impl RateLimitStore for Shared {
        fn count(&self, k: &str, n: DateTime<Utc>, w: Duration) -> u64 {
            self.0.count(k, n, w)
        }
        fn earliest(&self, k: &str, n: DateTime<Utc>, w: Duration) -> Option<DateTime<Utc>> {
            self.0.earliest(k, n, w)
        }
        fn record(&self, k: &str, n: DateTime<Utc>) {
            self.0.record(k, n)
        }
        fn cleanup(&self, n: DateTime<Utc>, w: Duration) {
            self.0.cleanup(n, w)
        }
    }
    let store = StdArc::new(InMemoryStore::new());
    let node_a = ApiRateLimiter::with_store(config(), Box::new(Shared(StdArc::clone(&store))));
    let node_b = ApiRateLimiter::with_store(config(), Box::new(Shared(StdArc::clone(&store))));

    let mk = || ApiRequest::new("/api/login", UserTier::Free, "4.4.4.4").with_user("shared");
    // 3 on A + 2 on B exhaust the shared per-user limit of 5.
    for _ in 0..3 {
        assert!(node_a
            .check_at(&mk(), now(), SystemHealth::healthy())
            .is_allowed());
    }
    for _ in 0..2 {
        assert!(node_b
            .check_at(&mk(), now(), SystemHealth::healthy())
            .is_allowed());
    }
    assert!(!node_b
        .check_at(&mk(), now(), SystemHealth::healthy())
        .is_allowed());
}

#[test]
fn config_versioning_changes_limits_live() {
    let mut mgr = ConfigManager::new(config()); // v1
    assert_eq!(mgr.current().policy_for("/api/login").base_limit, 5);
    // Tighten /api/login to 2 in a new version.
    let v = mgr
        .update(RateLimitConfig::default().with_endpoint("/api/login", EndpointPolicy::new(2, 60)));
    assert_eq!(v, 2);
    let limiter = ApiRateLimiter::new(mgr.current().clone());
    let req = ApiRequest::new("/api/login", UserTier::Free, "5.5.5.5").with_user("d");
    for _ in 0..2 {
        assert!(limiter
            .check_at(&req, now(), SystemHealth::healthy())
            .is_allowed());
    }
    assert!(!limiter
        .check_at(&req, now(), SystemHealth::healthy())
        .is_allowed());
}

#[test]
fn analytics_and_alerting_surface_abuse() {
    // A tight endpoint + an attacker hammering it produces a high throttle rate.
    let cfg = RateLimitConfig::default().with_endpoint("/api/login", EndpointPolicy::new(2, 60));
    let limiter = ApiRateLimiter::new(cfg);
    let req = ApiRequest::new("/api/login", UserTier::Free, "6.6.6.6").with_user("attacker");
    for _ in 0..100 {
        limiter.check_at(&req, now(), SystemHealth::healthy());
    }
    let snap = limiter.analytics_snapshot(5);
    assert_eq!(snap.total_requests, 100);
    assert!(snap.total_throttled > 90); // only 2 allowed
    assert!(snap.throttle_rate > 0.9);
    assert!(
        !limiter.alerts().is_empty(),
        "high throttle rate should alert"
    );
}
