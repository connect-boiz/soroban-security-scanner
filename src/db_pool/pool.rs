//! A bounded, validating connection pool with leak detection.
//!
//! Generic over a connection type `C` and a [`ConnectionFactory`], so it can be
//! exercised deterministically in tests and wired to a real driver (e.g.
//! `sqlx`/`tokio-postgres`) in production. Enforces min/max sizing and idle
//! timeout, validates connections before handing them out, and can reclaim
//! connections that callers leak (never return).
//!
//! Time is supplied by an injectable [`Clock`] so idle/leak timing is testable
//! without real sleeps.

use crate::db_pool::config::PoolConfig;
use crate::db_pool::monitoring::DbMonitor;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Errors surfaced by the pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbError {
    /// All `max_connections` are in use.
    PoolExhausted,
    /// The factory failed to create a new connection.
    CreateFailed(String),
}

/// Source of timestamps (unix seconds). `Fixed` makes timing deterministic.
#[derive(Clone)]
pub enum Clock {
    /// Wall-clock time.
    System,
    /// A test clock that can be advanced manually.
    Fixed(Arc<Mutex<i64>>),
}

impl Clock {
    /// Creates a fixed clock starting at `t`.
    pub fn fixed(t: i64) -> Self {
        Clock::Fixed(Arc::new(Mutex::new(t)))
    }

    /// Current time in unix seconds.
    pub fn now(&self) -> i64 {
        match self {
            Clock::System => chrono::Utc::now().timestamp(),
            Clock::Fixed(t) => *t.lock().expect("clock poisoned"),
        }
    }

    /// Advances a fixed clock by `secs` (no-op for the system clock).
    pub fn advance(&self, secs: i64) {
        if let Clock::Fixed(t) = self {
            *t.lock().expect("clock poisoned") += secs;
        }
    }
}

/// Creates and validates connections for the pool.
pub trait ConnectionFactory<C>: Send + Sync {
    /// Opens a new connection (applying TLS, timeouts, session hardening).
    fn create(&self) -> Result<C, DbError>;
    /// Runs the validation query; returns true if the connection is healthy.
    fn validate(&self, conn: &C) -> bool;
}

/// A point-in-time view of pool occupancy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolStats {
    /// Live connections (idle + checked out).
    pub total: usize,
    /// Connections sitting idle.
    pub idle: usize,
    /// Connections currently handed out.
    pub checked_out: usize,
    /// Configured maximum.
    pub max: usize,
    /// Configured minimum.
    pub min: usize,
}

struct IdleConn<C> {
    conn: C,
    last_used: i64,
}

struct State<C> {
    idle: Vec<IdleConn<C>>,
    checked_out: HashMap<u64, i64>, // id -> checkout time
    total: usize,
    next_id: u64,
}

struct Shared<C> {
    cfg: PoolConfig,
    factory: Box<dyn ConnectionFactory<C>>,
    clock: Clock,
    monitor: Arc<DbMonitor>,
    state: Mutex<State<C>>,
}

/// A bounded, validating connection pool.
pub struct ConnectionPool<C> {
    shared: Arc<Shared<C>>,
}

