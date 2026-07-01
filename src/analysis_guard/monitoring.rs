//! Monitoring for analysis failures and resource exhaustion.

use crate::analysis_guard::sandbox::SandboxError;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// A snapshot of analysis-engine health counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardStats {
    /// Jobs that completed successfully.
    pub succeeded: u64,
    /// Jobs rejected at input/AST validation.
    pub rejected_validation: u64,
    /// Jobs that crashed (contained panics).
    pub crashes: u64,
    /// Jobs that hit the timeout.
    pub timeouts: u64,
    /// Jobs that exceeded a CPU/memory budget.
    pub resource_exhaustions: u64,
    /// Other operation failures.
    pub failures: u64,
}

impl GuardStats {
    /// Total jobs observed.
    pub fn total(&self) -> u64 {
        self.succeeded
            + self.rejected_validation
            + self.crashes
            + self.timeouts
            + self.resource_exhaustions
            + self.failures
    }
}

/// Thread-safe monitor for the analysis guard.
#[derive(Default)]
pub struct GuardMonitor {
    succeeded: AtomicU64,
    rejected_validation: AtomicU64,
    crashes: AtomicU64,
    timeouts: AtomicU64,
    resource_exhaustions: AtomicU64,
    failures: AtomicU64,
}

impl GuardMonitor {
    /// Creates a monitor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a successful analysis.
    pub fn record_success(&self) {
        self.succeeded.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a validation rejection (input or AST).
    pub fn record_validation_rejection(&self) {
        self.rejected_validation.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a sandbox failure, classifying it onto the right counter.
    pub fn record_sandbox_error(&self, error: &SandboxError) {
        let counter = match error {
            SandboxError::Crashed => &self.crashes,
            SandboxError::Timeout { .. } => &self.timeouts,
            SandboxError::CpuExceeded { .. } | SandboxError::MemoryExceeded { .. } => {
                &self.resource_exhaustions
            }
            SandboxError::OperationFailed(_) => &self.failures,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Current snapshot.
    pub fn stats(&self) -> GuardStats {
        GuardStats {
            succeeded: self.succeeded.load(Ordering::Relaxed),
            rejected_validation: self.rejected_validation.load(Ordering::Relaxed),
            crashes: self.crashes.load(Ordering::Relaxed),
            timeouts: self.timeouts.load(Ordering::Relaxed),
            resource_exhaustions: self.resource_exhaustions.load(Ordering::Relaxed),
            failures: self.failures.load(Ordering::Relaxed),
        }
    }

    /// Whether no parser crash has ever been observed (the zero-crash goal).
    pub fn is_crash_free(&self) -> bool {
        self.crashes.load(Ordering::Relaxed) == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_sandbox_errors() {
        let m = GuardMonitor::new();
        m.record_sandbox_error(&SandboxError::Crashed);
        m.record_sandbox_error(&SandboxError::Timeout {
            used_ms: 9,
            limit_ms: 5,
        });
        m.record_sandbox_error(&SandboxError::CpuExceeded { used: 9, limit: 5 });
        m.record_sandbox_error(&SandboxError::MemoryExceeded { used: 9, limit: 5 });
        m.record_sandbox_error(&SandboxError::OperationFailed("x".to_string()));
        let s = m.stats();
        assert_eq!(s.crashes, 1);
        assert_eq!(s.timeouts, 1);
        assert_eq!(s.resource_exhaustions, 2);
        assert_eq!(s.failures, 1);
        assert!(!m.is_crash_free());
    }

    #[test]
    fn success_and_rejection_counters() {
        let m = GuardMonitor::new();
        m.record_success();
        m.record_validation_rejection();
        let s = m.stats();
        assert_eq!(s.succeeded, 1);
        assert_eq!(s.rejected_validation, 1);
        assert_eq!(s.total(), 2);
        assert!(m.is_crash_free());
    }
}
