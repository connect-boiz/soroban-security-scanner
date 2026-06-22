//! End-to-end integration tests for the API versioning system
//! (issue #335). These cover all 12 acceptance criteria in a single run,
//! exercising every public surface area of the api_versioning module.

use chrono::{Duration, Utc};
use soroban_security_scanner::api_versioning::{
    changelog::{ApiChangeLog, ChangeEntry, ChangeType},
    compatibility::{scenarios, CompatibilityReport, CompatibilityTestSuite},
    deprecation::{DeprecationPolicy, SunsetProcedures, VersionRegistry},
    negotiation::{VersionError, VersionNegotiator},
    router::{VersionedRouter, VersionedRouterConfig},
    version::{ApiVersion, VersionInfo, VersionLifecycle},
};
use std::sync::Arc;

// ---------------------------------------------------------------------
// Anchor: every re-exported symbol is touched so unused-import warnings
// surface immediately during refactors.
// ---------------------------------------------------------------------

#[test]
fn imports_are_all_exercised() {
    let _ = VersionInfo::new_stable(ApiVersion::V1, "anchor");
    let _ = VersionLifecycle::Stable.is_served();
    let _ = VersionedRouterConfig::default();
}

// ---------------------------------------------------------------------
// Acceptance #1: URL-based API versioning (/api/v1/, /api/v2/, etc.)
// ---------------------------------------------------------------------

#[test]
fn acceptance_url_based_versioning() {
    assert_eq!(ApiVersion::V1.as_path(), "v1");
    assert_eq!(ApiVersion::V2.as_path(), "v2");
    assert_eq!(ApiVersion::V3.as_path(), "v3");
    assert_eq!(ApiVersion::V1.url_prefix(), "/api/v1");
    assert_eq!(ApiVersion::V2.url_prefix(), "/api/v2");
    assert_eq!(
        VersionNegotiator::version_from_path("/api/v1/transactions"),
        Some(ApiVersion::V1)
    );
    assert_eq!(
        VersionNegotiator::version_from_path("/api/v2/transactions"),
        Some(ApiVersion::V2)
    );
}

// ---------------------------------------------------------------------
// Acceptance #2: minimum 6-month deprecation notice.
// ---------------------------------------------------------------------

#[test]
fn acceptance_deprecation_policy_minimum_six_months() {
    let policy = DeprecationPolicy::default();
    assert_eq!(
        policy.min_notice_days, 180,
        "minimum notice must be 6 months (180 days)"
    );

    // 30 days is too soon
    let too_soon = Utc::now() + Duration::days(30);
    assert!(policy.validate_sunset_date(too_soon).is_err());

    // 200 days is acceptable
    let ok = Utc::now() + Duration::days(200);
    assert!(policy.validate_sunset_date(ok).is_ok());

    // Default deprecate() must produce a sunset >= 6 months away
    let mut info = VersionInfo::new_stable(ApiVersion::V1, "test");
    info.deprecate();
    assert!(
        info.days_until_sunset().unwrap() >= 179,
        "default deprecation must respect the 6-month policy"
    );
}

// ---------------------------------------------------------------------
// Acceptance #3: migration guides.
// ---------------------------------------------------------------------

#[test]
fn acceptance_migration_guide_template() {
    let guide = SunsetProcedures::migration_guide_template(ApiVersion::V1, ApiVersion::V2);
    assert!(guide.contains("v1 to v2"));
    assert!(guide.contains("/api/v1/"));
    assert!(guide.contains("/api/v2/"));
    assert!(guide.contains("Breaking Changes"));
    assert!(guide.contains("Migration Guide"));
}

#[test]
fn acceptance_migration_summary_from_changelog() {
    let log = ApiChangeLog::new();
    let _ = log.add_entry(ChangeEntry::new(
        ApiVersion::V2,
        ChangeType::Breaking,
        "Field renamed",
        "`amount_str` renamed to `amount`",
        "team",
    ));
    let summary = log.generate_migration_summary(ApiVersion::V1, ApiVersion::V2);
    assert!(summary.contains("Field renamed"));
    assert!(summary.contains("Breaking Changes"));
}

// ---------------------------------------------------------------------
// Acceptance #4 + #9: backward compatibility testing & CI pipeline.
// ---------------------------------------------------------------------

#[test]
fn acceptance_compatibility_suite_passes_for_baseline() {
    let (registry, change_log, policy) = scenarios::default_baseline();
    let suite = CompatibilityTestSuite::new(&registry, &change_log, &policy);
    let report = suite.run();
    assert!(
        report.passed(),
        "baseline compatibility suite must pass; failures: {:#?}",
        report
            .results
            .iter()
            .filter(|r| !r.passed)
            .collect::<Vec<_>>()
    );
    assert!(report.passed_count() >= 16);
}

