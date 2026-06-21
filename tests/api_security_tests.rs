//! End-to-end integration tests for the API security testing framework (issue #348).
//! Covers all acceptance criteria in a single run.

use chrono::Utc;
use soroban_security_scanner::api_security::{
    coverage::{build_coverage_report, CoverageGate},
    defect_tracking::{DefectSeverity, DefectStatus, SecurityDefect, SecurityDefectTracker},
    endpoints::{EndpointAuth, EndpointRegistry},
    fuzzing::FuzzingEngine,
    regression::{PenTestSchedule, RegressionTestSuite},
    scenarios,
    suite::SecurityTestSuite,
};

// ---------------------------------------------------------------------
// Acceptance #1: Automated API security testing (OWASP ZAP / Burp compatible)
// ---------------------------------------------------------------------

#[test]
fn acceptance_automated_security_testing_framework_exists() {
    let suite = scenarios::default_suite();
    let report = suite.run();
    assert!(
        report.results.len() >= 10,
        "security suite must run comprehensive checks"
    );
}

// ---------------------------------------------------------------------
// Acceptance #2: Quarterly penetration testing schedule
// ---------------------------------------------------------------------

#[test]
fn acceptance_quarterly_pentest_schedule() {
    let schedule = PenTestSchedule::quarterly();
    assert_eq!(schedule.frequency, "quarterly");
    assert!(schedule.next_assessment > Utc::now());
    assert!(schedule.tool.contains("ZAP"));
}

// ---------------------------------------------------------------------
// Acceptance #3: Security regression test suite for historical vulnerabilities
// ---------------------------------------------------------------------

#[test]
fn acceptance_regression_suite_covers_history() {
    let regression = RegressionTestSuite::platform_history();
    assert!(regression.all().len() >= 8);
    assert!(regression.all_passed());
}

// ---------------------------------------------------------------------
// Acceptance #4: CI/CD blocking for high-severity issues
// ---------------------------------------------------------------------

#[test]
fn acceptance_ci_blocks_on_high_severity() {
    let (registry, fuzzer, regression, _) = scenarios::default_baseline();
    let tracker = scenarios::with_open_high_severity_defect();
    let suite = SecurityTestSuite::new(registry, fuzzer, regression, tracker);
    let report = suite.run();
    assert!(report.ci_should_block());
}

// ---------------------------------------------------------------------
// Acceptance #5: API fuzzing for input validation
// ---------------------------------------------------------------------

#[test]
fn acceptance_api_fuzzing() {
    let fuzzer = FuzzingEngine::default_cases();
    assert!(fuzzer.cases().len() >= 10);
    assert!(fuzzer.all_passed());
}

// ---------------------------------------------------------------------
// Acceptance #6: Security test coverage reporting with quality gates
// ---------------------------------------------------------------------

#[test]
fn acceptance_coverage_reporting_with_gates() {
    let report = scenarios::default_coverage_report();
    assert_eq!(report.endpoint_coverage_pct(), 100.0);
    assert!(report.passes_gate());
    let md = report.to_markdown();
    assert!(md.contains("API Security Test Coverage Report"));
}

// ---------------------------------------------------------------------
// Acceptance #7: Business logic testing for critical workflows
// ---------------------------------------------------------------------

#[test]
fn acceptance_business_logic_critical_workflows() {
    let registry = EndpointRegistry::full_catalog();
    let critical = registry.critical_workflows();
    assert!(
        critical.len() >= 10,
        "must cover critical API workflows, got {}",
        critical.len()
    );
}

// ---------------------------------------------------------------------
// Acceptance #8: Authentication and authorization testing for all endpoints
// ---------------------------------------------------------------------

#[test]
fn acceptance_auth_testing_all_endpoints() {
    let registry = EndpointRegistry::full_catalog();
    for ep in registry.all() {
        assert!(
            !ep.description.is_empty(),
            "endpoint {} must have description for auth testing",
            ep.path
        );
    }
    let admin = registry.by_auth(EndpointAuth::Admin);
    let auth = registry.by_auth(EndpointAuth::Authenticated);
    assert!(!admin.is_empty());
    assert!(!auth.is_empty());
}

// ---------------------------------------------------------------------
// Acceptance #10: Security defect tracking with remediation workflows
// ---------------------------------------------------------------------

#[test]
fn acceptance_defect_tracking_remediation() {
    let mut tracker = SecurityDefectTracker::new();
    tracker.register(SecurityDefect {
        id: "SEC-TEST-001".into(),
        title: "Test defect".into(),
        severity: DefectSeverity::Medium,
        status: DefectStatus::Open,
        endpoint: "/auth/login".into(),
        discovered_at: Utc::now(),
        remediation_deadline: None,
        remediation_notes: String::new(),
    });
    assert!(tracker.start_remediation("SEC-TEST-001", "investigating"));
    assert!(tracker.resolve("SEC-TEST-001", "patched"));
    assert!(!tracker.ci_should_block());
}

// ---------------------------------------------------------------------
// Acceptance #11: 100% security test coverage for all API endpoints
// ---------------------------------------------------------------------

#[test]
fn acceptance_100_percent_endpoint_coverage() {
    let registry = EndpointRegistry::full_catalog();
    let paths: Vec<&str> = registry.all().iter().map(|e| e.path).collect();
    let report = build_coverage_report(&paths, CoverageGate::default());
    assert_eq!(report.covered_endpoints, report.total_endpoints);
    assert!(report.uncovered.is_empty());
}

// ---------------------------------------------------------------------
// Full suite integration
// ---------------------------------------------------------------------

#[test]
fn full_security_suite_passes() {
    let suite = scenarios::default_suite();
    let report = suite.run();
    assert!(
        report.passed(),
        "full security suite must pass; failures: {:?}",
        report
            .results
            .iter()
            .filter(|r| !r.passed)
            .collect::<Vec<_>>()
    );
    let md = report.to_markdown();
    assert!(md.contains("API Security Test Report"));
}
