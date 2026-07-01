//! Query execution-plan analysis.
//!
//! A small model of a query plan (the kind `EXPLAIN` returns) plus heuristics
//! that flag the expensive shapes — sequential scans over large tables and
//! plans whose estimated cost is dominated by an un-indexed step.

use serde::{Deserialize, Serialize};

/// How a table is accessed in a plan node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanType {
    /// Full sequential scan (reads every row).
    SeqScan,
    /// Index scan.
    IndexScan,
    /// Index-only scan (covered by the index).
    IndexOnlyScan,
}

/// A node in an execution plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanNode {
    /// Table accessed.
    pub table: String,
    /// Access method.
    pub scan: ScanType,
    /// Estimated rows examined.
    pub estimated_rows: u64,
    /// Estimated cost units.
    pub estimated_cost: u64,
}

/// A finding from plan analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanFinding {
    /// Table the finding concerns.
    pub table: String,
    /// Description of the issue.
    pub issue: String,
}

/// Rows over which a sequential scan is considered expensive.
pub const LARGE_TABLE_ROWS: u64 = 10_000;

/// Analyzes a plan's nodes and returns performance findings.
pub fn analyze(nodes: &[PlanNode]) -> Vec<PlanFinding> {
    let mut findings = Vec::new();
    for node in nodes {
        if node.scan == ScanType::SeqScan && node.estimated_rows >= LARGE_TABLE_ROWS {
            findings.push(PlanFinding {
                table: node.table.clone(),
                issue: format!(
                    "sequential scan over ~{} rows; add an index on the filter/join column",
                    node.estimated_rows
                ),
            });
        }
    }
    findings
}

/// Whether a plan is acceptable (no expensive sequential scans).
pub fn is_efficient(nodes: &[PlanNode]) -> bool {
    analyze(nodes).is_empty()
}

/// Total estimated cost of a plan.
pub fn total_cost(nodes: &[PlanNode]) -> u64 {
    nodes.iter().map(|n| n.estimated_cost).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_seq_scan_flagged() {
        let nodes = vec![PlanNode {
            table: "events".to_string(),
            scan: ScanType::SeqScan,
            estimated_rows: 1_000_000,
            estimated_cost: 50_000,
        }];
        let findings = analyze(&nodes);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].issue.contains("sequential scan"));
        assert!(!is_efficient(&nodes));
    }

    #[test]
    fn small_seq_scan_ok() {
        let nodes = vec![PlanNode {
            table: "config".to_string(),
            scan: ScanType::SeqScan,
            estimated_rows: 50,
            estimated_cost: 10,
        }];
        assert!(is_efficient(&nodes));
    }

    #[test]
    fn index_scans_are_efficient() {
        let nodes = vec![
            PlanNode {
                table: "users".to_string(),
                scan: ScanType::IndexScan,
                estimated_rows: 1_000_000,
                estimated_cost: 8,
            },
            PlanNode {
                table: "orders".to_string(),
                scan: ScanType::IndexOnlyScan,
                estimated_rows: 500_000,
                estimated_cost: 4,
            },
        ];
        assert!(is_efficient(&nodes));
        assert_eq!(total_cost(&nodes), 12);
    }
}
