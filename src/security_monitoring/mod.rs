//! Real-time security monitoring, alerting and incident response (issue #333).
//!
//! A self-contained security operations pipeline: it ingests security events
//! from every infrastructure component, detects threats with both fixed rules
//! and an unsupervised ML anomaly model, correlates findings into prioritized
//! incidents, fans out real-time alerts across channels, forwards telemetry to
//! a SIEM, runs automated response playbooks, and reports MTTD/MTTR and a
//! security-posture score.
//!
//! # Acceptance-criteria mapping
//!
//! | Requirement | Where |
//! |-------------|-------|
//! | Monitoring for auth failures / authz violations / suspicious activity | [`detection::RuleEngine`] |
//! | Real-time multi-channel alerting (email/SMS/Slack/PagerDuty) | [`alerting::AlertDispatcher`] |
//! | Security dashboard (threats, trends, incident tracking) | [`engine::SecurityMonitor::dashboard`] |
//! | Automated incident-response triggers | [`playbook::PlaybookRegistry`] |
//! | SIEM integration (Splunk / ELK / AWS Security Hub) | [`siem::SiemForwarder`] |
//! | ML anomaly detection | [`anomaly::AnomalyDetector`] |
//! | Security metrics (MTTD, MTTR, posture score) | [`metrics::compute_metrics`] |
//! | Security playbooks with automated responses | [`playbook::Playbook`] |
//! | Event correlation & incident prioritization | [`incident::Incident`] |
//! | Monitoring across network/database/application | [`event::Component`] |
//! | <5 minute MTTD for critical events | [`metrics::MTTD_TARGET_SECS`], [`engine::SecurityMonitor::meets_mttd_target`] |
//! | Comprehensive testing | per-module tests + [`tests`] |
//!
//! # Example
//!
//! ```
//! use soroban_security_scanner::security_monitoring::*;
//!
//! let mut dispatcher = AlertDispatcher::new(AlertRouting::default());
//! let mut monitor = SecurityMonitor::new(
//!     DetectionConfig::default(),
//!     AnomalyConfig::default(),
//!     dispatcher,
//!     PlaybookRegistry::with_defaults(),
//!     None,
//! );
//!
//! let event = SecurityEvent::new(1_700_000_000, EventKind::AttackSignature, Component::Network, SecuritySeverity::High)
//!     .with_ip("8.8.8.8")
//!     .with_detail("SQL injection attempt");
//! let outcome = monitor.ingest(&event);
//! assert_eq!(outcome.findings[0].severity, SecuritySeverity::Critical);
//! ```

pub mod alerting;
pub mod anomaly;
pub mod detection;
pub mod engine;
pub mod event;
pub mod incident;
pub mod metrics;
pub mod playbook;
pub mod siem;

#[cfg(test)]
mod tests;

pub use alerting::{
    AlertDispatcher, AlertMessage, AlertRouting, ChannelKind, DispatchResult, NotificationChannel,
};
pub use anomaly::{AnomalyConfig, AnomalyDetector, AnomalyScore};
pub use detection::{DetectionConfig, Finding, RuleEngine};
pub use engine::{DashboardSnapshot, ProcessOutcome, SecurityMonitor};
pub use event::{Component, EventKind, SecurityEvent, SecuritySeverity};
pub use incident::{Incident, IncidentStatus, Priority};
pub use metrics::{compute_metrics, SecurityMetrics, MTTD_TARGET_SECS};
pub use playbook::{Playbook, PlaybookRegistry, PlaybookRun, ResponseAction, Trigger};
pub use siem::{format_event, InMemorySiemSink, SiemFormat, SiemForwarder, SiemSink};
