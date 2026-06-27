//! Integration tests exercising the limiter end-to-end under a range of
//! load and concurrency conditions.

use super::*;
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use std::thread;
use uuid::Uuid;

fn base_time() -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000, 0).unwrap()
}

#[test]
fn enforcement_accuracy_is_exact_over_full_window() {
    // Drive exactly enough traffic to hit the global ceiling and assert the
    // count of allowed requests equals the configured limit precisely
    // (no over- or under-counting).
    let limiter = SubmissionRateLimiter::new(SubmissionRateLimitConfig::default());
    let now = base_time();

    let mut allowed = 0u64;
    let mut blocked = 0u64;
    // 1200 attempts against a 1000/h global ceiling, spread across many IPs and
    // users so only the global scope is the binding constraint.
    for i in 0..1200 {
        let req = SubmissionRequest::submission(
            Tier::User,
            Some(Uuid::new_v4()),
            format!("198.51.100.{}", i % 250 + 1).parse().unwrap(),
            "/api/v1/vulnerabilities",
        );
        match limiter.check_at(&req, now, SystemHealth::healthy()) {
            Decision::Allowed { .. } => allowed += 1,
            Decision::Blocked { .. } => blocked += 1,
            Decision::Bypassed { .. } => {}
        }
    }

    assert_eq!(allowed, 1000, "exactly the global limit should be allowed");
    assert_eq!(blocked, 200);
}

#[test]
fn window_slides_and_frees_capacity() {
    let limiter = SubmissionRateLimiter::new(SubmissionRateLimitConfig::default());
    let uid = Uuid::new_v4();
    let ip = "203.0.113.7".parse().unwrap();
    let t0 = base_time();

    // Exhaust the per-user limit at t0.
    for _ in 0..10 {
        let req = SubmissionRequest::submission(Tier::User, Some(uid), ip, "/api/v1/vulnerabilities");
        assert!(limiter.check_at(&req, t0, SystemHealth::healthy()).is_allowed());
    }
    let req = SubmissionRequest::submission(Tier::User, Some(uid), ip, "/api/v1/vulnerabilities");
    assert!(!limiter.check_at(&req, t0, SystemHealth::healthy()).is_allowed());

    // One hour and a second later, the original events have aged out.
    let later = t0 + Duration::seconds(3601);
    assert!(limiter.check_at(&req, later, SystemHealth::healthy()).is_allowed());
}

#[test]
fn concurrent_submissions_never_exceed_limit() {
    // Hammer a single user/IP from many threads and assert the limiter never
    // admits more than the configured ceiling (thread-safety / accuracy).
    let limiter = Arc::new(SubmissionRateLimiter::new(SubmissionRateLimitConfig::default()));
    let uid = Uuid::new_v4();
    let now = base_time();

    let threads: Vec<_> = (0..8)
        .map(|_| {
            let limiter = Arc::clone(&limiter);
            thread::spawn(move || {
                let mut local_allowed = 0u64;
                for _ in 0..50 {
                    let req = SubmissionRequest::submission(
                        Tier::User,
                        Some(uid),
                        "203.0.113.99".parse().unwrap(),
                        "/api/v1/vulnerabilities",
                    );
                    if limiter.check_at(&req, now, SystemHealth::healthy()).is_allowed() {
                        local_allowed += 1;
                    }
                }
                local_allowed
            })
        })
        .collect();

    let total_allowed: u64 = threads.into_iter().map(|t| t.join().unwrap()).sum();
    // Per-user limit is the binding constraint at 10.
    assert_eq!(total_allowed, 10);
}

#[test]
fn adaptive_recovers_when_load_subsides() {
    let limiter = SubmissionRateLimiter::new(SubmissionRateLimitConfig::default());
    let uid = Uuid::new_v4();
    let ip = "203.0.113.55".parse().unwrap();
    let now = base_time();

    // Under heavy load only 2 of the user's 10 slots are available.
    let stressed = SystemHealth::new(1.0, 0.0);
    let req = SubmissionRequest::submission(Tier::User, Some(uid), ip, "/api/v1/vulnerabilities");
    assert!(limiter.check_at(&req, now, stressed).is_allowed());
    assert!(limiter.check_at(&req, now, stressed).is_allowed());
    assert!(!limiter.check_at(&req, now, stressed).is_allowed());

    // When load subsides the full allowance returns (already used 2, 8 left).
    let healthy = SystemHealth::healthy();
    for _ in 0..8 {
        assert!(limiter.check_at(&req, now, healthy).is_allowed());
    }
    assert!(!limiter.check_at(&req, now, healthy).is_allowed());
}