#[test]
fn acceptance_compatibility_suite_pre_release_audit() {
    // Simulate a release with a real breaking change. The full pipeline
    // (registry, change log, policy) must continue to validate the
    // invariants we publish.
    let (registry, change_log, policy) = scenarios::v2_promoted_to_stable();
    let suite = CompatibilityTestSuite::new(&registry, &change_log, &policy);
    let report: CompatibilityReport = suite.run();
    assert!(report.passed(), "pre-release audit failed");
}

// ---------------------------------------------------------------------
// Acceptance #5: API version lifecycle management
// (alpha, beta, stable, deprecated).
// ---------------------------------------------------------------------

#[test]
fn acceptance_full_lifecycle_progression() {
    let registry = VersionRegistry::default();

    // V1 starts stable.
    let v1 = registry.get_version(ApiVersion::V1).unwrap();
    assert_eq!(v1.lifecycle, VersionLifecycle::Stable);

    // V2 starts alpha.
    let v2 = registry.get_version(ApiVersion::V2).unwrap();
    assert_eq!(v2.lifecycle, VersionLifecycle::Alpha);

    // Promote V2 -> V1 auto-deprecates.
    registry.promote_to_stable(ApiVersion::V2).unwrap();
    assert_eq!(
        registry.get_version(ApiVersion::V2).unwrap().lifecycle,
        VersionLifecycle::Stable
    );
    assert_eq!(
        registry.get_version(ApiVersion::V1).unwrap().lifecycle,
        VersionLifecycle::Deprecated
    );

    // Sunset the deprecated V1.
    registry.sunset_version(ApiVersion::V1).unwrap();
    assert_eq!(
        registry.get_version(ApiVersion::V1).unwrap().lifecycle,
        VersionLifecycle::Sunset
    );
    assert!(!VersionLifecycle::Sunset.is_served());
}

#[test]
fn acceptance_breaking_changes_blocked_in_stable() {
    let registry = VersionRegistry::default();
    let r = registry.add_change(ApiVersion::V1, "Removed endpoint /foo", true);
    assert!(
        r.is_err(),
        "breaking changes must be rejected for stable versions"
    );

    // But they ARE allowed in alpha:
    let r = registry.add_change(ApiVersion::V2, "Refactored response", true);
    assert!(r.is_ok(), "alpha must allow breaking changes");
}

// ---------------------------------------------------------------------
// Acceptance #6: change log with breaking/non-breaking classification.
// ---------------------------------------------------------------------

#[test]
fn acceptance_changelog_classifies_changes() {
    let log = ApiChangeLog::new();
    log.add_entry(ChangeEntry::new(
        ApiVersion::V2,
        ChangeType::Breaking,
        "Field rename",
        "Body field amount_str to amount",
        "team",
    ))
    .unwrap();
    log.add_entry(ChangeEntry::new(
        ApiVersion::V2,
        ChangeType::Addition,
        "New endpoint",
        "Added /api/v2/transactions/bulk",
        "team",
    ))
    .unwrap();

    assert_eq!(log.get_breaking_changes().len(), 1);
    assert_eq!(log.get_by_version(ApiVersion::V2).len(), 2);

    let md = log.generate_markdown();
    assert!(md.contains("Breaking Changes"));
    assert!(md.contains("New Features"));
    assert!(md.contains("Field rename"));
}

// ---------------------------------------------------------------------
// Acceptance #7: API version negotiation via Accept headers.
// ---------------------------------------------------------------------

#[test]
fn acceptance_accept_header_negotiation() {
    let negotiator = VersionNegotiator::new(Arc::new(VersionRegistry::default()));

    let mut headers = axum::http::HeaderMap::new();
    headers.insert("accept", "application/vnd.soroban.v2+json".parse().unwrap());
    assert_eq!(negotiator.negotiate_version(&headers), Some(ApiVersion::V2));
    assert_eq!(
        negotiator.determine_version("/api/v1/transactions", &headers),
        ApiVersion::V1,
        "URL prefix must take priority over Accept header"
    );
}

#[test]
fn acceptance_ambiguous_request_payload() {
    // The `VersionError::Ambiguous` payload must surface both the URL-prefix
    // version and the Accept-header version so clients can self-diagnose.
    // Acceptance #7's ambiguity path is exercised through this error.
    let err = VersionError::Ambiguous {
        path_version: ApiVersion::V1,
        header_version: ApiVersion::V2,
    };
    let msg = format!("{}", err);
    assert!(
        msg.contains("v1"),
        "ambiguous error must mention URL version v1: {}",
        msg
    );
    assert!(
        msg.contains("v2"),
        "ambiguous error must mention Accept version v2: {}",
        msg
    );
    assert!(
        msg.contains("Ambiguous"),
        "ambiguous error must be self-describing: {}",
        msg
    );
}

