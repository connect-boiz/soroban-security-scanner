//! Slow-query monitoring and alerting.
//!
//! Logs queries whose execution time crosses a threshold (default 1 s) and
//! raises an alert for each, keeping a bounded, most-recent log for the
//! performance dashboard.

use crate::query_optimization::normalize::normalize;
use serde::{Deserialize, Serialize};

/// Default slow-query threshold in milliseconds.
pub const DEFAULT_SLOW_THRESHOLD_MS: u64 = 1000;

/// A logged slow query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlowQuery {
    /// Normalized query fingerprint.
    pub fingerprint: String,
    /// Execution time in milliseconds.
    pub duration_ms: u64,
    /// When it ran (unix seconds).
    pub at: i64,
}

/// Monitors query latencies and records slow ones.
#[derive(Debug, Clone)]
pub struct SlowQueryMonitor {
    threshold_ms: u64,
    max_log: usize,
    log: Vec<SlowQuery>,
    total_slow: u64,
}

impl Default for SlowQueryMonitor {
    fn default() -> Self {
        Self::new(DEFAULT_SLOW_THRESHOLD_MS, 1000)
    }
}

impl SlowQueryMonitor {
    /// Creates a monitor with a threshold and a bounded log size.
    pub fn new(threshold_ms: u64, max_log: usize) -> Self {
        Self {
            threshold_ms,
            max_log,
            log: Vec::new(),
            total_slow: 0,
        }
    }

    /// Observes a query execution. Returns `Some(SlowQuery)` (an alert) if it
    /// crossed the threshold.
    pub fn observe(&mut self, sql: &str, duration_ms: u64, at: i64) -> Option<SlowQuery> {
        if duration_ms < self.threshold_ms {
            return None;
        }
        let entry = SlowQuery {
            fingerprint: normalize(sql),
            duration_ms,
            at,
        };
        self.total_slow += 1;
        self.log.push(entry.clone());
        let overflow = self.log.len().saturating_sub(self.max_log);
        if overflow > 0 {
            self.log.drain(0..overflow);
        }
        Some(entry)
    }

    /// Total slow queries observed.
    pub fn total_slow(&self) -> u64 {
        self.total_slow
    }

    /// Recent slow-query log (most recent last).
    pub fn recent(&self, limit: usize) -> &[SlowQuery] {
        let start = self.log.len().saturating_sub(limit);
        &self.log[start..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_query_does_not_alert() {
        let mut m = SlowQueryMonitor::default();
        assert!(m.observe("SELECT 1", 50, 1000).is_none());
        assert_eq!(m.total_slow(), 0);
    }

    #[test]
    fn slow_query_alerts_and_logs() {
        let mut m = SlowQueryMonitor::default();
        let alert = m
            .observe("SELECT * FROM big_table WHERE x = 1", 1500, 1000)
            .unwrap();
        assert_eq!(alert.duration_ms, 1500);
        assert!(alert.fingerprint.contains("big_table"));
        assert_eq!(m.total_slow(), 1);
    }

    #[test]
    fn threshold_boundary_inclusive() {
        let mut m = SlowQueryMonitor::new(1000, 10);
        assert!(m.observe("q", 999, 1).is_none());
        assert!(m.observe("q", 1000, 1).is_some());
    }

    #[test]
    fn log_is_bounded() {
        let mut m = SlowQueryMonitor::new(100, 3);
        for i in 0..10 {
            m.observe("q", 200, i);
        }
        assert_eq!(m.recent(100).len(), 3);
        assert_eq!(m.total_slow(), 10);
    }
}
