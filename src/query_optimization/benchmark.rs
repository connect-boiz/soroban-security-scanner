//! Query performance benchmarking and regression detection.
//!
//! Compares a current benchmark run against a stored baseline and flags
//! queries whose latency regressed beyond a tolerance — the check a CI
//! performance gate runs to stop slow queries from merging.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A baseline of per-query latencies (ms), keyed by a benchmark name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Baseline {
    latencies_ms: HashMap<String, u64>,
}

impl Baseline {
    /// Creates an empty baseline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records/updates the baseline latency for a benchmark.
    pub fn set(&mut self, name: impl Into<String>, latency_ms: u64) {
        self.latencies_ms.insert(name.into(), latency_ms);
    }

    /// The baseline latency for a benchmark, if recorded.
    pub fn get(&self, name: &str) -> Option<u64> {
        self.latencies_ms.get(name).copied()
    }
}

/// A detected performance regression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Regression {
    /// Benchmark name.
    pub name: String,
    /// Baseline latency (ms).
    pub baseline_ms: u64,
    /// Current latency (ms).
    pub current_ms: u64,
    /// Fractional slowdown (e.g. 0.5 = 50% slower).
    pub slowdown: f64,
}

/// Compares a current run against the baseline. A benchmark is a regression if
/// it is slower than `baseline * (1 + tolerance)`. New benchmarks (absent from
/// the baseline) are ignored. Returns regressions sorted worst-first.
pub fn detect_regressions(
    baseline: &Baseline,
    current: &[(String, u64)],
    tolerance: f64,
) -> Vec<Regression> {
    let mut regressions = Vec::new();
    for (name, &current_ms) in current.iter().map(|(n, v)| (n, v)) {
        if let Some(baseline_ms) = baseline.get(name) {
            if baseline_ms == 0 {
                continue;
            }
            let limit = baseline_ms as f64 * (1.0 + tolerance);
            if (current_ms as f64) > limit {
                regressions.push(Regression {
                    name: name.clone(),
                    baseline_ms,
                    current_ms,
                    slowdown: (current_ms as f64 - baseline_ms as f64) / baseline_ms as f64,
                });
            }
        }
    }
    regressions.sort_by(|a, b| {
        b.slowdown
            .partial_cmp(&a.slowdown)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    regressions
}

/// Whether a current run is within tolerance of the baseline (no regressions).
pub fn passes(baseline: &Baseline, current: &[(String, u64)], tolerance: f64) -> bool {
    detect_regressions(baseline, current, tolerance).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline() -> Baseline {
        let mut b = Baseline::new();
        b.set("user_lookup", 20);
        b.set("order_list", 50);
        b
    }

    #[test]
    fn within_tolerance_passes() {
        let current = vec![
            ("user_lookup".to_string(), 22),
            ("order_list".to_string(), 52),
        ];
        assert!(passes(&baseline(), &current, 0.20)); // 20% tolerance
    }

    #[test]
    fn regression_detected() {
        let current = vec![("user_lookup".to_string(), 40)]; // 100% slower
        let regs = detect_regressions(&baseline(), &current, 0.20);
        assert_eq!(regs.len(), 1);
        assert_eq!(regs[0].name, "user_lookup");
        assert!((regs[0].slowdown - 1.0).abs() < 1e-9);
    }

    #[test]
    fn new_benchmark_ignored() {
        let current = vec![("brand_new".to_string(), 9999)];
        assert!(detect_regressions(&baseline(), &current, 0.20).is_empty());
    }

    #[test]
    fn regressions_sorted_worst_first() {
        let current = vec![
            ("user_lookup".to_string(), 30), // +50%
            ("order_list".to_string(), 150), // +200%
        ];
        let regs = detect_regressions(&baseline(), &current, 0.10);
        assert_eq!(regs.len(), 2);
        assert_eq!(regs[0].name, "order_list"); // worst first
    }
}
