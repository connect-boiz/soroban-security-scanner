//! Log retention policy with automatic archival.
//!
//! Records older than the hot-retention window are evicted from the live store
//! and handed to an [`Archive`]; records past the total-retention window are
//! dropped permanently.

use crate::observability::record::LogRecord;
use crate::observability::sink::AggregatingSink;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// Retention windows, in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// How long records stay in the hot store before archival.
    pub hot_secs: i64,
    /// How long archived records are kept before permanent deletion.
    pub archive_secs: i64,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            hot_secs: 7 * 24 * 3600,      // 7 days hot
            archive_secs: 90 * 24 * 3600, // 90 days archived
        }
    }
}

/// A cold archive for evicted records.
pub trait Archive: Send + Sync {
    /// Stores records into the archive.
    fn store(&self, records: &[LogRecord]);
    /// Drops archived records older than `cutoff`. Returns the number purged.
    fn purge_before(&self, cutoff: i64) -> usize;
    /// Number of archived records.
    fn len(&self) -> usize;
    /// Whether the archive is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// In-memory archive (object storage / cold tier in production).
#[derive(Default)]
pub struct InMemoryArchive {
    records: Mutex<Vec<LogRecord>>,
}

impl InMemoryArchive {
    /// Creates an empty archive.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Archive for InMemoryArchive {
    fn store(&self, records: &[LogRecord]) {
        self.records
            .lock()
            .expect("archive poisoned")
            .extend_from_slice(records);
    }

    fn purge_before(&self, cutoff: i64) -> usize {
        let mut recs = self.records.lock().expect("archive poisoned");
        let before = recs.len();
        recs.retain(|r| r.timestamp >= cutoff);
        before - recs.len()
    }

    fn len(&self) -> usize {
        self.records.lock().expect("archive poisoned").len()
    }
}

/// Applies a retention policy: archives hot-expired records and purges
/// archive-expired ones. Returns `(archived, purged)` counts.
pub fn apply_retention(
    sink: &AggregatingSink,
    archive: &dyn Archive,
    policy: &RetentionPolicy,
    now: i64,
) -> (usize, usize) {
    let archived = sink.evict_before(now - policy.hot_secs);
    archive.store(&archived);
    let purged = archive.purge_before(now - policy.archive_secs);
    (archived.len(), purged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::level::LogLevel;
    use crate::observability::sink::LogSink;

    fn rec(ts: i64) -> LogRecord {
        LogRecord::new(ts, LogLevel::Info, "t", "m")
    }

    #[test]
    fn hot_expired_records_are_archived() {
        let sink = AggregatingSink::new();
        let archive = InMemoryArchive::new();
        let policy = RetentionPolicy::default();
        let now = 100 * 24 * 3600;

        sink.emit(&rec(now - 1000)).unwrap(); // fresh
        sink.emit(&rec(now - 10 * 24 * 3600)).unwrap(); // older than 7d hot

        let (archived, _) = apply_retention(&sink, &archive, &policy, now);
        assert_eq!(archived, 1);
        assert_eq!(sink.len(), 1); // fresh one remains hot
        assert_eq!(archive.len(), 1);
    }

    #[test]
    fn archive_expired_records_are_purged() {
        let archive = InMemoryArchive::new();
        let policy = RetentionPolicy::default();
        let now = 200 * 24 * 3600;
        // Pre-seed an ancient archived record (older than 90d).
        archive.store(&[rec(now - 120 * 24 * 3600)]);
        archive.store(&[rec(now - 1000)]);

        let sink = AggregatingSink::new();
        let (_, purged) = apply_retention(&sink, &archive, &policy, now);
        assert_eq!(purged, 1);
        assert_eq!(archive.len(), 1);
    }
}
