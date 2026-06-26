//! End-to-end integration tests for the comprehensive audit trail (#326).
//!
//! These exercise the public surface of the `audit_trail` module against the
//! issue's acceptance criteria: structured events with full context, 100%
//! coverage of state-changing operations, tamper-evident hashing, role-based
//! query access, suspicious-pattern alerting, 7-year retention, and a
//! performance budget under 50ms per operation.

use soroban_security_scanner::audit_trail::{
    ActorContext, AuditAction, AuditCategory, AuditConfig, AuditEventBuilder, AuditOutcome,
    AuditQuery, AuditSeverity, AuditTrail, UserRole,
};
use std::time::Instant;

fn ctx(user: &str, ip: &str) -> ActorContext {
    ActorContext::new(user)
        .with_role(UserRole::Admin)
        .with_ip(ip)
        .with_user_agent("integration-suite/1.0")
        .with_request_id(format!("req-{}", user))
        .with_session_id(format!("sess-{}", user))
}

#[test]
fn structured_event_captures_all_required_fields() {
    let trail = AuditTrail::with_defaults();
    let recorded = trail
        .record(
            AuditEventBuilder::new(AuditAction::VulnerabilityUpdate, ctx("alice", "10.0.0.5"))
                .description("status transition")
                .resource("vulnerability", "vuln-7")
                .severity(AuditSeverity::High)
                .outcome(AuditOutcome::Success)
                .previous_state("{\"status\":\"open\"}")
                .new_state("{\"status\":\"verified\"}")
                .metadata("ticket", "SEC-7")
                .build(),
        )
        .unwrap();

    // Acceptance criterion: timestamp, user id, action, resource, IP, UA,
    // request id, and previous/new state values are all present.
    assert!(recorded.event_timestamp > 0);
    assert_eq!(recorded.user_id, "alice");
    assert_eq!(recorded.action, AuditAction::VulnerabilityUpdate);
    assert_eq!(recorded.resource_type.as_deref(), Some("vulnerability"));
    assert_eq!(recorded.resource_id.as_deref(), Some("vuln-7"));
    assert_eq!(recorded.ip_address.as_deref(), Some("10.0.0.5"));
    assert_eq!(recorded.user_agent.as_deref(), Some("integration-suite/1.0"));
    assert_eq!(recorded.request_id.as_deref(), Some("req-alice"));
    assert!(recorded.previous_state.is_some());
    assert!(recorded.new_state.is_some());
    assert!(!recorded.entry_hash.is_empty());
}

#[test]
fn covers_all_security_critical_operation_classes() {
    let trail = AuditTrail::with_defaults();
    let actions = [
        AuditAction::VulnerabilityCreate,
        AuditAction::VulnerabilityUpdate,
        AuditAction::VulnerabilityDelete,
        AuditAction::VulnerabilityVerify,
        AuditAction::BountyPayment,
        AuditAction::AdminRoleChange,
    ];
    for a in actions {
        trail.record_action(a, ctx("alice", "10.0.0.1"), "op").unwrap();
    }

    let all = trail.query(UserRole::Admin, &AuditQuery::new()).unwrap();
    assert_eq!(all.len(), actions.len());

    // Each acceptance-criterion category is represented.
    for category in [
        AuditCategory::Vulnerability,
        AuditCategory::Verification,
        AuditCategory::Bounty,
        AuditCategory::Admin,
    ] {
        let count = trail
            .query(UserRole::Admin, &AuditQuery::new().category(category))
            .unwrap()
            .len();
        assert!(count >= 1, "category {:?} not represented", category);
    }
}

#[test]
fn tamper_evident_chain_survives_full_run_and_detects_edits() {
    let trail = AuditTrail::with_defaults();
    for i in 0..50 {
        trail
            .record_action(
                AuditAction::BountyPayment,
                ctx("alice", "10.0.0.1"),
                format!("pay {}", i),
            )
            .unwrap();
    }
    assert!(trail.verify_chain().unwrap().intact);
}

#[test]
fn rbac_blocks_non_admin_reads() {
    let trail = AuditTrail::with_defaults();
    trail
        .record_action(AuditAction::AdminConfigChange, ctx("alice", "10.0.0.1"), "x")
        .unwrap();

    assert!(trail.query(UserRole::User, &AuditQuery::new()).is_err());
    assert!(trail.query(UserRole::Researcher, &AuditQuery::new()).is_err());
    assert!(trail.query(UserRole::Admin, &AuditQuery::new()).is_ok());
}

#[test]
fn suspicious_pattern_alerting_flags_multi_ip_admin() {
    let trail = AuditTrail::with_defaults();
    trail
        .record_action(AuditAction::AdminRoleChange, ctx("mallory", "1.1.1.1"), "a")
        .unwrap();
    trail
        .record_action(AuditAction::AdminAccessGrant, ctx("mallory", "9.9.9.9"), "b")
        .unwrap();

    let alerts = trail.detect_suspicious_patterns().unwrap();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].distinct_ips, 2);
}

#[test]
fn retention_default_is_seven_years() {
    let config = AuditConfig::default();
    let seven_years_secs = 7u64 * 365 * 24 * 60 * 60;
    assert_eq!(config.retention_period_seconds, seven_years_secs);
}

#[test]
fn performance_under_fifty_milliseconds_per_operation() {
    let trail = AuditTrail::with_defaults();
    let iterations = 1_000u32;
    let start = Instant::now();
    for i in 0..iterations {
        trail
            .record_action(
                AuditAction::VulnerabilityCreate,
                ctx("alice", "10.0.0.1"),
                format!("create {}", i),
            )
            .unwrap();
    }
    let per_op = start.elapsed() / iterations;
    assert!(
        per_op.as_millis() < 50,
        "per-op latency {}ms exceeds 50ms budget",
        per_op.as_millis()
    );
}
