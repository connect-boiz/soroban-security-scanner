//! Security playbooks: automated response procedures.
//!
//! A playbook binds a trigger condition (priority and/or specific rules) to an
//! ordered list of response actions. When an incident matches, the engine
//! executes the actions and records the run for audit.

use crate::security_monitoring::incident::{Incident, Priority};
use serde::{Deserialize, Serialize};

/// An automated response action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseAction {
    /// Block a source IP at the edge.
    BlockIp,
    /// Temporarily disable the affected account.
    DisableAccount,
    /// Force re-authentication / revoke sessions.
    RevokeSessions,
    /// Open a ticket in the incident tracker.
    OpenTicket,
    /// Page the on-call responder.
    PageOnCall,
    /// Capture a forensic snapshot.
    SnapshotForensics,
}

/// The condition under which a playbook triggers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trigger {
    /// Minimum incident priority required.
    pub min_priority: Priority,
    /// If non-empty, at least one of these rules must be present on the incident.
    pub any_rules: Vec<String>,
}

impl Trigger {
    /// Whether `incident` satisfies this trigger.
    pub fn matches(&self, incident: &Incident) -> bool {
        if incident.priority() < self.min_priority {
            return false;
        }
        if self.any_rules.is_empty() {
            return true;
        }
        self.any_rules.iter().any(|r| incident.rules.contains(r))
    }
}

/// A named playbook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Playbook {
    /// Playbook name.
    pub name: String,
    /// Trigger condition.
    pub trigger: Trigger,
    /// Ordered response actions.
    pub actions: Vec<ResponseAction>,
}

/// Record of an executed playbook against an incident.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaybookRun {
    /// Playbook that ran.
    pub playbook: String,
    /// Incident subject it acted on.
    pub subject: String,
    /// Actions that were executed, in order.
    pub executed: Vec<ResponseAction>,
}

/// Registry of playbooks, evaluated against incidents.
#[derive(Default)]
pub struct PlaybookRegistry {
    playbooks: Vec<Playbook>,
}

impl PlaybookRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a playbook.
    pub fn register(&mut self, playbook: Playbook) {
        self.playbooks.push(playbook);
    }

    /// Returns the standard default playbooks (brute force, attack signature).
    pub fn with_defaults() -> Self {
        let mut reg = Self::new();
        reg.register(Playbook {
            name: "critical-attack-response".to_string(),
            trigger: Trigger {
                min_priority: Priority::P1,
                any_rules: vec![],
            },
            actions: vec![
                ResponseAction::PageOnCall,
                ResponseAction::SnapshotForensics,
                ResponseAction::OpenTicket,
            ],
        });
        reg.register(Playbook {
            name: "brute-force-lockout".to_string(),
            trigger: Trigger {
                min_priority: Priority::P2,
                any_rules: vec!["credential-brute-force".to_string()],
            },
            actions: vec![ResponseAction::BlockIp, ResponseAction::RevokeSessions],
        });
        reg
    }

    /// Executes all matching playbooks for an incident, returning the runs.
    pub fn execute(&self, incident: &Incident) -> Vec<PlaybookRun> {
        self.playbooks
            .iter()
            .filter(|p| p.trigger.matches(incident))
            .map(|p| PlaybookRun {
                playbook: p.name.clone(),
                subject: incident.subject.clone(),
                executed: p.actions.clone(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security_monitoring::detection::Finding;
    use crate::security_monitoring::event::SecuritySeverity;

    fn incident(sev: SecuritySeverity, rule: &str) -> Incident {
        Incident::open(
            &Finding {
                rule: rule.to_string(),
                subject: "alice".to_string(),
                severity: sev,
                detail: "d".to_string(),
                at: 1000,
            },
            1000,
            1010,
        )
    }

    #[test]
    fn critical_triggers_attack_response() {
        let reg = PlaybookRegistry::with_defaults();
        let runs = reg.execute(&incident(SecuritySeverity::Critical, "attack-signature"));
        assert!(runs
            .iter()
            .any(|r| r.playbook == "critical-attack-response"));
        let run = runs
            .iter()
            .find(|r| r.playbook == "critical-attack-response")
            .unwrap();
        assert!(run.executed.contains(&ResponseAction::PageOnCall));
    }

    #[test]
    fn brute_force_playbook_requires_matching_rule() {
        let reg = PlaybookRegistry::with_defaults();
        // High + 3 findings → P1, with the brute-force rule present.
        let mut inc = incident(SecuritySeverity::High, "credential-brute-force");
        inc.correlate(&Finding {
            rule: "credential-brute-force".to_string(),
            subject: "alice".to_string(),
            severity: SecuritySeverity::High,
            detail: "d".to_string(),
            at: 1001,
        });
        let runs = reg.execute(&inc);
        assert!(runs.iter().any(|r| r.playbook == "brute-force-lockout"));
    }

    #[test]
    fn low_priority_triggers_nothing() {
        let reg = PlaybookRegistry::with_defaults();
        let runs = reg.execute(&incident(SecuritySeverity::Low, "x"));
        assert!(runs.is_empty());
    }

    #[test]
    fn trigger_rule_filter() {
        let t = Trigger {
            min_priority: Priority::P3,
            any_rules: vec!["api-abuse".to_string()],
        };
        let inc = incident(SecuritySeverity::Medium, "sensitive-change"); // P3 but wrong rule
        assert!(!t.matches(&inc));
        let inc2 = incident(SecuritySeverity::Medium, "api-abuse");
        assert!(t.matches(&inc2));
    }
}
