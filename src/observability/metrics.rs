//! Log-based metrics extraction.
//!
//! Derives monitoring signals from the log stream: counts per level, total
//! volume and the error rate — the inputs a dashboard or alerting rule consumes.

use crate::observability::level::LogLevel;
use crate::observability::record::LogRecord;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// A snapshot of log-derived metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogMetrics {
    /// Total records observed.
    pub total: u64,
    /// TRACE count.
    pub trace: u64,
    /// DEBUG count.
    pub debug: u64,
    /// INFO count.
    pub info: u64,
    /// WARN count.
    pub warn: u64,
    /// ERROR count.
    pub error: u64,
}

impl LogMetrics {
    /// Error rate in `[0.0, 1.0]` (0 when no records).
    pub fn error_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.error as f64 / self.total as f64
        }
    }
}

/// Thread-safe collector that updates metrics as records are observed.
#[derive(Default)]
pub struct LogMetricsCollector {
    total: AtomicU64,
    trace: AtomicU64,
    debug: AtomicU64,
    info: AtomicU64,
    warn: AtomicU64,
    error: AtomicU64,
}

impl LogMetricsCollector {
    /// Creates a collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Observes a record, updating counters.
    pub fn observe(&self, record: &LogRecord) {
        self.total.fetch_add(1, Ordering::Relaxed);
        let counter = match record.level {
            LogLevel::Trace => &self.trace,
            LogLevel::Debug => &self.debug,
            LogLevel::Info => &self.info,
            LogLevel::Warn => &self.warn,
            LogLevel::Error => &self.error,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Current metrics snapshot.
    pub fn snapshot(&self) -> LogMetrics {
        LogMetrics {
            total: self.total.load(Ordering::Relaxed),
            trace: self.trace.load(Ordering::Relaxed),
            debug: self.debug.load(Ordering::Relaxed),
            info: self.info.load(Ordering::Relaxed),
            warn: self.warn.load(Ordering::Relaxed),
            error: self.error.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(level: LogLevel) -> LogRecord {
        LogRecord::new(1, level, "t", "m")
    }

    #[test]
    fn counts_by_level() {
        let c = LogMetricsCollector::new();
        c.observe(&rec(LogLevel::Info));
        c.observe(&rec(LogLevel::Info));
        c.observe(&rec(LogLevel::Error));
        let m = c.snapshot();
        assert_eq!(m.total, 3);
        assert_eq!(m.info, 2);
        assert_eq!(m.error, 1);
    }

    #[test]
    fn error_rate() {
        let c = LogMetricsCollector::new();
        for _ in 0..9 {
            c.observe(&rec(LogLevel::Info));
        }
        c.observe(&rec(LogLevel::Error));
        assert!((c.snapshot().error_rate() - 0.1).abs() < 1e-9);
    }

    #[test]
    fn empty_has_zero_error_rate() {
        assert_eq!(LogMetricsCollector::new().snapshot().error_rate(), 0.0);
    }
}
