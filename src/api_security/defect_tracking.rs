//! Security defect tracking with remediation workflows.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

/// Severity levels aligned with CI blocking policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DefectSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl DefectSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Critical => "CRITICAL",
        }
    }

    pub fn blocks_ci(&self) -> bool {
        matches!(self, Self::High | Self::Critical)
    }
}

/// Lifecycle status for a tracked security defect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefectStatus {
    Open,
    InRemediation,
    Resolved,
    Verified,
    WontFix,
}

/// A single security defect record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityDefect {
    pub id: String,
    pub title: String,
    pub severity: DefectSeverity,
    pub status: DefectStatus,
    pub endpoint: String,
    pub discovered_at: DateTime<Utc>,
    pub remediation_deadline: Option<DateTime<Utc>>,
    pub remediation_notes: String,
}

/// In-memory defect tracker with remediation workflow transitions.
#[derive(Debug, Clone, Default)]
pub struct SecurityDefectTracker {
    defects: Vec<SecurityDefect>,
}

impl SecurityDefectTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, defect: SecurityDefect) {
        self.defects.push(defect);
    }

    pub fn all(&self) -> &[SecurityDefect] {
        &self.defects
    }

    pub fn open_blocking(&self) -> Vec<&SecurityDefect> {
        self.defects
            .iter()
            .filter(|d| {
                d.severity.blocks_ci()
                    && matches!(d.status, DefectStatus::Open | DefectStatus::InRemediation)
            })
            .collect()
    }

    pub fn start_remediation(&mut self, id: &str, notes: &str) -> bool {
        if let Some(d) = self.defects.iter_mut().find(|d| d.id == id) {
            d.status = DefectStatus::InRemediation;
            d.remediation_notes = notes.to_string();
            true
        } else {
            false
        }
    }

    pub fn resolve(&mut self, id: &str, notes: &str) -> bool {
        if let Some(d) = self.defects.iter_mut().find(|d| d.id == id) {
            d.status = DefectStatus::Resolved;
            d.remediation_notes = notes.to_string();
            true
        } else {
            false
        }
    }

    pub fn verify(&mut self, id: &str) -> bool {
        if let Some(d) = self.defects.iter_mut().find(|d| d.id == id) {
            d.status = DefectStatus::Verified;
            true
        } else {
            false
        }
    }

    pub fn ci_should_block(&self) -> bool {
        !self.open_blocking().is_empty()
    }

    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# Security Defect Tracker\n");
        let _ = writeln!(
            out,
            "**Blocking defects:** {}\n",
            self.open_blocking().len()
        );
        out.push_str("| ID | Severity | Status | Endpoint | Title |\n");
        out.push_str("|----|----------|--------|----------|-------|\n");
        for d in &self.defects {
            let _ = writeln!(
                out,
                "| {} | {} | {:?} | {} | {} |",
                d.id,
                d.severity.as_str(),
                d.status,
                d.endpoint,
                d.title
            );
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_defect(severity: DefectSeverity) -> SecurityDefect {
        SecurityDefect {
            id: "SEC-001".into(),
            title: "Test defect".into(),
            severity,
            status: DefectStatus::Open,
            endpoint: "/api/admin/users".into(),
            discovered_at: Utc::now(),
            remediation_deadline: None,
            remediation_notes: String::new(),
        }
    }

    #[test]
    fn high_severity_blocks_ci_until_resolved() {
        let mut tracker = SecurityDefectTracker::new();
        tracker.register(sample_defect(DefectSeverity::High));
        assert!(tracker.ci_should_block());
        tracker.start_remediation("SEC-001", "patching auth middleware");
        assert!(tracker.ci_should_block());
        tracker.resolve("SEC-001", "fixed in commit abc");
        assert!(!tracker.ci_should_block());
    }

    #[test]
    fn low_severity_does_not_block_ci() {
        let mut tracker = SecurityDefectTracker::new();
        tracker.register(sample_defect(DefectSeverity::Low));
        assert!(!tracker.ci_should_block());
    }
}
