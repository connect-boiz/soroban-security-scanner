//! Security event model.
//!
//! The common vocabulary the monitoring pipeline ingests: a typed security
//! event tagged with the infrastructure component it came from, a severity, and
//! the actor/source identifiers used for correlation.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Severity of a security event or finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SecuritySeverity {
    /// Informational.
    Info,
    /// Low.
    Low,
    /// Medium.
    Medium,
    /// High.
    High,
    /// Critical — drives the <5 minute MTTD target.
    Critical,
}

impl SecuritySeverity {
    /// Stable label.
    pub fn as_str(&self) -> &'static str {
        match self {
            SecuritySeverity::Info => "INFO",
            SecuritySeverity::Low => "LOW",
            SecuritySeverity::Medium => "MEDIUM",
            SecuritySeverity::High => "HIGH",
            SecuritySeverity::Critical => "CRITICAL",
        }
    }

    /// Numeric weight for prioritization/scoring.
    pub fn weight(&self) -> u32 {
        match self {
            SecuritySeverity::Info => 0,
            SecuritySeverity::Low => 1,
            SecuritySeverity::Medium => 2,
            SecuritySeverity::High => 3,
            SecuritySeverity::Critical => 4,
        }
    }
}

/// The infrastructure component a security event originates from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Component {
    /// Application layer (API handlers, business logic).
    Application,
    /// Database layer.
    Database,
    /// Network/edge layer.
    Network,
    /// Authentication subsystem.
    Auth,
}

/// The kind of security event observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventKind {
    /// A failed authentication attempt.
    AuthFailure,
    /// A successful authentication (used for baselining).
    AuthSuccess,
    /// An authorization/permission violation.
    AuthzViolation,
    /// API call rate or pattern that looks abusive.
    SuspiciousApiUsage,
    /// Input that matches an attack signature (injection, traversal, …).
    AttackSignature,
    /// A configuration or integrity change to a sensitive resource.
    SensitiveChange,
}

impl EventKind {
    /// Whether this kind, on its own, is security-relevant (vs. baseline noise).
    pub fn is_security_relevant(&self) -> bool {
        !matches!(self, EventKind::AuthSuccess)
    }
}

/// A single security event ingested by the monitor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// Unique event id.
    pub id: Uuid,
    /// When the event occurred (unix seconds).
    pub at: i64,
    /// What kind of event it is.
    pub kind: EventKind,
    /// Which component reported it.
    pub component: Component,
    /// Intrinsic severity of the event.
    pub severity: SecuritySeverity,
    /// Source IP (string form), if known.
    pub source_ip: Option<String>,
    /// Acting user/principal, if known.
    pub principal: Option<String>,
    /// Free-form detail.
    pub detail: String,
}

impl SecurityEvent {
    /// Builds an event with a generated id.
    pub fn new(at: i64, kind: EventKind, component: Component, severity: SecuritySeverity) -> Self {
        Self {
            id: Uuid::new_v4(),
            at,
            kind,
            component,
            severity,
            source_ip: None,
            principal: None,
            detail: String::new(),
        }
    }

    /// Sets the source IP.
    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.source_ip = Some(ip.into());
        self
    }

    /// Sets the acting principal.
    pub fn with_principal(mut self, principal: impl Into<String>) -> Self {
        self.principal = Some(principal.into());
        self
    }

    /// Sets the detail message.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }

    /// The correlation key: prefer principal, fall back to IP, else "unknown".
    pub fn correlation_key(&self) -> String {
        self.principal
            .clone()
            .or_else(|| self.source_ip.clone())
            .unwrap_or_else(|| "unknown".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_ordering_and_weight() {
        assert!(SecuritySeverity::Critical > SecuritySeverity::High);
        assert!(SecuritySeverity::Info < SecuritySeverity::Low);
        assert_eq!(SecuritySeverity::Critical.weight(), 4);
    }

    #[test]
    fn auth_success_is_not_security_relevant() {
        assert!(!EventKind::AuthSuccess.is_security_relevant());
        assert!(EventKind::AuthFailure.is_security_relevant());
    }

    #[test]
    fn correlation_prefers_principal_then_ip() {
        let e = SecurityEvent::new(
            1,
            EventKind::AuthFailure,
            Component::Auth,
            SecuritySeverity::Medium,
        )
        .with_principal("alice")
        .with_ip("10.0.0.1");
        assert_eq!(e.correlation_key(), "alice");

        let e2 = SecurityEvent::new(
            1,
            EventKind::AuthFailure,
            Component::Auth,
            SecuritySeverity::Medium,
        )
        .with_ip("10.0.0.1");
        assert_eq!(e2.correlation_key(), "10.0.0.1");

        let e3 = SecurityEvent::new(
            1,
            EventKind::AttackSignature,
            Component::Network,
            SecuritySeverity::High,
        );
        assert_eq!(e3.correlation_key(), "unknown");
    }
}
