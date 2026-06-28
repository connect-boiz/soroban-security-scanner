//! Encryption performance monitoring.
//!
//! Tracks encryption/decryption throughput and latency so operators can verify
//! the "minimal performance impact" requirement, and flags when average latency
//! exceeds a budget.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// Default per-operation latency budget in microseconds.
pub const DEFAULT_LATENCY_BUDGET_US: u64 = 1000;

/// A snapshot of encryption performance counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerfStats {
    /// Number of encrypt operations.
    pub encrypt_ops: u64,
    /// Number of decrypt operations.
    pub decrypt_ops: u64,
    /// Total bytes processed.
    pub bytes_processed: u64,
    /// Total time spent, in microseconds.
    pub total_micros: u64,
}

impl PerfStats {
    /// Total operations.
    pub fn total_ops(&self) -> u64 {
        self.encrypt_ops + self.decrypt_ops
    }

    /// Average latency per operation in microseconds (0 if no ops).
    pub fn avg_latency_us(&self) -> f64 {
        let ops = self.total_ops();
        if ops == 0 {
            0.0
        } else {
            self.total_micros as f64 / ops as f64
        }
    }
}

/// Thread-safe encryption performance monitor.
#[derive(Default)]
pub struct PerfMonitor {
    encrypt_ops: AtomicU64,
    decrypt_ops: AtomicU64,
    bytes_processed: AtomicU64,
    total_micros: AtomicU64,
}

impl PerfMonitor {
    /// Creates a monitor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records an encrypt operation over `bytes` taking `micros`.
    pub fn record_encrypt(&self, bytes: usize, micros: u64) {
        self.encrypt_ops.fetch_add(1, Ordering::Relaxed);
        self.bytes_processed
            .fetch_add(bytes as u64, Ordering::Relaxed);
        self.total_micros.fetch_add(micros, Ordering::Relaxed);
    }

    /// Records a decrypt operation over `bytes` taking `micros`.
    pub fn record_decrypt(&self, bytes: usize, micros: u64) {
        self.decrypt_ops.fetch_add(1, Ordering::Relaxed);
        self.bytes_processed
            .fetch_add(bytes as u64, Ordering::Relaxed);
        self.total_micros.fetch_add(micros, Ordering::Relaxed);
    }

    /// Current stats snapshot.
    pub fn stats(&self) -> PerfStats {
        PerfStats {
            encrypt_ops: self.encrypt_ops.load(Ordering::Relaxed),
            decrypt_ops: self.decrypt_ops.load(Ordering::Relaxed),
            bytes_processed: self.bytes_processed.load(Ordering::Relaxed),
            total_micros: self.total_micros.load(Ordering::Relaxed),
        }
    }

    /// Whether average latency is within `budget_us`.
    pub fn within_budget(&self, budget_us: u64) -> bool {
        self.stats().avg_latency_us() <= budget_us as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_ops_and_latency() {
        let m = PerfMonitor::new();
        m.record_encrypt(100, 200);
        m.record_decrypt(100, 400);
        let s = m.stats();
        assert_eq!(s.encrypt_ops, 1);
        assert_eq!(s.decrypt_ops, 1);
        assert_eq!(s.bytes_processed, 200);
        assert_eq!(s.total_ops(), 2);
        assert_eq!(s.avg_latency_us(), 300.0);
    }

    #[test]
    fn empty_monitor_has_zero_latency() {
        let m = PerfMonitor::new();
        assert_eq!(m.stats().avg_latency_us(), 0.0);
        assert!(m.within_budget(DEFAULT_LATENCY_BUDGET_US));
    }

    #[test]
    fn budget_enforcement() {
        let m = PerfMonitor::new();
        m.record_encrypt(10, 500);
        assert!(m.within_budget(1000));
        m.record_encrypt(10, 3000); // avg now 1750
        assert!(!m.within_budget(1000));
    }
}
