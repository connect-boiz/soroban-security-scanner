//! The security monitoring engine.
//!
//! Orchestrates the full pipeline for each ingested event: rule detection →
//! ML anomaly scoring → correlation into incidents → alert dispatch → SIEM
//! forwarding → automated playbook execution. Also exposes a dashboard snapshot
//! and the security metrics (MTTD/MTTR/posture).

use crate::security_monitoring::alerting::{AlertDispatcher, DispatchResult};
use crate::security_monitoring::anomaly::{AnomalyConfig, AnomalyDetector};
use crate::security_monitoring::detection::{DetectionConfig, Finding, RuleEngine};
use crate::security_monitoring::event::{SecurityEvent, SecuritySeverity};
use crate::security_monitoring::incident::{Incident, IncidentStatus, Priority};
use crate::security_monitoring::metrics::{compute_metrics, SecurityMetrics};
use crate::security_monitoring::playbook::{PlaybookRegistry, PlaybookRun};
use crate::security_monitoring::siem::SiemForwarder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Correlation window: findings about the same subject within this many seconds
/// fold into the same open incident.
const CORRELATION_WINDOW_SECS: i64 = 600;

/// What happened when an event was processed.
#[derive(Debug, Clone, Default)]
pub struct ProcessOutcome {
    /// Findings the rules produced.
    pub findings: Vec<Finding>,
    /// Incident ids opened or updated.
    pub incidents: Vec<Uuid>,
    /// Whether the ML detector flagged the event as anomalous.
    pub anomalous: bool,
    /// Alerts dispatched, keyed by incident id.
    pub dispatched: Vec<DispatchResult>,
    /// Playbook runs triggered.
    pub playbook_runs: Vec<PlaybookRun>,
}

/// A dashboard snapshot for threat visualization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DashboardSnapshot {
    /// Current security metrics.
    pub metrics: SecurityMetrics,
    /// Open incidents ordered by priority (highest first).
    pub top_incidents: Vec<Incident>,
    /// Count of incidents per priority bucket (P1..P4).
    pub by_priority: [usize; 4],
}

/// The orchestrating security monitor.
pub struct SecurityMonitor {
    rules: RuleEngine,
    anomaly: AnomalyDetector,
    dispatcher: AlertDispatcher,
    playbooks: PlaybookRegistry,
    siem: Option<SiemForwarder>,
    incidents: HashMap<Uuid, Incident>,
    /// Most recent open incident per subject, for correlation.
    open_by_subject: HashMap<String, Uuid>,
}

impl SecurityMonitor {
    /// Builds a monitor. The dispatcher should already have its channels
    /// registered; SIEM forwarding is optional.
    pub fn new(
        detection: DetectionConfig,
        anomaly: AnomalyConfig,
        dispatcher: AlertDispatcher,
        playbooks: PlaybookRegistry,
        siem: Option<SiemForwarder>,
    ) -> Self {
        Self {
            rules: RuleEngine::new(detection),
            anomaly: AnomalyDetector::new(anomaly),
            dispatcher,
            playbooks,
            siem,
            incidents: HashMap::new(),
            open_by_subject: HashMap::new(),
        }
    }

    /// Ingests and fully processes one security event.
    pub fn ingest(&mut self, event: &SecurityEvent) -> ProcessOutcome {
        let mut outcome = ProcessOutcome::default();

        // 1. Always forward to SIEM (raw telemetry), best-effort.
        if let Some(siem) = &self.siem {
            let _ = siem.forward(event);
        }

        // 2. ML anomaly scoring (1.0 per occurrence of this event kind/subject).
        let subject = event.correlation_key();
        if let Some(score) = self
            .anomaly
            .observe(&format!("{subject}:{:?}", event.kind), 1.0)
        {
            outcome.anomalous = score.anomalous;
        }

        // 3. Rule detection.
        outcome.findings = self.rules.evaluate(event);

        // 4. Correlate each finding into an incident and respond.
        for finding in &outcome.findings {
            let incident_id = self.correlate(finding, event.at);
            outcome.incidents.push(incident_id);

            let incident = self.incidents.get(&incident_id).unwrap().clone();
            // Alert.
            outcome.dispatched.push(self.dispatcher.dispatch(&incident));
            // Automated response.
            outcome
                .playbook_runs
                .extend(self.playbooks.execute(&incident));
        }

        outcome
    }

