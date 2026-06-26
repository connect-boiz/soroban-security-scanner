//! Unit tests for the audit trail module.
//!
//! Included from `audit_trail.rs` via `#[path = "audit_trail_tests.rs"]`, so the
//! contents form the body of a child module and have access to `super::*`.

use super::*;

fn admin_ctx(user: &str) -> ActorContext {
    ActorContext::new(user)
        .with_role(UserRole::Admin)
        .with_ip("10.0.0.1")
        .with_user_agent("test-agent/1.0")
        .with_request_id("req-123")
}

#[test]
fn test_config_defaults_seven_year_retention() {
    let config = AuditConfig::default();
    assert!(config.enabled);
    // 7 years expressed in seconds.
    assert_eq!(config.retention_period_seconds, 7 * SECONDS_PER_YEAR);
    assert_eq!(config.suspicious_min_distinct_ips, 2);
}

#[test]
fn test_action_category_mapping() {
    assert_eq!(
        AuditAction::VulnerabilityCreate.category(),
        AuditCategory::Vulnerability
    );
    assert_eq!(
        AuditAction::VulnerabilityVerify.category(),
        AuditCategory::Verification
    );
    assert_eq!(AuditAction::BountyPayment.category(), AuditCategory::Bounty);
    assert_eq!(
        AuditAction::AdminRoleChange.category(),
        AuditCategory::Admin
    );
    assert_eq!(AuditAction::AuthLogin.category(), AuditCategory::Auth);
}

#[test]
fn test_record_assigns_id_and_captures_context() {
    let trail = AuditTrail::with_defaults();
    let event = AuditEventBuilder::new(AuditAction::VulnerabilityCreate, admin_ctx("alice"))
        .description("created vuln report")
        .resource("vulnerability", "vuln-42")
        .severity(AuditSeverity::High)
        .build();

    let recorded = trail.record(event).unwrap();

    assert!(!recorded.audit_id.is_empty());
    assert_eq!(recorded.user_id, "alice");
    assert_eq!(recorded.user_role, UserRole::Admin);
    assert_eq!(recorded.ip_address.as_deref(), Some("10.0.0.1"));
    assert_eq!(recorded.user_agent.as_deref(), Some("test-agent/1.0"));
    assert_eq!(recorded.request_id.as_deref(), Some("req-123"));
    assert_eq!(recorded.resource_id.as_deref(), Some("vuln-42"));
    assert!(!recorded.entry_hash.is_empty());
    assert!(recorded.previous_entry_hash.is_empty()); // first entry
}

#[test]
fn test_state_change_tracking() {
    let trail = AuditTrail::with_defaults();
    let event = AuditEventBuilder::new(AuditAction::VulnerabilityUpdate, admin_ctx("alice"))
        .previous_state("{\"status\":\"open\"}")
        .new_state("{\"status\":\"verified\"}")
        .build();
    let recorded = trail.record(event).unwrap();
    assert_eq!(recorded.previous_state.as_deref(), Some("{\"status\":\"open\"}"));
    assert_eq!(recorded.new_state.as_deref(), Some("{\"status\":\"verified\"}"));
}

#[test]
fn test_hash_chain_links_entries() {
    let trail = AuditTrail::with_defaults();
    let first = trail
        .record_action(AuditAction::AuthLogin, admin_ctx("alice"), "login")
        .unwrap();
    let second = trail
        .record_action(AuditAction::BountyPayment, admin_ctx("alice"), "paid bounty")
        .unwrap();

    // The second entry must point back to the first.
    assert_eq!(second.previous_entry_hash, first.entry_hash);
    assert_ne!(second.entry_hash, first.entry_hash);
}

#[test]
fn test_verify_chain_intact() {
    let trail = AuditTrail::with_defaults();
    for i in 0..10 {
        trail
            .record_action(
                AuditAction::AdminConfigChange,
                admin_ctx("alice"),
                format!("change {}", i),
            )
            .unwrap();
    }
    let result = trail.verify_chain().unwrap();
    assert!(result.intact);
    assert_eq!(result.verified_count, 10);
    assert!(result.mismatches.is_empty());
}

#[test]
fn test_verify_chain_detects_tampering() {
    let trail = AuditTrail::with_defaults();
    trail
        .record_action(AuditAction::BountyPayment, admin_ctx("alice"), "pay 1")
        .unwrap();
    trail
        .record_action(AuditAction::BountyPayment, admin_ctx("alice"), "pay 2")
        .unwrap();

    // Tamper directly with stored data, leaving the stale hash in place.
    {
        let mut entries = trail.entries.lock().unwrap();
        entries[0].description = "pay 1000000".to_string();
    }

    let result = trail.verify_chain().unwrap();
    assert!(!result.intact);
    assert!(!result.mismatches.is_empty());
}

#[test]
fn test_query_requires_admin_role() {
    let trail = AuditTrail::with_defaults();
    trail
        .record_action(AuditAction::VulnerabilityCreate, admin_ctx("alice"), "x")
        .unwrap();

    // Non-admin roles are rejected.
    assert!(trail.query(UserRole::User, &AuditQuery::new()).is_err());
    assert!(trail.query(UserRole::Researcher, &AuditQuery::new()).is_err());

    // Admin-class roles succeed.
    assert!(trail.query(UserRole::Admin, &AuditQuery::new()).is_ok());
    assert!(trail.query(UserRole::Auditor, &AuditQuery::new()).is_ok());
    assert!(trail
        .query(UserRole::SecurityAdmin, &AuditQuery::new())
        .is_ok());
}