impl<C: Send + 'static> ConnectionPool<C> {
    /// Builds a pool with the system clock and a fresh monitor.
    pub fn new(cfg: PoolConfig, factory: Box<dyn ConnectionFactory<C>>) -> Self {
        Self::with_clock(cfg, factory, Clock::System, Arc::new(DbMonitor::default()))
    }

    /// Builds a pool with an explicit clock and monitor (for tests / wiring).
    pub fn with_clock(
        cfg: PoolConfig,
        factory: Box<dyn ConnectionFactory<C>>,
        clock: Clock,
        monitor: Arc<DbMonitor>,
    ) -> Self {
        Self {
            shared: Arc::new(Shared {
                cfg,
                factory,
                clock,
                monitor,
                state: Mutex::new(State {
                    idle: Vec::new(),
                    checked_out: HashMap::new(),
                    total: 0,
                    next_id: 0,
                }),
            }),
        }
    }

    /// The shared monitor.
    pub fn monitor(&self) -> &Arc<DbMonitor> {
        &self.shared.monitor
    }

    /// Pre-creates `min_connections` idle connections.
    pub fn warm_up(&self) -> Result<(), DbError> {
        let now = self.shared.clock.now();
        let mut state = self.shared.state.lock().expect("pool poisoned");
        while state.total < self.shared.cfg.min_connections {
            let conn = self.shared.factory.create()?;
            state.idle.push(IdleConn { conn, last_used: now });
            state.total += 1;
        }
        Ok(())
    }

    /// Acquires a connection, creating one if capacity allows.
    pub fn acquire(&self) -> Result<PooledConnection<C>, DbError> {
        let now = self.shared.clock.now();
        let mut state = self.shared.state.lock().expect("pool poisoned");

        // Drop idle connections that have exceeded the idle timeout.
        let idle_timeout = self.shared.cfg.idle_timeout_secs;
        let before = state.idle.len();
        state.idle.retain(|c| now - c.last_used <= idle_timeout);
        state.total -= before - state.idle.len();

        // Reuse a valid idle connection; discard invalid ones.
        let mut connection = None;
        while let Some(idle) = state.idle.pop() {
            if self.shared.factory.validate(&idle.conn) {
                connection = Some(idle.conn);
                break;
            } else {
                state.total -= 1; // stale; discard
            }
        }

        // Otherwise create a new one if under the cap.
        let conn = match connection {
            Some(c) => c,
            None => {
                if state.total >= self.shared.cfg.max_connections {
                    self.shared.monitor.record_exhaustion();
                    return Err(DbError::PoolExhausted);
                }
                let c = self.shared.factory.create()?;
                state.total += 1;
                c
            }
        };

        let id = state.next_id;
        state.next_id += 1;
        state.checked_out.insert(id, now);
        self.shared
            .monitor
            .record_acquire(state.checked_out.len(), self.shared.cfg.max_connections);

        Ok(PooledConnection {
            shared: Arc::clone(&self.shared),
            id,
            conn: Some(conn),
        })
    }

    /// Reclaims connections checked out longer than the leak timeout.
    ///
    /// Returns the number reclaimed. Their guards, when eventually dropped,
    /// will find their id gone and silently discard the connection — freeing
    /// the leaked slot for new work (automatic recovery).
    pub fn reclaim_leaks(&self) -> usize {
        let now = self.shared.clock.now();
        let leak_timeout = self.shared.cfg.leak_timeout_secs;
        let mut state = self.shared.state.lock().expect("pool poisoned");

        let leaked: Vec<u64> = state
            .checked_out
            .iter()
            .filter(|(_, since)| now - **since > leak_timeout)
            .map(|(id, _)| *id)
            .collect();

        for id in &leaked {
            state.checked_out.remove(id);
            state.total -= 1; // slot freed; the leaked conn is abandoned
        }
        let count = leaked.len();
        drop(state);
        self.shared.monitor.record_leaks(count as u64);
        count
    }

    /// Current pool statistics.
    pub fn stats(&self) -> PoolStats {
        let state = self.shared.state.lock().expect("pool poisoned");
        PoolStats {
            total: state.total,
            idle: state.idle.len(),
            checked_out: state.checked_out.len(),
            max: self.shared.cfg.max_connections,
            min: self.shared.cfg.min_connections,
        }
    }
}

/// A checked-out connection that returns itself to the pool on drop.
pub struct PooledConnection<C: Send + 'static> {
    shared: Arc<Shared<C>>,
    id: u64,
    conn: Option<C>,
}

impl<C: Send + 'static> PooledConnection<C> {
    /// Borrows the underlying connection.
    pub fn get(&self) -> &C {
        self.conn.as_ref().expect("connection already taken")
    }

    /// Mutably borrows the underlying connection.
    pub fn get_mut(&mut self) -> &mut C {
        self.conn.as_mut().expect("connection already taken")
    }
}

impl<C: Send + 'static> std::fmt::Debug for PooledConnection<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PooledConnection")
            .field("id", &self.id)
            .field("active", &self.conn.is_some())
            .finish()
    }
}

