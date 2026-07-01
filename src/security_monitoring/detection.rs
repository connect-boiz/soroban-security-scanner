//! Rule-based security detection.
//!
//! Stateful detectors that watch the event stream for known-bad patterns —
//! credential brute force, repeated authorization violations, and abusive API
//! usage — and emit [`Finding`]s. Thresholds are evaluated over a sliding time
//! window keyed by actor/IP.

use crate::security_monitoring::event::{EventKind, SecurityEvent, SecuritySeverity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A detection produced by a rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// Stable rule id that fired.
    pub rule: String,
    /// Correlation key (principal/IP) the finding is about.
    pub subject: String,
    /// Severity of the finding.
    pub severity: SecuritySeverity,
    /// Human-readable description.
    pub detail: String,
    /// When it fired (unix seconds).
    pub at: i64,
}

/// Thresholds controlling the detectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectionConfig {
    /// Auth failures within the window that trigger a brute-force finding.
    pub auth_failure_threshold: u32,
    /// Authorization violations within the window that trigger a finding.
    pub authz_violation_threshold: u32,
    /// API events within the window that count as abusive.
    pub api_rate_threshold: u32,
    /// Sliding window length in seconds.
    pub window_secs: i64,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            auth_failure_threshold: 5,
            authz_violation_threshold: 3,
            api_rate_threshold: 100,
            window_secs: 300,
        }
    }
}

/// Tracks event timestamps per (rule, subject) for windowed counting.
#[derive(Default)]
pub struct RuleEngine {
    config: DetectionConfig,
    auth_failures: HashMap<String, Vec<i64>>,
    authz_violations: HashMap<String, Vec<i64>>,
    api_events: HashMap<String, Vec<i64>>,
}

impl RuleEngine {
    /// Creates a rule engine.
    pub fn new(config: DetectionConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    /// Feeds an event through the rules, returning any findings it triggers.
    pub fn evaluate(&mut self, event: &SecurityEvent) -> Vec<Finding> {
        let key = event.correlation_key();
        let mut findings = Vec::new();
        let window = self.config.window_secs;

        match event.kind {
            EventKind::AuthFailure => {
                let count = record_and_count(&mut self.auth_failures, &key, event.at, window);
                if count >= self.config.auth_failure_threshold {
                    findings.push(Finding {
                        rule: "credential-brute-force".to_string(),
                        subject: key,
                        severity: SecuritySeverity::High,
                        detail: format!(
                            "{count} auth failures within {}s",
                            self.config.window_secs
                        ),
                        at: event.at,
                    });
                }
            }
            EventKind::AuthzViolation => {
                let count = record_and_count(&mut self.authz_violations, &key, event.at, window);
                if count >= self.config.authz_violation_threshold {
                    findings.push(Finding {
                        rule: "privilege-escalation-attempt".to_string(),
                        subject: key,
                        severity: SecuritySeverity::High,
                        detail: format!("{count} authorization violations within {window}s"),
                        at: event.at,
                    });
                }
            }
            EventKind::SuspiciousApiUsage => {
                let count = record_and_count(&mut self.api_events, &key, event.at, window);
                if count >= self.config.api_rate_threshold {
                    findings.push(Finding {
                        rule: "api-abuse".to_string(),
                        subject: key,
                        severity: SecuritySeverity::Medium,
                        detail: format!("{count} API events within {window}s"),
                        at: event.at,
                    });
                }
            }
            EventKind::AttackSignature => {
                // An attack signature is actionable on first sight.
                findings.push(Finding {
                    rule: "attack-signature".to_string(),
                    subject: key,
                    severity: SecuritySeverity::Critical,
                    detail: if event.detail.is_empty() {
                        "attack signature matched".to_string()
                    } else {
                        event.detail.clone()
                    },
                    at: event.at,
                });
            }
            EventKind::SensitiveChange => {
                findings.push(Finding {
                    rule: "sensitive-change".to_string(),
                    subject: key,
                    severity: SecuritySeverity::Medium,
                    detail: "change to a sensitive resource".to_string(),
                    at: event.at,
                });
            }
            EventKind::AuthSuccess => {}
        }

        findings
    }
}

/// Records a timestamp under `key`, prunes outside the window, and returns the
/// in-window count.
fn record_and_count(map: &mut HashMap<String, Vec<i64>>, key: &str, at: i64, window: i64) -> u32 {
    let cutoff = at - window;
    let entry = map.entry(key.to_string()).or_default();
    entry.push(at);
    entry.retain(|t| *t > cutoff);
    entry.len() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security_monitoring::event::Component;

    fn engine() -> RuleEngine {
        RuleEngine::new(DetectionConfig::default())
    }

    fn auth_fail(at: i64, who: &str) -> SecurityEvent {
        SecurityEvent::new(
            at,
            EventKind::AuthFailure,
            Component::Auth,
            SecuritySeverity::Low,
        )
        .with_principal(who)
    }

    #[test]
    fn brute_force_fires_at_threshold() {
        let mut e = engine();
        for i in 0..4 {
            assert!(e.evaluate(&auth_fail(1000 + i, "alice")).is_empty());
        }
        let findings = e.evaluate(&auth_fail(1004, "alice")); // 5th
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule, "credential-brute-force");
        assert_eq!(findings[0].severity, SecuritySeverity::High);
    }

    #[test]
    fn failures_outside_window_do_not_count() {
        let mut e = engine();
        for i in 0..4 {
            e.evaluate(&auth_fail(1000 + i, "bob"));
        }
        // Far in the future: window resets, single failure shouldn't fire.
        assert!(e.evaluate(&auth_fail(1000 + 10_000, "bob")).is_empty());
    }

    #[test]
    fn distinct_subjects_are_independent() {
        let mut e = engine();
        for i in 0..5 {
            e.evaluate(&auth_fail(1000 + i, "alice"));
        }
        // Bob's first failure should not fire from Alice's count.
        assert!(e.evaluate(&auth_fail(1006, "bob")).is_empty());
    }

    #[test]
    fn attack_signature_fires_immediately_as_critical() {
        let mut e = engine();
        let ev = SecurityEvent::new(
            1000,
            EventKind::AttackSignature,
            Component::Network,
            SecuritySeverity::High,
        )
        .with_ip("8.8.8.8")
        .with_detail("SQL injection in /api/scan");
        let findings = e.evaluate(&ev);
        assert_eq!(findings[0].severity, SecuritySeverity::Critical);
        assert_eq!(findings[0].detail, "SQL injection in /api/scan");
    }

    #[test]
    fn authz_violation_threshold() {
        let mut e = engine();
        let mk = |at: i64| {
            SecurityEvent::new(
                at,
                EventKind::AuthzViolation,
                Component::Application,
                SecuritySeverity::Medium,
            )
            .with_principal("mallory")
        };
        assert!(e.evaluate(&mk(1000)).is_empty());
        assert!(e.evaluate(&mk(1001)).is_empty());
        let f = e.evaluate(&mk(1002)); // 3rd
        assert_eq!(f[0].rule, "privilege-escalation-attempt");
    }
}
