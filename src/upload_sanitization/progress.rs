//! Upload progress tracking and timeout handling.
//!
//! Tracks a streaming upload's byte progress against an expected total, detects
//! stalls/timeouts based on the last-activity time, and refuses to accept more
//! bytes than declared (a guard against length-mismatch smuggling).

use serde::{Deserialize, Serialize};

/// State of an in-flight upload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UploadState {
    /// Still receiving data.
    InProgress,
    /// All declared bytes received.
    Complete,
    /// Timed out due to inactivity.
    TimedOut,
    /// Aborted (e.g. declared size exceeded or cancelled).
    Aborted,
}

/// Tracks a single upload's progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadProgress {
    /// Total bytes the client declared it will send.
    pub total_bytes: u64,
    /// Bytes received so far.
    pub received_bytes: u64,
    /// Unix timestamp (secs) of the last received chunk.
    pub last_activity: i64,
    /// Inactivity timeout in seconds.
    pub timeout_secs: i64,
    /// Current state.
    pub state: UploadState,
}

impl UploadProgress {
    /// Starts tracking an upload of `total_bytes`, beginning at `now`.
    pub fn start(total_bytes: u64, now: i64, timeout_secs: i64) -> Self {
        Self {
            total_bytes,
            received_bytes: 0,
            last_activity: now,
            timeout_secs,
            state: UploadState::InProgress,
        }
    }

    /// Fraction complete in `[0.0, 1.0]`.
    pub fn fraction(&self) -> f64 {
        if self.total_bytes == 0 {
            return 1.0;
        }
        (self.received_bytes as f64 / self.total_bytes as f64).min(1.0)
    }

    /// Records a received chunk at `now`. Returns the resulting state.
    ///
    /// Receiving more than `total_bytes` aborts the upload — a declared/actual
    /// length mismatch is treated as hostile.
    pub fn record_chunk(&mut self, len: u64, now: i64) -> UploadState {
        if self.state != UploadState::InProgress {
            return self.state;
        }
        // Enforce timeout based on the gap since last activity.
        if now - self.last_activity > self.timeout_secs {
            self.state = UploadState::TimedOut;
            return self.state;
        }
        self.received_bytes = self.received_bytes.saturating_add(len);
        self.last_activity = now;
        if self.received_bytes > self.total_bytes {
            self.state = UploadState::Aborted;
        } else if self.received_bytes == self.total_bytes {
            self.state = UploadState::Complete;
        }
        self.state
    }

    /// Re-evaluates the timeout without new data (e.g. on a watchdog tick).
    pub fn poll_timeout(&mut self, now: i64) -> UploadState {
        if self.state == UploadState::InProgress && now - self.last_activity > self.timeout_secs {
            self.state = UploadState::TimedOut;
        }
        self.state
    }

    /// Whether the upload completed successfully.
    pub fn is_complete(&self) -> bool {
        self.state == UploadState::Complete
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completes_when_all_bytes_received() {
        let mut p = UploadProgress::start(100, 1000, 30);
        assert_eq!(p.record_chunk(60, 1001), UploadState::InProgress);
        assert!((p.fraction() - 0.6).abs() < 1e-9);
        assert_eq!(p.record_chunk(40, 1002), UploadState::Complete);
        assert!(p.is_complete());
    }

    #[test]
    fn aborts_on_size_overrun() {
        let mut p = UploadProgress::start(100, 1000, 30);
        assert_eq!(p.record_chunk(150, 1001), UploadState::Aborted);
    }

    #[test]
    fn times_out_on_inactivity() {
        let mut p = UploadProgress::start(100, 1000, 30);
        p.record_chunk(10, 1005);
        // Next chunk arrives after the timeout window.
        assert_eq!(p.record_chunk(10, 1100), UploadState::TimedOut);
    }

    #[test]
    fn poll_detects_stall() {
        let mut p = UploadProgress::start(100, 1000, 30);
        assert_eq!(p.poll_timeout(1010), UploadState::InProgress);
        assert_eq!(p.poll_timeout(1040), UploadState::TimedOut);
    }

    #[test]
    fn zero_length_upload_is_fully_fractioned() {
        let p = UploadProgress::start(0, 1000, 30);
        assert_eq!(p.fraction(), 1.0);
    }
}
