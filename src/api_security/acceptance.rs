//! Acceptance tests for issue #348 — run via `cargo test --lib api_security::acceptance`.

#[cfg(test)]
mod tests {
    use crate::api_security::{
        coverage::{build_coverage_report, CoverageGate},
        defect_tracking::{DefectSeverity, DefectStatus, SecurityDefect, SecurityDefectTracker},
        endpoints::{EndpointAuth, EndpointRegistry},
        fuzzing::FuzzingEngine,
        regression::{PenTestSchedule, RegressionTestSuite},
        scenarios,
        suite::SecurityTestSuite,
    };
    use chrono::Utc;

    #[test]
    fn acceptance_automated_security_testing_framework_exists() {
        let suite = scenarios::default_suite();
        let report = suite.run();
        assert!(report.results.len() >= 10);
    }

    #[test]
    fn acceptance_quarterly_pentest_schedule() {
        let schedule = PenTestSchedule::quarterly();
        assert_eq!(schedule.frequency, "quarterly");
        assert!(schedule.next_assessment > Utc::now());
        assert!(schedule.tool.contains("ZAP"));
    }

    #[test]
    fn acceptance_regression_suite_covers_history() {
        let regression = RegressionTestSuite::platform_history();
        assert!(regression.all().len() >= 8);
        assert!(regression.all_passed());
    }

    #[test]
    fn acceptance_ci_blocks_on_high_severity() {
        let (registry, fuzzer, regression, _) = scenarios::default_baseline();
        let tracker = scenarios::with_open_high_severity_defect();
        let suite = SecurityTestSuite::new(registry, fuzzer, regression, tracker);
        assert!(suite.run().ci_should_block());
    }

    #[test]
    fn acceptance_api_fuzzing() {
        let fuzzer = FuzzingEngine::default_cases();
        assert!(fuzzer.cases().len() >= 10);
        assert!(fuzzer.all_passed());
    }

    #[test]
    fn acceptance_coverage_reporting_with_gates() {
        let report = scenarios::default_coverage_report();
        assert_eq!(report.endpoint_coverage_pct(), 100.0);
        assert!(report.passes_gate());
        assert!(report
            .to_markdown()
            .contains("API Security Test Coverage Report"));
    }

    #[test]
    fn acceptance_business_logic_critical_workflows() {
        let registry = EndpointRegistry::full_catalog();
        let critical = registry.critical_workflows();
        assert!(critical.len() >= 10);
    }

    #[test]
    fn acceptance_auth_testing_all_endpoints() {
        let registry = EndpointRegistry::full_catalog();
        for ep in registry.all() {
            assert!(
                !ep.description.is_empty(),
                "missing description for {}",
                ep.path
            );
        }
        assert!(!registry.by_auth(EndpointAuth::Admin).is_empty());
        assert!(!registry.by_auth(EndpointAuth::Authenticated).is_empty());
    }

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

    #[test]
    fn acceptance_100_percent_endpoint_coverage() {
        let registry = EndpointRegistry::full_catalog();
        let paths: Vec<&str> = registry.all().iter().map(|e| e.path).collect();
        let report = build_coverage_report(&paths, CoverageGate::default());
        assert_eq!(report.covered_endpoints, report.total_endpoints);
        assert!(report.uncovered.is_empty());
    }

    #[test]
    fn full_security_suite_passes() {
        let report = scenarios::default_suite().run();
        assert!(report.passed());
    }
}
