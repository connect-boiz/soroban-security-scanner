//! Pre-built security test scenarios for CI and local runs.

use crate::api_security::{
    coverage::{build_coverage_report, CoverageGate},
    defect_tracking::{DefectSeverity, DefectStatus, SecurityDefect, SecurityDefectTracker},
    endpoints::EndpointRegistry,
    fuzzing::FuzzingEngine,
    regression::{PenTestSchedule, RegressionTestSuite},
    suite::SecurityTestSuite,
};
use chrono::Utc;

/// Default baseline: full catalog, no open blocking defects.
pub fn default_baseline() -> (
    EndpointRegistry,
    FuzzingEngine,
    RegressionTestSuite,
    SecurityDefectTracker,
) {
    let registry = EndpointRegistry::full_catalog();
    let fuzzer = FuzzingEngine::default_cases();
    let regression = RegressionTestSuite::platform_history();
    let tracker = SecurityDefectTracker::new();
    (registry, fuzzer, regression, tracker)
}

/// Scenario with a simulated high-severity open defect (should fail CI gate).
pub fn with_open_high_severity_defect() -> SecurityDefectTracker {
    let mut tracker = SecurityDefectTracker::new();
    tracker.register(SecurityDefect {
        id: "SEC-SIM-001".into(),
        title: "Simulated auth bypass for CI gate test".into(),
        severity: DefectSeverity::High,
        status: DefectStatus::Open,
        endpoint: "/api/admin/users".into(),
        discovered_at: Utc::now(),
        remediation_deadline: None,
        remediation_notes: String::new(),
    });
    tracker
}

/// Build a security test suite from the default baseline.
pub fn default_suite() -> SecurityTestSuite {
    let (registry, fuzzer, regression, tracker) = default_baseline();
    SecurityTestSuite::new(registry, fuzzer, regression, tracker)
}

/// Build a full coverage report for the default endpoint catalog.
pub fn default_coverage_report() -> crate::api_security::coverage::SecurityCoverageReport {
    let registry = EndpointRegistry::full_catalog();
    let paths: Vec<&str> = registry.all().iter().map(|e| e.path).collect();
    build_coverage_report(&paths, CoverageGate::default())
}

/// Quarterly penetration test schedule for documentation and CI metadata.
pub fn quarterly_pentest() -> PenTestSchedule {
    PenTestSchedule::quarterly()
}