#[test]
fn distributed_store_shares_state_across_limiters() {
    // Two limiter instances sharing one store behave as one logical limiter —
    // the property a Redis-backed store provides across backend instances.
    let store: Arc<InMemoryStore> = Arc::new(InMemoryStore::new());

    struct Shared(Arc<InMemoryStore>);
    impl RateLimitStore for Shared {
        fn count(&self, k: &str, now: DateTime<Utc>, w: Duration) -> u64 {
            self.0.count(k, now, w)
        }
        fn earliest(&self, k: &str, now: DateTime<Utc>, w: Duration) -> Option<DateTime<Utc>> {
            self.0.earliest(k, now, w)
        }
        fn record(&self, k: &str, now: DateTime<Utc>) {
            self.0.record(k, now)
        }
        fn cleanup(&self, now: DateTime<Utc>, w: Duration) {
            self.0.cleanup(now, w)
        }
        fn reset(&self) {
            self.0.reset()
        }
    }

    let node_a = SubmissionRateLimiter::with_store(
        SubmissionRateLimitConfig::default(),
        Box::new(Shared(Arc::clone(&store))),
    );
    let node_b = SubmissionRateLimiter::with_store(
        SubmissionRateLimitConfig::default(),
        Box::new(Shared(Arc::clone(&store))),
    );

    let uid = Uuid::new_v4();
    let ip = "203.0.113.77".parse().unwrap();
    let now = base_time();

    // 5 on node A + 5 on node B exhaust the shared per-user limit of 10.
    for _ in 0..5 {
        let req = SubmissionRequest::submission(Tier::User, Some(uid), ip, "/api/v1/vulnerabilities");
        assert!(node_a.check_at(&req, now, SystemHealth::healthy()).is_allowed());
    }
    for _ in 0..5 {
        let req = SubmissionRequest::submission(Tier::User, Some(uid), ip, "/api/v1/vulnerabilities");
        assert!(node_b.check_at(&req, now, SystemHealth::healthy()).is_allowed());
    }
    let req = SubmissionRequest::submission(Tier::User, Some(uid), ip, "/api/v1/vulnerabilities");
    assert!(!node_b.check_at(&req, now, SystemHealth::healthy()).is_allowed());
}

#[test]
fn monitoring_raises_alert_under_attack() {
    // An aggressive single user generating mostly blocks should trip the
    // monitor's block-ratio alert.
    let limiter = SubmissionRateLimiter::new(SubmissionRateLimitConfig::default());
    let uid = Uuid::new_v4();
    let ip = "203.0.113.88".parse().unwrap();
    let now = base_time();

    for _ in 0..200 {
        let req = SubmissionRequest::submission(Tier::User, Some(uid), ip, "/api/v1/vulnerabilities");
        limiter.check_at(&req, now, SystemHealth::healthy());
    }

    let stats = limiter.monitor().stats();
    assert_eq!(stats.total_requests, 200);
    assert!(stats.blocked > stats.allowed, "mostly blocked under attack");
    assert!(stats.block_ratio() > 0.5);
    assert!(!limiter.monitor().alerts().is_empty(), "an alert should fire");
}

#[test]
fn headers_round_trip_to_http_pairs() {
    let limiter = SubmissionRateLimiter::new(SubmissionRateLimitConfig::default());
    let req = SubmissionRequest::submission(
        Tier::User,
        Some(Uuid::new_v4()),
        "203.0.113.4".parse().unwrap(),
        "/api/v1/vulnerabilities",
    );
    if let Decision::Allowed { headers, .. } = limiter.check_at(&req, base_time(), SystemHealth::healthy()) {
        let pairs = headers.to_pairs();
        let names: Vec<_> = pairs.iter().map(|(k, _)| *k).collect();
        assert!(names.contains(&"X-RateLimit-Limit"));
        assert!(names.contains(&"X-RateLimit-Remaining"));
        assert!(names.contains(&"X-RateLimit-Reset"));
    } else {
        panic!("expected allowed");
    }
}
