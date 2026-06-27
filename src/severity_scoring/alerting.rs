//! Severity-based alerting and notification thresholds.
//!
//! Translates a computed [`SeverityScore`] (or a recalculation outcome) into
//! notification decisions: whether to alert, at what channel urgency, and why.

use serde::{Deserialize, Serialize};

use super::engine::{RecalcOutcome, SeverityScore};
use crate::Severity;

/// Urgency of a notification, used to pick a delivery channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NotificationUrgency {
    /// No notification required.
    Silent,
    /// Low-priority digest.
    Info,
    /// Standard notification.
    Warning,
    /// Immediate, high-priority page.
    Critical,
}

/// An emitted alert.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeverityAlert {
    pub urgency: NotificationUrgency,
    pub score: f64,
    pub severity: Severity,
    pub message: String,
}

/// Threshold policy mapping scores/severities to notification urgency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// Minimum contextual score that triggers any notification.
    pub notify_at_or_above: f64,
    /// Minimum contextual score that triggers a critical page.
    pub critical_at_or_above: f64,
    /// Always alert when a recalculation escalates severity, even below the
    /// notify threshold.
    pub alert_on_escalation: bool,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            notify_at_or_above: 4.0,  // Medium and above
            critical_at_or_above: 9.0, // Critical
            alert_on_escalation: true,
        }
    }
}

impl AlertThresholds {
    pub fn new(notify_at_or_above: f64, critical_at_or_above: f64) -> Self {
        Self {
            notify_at_or_above,
            critical_at_or_above,
            alert_on_escalation: true,
        }
    }

    /// Decide on an alert for a freshly-computed score.
    pub fn evaluate(&self, id: &str, score: &SeverityScore) -> Option<SeverityAlert> {
        let urgency = self.urgency_for(score.contextual_score);
        if urgency == NotificationUrgency::Silent {
            return None;
        }
        Some(SeverityAlert {
            urgency,
            score: score.contextual_score,
            severity: score.severity,
            message: format!(
                "finding {} scored {:.1} ({})",
                id,
                score.contextual_score,
                score.severity.as_str()
            ),
        })
    }

    /// Decide on an alert for a recalculation outcome, honoring the escalation
    /// rule even when the absolute score is below the notify threshold.
    pub fn evaluate_recalc(&self, outcome: &RecalcOutcome) -> Option<SeverityAlert> {
        let base_urgency = self.urgency_for(outcome.new_score);

        if base_urgency == NotificationUrgency::Silent {
            if self.alert_on_escalation && outcome.escalated {
                return Some(SeverityAlert {
                    urgency: NotificationUrgency::Info,
                    score: outcome.new_score,
                    severity: outcome.new_severity,
                    message: format!(
                        "finding {} risk escalated {:.1} -> {:.1}",
                        outcome.id, outcome.previous_score, outcome.new_score
                    ),
                });
            }
            return None;
        }

        Some(SeverityAlert {
            urgency: base_urgency,
            score: outcome.new_score,
            severity: outcome.new_severity,
            message: format!(
                "finding {} now {:.1} ({}){}",
                outcome.id,
                outcome.new_score,
                outcome.new_severity.as_str(),
                if outcome.escalated { " [escalated]" } else { "" }
            ),
        })
    }

    fn urgency_for(&self, score: f64) -> NotificationUrgency {
        if score >= self.critical_at_or_above {
            NotificationUrgency::Critical
        } else if score >= self.notify_at_or_above {
            NotificationUrgency::Warning
        } else {
            // Below the notify threshold: silent unless the escalation rule in
            // `evaluate_recalc` decides otherwise.
            NotificationUrgency::Silent
        }
    }
}
