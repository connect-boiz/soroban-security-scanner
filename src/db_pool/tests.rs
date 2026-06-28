//! End-to-end integration tests spanning config validation, pooling, retry,
//! monitoring, leak recovery and replica routing as wired together.

use super::*;
use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// A fake connection and factory for driver-agnostic pool testing.
struct FakeConn;

struct FlakyFactory {
    /// Number of `create` calls that should fail before succeeding.
    fail_first: AtomicUsize,
}

impl ConnectionFactory<FakeConn> for FlakyFactory {
    fn create(&self) -> Result<FakeConn, DbError> {
        if self.fail_first.load(Ordering::Relaxed) > 0 {
            self.fail_first.fetch_sub(1, Ordering::Relaxed);
            Err(DbError::CreateFailed("temporarily unavailable".to_string()))
        } else {
            Ok(FakeConn)
        }
    }
    fn validate(&self, _conn: &FakeConn) -> bool {
        true
    }
}

#[test]
fn secure_default_config_is_production_ready() {
    let mut cfg = DbConfig::default();
    cfg.tls.root_cert_path = Some("/etc/ssl/db-ca.pem".to_string());
    assert!(cfg.validate().is_ok());
    assert!(cfg.tls.is_secure());
    assert!(cfg.hardening.is_production_ready(&cfg.tls));
}

#[test]
fn pool_warm_up_then_serves_under_capacity() {
    let cfg = PoolConfig {
        min_connections: 3,
        max_connections: 5,
        idle_timeout_secs: 30,
        leak_timeout_secs: 300,
        validation_query: "SELECT 1".to_string(),
    };
    let factory = FlakyFactory {
        fail_first: AtomicUsize::new(0),
    };
    let pool = ConnectionPool::with_clock(
        cfg,
        Box::new(factory),
        Clock::fixed(1000),
        Arc::new(DbMonitor::default()),
    );
    pool.warm_up().unwrap();
    assert_eq!(pool.stats().idle, 3);

    let guards: Vec<_> = (0..5).map(|_| pool.acquire().unwrap()).collect();
    assert_eq!(pool.stats().checked_out, 5);
    // 6th exceeds max.
    assert_eq!(pool.acquire().unwrap_err(), DbError::PoolExhausted);
    drop(guards);
    assert_eq!(pool.stats().checked_out, 0);
}

#[test]
fn retry_backoff_recovers_transient_connect_failures() {
    // Factory fails twice, then succeeds: retry should ultimately connect.
    let factory = Arc::new(FlakyFactory {
        fail_first: AtomicUsize::new(2),
    });
    let policy = BackoffPolicy::default();
    let delays = RefCell::new(Vec::new());

    let factory_ref = Arc::clone(&factory);
    let result: Result<FakeConn, DbError> = retry_with(
        &policy,
        |_attempt| factory_ref.create(),
        |d| delays.borrow_mut().push(d),
    );

    assert!(result.is_ok());
    assert_eq!(delays.borrow().len(), 2); // two retries before success
}

#[test]
fn leak_recovery_keeps_pool_available_under_peak() {
    let cfg = PoolConfig {
        min_connections: 0,
        max_connections: 2,
        idle_timeout_secs: 30,
        leak_timeout_secs: 60,
        validation_query: "SELECT 1".to_string(),
    };
    let clock = Clock::fixed(1000);
    let pool = ConnectionPool::with_clock(
        cfg,
        Box::new(FlakyFactory {
            fail_first: AtomicUsize::new(0),
        }),
        clock.clone(),
        Arc::new(DbMonitor::default()),
    );

    // Two callers leak their connections, saturating the pool.
    std::mem::forget(pool.acquire().unwrap());
    std::mem::forget(pool.acquire().unwrap());
    assert_eq!(pool.acquire().unwrap_err(), DbError::PoolExhausted);

    // Leak detector reclaims the slots after the timeout; service recovers.
    clock.advance(61);
    assert_eq!(pool.reclaim_leaks(), 2);
    assert!(pool.acquire().is_ok());
    assert!(pool.monitor().stats().leaks_reclaimed >= 2);
}

#[test]
fn monitoring_records_slow_queries_and_pool_pressure() {
    let monitor = DbMonitor::default();
    monitor.record_query("SELECT * FROM big_table", 1500); // slow
    monitor.record_query("SELECT 1", 5); // fast
    monitor.record_acquire(95, 100); // near exhaustion

    let stats = monitor.stats();
    assert_eq!(stats.slow_queries, 1);
    assert_eq!(stats.queries, 2);
    assert!(monitor
        .alerts()
        .iter()
        .any(|a| a.code == "pool-near-exhaustion"));
}

#[test]
fn replica_router_offloads_reads_and_honors_health() {
    let router = ReplicaRouter::new("primary", vec!["replica-1".into(), "replica-2".into()]);
    // Writes pin to primary.
    assert!(router.route(QueryKind::Write).is_primary);
    // Reads spread across replicas.
    let first = router.route(QueryKind::Read).name.clone();
    let second = router.route(QueryKind::Read).name.clone();
    assert_ne!(first, second);
    // Take one replica down → reads concentrate on the survivor.
    router.set_health("replica-1", false);
    assert_eq!(router.route(QueryKind::Read).name, "replica-2");
}

#[test]
fn insecure_tls_fails_the_security_audit() {
    let cfg = DbConfig {
        tls: TlsConfig {
            mode: SslMode::Disable,
            ..Default::default()
        },
        ..Default::default()
    };
    // Config validation passes structurally for Disable (no cert needed)...
    assert!(cfg.validate().is_ok());
    // ...but the hardening audit flags the lack of encryption as High.
    assert!(!cfg.hardening.is_production_ready(&cfg.tls));
}
