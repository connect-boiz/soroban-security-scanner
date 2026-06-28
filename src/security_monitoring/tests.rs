//! End-to-end integration tests across the security-monitoring pipeline:
//! detection → correlation → alerting → SIEM → playbooks → metrics.

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// A channel that counts deliveries per kind.
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

/// A SIEM sink shared with the test via Arc.
struct SharedSink(Arc<InMemorySiemSink>);
impl SiemSink for SharedSink {
    fn ship(&self, record: &str) -> Result<(), String> {
        self.0.ship(record)
    }
}

fn build_monitor(alerts: Arc<AtomicUsize>, sink: Arc<InMemorySiemSink>) -> SecurityMonitor {
    let mut dispatcher = AlertDispatcher::new(AlertRouting::default());
    for kind in [
        ChannelKind::Email,
        ChannelKind::Slack,
        ChannelKind::Sms,
        ChannelKind::PagerDuty,
    ] {
        dispatcher.register(Box::new(CountingChannel {
            kind,
            count: Arc::clone(&alerts),
        }));
    }
    let siem = SiemForwarder::new(SiemFormat::SplunkHec, Box::new(SharedSink(sink)));
    SecurityMonitor::new(
        DetectionConfig::default(),
        AnomalyConfig::default(),
        dispatcher,
        PlaybookRegistry::with_defaults(),
        Some(siem),
    )
}

#[test]
fn critical_attack_is_detected_alerted_and_auto_responded() {
    let alerts = Arc::new(AtomicUsize::new(0));
    let sink = Arc::new(InMemorySiemSink::new());
    let mut mon = build_monitor(Arc::clone(&alerts), Arc::clone(&sink));

    let event = SecurityEvent::new(
        1_700_000_000,
        EventKind::AttackSignature,
        Component::Network,
        SecuritySeverity::High,
    )
    .with_ip("8.8.8.8")
    .with_detail("path traversal in /api/upload");

    let outcome = mon.ingest(&event);

    // Detected as critical.
    assert_eq!(outcome.findings[0].severity, SecuritySeverity::Critical);
    // SIEM received the raw event.
    assert_eq!(sink.len(), 1);
    // P1 fans out to all four channels.
    assert_eq!(alerts.load(Ordering::Relaxed), 4);
    // The critical-attack playbook ran with on-call paging.
    assert!(outcome
        .playbook_runs
        .iter()
        .any(|r| r.executed.contains(&ResponseAction::PageOnCall)));
    // MTTD target met (detected same instant).
    assert!(mon.meets_mttd_target());
}

#[test]
fn brute_force_campaign_correlates_and_escalates() {
    let alerts = Arc::new(AtomicUsize::new(0));
    let sink = Arc::new(InMemorySiemSink::new());
    let mut mon = build_monitor(alerts, sink);

    // Six auth failures from one principal cross the brute-force threshold.
    for i in 0..6 {
        mon.ingest(
            &SecurityEvent::new(
                1_700_000_000 + i,
                EventKind::AuthFailure,
                Component::Auth,
                SecuritySeverity::Low,
            )
            .with_principal("victim")
            .with_ip("203.0.113.7"),
        );
    }

    let incidents = mon.incidents();
    assert_eq!(
        incidents.len(),
        1,
        "all failures correlate into one incident"
    );
    assert!(incidents[0].finding_count >= 1);
    assert_eq!(incidents[0].severity, SecuritySeverity::High);
}

#[test]
fn metrics_and_posture_reflect_resolution() {
    let alerts = Arc::new(AtomicUsize::new(0));
    let sink = Arc::new(InMemorySiemSink::new());
    let mut mon = build_monitor(alerts, sink);

    let outcome = mon.ingest(
        &SecurityEvent::new(
            1000,
            EventKind::AttackSignature,
            Component::Application,
            SecuritySeverity::High,
        )
        .with_ip("1.2.3.4"),
    );
    let id = outcome.incidents[0];

    // Open critical incident drags posture below 100.
    assert!(mon.metrics().posture_score < 100.0);

    // Acknowledge then resolve.
    assert!(mon.acknowledge(id, 1100));
    assert!(mon.resolve(id, 1400));

    let metrics = mon.metrics();
    assert_eq!(metrics.open_incidents, 0);
    assert!(metrics.mttr_secs > 0.0);
    assert_eq!(metrics.posture_score, 100.0);
}

#[test]
fn multi_component_monitoring() {
    let alerts = Arc::new(AtomicUsize::new(0));
    let sink = Arc::new(InMemorySiemSink::new());
    let mut mon = build_monitor(alerts, Arc::clone(&sink));

    // Events from network, database and application layers.
    for (component, kind) in [
        (Component::Network, EventKind::AttackSignature),
        (Component::Database, EventKind::SensitiveChange),
        (Component::Application, EventKind::AuthzViolation),
    ] {
        mon.ingest(
            &SecurityEvent::new(1000, kind, component, SecuritySeverity::Medium)
                .with_principal(&format!("{component:?}")),
        );
    }
    // Every event was forwarded to the SIEM regardless of component.
    assert_eq!(sink.len(), 3);
}

#[test]
fn dashboard_surfaces_threat_landscape() {
    let alerts = Arc::new(AtomicUsize::new(0));
    let sink = Arc::new(InMemorySiemSink::new());
    let mut mon = build_monitor(alerts, sink);

    mon.ingest(
        &SecurityEvent::new(
            1000,
            EventKind::AttackSignature,
            Component::Network,
            SecuritySeverity::High,
        )
        .with_ip("1.1.1.1"),
    ); // P1
    mon.ingest(
        &SecurityEvent::new(
            1000,
            EventKind::SensitiveChange,
            Component::Database,
            SecuritySeverity::Medium,
        )
        .with_principal("svc-account"),
    ); // P3

    let dash = mon.dashboard();
    assert_eq!(dash.top_incidents[0].priority(), Priority::P1);
    assert_eq!(dash.by_priority[0], 1); // one P1
    assert_eq!(dash.by_priority[2], 1); // one P3
    assert_eq!(dash.metrics.total_incidents, 2);
}

/// A SIEM sink that records nothing but proves the forwarder is invoked even
/// when no rule fires (raw telemetry capture).
#[test]
fn benign_events_still_feed_siem() {
    let records = Arc::new(Mutex::new(0usize));
    struct CountSink(Arc<Mutex<usize>>);
    impl SiemSink for CountSink {
        fn ship(&self, _r: &str) -> Result<(), String> {
            *self.0.lock().unwrap() += 1;
            Ok(())
        }
    }
    let mut dispatcher = AlertDispatcher::new(AlertRouting::default());
    dispatcher.register(Box::new(CountingChannel {
        kind: ChannelKind::Email,
        count: Arc::new(AtomicUsize::new(0)),
    }));
    let mut mon = SecurityMonitor::new(
        DetectionConfig::default(),
        AnomalyConfig::default(),
        dispatcher,
        PlaybookRegistry::with_defaults(),
        Some(SiemForwarder::new(
            SiemFormat::ElkEcs,
            Box::new(CountSink(Arc::clone(&records))),
        )),
    );

    // An auth success is benign (no finding) but still telemetry.
    let outcome = mon.ingest(
        &SecurityEvent::new(
            1000,
            EventKind::AuthSuccess,
            Component::Auth,
            SecuritySeverity::Info,
        )
        .with_principal("alice"),
    );
    assert!(outcome.findings.is_empty());
    assert_eq!(*records.lock().unwrap(), 1);
}