    /// Folds a finding into an existing open incident for the subject (within
    /// the correlation window) or opens a new one. Returns the incident id.
    fn correlate(&mut self, finding: &Finding, event_at: i64) -> Uuid {
        if let Some(&existing) = self.open_by_subject.get(&finding.subject) {
            if let Some(inc) = self.incidents.get_mut(&existing) {
                let recent = event_at - inc.detected_at <= CORRELATION_WINDOW_SECS;
                if inc.status != IncidentStatus::Resolved && recent {
                    inc.correlate(finding);
                    return existing;
                }
            }
        }
        // Open a new incident. Detection is "now" (event_at); first_event_at is
        // the finding's originating event time.
        let incident = Incident::open(finding, finding.at, event_at);
        let id = incident.id;
        self.open_by_subject.insert(finding.subject.clone(), id);
        self.incidents.insert(id, incident);
        id
    }

    /// Acknowledges an incident.
    pub fn acknowledge(&mut self, id: Uuid, at: i64) -> bool {
        match self.incidents.get_mut(&id) {
            Some(inc) => {
                inc.acknowledge(at);
                true
            }
            None => false,
        }
    }

    /// Resolves an incident and clears it from the open-subject index.
    pub fn resolve(&mut self, id: Uuid, at: i64) -> bool {
        match self.incidents.get_mut(&id) {
            Some(inc) => {
                inc.resolve(at);
                self.open_by_subject.retain(|_, v| *v != id);
                true
            }
            None => false,
        }
    }

    /// All incidents (unordered).
    pub fn incidents(&self) -> Vec<Incident> {
        self.incidents.values().cloned().collect()
    }

    /// Current security metrics.
    pub fn metrics(&self) -> SecurityMetrics {
        compute_metrics(&self.incidents())
    }

    /// A dashboard snapshot: metrics, top open incidents and priority counts.
    pub fn dashboard(&self) -> DashboardSnapshot {
        let mut all = self.incidents();
        let mut by_priority = [0usize; 4];
        for inc in &all {
            let idx = match inc.priority() {
                Priority::P1 => 0,
                Priority::P2 => 1,
                Priority::P3 => 2,
                Priority::P4 => 3,
            };
            by_priority[idx] += 1;
        }

        let mut open: Vec<Incident> = all
            .drain(..)
            .filter(|i| i.status != IncidentStatus::Resolved)
            .collect();
        // Highest priority first, then most severe.
        open.sort_by_key(|i| std::cmp::Reverse((i.priority(), i.severity)));
        open.truncate(10);

        DashboardSnapshot {
            metrics: self.metrics(),
            top_incidents: open,
            by_priority,
        }
    }

    /// Convenience: does the current MTTD meet the <5min critical target?
    pub fn meets_mttd_target(&self) -> bool {
        self.metrics().meets_mttd_target
    }

