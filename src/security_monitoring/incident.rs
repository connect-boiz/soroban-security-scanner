//! Incident model: correlation, prioritization and lifecycle.
//!
//! Findings about the same subject are correlated into a single [`Incident`].
//! Each incident tracks its lifecycle (open → acknowledged → resolved) with the
//! timestamps needed to compute MTTD and MTTR, and a priority derived from the
//! highest observed severity and the number of correlated findings.

use crate::security_monitoring::detection::Finding;
use crate::security_monitoring::event::SecuritySeverity;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Lifecycle state of an incident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IncidentStatus {
    /// Newly opened, not yet triaged.
    Open,
    /// A responder has acknowledged it.
    Acknowledged,
    /// Resolved/closed.
    Resolved,
}

/// Priority bucket for triage ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    /// P4 — lowest.
    P4,
    /// P3.
    P3,
    /// P2.
    P2,
    /// P1 — highest, page immediately.
    P1,
}

/// A correlated security incident.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Incident {
    /// Unique incident id.
    pub id: Uuid,
    /// The subject (principal/IP) the incident concerns.
    pub subject: String,
    /// Highest severity observed across correlated findings.
    pub severity: SecuritySeverity,
    /// Rule ids that contributed.
    pub rules: Vec<String>,
    /// Number of correlated findings.
    pub finding_count: u32,
    /// Current status.
    pub status: IncidentStatus,
    /// When the first contributing event occurred (unix secs).
    pub first_event_at: i64,
    /// When the incident was detected/opened (unix secs).
    pub detected_at: i64,
    /// When acknowledged (unix secs), if so.
    pub acknowledged_at: Option<i64>,
    /// When resolved (unix secs), if so.
    pub resolved_at: Option<i64>,
}

impl Incident {
    /// Opens an incident from an initial finding. `first_event_at` is the
    /// originating event time (often == finding.at) used for MTTD.
    pub fn open(finding: &Finding, first_event_at: i64, detected_at: i64) -> Self {
        Self {
            id: Uuid::new_v4(),
            subject: finding.subject.clone(),
            severity: finding.severity,
            rules: vec![finding.rule.clone()],
            finding_count: 1,
            status: IncidentStatus::Open,
            first_event_at,
            detected_at,
            acknowledged_at: None,
            resolved_at: None,
        }
    }

    /// Correlates another finding into this incident, escalating severity and
    /// recording the rule.
    pub fn correlate(&mut self, finding: &Finding) {
        self.finding_count += 1;
        if finding.severity > self.severity {
            self.severity = finding.severity;
        }
        if !self.rules.contains(&finding.rule) {
            self.rules.push(finding.rule.clone());
        }
    }

    /// Priority derived from severity and corroborating finding count.
    pub fn priority(&self) -> Priority {
        match self.severity {
            SecuritySeverity::Critical => Priority::P1,
            SecuritySeverity::High => {
                if self.finding_count >= 3 {
                    Priority::P1
                } else {
                    Priority::P2
                }
            }
            SecuritySeverity::Medium => Priority::P3,
            SecuritySeverity::Low | SecuritySeverity::Info => Priority::P4,
        }
    }

    /// Mean-time-to-detect for this incident: detection minus first event.
    pub fn mttd_secs(&self) -> i64 {
        (self.detected_at - self.first_event_at).max(0)
    }

    /// Acknowledges the incident.
    pub fn acknowledge(&mut self, at: i64) {
        if self.status == IncidentStatus::Open {
            self.status = IncidentStatus::Acknowledged;
            self.acknowledged_at = Some(at);
        }
    }

    /// Resolves the incident.
    pub fn resolve(&mut self, at: i64) {
        self.status = IncidentStatus::Resolved;
        self.resolved_at = Some(at);
    }

    /// Mean-time-to-resolve, if resolved.
    pub fn mttr_secs(&self) -> Option<i64> {
        self.resolved_at.map(|r| (r - self.detected_at).max(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(rule: &str, sev: SecuritySeverity) -> Finding {
        Finding {
            rule: rule.to_string(),
            subject: "alice".to_string(),
            severity: sev,
            detail: "x".to_string(),
            at: 1000,
        }
    }

    #[test]
    fn correlation_escalates_severity_and_counts() {
        let mut inc = Incident::open(&finding("brute-force", SecuritySeverity::Medium), 990, 1000);
        inc.correlate(&finding("attack-signature", SecuritySeverity::Critical));
        assert_eq!(inc.severity, SecuritySeverity::Critical);
        assert_eq!(inc.finding_count, 2);
        assert_eq!(inc.rules.len(), 2);
    }

    #[test]
    fn duplicate_rule_not_double_listed() {
        let mut inc = Incident::open(&finding("brute-force", SecuritySeverity::High), 990, 1000);
        inc.correlate(&finding("brute-force", SecuritySeverity::High));
        assert_eq!(inc.rules.len(), 1);
        assert_eq!(inc.finding_count, 2);
    }

    #[test]
    fn priority_mapping() {
        let crit = Incident::open(&finding("x", SecuritySeverity::Critical), 1, 1);
        assert_eq!(crit.priority(), Priority::P1);

        let mut high = Incident::open(&finding("x", SecuritySeverity::High), 1, 1);
        assert_eq!(high.priority(), Priority::P2);
        high.correlate(&finding("y", SecuritySeverity::High));
        high.correlate(&finding("z", SecuritySeverity::High));
        assert_eq!(high.priority(), Priority::P1); // 3 findings escalate

        let med = Incident::open(&finding("x", SecuritySeverity::Medium), 1, 1);
        assert_eq!(med.priority(), Priority::P3);
    }

    #[test]
    fn mttd_and_mttr() {
        let mut inc = Incident::open(&finding("x", SecuritySeverity::High), 1000, 1120);
        assert_eq!(inc.mttd_secs(), 120); // detected 2 min after first event
        assert!(inc.mttr_secs().is_none());
        inc.acknowledge(1200);
        inc.resolve(1600);
        assert_eq!(inc.mttr_secs(), Some(480));
        assert_eq!(inc.status, IncidentStatus::Resolved);
    }

    #[test]
    fn priority_ordering() {
        assert!(Priority::P1 > Priority::P2);
        assert!(Priority::P2 > Priority::P3);
    }
}