impl<C: Send + 'static> Drop for PooledConnection<C> {
    fn drop(&mut self) {
        let now = self.shared.clock.now();
        let mut state = self.shared.state.lock().expect("pool poisoned");
        // Only return the connection if its checkout is still tracked; if it
        // was reclaimed as a leak, the slot is already freed — discard it.
        if state.checked_out.remove(&self.id).is_some() {
            if let Some(conn) = self.conn.take() {
                state.idle.push(IdleConn { conn, last_used: now });
            }
            drop(state);
            self.shared.monitor.record_release();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A fake connection carrying a creation serial.
    struct FakeConn {
        #[allow(dead_code)]
        serial: usize,
        healthy: bool,
    }

    /// Factory that counts creations and can be told to produce unhealthy or
    /// failing connections.
    struct FakeFactory {
        created: AtomicUsize,
        healthy: bool,
        fail: bool,
    }

    impl FakeFactory {
        fn healthy() -> Self {
            Self {
                created: AtomicUsize::new(0),
                healthy: true,
                fail: false,
            }
        }
    }

    impl ConnectionFactory<FakeConn> for FakeFactory {
        fn create(&self) -> Result<FakeConn, DbError> {
            if self.fail {
                return Err(DbError::CreateFailed("refused".to_string()));
            }
            let serial = self.created.fetch_add(1, Ordering::Relaxed);
            Ok(FakeConn {
                serial,
                healthy: self.healthy,
            })
        }

        fn validate(&self, conn: &FakeConn) -> bool {
            conn.healthy
        }
    }

    fn cfg() -> PoolConfig {
        PoolConfig {
            min_connections: 2,
            max_connections: 3,
            idle_timeout_secs: 30,
            leak_timeout_secs: 60,
            validation_query: "SELECT 1".to_string(),
        }
    }

    fn pool(factory: FakeFactory, clock: Clock) -> ConnectionPool<FakeConn> {
        ConnectionPool::with_clock(cfg(), Box::new(factory), clock, Arc::new(DbMonitor::default()))
    }

    #[test]
    fn warm_up_creates_min_connections() {
        let p = pool(FakeFactory::healthy(), Clock::fixed(1000));
        p.warm_up().unwrap();
        let s = p.stats();
        assert_eq!(s.total, 2);
        assert_eq!(s.idle, 2);
    }

    #[test]
    fn acquire_and_release_reuses_connection() {
        let p = pool(FakeFactory::healthy(), Clock::fixed(1000));
        {
            let c = p.acquire().unwrap();
            assert_eq!(p.stats().checked_out, 1);
            drop(c);
        }
        assert_eq!(p.stats().checked_out, 0);
        assert_eq!(p.stats().idle, 1);
        // Re-acquire reuses the idle connection (total stays 1).
        let _c = p.acquire().unwrap();
        assert_eq!(p.stats().total, 1);
    }

    #[test]
    fn enforces_max_connections() {
        let p = pool(FakeFactory::healthy(), Clock::fixed(1000));
        let _a = p.acquire().unwrap();
        let _b = p.acquire().unwrap();
        let _c = p.acquire().unwrap();
        assert_eq!(p.acquire().unwrap_err(), DbError::PoolExhausted);
        assert_eq!(p.monitor().stats().exhaustion_events, 1);
    }

    #[test]
    fn idle_timeout_evicts_stale_connections() {
        let clock = Clock::fixed(1000);
        let p = pool(FakeFactory::healthy(), clock.clone());
        p.warm_up().unwrap();
        assert_eq!(p.stats().idle, 2);
        // Advance past idle timeout; next acquire prunes the stale idle conns.
        clock.advance(31);
        let _c = p.acquire().unwrap();
        // Both idle were pruned, one fresh created → total 1.
        assert_eq!(p.stats().total, 1);
    }

    #[test]
    fn invalid_idle_connection_is_discarded() {
        let clock = Clock::fixed(1000);
        // Factory yields unhealthy connections.
        let factory = FakeFactory {
            created: AtomicUsize::new(0),
            healthy: false,
            fail: false,
        };
        let p = pool(factory, clock);
        // Manually warm then release so an idle (unhealthy) conn exists.
        let c = p.acquire().unwrap();
        drop(c);
        assert_eq!(p.stats().idle, 1);
        // Next acquire validates the idle conn, finds it unhealthy, discards it,
        // and creates a replacement.
        let _c = p.acquire().unwrap();
        assert_eq!(p.stats().checked_out, 1);
    }

    #[test]
    fn create_failure_propagates() {
        let factory = FakeFactory {
            created: AtomicUsize::new(0),
            healthy: true,
            fail: true,
        };
        let p = pool(factory, Clock::fixed(1000));
        assert_eq!(p.acquire().unwrap_err(), DbError::CreateFailed("refused".to_string()));
    }

    #[test]
    fn leak_detection_reclaims_slots() {
        let clock = Clock::fixed(1000);
        let p = pool(FakeFactory::healthy(), clock.clone());
        // Leak two connections (forget the guards).
        std::mem::forget(p.acquire().unwrap());
        std::mem::forget(p.acquire().unwrap());
        assert_eq!(p.stats().checked_out, 2);
        // Before the leak timeout, nothing is reclaimed.
        assert_eq!(p.reclaim_leaks(), 0);
        // After it, both slots are recovered.
        clock.advance(61);
        assert_eq!(p.reclaim_leaks(), 2);
        assert_eq!(p.stats().total, 0);
        assert_eq!(p.monitor().stats().leaks_reclaimed, 2);
    }

    #[test]
    fn connection_is_usable_through_guard() {
        let p = pool(FakeFactory::healthy(), Clock::fixed(1000));
        let mut c = p.acquire().unwrap();
        assert!(c.get().healthy);
        let _ = c.get_mut();
    }
}