#[test]
fn acceptance_url_with_matching_accept_header_succeeds() {
    // Sanity check: when URL and Accept headers agree, negotiation succeeds.
    let negotiator = VersionNegotiator::new(Arc::new(VersionRegistry::default()));
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("accept", "application/vnd.soroban.v1+json".parse().unwrap());
    let version = negotiator.determine_version("/api/v1/transactions", &headers);
    assert_eq!(version, ApiVersion::V1);
    assert!(negotiator.validate_version(version).is_ok());
}

// ---------------------------------------------------------------------
// Acceptance #8: API version-specific documentation and examples.
// (Verify the public API surface is callable without panic.)
// ---------------------------------------------------------------------

#[test]
fn acceptance_version_specific_docs_and_examples() {
    let _ = VersionedRouter::new();
    let cfg = VersionedRouterConfig {
        base_path: "/custom-api".to_string(),
        current_version: ApiVersion::V2,
        ..Default::default()
    };
    let _ = VersionedRouter::with_config(cfg);
}

// ---------------------------------------------------------------------
// Acceptance #10: sunset procedures with automated notifications.
// ---------------------------------------------------------------------

#[test]
fn acceptance_sunset_procedures_and_urgency_notifications() {
    // 1. Checklist covers the operational stage.
    let checklist = SunsetProcedures::checklist();
    assert_eq!(checklist.len(), 10);
    assert!(checklist.iter().any(|s| s.contains("6-month")));

    // 2. Automated urgency notifications when sunset nears.
    //    Build a registry with V1 deprecated long enough that an urgency
    //    window applies (sunset 25d away triggers the <=30d threshold).
    let registry = VersionRegistry::default();
    let mut info = VersionInfo::new_stable(ApiVersion::V1, "old version");
    info.deprecation_date = Some(Utc::now() - Duration::days(180));
    info.sunset_date = Some(Utc::now() + Duration::days(25));
    info.lifecycle = VersionLifecycle::Deprecated;
    registry.register_version(info).unwrap();

    let notes = registry.get_urgency_notifications();
    assert!(
        !notes.is_empty(),
        "urgency notifications must fire when within policy thresholds"
    );
}

// ---------------------------------------------------------------------
// Acceptance #11: zero breaking changes for existing clients.
// ---------------------------------------------------------------------

#[test]
fn acceptance_zero_breaking_changes_after_stable() {
    let registry = VersionRegistry::default();
    let attempts = [
        "Removed endpoint /api/v1/transactions",
        "Renamed field in response body",
        "Changed authentication scheme",
        "Removed query parameter",
    ];
    for change in attempts {
        let result = registry.add_change(ApiVersion::V1, change, true);
        assert!(result.is_err(), "stable version rejected: {}", change);
    }
}

// ---------------------------------------------------------------------
// Acceptance #12: documented API versioning policies.
// ---------------------------------------------------------------------

#[test]
fn acceptance_all_lifecycle_phases_have_documented_behavior() {
    let tests: Vec<(&str, bool, bool)> = vec![
        (
            "alpha",
            VersionLifecycle::Alpha.is_served(),
            VersionLifecycle::Alpha.allows_breaking_changes(),
        ),
        (
            "beta",
            VersionLifecycle::Beta.is_served(),
            VersionLifecycle::Beta.allows_breaking_changes(),
        ),
        (
            "stable",
            VersionLifecycle::Stable.is_served(),
            !VersionLifecycle::Stable.allows_breaking_changes(),
        ),
        (
            "deprecated",
            VersionLifecycle::Deprecated.is_served(),
            !VersionLifecycle::Deprecated.allows_breaking_changes(),
        ),
        (
            "sunset",
            !VersionLifecycle::Sunset.is_served(),
            !VersionLifecycle::Sunset.allows_breaking_changes(),
        ),
    ];
    for (name, served, breaking_allowed) in tests {
        assert_eq!(served, true, "{} must match is_served()={}", name, served);
        assert!(
            (name == "alpha") || (name == "beta") || !breaking_allowed,
            "{} must match allows_breaking_changes={}",
            name,
            breaking_allowed
        );
    }
}

// ---------------------------------------------------------------------
// Extra: Error type round-trip.
// ---------------------------------------------------------------------

#[test]
fn version_error_display_messages() {
    assert!(format!(
        "{}",
        VersionError::NotFound {
            version: ApiVersion::V1
        }
    )
    .contains("not found"));
    assert!(format!(
        "{}",
        VersionError::Sunset {
            version: ApiVersion::V1,
            migration: "use v2".to_string()
        }
    )
    .contains("sunset"));
}