    /// Highest severity currently open, if any.
    pub fn worst_open_severity(&self) -> Option<SecuritySeverity> {
        self.incidents
            .values()
            .filter(|i| i.status != IncidentStatus::Resolved)
            .map(|i| i.severity)
            .max()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security_monitoring::alerting::{
        AlertMessage, AlertRouting, ChannelKind, NotificationChannel,
    };
    use crate::security_monitoring::event::{Component, EventKind};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct CountingChannel {
        kind: ChannelKind,
        count: Arc<AtomicUsize>,
    }
    impl NotificationChannel for CountingChannel {
        fn kind(&self) -> ChannelKind {
            self.kind
        }
        fn deliver(&self, _m: &AlertMessage) -> Result<(), String> {
            self.count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    fn monitor(counter: Arc<AtomicUsize>) -> SecurityMonitor {
        let mut dispatcher = AlertDispatcher::new(AlertRouting::default());
        for kind in [
            ChannelKind::Email,
            ChannelKind::Slack,
            ChannelKind::Sms,
            ChannelKind::PagerDuty,
        ] {
            dispatcher.register(Box::new(CountingChannel {
                kind,
                count: Arc::clone(&counter),
            }));
        }
        SecurityMonitor::new(
            DetectionConfig::default(),
            AnomalyConfig::default(),
            dispatcher,
            PlaybookRegistry::with_defaults(),
            None,
        )
    }

    fn auth_fail(at: i64, who: &str) -> SecurityEvent {
        SecurityEvent::new(
            at,
            EventKind::AuthFailure,
            Component::Auth,
            SecuritySeverity::Low,
        )
        .with_principal(who)
        .with_ip("10.0.0.1")
    }

    #[test]
    fn brute_force_opens_incident_and_alerts() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut mon = monitor(Arc::clone(&counter));
        let mut outcome = ProcessOutcome::default();
        for i in 0..5 {
            outcome = mon.ingest(&auth_fail(1000 + i, "alice"));
        }
        // The 5th event fires the brute-force rule and opens an incident.
        assert!(!outcome.findings.is_empty());
        assert_eq!(outcome.incidents.len(), 1);
        assert_eq!(mon.incidents().len(), 1);
        // High severity → P2 → Slack+Email (2 channels) at least once delivered.
        assert!(counter.load(Ordering::Relaxed) >= 2);
    }

    #[test]
    fn attack_signature_triggers_p1_playbook() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut mon = monitor(Arc::clone(&counter));
        let ev = SecurityEvent::new(
            1000,
            EventKind::AttackSignature,
            Component::Network,
            SecuritySeverity::High,
        )
        .with_ip("8.8.8.8")
        .with_detail("RCE attempt");
        let outcome = mon.ingest(&ev);
        assert_eq!(outcome.findings[0].severity, SecuritySeverity::Critical);
        assert!(outcome
            .playbook_runs
            .iter()
            .any(|r| r.playbook == "critical-attack-response"));
        // P1 fans out to 4 channels.
        assert_eq!(counter.load(Ordering::Relaxed), 4);
    }

    #[test]
    fn repeated_findings_correlate_into_one_incident() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut mon = monitor(counter);
        // Two attack signatures for the same subject within the window.
        let mk = |at: i64| {
            SecurityEvent::new(
                at,
                EventKind::AttackSignature,
                Component::Network,
                SecuritySeverity::High,
            )
            .with_ip("8.8.8.8")
        };
        mon.ingest(&mk(1000));
        mon.ingest(&mk(1100));
        assert_eq!(mon.incidents().len(), 1);
        assert_eq!(mon.incidents()[0].finding_count, 2);
    }

    #[test]
    fn resolve_clears_open_correlation() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut mon = monitor(counter);
        let outcome = mon.ingest(
            &SecurityEvent::new(
                1000,
                EventKind::AttackSignature,
                Component::Network,
                SecuritySeverity::High,
            )
            .with_ip("9.9.9.9"),
        );
        let id = outcome.incidents[0];
        assert!(mon.resolve(id, 1200));
        // A later event for the same subject opens a NEW incident.
        mon.ingest(
            &SecurityEvent::new(
                2000,
                EventKind::AttackSignature,
                Component::Network,
                SecuritySeverity::High,
            )
            .with_ip("9.9.9.9"),
        );
        assert_eq!(mon.incidents().len(), 2);
    }

    #[test]
    fn dashboard_orders_by_priority_and_counts() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut mon = monitor(counter);
        mon.ingest(
            &SecurityEvent::new(
                1000,
                EventKind::AttackSignature,
                Component::Network,
                SecuritySeverity::High,
            )
            .with_ip("1.1.1.1"),
        ); // critical → P1
        mon.ingest(
            &SecurityEvent::new(
                1000,
                EventKind::SensitiveChange,
                Component::Database,
                SecuritySeverity::Medium,
            )
            .with_principal("svc"),
        ); // medium → P3
        let dash = mon.dashboard();
        assert_eq!(dash.top_incidents[0].priority(), Priority::P1);
        assert_eq!(dash.by_priority[0], 1); // one P1
    }

    #[test]
    fn fast_detection_meets_mttd_target() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut mon = monitor(counter);
        mon.ingest(
            &SecurityEvent::new(
                1000,
                EventKind::AttackSignature,
                Component::Network,
                SecuritySeverity::High,
            )
            .with_ip("1.1.1.1"),
        );
        // Detected same second as the event → MTTD 0.
        assert!(mon.meets_mttd_target());
        assert_eq!(mon.worst_open_severity(), Some(SecuritySeverity::Critical));
    }
}
