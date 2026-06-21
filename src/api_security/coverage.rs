//! Security test coverage reporting with quality gates.

use crate::api_security::endpoints::EndpointRegistry;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::Write as _;

/// Minimum coverage percentage required to pass the quality gate.
pub const DEFAULT_COVERAGE_GATE: f64 = 100.0;

/// Quality gate configuration for security test coverage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageGate {
    pub min_endpoint_coverage_pct: f64,
    pub min_critical_workflow_coverage_pct: f64,
    pub block_on_high_severity: bool,
}

impl Default for CoverageGate {
    fn default() -> Self {
        Self {
            min_endpoint_coverage_pct: DEFAULT_COVERAGE_GATE,
            min_critical_workflow_coverage_pct: DEFAULT_COVERAGE_GATE,
            block_on_high_severity: true,
        }
    }
}

/// Coverage report for the security test suite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityCoverageReport {
    pub total_endpoints: usize,
    pub covered_endpoints: usize,
    pub total_critical: usize,
    pub covered_critical: usize,
    pub uncovered: Vec<String>,
    pub timestamp: DateTime<Utc>,
    pub gate: CoverageGate,
}

impl SecurityCoverageReport {
    pub fn endpoint_coverage_pct(&self) -> f64 {
        if self.total_endpoints == 0 {
            return 100.0;
        }
        (self.covered_endpoints as f64 / self.total_endpoints as f64) * 100.0
    }

    pub fn critical_coverage_pct(&self) -> f64 {
        if self.total_critical == 0 {
            return 100.0;
        }
        (self.covered_critical as f64 / self.total_critical as f64) * 100.0
    }

    pub fn passes_gate(&self) -> bool {
        self.endpoint_coverage_pct() >= self.gate.min_endpoint_coverage_pct
            && self.critical_coverage_pct() >= self.gate.min_critical_workflow_coverage_pct
    }

    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# API Security Test Coverage Report");
        let _ = writeln!(out, "Generated: {}\n", self.timestamp.to_rfc3339());
        let _ = writeln!(
            out,
            "**Result:** {}\n",
            if self.passes_gate() {
                "✅ PASS"
            } else {
                "❌ FAIL"
            }
        );
        let _ = writeln!(
            out,
            "| Metric | Covered | Total | % |\n|--------|---------|-------|---|"
        );
        let _ = writeln!(
            out,
            "| Endpoints | {} | {} | {:.1}% |",
            self.covered_endpoints,
            self.total_endpoints,
            self.endpoint_coverage_pct()
        );
        let _ = writeln!(
            out,
            "| Critical workflows | {} | {} | {:.1}% |",
            self.covered_critical,
            self.total_critical,
            self.critical_coverage_pct()
        );
        if !self.uncovered.is_empty() {
            out.push_str("\n## Uncovered Endpoints\n\n");
            for path in &self.uncovered {
                let _ = writeln!(out, "- `{path}`");
            }
        }
        out
    }
}

/// Build a coverage report from the set of endpoint paths that have security tests.
pub fn build_coverage_report(covered_paths: &[&str], gate: CoverageGate) -> SecurityCoverageReport {
    let registry = EndpointRegistry::full_catalog();
    let covered: HashSet<&str> = covered_paths.iter().copied().collect();
    let all = registry.all();
    let uncovered: Vec<String> = all
        .iter()
        .filter(|e| !covered.contains(e.path))
        .map(|e| format!("{} {}", e.method.as_str(), e.path))
        .collect();
    let critical = registry.critical_workflows();
    let covered_critical = critical.iter().filter(|e| covered.contains(e.path)).count();

    SecurityCoverageReport {
        total_endpoints: all.len(),
        covered_endpoints: all.len() - uncovered.len(),
        total_critical: critical.len(),
        covered_critical,
        uncovered,
        timestamp: Utc::now(),
        gate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_coverage_passes_gate() {
        let registry = EndpointRegistry::full_catalog();
        let paths: Vec<&str> = registry.all().iter().map(|e| e.path).collect();
        let report = build_coverage_report(&paths, CoverageGate::default());
        assert!(report.passes_gate());
        assert_eq!(report.endpoint_coverage_pct(), 100.0);
    }

    #[test]
    fn partial_coverage_fails_gate() {
        let report = build_coverage_report(&["/health"], CoverageGate::default());
        assert!(!report.passes_gate());
    }
}
