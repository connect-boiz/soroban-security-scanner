//! Log sinks and centralized aggregation.
//!
//! A [`LogSink`] is a destination for structured records — stdout, a file, or a
//! centralized aggregator (ELK/CloudWatch) in production. The buffering
//! [`AggregatingSink`] stands in for the central store in tests and as a local
//! fallback, and supports access-controlled reads with an audit trail so log
//! data itself is protected.

use crate::observability::level::LogLevel;
use crate::observability::record::LogRecord;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// A destination for log records. Implementations must be thread-safe.
pub trait LogSink: Send + Sync {
    /// Ships one record. Returns `Ok(())` on accepted delivery.
    fn emit(&self, record: &LogRecord) -> Result<(), String>;
}

/// Who is reading the aggregated logs, for access control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogReaderRole {
    /// May read all logs.
    Operator,
    /// May read non-error operational logs only.
    Developer,
    /// May not read logs.
    Unauthorized,
}

/// An audit record of a log-read access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogAccessRecord {
    /// Reader principal.
    pub principal: String,
    /// Role presented.
    pub role: LogReaderRole,
    /// Whether access was granted.
    pub granted: bool,
    /// Number of records returned (0 if denied).
    pub returned: usize,
}

/// A buffering, access-controlled aggregator standing in for a central store.
#[derive(Default)]
pub struct AggregatingSink {
    records: Mutex<Vec<LogRecord>>,
    access_log: Mutex<Vec<LogAccessRecord>>,
}

impl AggregatingSink {
    /// Creates an empty aggregator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of buffered records.
    pub fn len(&self) -> usize {
        self.records.lock().expect("sink poisoned").len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// A read-only snapshot of buffered records (internal/admin use).
    pub fn snapshot(&self) -> Vec<LogRecord> {
        self.records.lock().expect("sink poisoned").clone()
    }

    /// Access-controlled read. Operators see everything; developers see
    /// non-error records; unauthorized readers get nothing. Every attempt is
    /// audited.
    pub fn read(&self, principal: &str, role: LogReaderRole) -> Vec<LogRecord> {
        let granted = role != LogReaderRole::Unauthorized;
        let result: Vec<LogRecord> = if !granted {
            Vec::new()
        } else {
            let all = self.records.lock().expect("sink poisoned");
            match role {
                LogReaderRole::Operator => all.clone(),
                LogReaderRole::Developer => all
                    .iter()
                    .filter(|r| r.level != LogLevel::Error)
                    .cloned()
                    .collect(),
                LogReaderRole::Unauthorized => Vec::new(),
            }
        };
        self.access_log
            .lock()
            .expect("sink poisoned")
            .push(LogAccessRecord {
                principal: principal.to_string(),
                role,
                granted,
                returned: result.len(),
            });
        result
    }

    /// The log-read audit trail.
    pub fn access_log(&self) -> Vec<LogAccessRecord> {
        self.access_log.lock().expect("sink poisoned").clone()
    }

    /// Removes records older than `cutoff` (used by retention). Returns the
    /// removed records (for archival).
    pub fn evict_before(&self, cutoff: i64) -> Vec<LogRecord> {
        let mut records = self.records.lock().expect("sink poisoned");
        let (old, keep): (Vec<_>, Vec<_>) = records.drain(..).partition(|r| r.timestamp < cutoff);
        *records = keep;
        old
    }
}

impl LogSink for AggregatingSink {
    fn emit(&self, record: &LogRecord) -> Result<(), String> {
        self.records
            .lock()
            .expect("sink poisoned")
            .push(record.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(level: LogLevel, ts: i64) -> LogRecord {
        LogRecord::new(ts, level, "t", "m")
    }

    #[test]
    fn emit_buffers_records() {
        let s = AggregatingSink::new();
        assert!(s.is_empty());
        s.emit(&rec(LogLevel::Info, 1000)).unwrap();
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn access_control_by_role() {
        let s = AggregatingSink::new();
        s.emit(&rec(LogLevel::Info, 1000)).unwrap();
        s.emit(&rec(LogLevel::Error, 1001)).unwrap();

        assert_eq!(s.read("op", LogReaderRole::Operator).len(), 2);
        assert_eq!(s.read("dev", LogReaderRole::Developer).len(), 1); // no error
        assert_eq!(s.read("nobody", LogReaderRole::Unauthorized).len(), 0);
    }

    #[test]
    fn reads_are_audited() {
        let s = AggregatingSink::new();
        s.emit(&rec(LogLevel::Info, 1000)).unwrap();
        s.read("op", LogReaderRole::Operator);
        s.read("nobody", LogReaderRole::Unauthorized);
        let log = s.access_log();
        assert_eq!(log.len(), 2);
        assert!(log[0].granted);
        assert!(!log[1].granted);
    }

    #[test]
    fn evict_before_returns_old_records() {
        let s = AggregatingSink::new();
        s.emit(&rec(LogLevel::Info, 1000)).unwrap();
        s.emit(&rec(LogLevel::Info, 2000)).unwrap();
        let evicted = s.evict_before(1500);
        assert_eq!(evicted.len(), 1);
        assert_eq!(s.len(), 1);
    }
}
