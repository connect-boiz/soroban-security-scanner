//! Query performance metrics with percentile latency.
//!
//! Aggregates per-fingerprint execution durations and computes p50/p95/p99
//! latency, against the acceptance target of a <100 ms 95th-percentile response
//! time.

use crate::query_optimization::normalize::normalize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Target 95th-percentile latency in milliseconds.
pub const P95_TARGET_MS: u64 = 100;

/// Latency statistics for a query fingerprint (or the whole workload).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatencyStats {
    /// Number of executions.
    pub count: u64,
    /// Minimum latency (ms).
    pub min_ms: u64,
    /// Maximum latency (ms).
    pub max_ms: u64,
    /// Mean latency (ms).
    pub avg_ms: u64,
    /// 50th percentile (ms).
    pub p50_ms: u64,
    /// 95th percentile (ms).
    pub p95_ms: u64,
    /// 99th percentile (ms).
    pub p99_ms: u64,
}

impl LatencyStats {
    /// Whether the p95 meets the target.
    pub fn meets_p95_target(&self) -> bool {
        self.p95_ms <= P95_TARGET_MS
    }
}

/// Computes latency stats from a set of durations (ms).
pub fn latency_stats(durations_ms: &[u64]) -> Option<LatencyStats> {
    if durations_ms.is_empty() {
        return None;
    }
    let mut sorted = durations_ms.to_vec();
    sorted.sort_unstable();
    let count = sorted.len() as u64;
    let sum: u64 = sorted.iter().sum();
    Some(LatencyStats {
        count,
        min_ms: sorted[0],
        max_ms: sorted[sorted.len() - 1],
        avg_ms: sum / count,
        p50_ms: percentile(&sorted, 0.50),
        p95_ms: percentile(&sorted, 0.95),
        p99_ms: percentile(&sorted, 0.99),
    })
}

/// Nearest-rank percentile over a pre-sorted slice.
fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    // rank = ceil(p * n), 1-based; clamp to [1, n].
    let n = sorted.len();
    let rank = (p * n as f64).ceil() as usize;
    let idx = rank.clamp(1, n) - 1;
    sorted[idx]
}

/// Records executions per query fingerprint.
#[derive(Debug, Clone, Default)]
pub struct QueryMetrics {
    durations: HashMap<String, Vec<u64>>,
}

impl QueryMetrics {
    /// Creates an empty collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records an execution of `sql` taking `duration_ms`.
    pub fn record(&mut self, sql: &str, duration_ms: u64) {
        self.durations
            .entry(normalize(sql))
            .or_default()
            .push(duration_ms);
    }

    /// Stats for a specific query fingerprint.
    pub fn stats_for(&self, sql: &str) -> Option<LatencyStats> {
        self.durations
            .get(&normalize(sql))
            .and_then(|d| latency_stats(d))
    }

    /// Stats across the entire workload.
    pub fn overall(&self) -> Option<LatencyStats> {
        let all: Vec<u64> = self.durations.values().flatten().copied().collect();
        latency_stats(&all)
    }

    /// Number of distinct query fingerprints tracked.
    pub fn distinct_queries(&self) -> usize {
        self.durations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_nearest_rank() {
        let data: Vec<u64> = (1..=100).collect(); // 1..100
        let s = latency_stats(&data).unwrap();
        assert_eq!(s.min_ms, 1);
        assert_eq!(s.max_ms, 100);
        assert_eq!(s.p50_ms, 50);
        assert_eq!(s.p95_ms, 95);
        assert_eq!(s.p99_ms, 99);
    }

    #[test]
    fn single_value() {
        let s = latency_stats(&[42]).unwrap();
        assert_eq!(s.p95_ms, 42);
        assert_eq!(s.avg_ms, 42);
    }

    #[test]
    fn empty_is_none() {
        assert!(latency_stats(&[]).is_none());
    }

    #[test]
    fn p95_target_check() {
        let fast = latency_stats(&vec![10; 100]).unwrap();
        assert!(fast.meets_p95_target());
        let mut slow = vec![10u64; 90];
        slow.extend(vec![500u64; 10]); // 10% are 500ms → p95 = 500
        let s = latency_stats(&slow).unwrap();
        assert!(!s.meets_p95_target());
    }

    #[test]
    fn metrics_group_by_fingerprint() {
        let mut m = QueryMetrics::new();
        m.record("SELECT * FROM users WHERE id = 1", 20);
        m.record("SELECT * FROM users WHERE id = 2", 30);
        m.record("SELECT * FROM orders", 200);
        // The two parameterized user lookups share a fingerprint.
        assert_eq!(m.distinct_queries(), 2);
        let users = m.stats_for("SELECT * FROM users WHERE id = 5").unwrap();
        assert_eq!(users.count, 2);
        assert_eq!(users.avg_ms, 25);
    }
}