#[test]
fn test_query_filters_by_user_and_action() {
    let trail = AuditTrail::with_defaults();
    trail
        .record_action(AuditAction::VulnerabilityCreate, admin_ctx("alice"), "a")
        .unwrap();
    trail
        .record_action(AuditAction::VulnerabilityCreate, admin_ctx("bob"), "b")
        .unwrap();
    trail
        .record_action(AuditAction::BountyPayment, admin_ctx("alice"), "c")
        .unwrap();

    let alice_creates = trail
        .query(
            UserRole::Admin,
            &AuditQuery::new()
                .user_id("alice")
                .action(AuditAction::VulnerabilityCreate),
        )
        .unwrap();
    assert_eq!(alice_creates.len(), 1);
    assert_eq!(alice_creates[0].user_id, "alice");
}

#[test]
fn test_query_pagination() {
    let trail = AuditTrail::with_defaults();
    for i in 0..25 {
        trail
            .record_action(
                AuditAction::AdminConfigChange,
                admin_ctx("alice"),
                format!("c{}", i),
            )
            .unwrap();
    }
    let page = trail
        .query(UserRole::Admin, &AuditQuery::new().paginate(0, 10))
        .unwrap();
    assert_eq!(page.len(), 10);

    let page2 = trail
        .query(UserRole::Admin, &AuditQuery::new().paginate(20, 10))
        .unwrap();
    assert_eq!(page2.len(), 5);
}

#[test]
fn test_detect_suspicious_multiple_ips() {
    let trail = AuditTrail::with_defaults();
    // Same user, admin actions, two distinct IPs.
    trail
        .record(
            AuditEventBuilder::new(
                AuditAction::AdminRoleChange,
                ActorContext::new("mallory")
                    .with_role(UserRole::Admin)
                    .with_ip("1.1.1.1"),
            )
            .build(),
        )
        .unwrap();
    trail
        .record(
            AuditEventBuilder::new(
                AuditAction::AdminAccessGrant,
                ActorContext::new("mallory")
                    .with_role(UserRole::Admin)
                    .with_ip("2.2.2.2"),
            )
            .build(),
        )
        .unwrap();

    let alerts = trail.detect_suspicious_patterns().unwrap();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].user_id, "mallory");
    assert_eq!(alerts[0].distinct_ips, 2);
}

#[test]
fn test_no_alert_for_single_ip() {
    let trail = AuditTrail::with_defaults();
    for _ in 0..5 {
        trail
            .record(
                AuditEventBuilder::new(
                    AuditAction::AdminConfigChange,
                    ActorContext::new("alice")
                        .with_role(UserRole::Admin)
                        .with_ip("1.1.1.1"),
                )
                .build(),
            )
            .unwrap();
    }
    let alerts = trail.detect_suspicious_patterns().unwrap();
    assert!(alerts.is_empty());
}

#[test]
fn test_archival_eligibility() {
    let mut config = AuditConfig::default();
    config.retention_period_seconds = 1; // 1-second retention for the test
    let trail = AuditTrail::new(config);

    // Record an entry whose event time is far in the past.
    let mut event = AuditEventBuilder::new(AuditAction::AuthLogin, admin_ctx("alice")).build();
    event.event_timestamp = 100; // ancient
    trail.record(event).unwrap();

    let eligible = trail.entries_eligible_for_archival().unwrap();
    assert_eq!(eligible.len(), 1);
}

#[test]
fn test_csv_and_json_export() {
    let trail = AuditTrail::with_defaults();
    trail
        .record_action(AuditAction::BountyPayment, admin_ctx("alice"), "pay")
        .unwrap();
    let all = trail.query(UserRole::Admin, &AuditQuery::new()).unwrap();

    let csv = trail.to_csv(&all);
    assert!(csv.contains("audit_id,event_timestamp"));
    assert!(csv.contains("bounty.payment"));

    let json = trail.to_json(&all).unwrap();
    assert!(json.contains("\"action\""));
    assert!(json.contains("alice"));
}

#[test]
fn test_disabled_trail_is_noop() {
    let mut config = AuditConfig::default();
    config.enabled = false;
    let trail = AuditTrail::new(config);
    trail
        .record_action(AuditAction::AuthLogin, admin_ctx("alice"), "x")
        .unwrap();
    assert_eq!(trail.len().unwrap(), 0);
}

#[test]
fn test_eviction_keeps_chain_consistent() {
    let mut config = AuditConfig::default();
    config.max_entries_in_memory = 3;
    let trail = AuditTrail::new(config);

    for i in 0..6 {
        trail
            .record_action(
                AuditAction::AdminConfigChange,
                admin_ctx("alice"),
                format!("c{}", i),
            )
            .unwrap();
    }

    assert_eq!(trail.len().unwrap(), 3);
    // After eviction-driven rebuilds, the retained chain must still verify.
    let result = trail.verify_chain().unwrap();
    assert!(result.intact);
}
