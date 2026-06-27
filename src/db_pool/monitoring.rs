//! Connection-pool and query monitoring with alerting.
//!
//! Tracks pool utilization, connection-leak events, and query latency. Raises
//! alerts when the pool nears exhaustion and logs slow queries (default >1s)
//! for performance investigation. All counters are thread-safe.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Default slow-query threshold in milliseconds.
pub const DEFAULT_SLOW_QUERY_MS: u64 = 1000;
/// Default pool-utilization fraction at which an exhaustion alert fires.
pub const DEFAULT_EXHAUSTION_ALERT_RATIO: f64 = 0.9;

/// A logged slow query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlowQuery {
    /// A redacted/normalized statement label.
    pub statement: String,
    /// Observed duration in milliseconds.
    pub duration_ms: u64,
}

/// A raised monitoring alert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Alert {
    /// Stable alert code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Snapshot of monitoring counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorStats {
    /// Successful connection acquisitions.
    pub acquires: u64,
    /// Connections returned to the pool.
    pub releases: u64,
    /// Acquisition failures due to exhaustion.
    pub exhaustion_events: u64,
    /// Connection leaks detected and reclaimed.
    pub leaks_reclaimed: u64,
    /// Total queries observed.
    pub queries: u64,
    /// Queries exceeding the slow threshold.
    pub slow_queries: u64,
}

/// Thread-safe monitor for a single pool.
pub struct DbMonitor {
    acquires: AtomicU64,
    releases: AtomicU64,
    exhaustion_events: AtomicU64,
    leaks_reclaimed: AtomicU64,
    queries: AtomicU64,
    slow_queries: AtomicU64,
    slow_threshold_ms: u64,
    exhaustion_ratio: f64,
    recent_slow: Mutex<Vec<SlowQuery>>,
    max_recent_slow: usize,
    alerts: Mutex<Vec<Alert>>,
}

impl Default for DbMonitor {
    fn default() -> Self {
        Self::new(DEFAULT_SLOW_QUERY_MS, DEFAULT_EXHAUSTION_ALERT_RATIO)
    }
}

impl DbMonitor {
    /// Creates a monitor with the given slow-query threshold and exhaustion
    /// alert ratio.
    pub fn new(slow_threshold_ms: u64, exhaustion_ratio: f64) -> Self {
        Self {
            acquires: AtomicU64::new(0),
            releases: AtomicU64::new(0),
            exhaustion_events: AtomicU64::new(0),
            leaks_reclaimed: AtomicU64::new(0),
            queries: AtomicU64::new(0),
            slow_queries: AtomicU64::new(0),
            slow_threshold_ms,
            exhaustion_ratio,
            recent_slow: Mutex::new(Vec::new()),
            max_recent_slow: 100,
            alerts: Mutex::new(Vec::new()),
        }
    }

    /// Records a successful acquire and checks the exhaustion alert threshold.
    pub fn record_acquire(&self, checked_out: usize, max: usize) {
        self.acquires.fetch_add(1, Ordering::Relaxed);
        if max > 0 && (checked_out as f64 / max as f64) >= self.exhaustion_ratio {
            self.raise(Alert {
                code: "pool-near-exhaustion".to_string(),
                message: format!(
                    "pool utilization {checked_out}/{max} at or above alert threshold"
                ),
            });
        }
    }

    /// Records a connection returned to the pool.
    pub fn record_release(&self) {
        self.releases.fetch_add(1, Ordering::Relaxed);
    }

    /// Records an acquisition that failed because the pool was exhausted.
    pub fn record_exhaustion(&self) {
        self.exhaustion_events.fetch_add(1, Ordering::Relaxed);
        self.raise(Alert {
            code: "pool-exhausted".to_string(),
            message: "connection acquisition failed: pool exhausted".to_string(),
        });
    }

    /// Records that `count` leaked connections were reclaimed.
    pub fn record_leaks(&self, count: u64) {
        if count == 0 {
            return;
        }
        self.leaks_reclaimed.fetch_add(count, Ordering::Relaxed);
        self.raise(Alert {
            code: "connection-leak".to_string(),
            message: format!("{count} leaked connection(s) reclaimed"),
        });
    }

    /// Records a query's latency, logging it if slow.
    pub fn record_query(&self, statement: &str, duration_ms: u64) {
        self.queries.fetch_add(1, Ordering::Relaxed);
        if duration_ms >= self.slow_threshold_ms {
            self.slow_queries.fetch_add(1, Ordering::Relaxed);
            let mut recent = self.recent_slow.lock().expect("recent_slow poisoned");
            recent.push(SlowQuery {
                statement: statement.to_string(),
                duration_ms,
            });
            let overflow = recent.len().saturating_sub(self.max_recent_slow);
            if overflow > 0 {
                recent.drain(0..overflow);
            }
        }
    }

    /// Current counter snapshot.
    pub fn stats(&self) -> MonitorStats {
        MonitorStats {
            acquires: self.acquires.load(Ordering::Relaxed),
            releases: self.releases.load(Ordering::Relaxed),
            exhaustion_events: self.exhaustion_events.load(Ordering::Relaxed),
            leaks_reclaimed: self.leaks_reclaimed.load(Ordering::Relaxed),
            queries: self.queries.load(Ordering::Relaxed),
            slow_queries: self.slow_queries.load(Ordering::Relaxed),
        }
    }

    /// Recent slow queries (most recent last).
    pub fn recent_slow_queries(&self) -> Vec<SlowQuery> {
        self.recent_slow.lock().expect("recent_slow poisoned").clone()
    }

    /// All raised alerts.
    pub fn alerts(&self) -> Vec<Alert> {
        self.alerts.lock().expect("alerts poisoned").clone()
    }

    fn raise(&self, alert: Alert) {
        self.alerts.lock().expect("alerts poisoned").push(alert);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slow_query_logged_above_threshold() {
        let m = DbMonitor::default();
        m.record_query("SELECT 1", 50);
        m.record_query("SELECT pg_sleep(2)", 2000);
        let stats = m.stats();
        assert_eq!(stats.queries, 2);
        assert_eq!(stats.slow_queries, 1);
        assert_eq!(m.recent_slow_queries().len(), 1);
        assert_eq!(m.recent_slow_queries()[0].duration_ms, 2000);
    }

    #[test]
    fn exhaustion_alert_fires_near_capacity() {
        let m = DbMonitor::default(); // 0.9 ratio
        m.record_acquire(89, 100); // below
        assert!(m.alerts().is_empty());
        m.record_acquire(90, 100); // at threshold
        assert!(m.alerts().iter().any(|a| a.code == "pool-near-exhaustion"));
    }

    #[test]
    fn exhaustion_event_alerts() {
        let m = DbMonitor::default();
        m.record_exhaustion();
        let stats = m.stats();
        assert_eq!(stats.exhaustion_events, 1);
        assert!(m.alerts().iter().any(|a| a.code == "pool-exhausted"));
    }

    #[test]
    fn leak_reclaim_recorded() {
        let m = DbMonitor::default();
        m.record_leaks(0); // no-op
        assert_eq!(m.stats().leaks_reclaimed, 0);
        m.record_leaks(3);
        assert_eq!(m.stats().leaks_reclaimed, 3);
        assert!(m.alerts().iter().any(|a| a.code == "connection-leak"));
    }

    #[test]
    fn recent_slow_queries_are_capped() {
        let m = DbMonitor::new(10, 0.9);
        for i in 0..150 {
            m.record_query(&format!("q{i}"), 100);
        }
        assert_eq!(m.recent_slow_queries().len(), 100);
    }
}
