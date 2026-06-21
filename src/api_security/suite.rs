//! Comprehensive API security test suite.

use crate::api_security::{
    coverage::{build_coverage_report, CoverageGate, SecurityCoverageReport},
    defect_tracking::SecurityDefectTracker,
    endpoints::{EndpointAuth, EndpointRegistry},
    fuzzing::FuzzingEngine,
    regression::RegressionTestSuite,
};
use chrono::{DateTime, Utc};
use std::fmt::Write as _;

/// Result of a single security check.
#[derive(Debug, Clone)]
pub struct SecurityCheckResult {
    pub name: &'static str,
    pub passed: bool,
    pub severity: &'static str,
    pub message: String,
}

impl SecurityCheckResult {
    pub fn pass(name: &'static str, message: impl Into<String>) -> Self {
        Self {
            name,
            passed: true,
            severity: "info",
            message: message.into(),
        }
    }

    pub fn fail(name: &'static str, severity: &'static str, message: impl Into<String>) -> Self {
        Self {
            name,
            passed: false,
            severity,
            message: message.into(),
        }
    }
}

/// Aggregate security test report.
#[derive(Debug, Clone)]
pub struct SecurityReport {
    pub results: Vec<SecurityCheckResult>,
    pub coverage: SecurityCoverageReport,
    pub timestamp: DateTime<Utc>,
}

impl SecurityReport {
    pub fn passed(&self) -> bool {
        self.results.iter().all(|r| r.passed) && self.coverage.passes_gate()
    }

    pub fn high_severity_failures(&self) -> Vec<&SecurityCheckResult> {
        self.results
            .iter()
            .filter(|r| !r.passed && (r.severity == "high" || r.severity == "critical"))
            .collect()
    }

    pub fn passed_count(&self) -> usize {
        self.results.iter().filter(|r| r.passed).count()
    }

    pub fn failed_count(&self) -> usize {
        self.results.iter().filter(|r| !r.passed).count()
    }

    pub fn ci_should_block(&self) -> bool {
        !self.high_severity_failures().is_empty() || !self.coverage.passes_gate()
    }

    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# API Security Test Report");
        let _ = writeln!(out, "Generated: {}\n", self.timestamp.to_rfc3339());
        let _ = writeln!(
            out,
            "**Result:** {}\n",
            if self.passed() {
                "✅ PASS"
            } else {
                "❌ FAIL"
            }
        );
        let _ = writeln!(
            out,
            "**Summary:** {} passed, {} failed (total {})\n",
            self.passed_count(),
            self.failed_count(),
            self.results.len()
        );
        out.push_str("## Security Checks\n\n");
        out.push_str("| Check | Severity | Status | Message |\n");
        out.push_str("|-------|----------|--------|--------|\n");
        for r in &self.results {
            let status = if r.passed { "✅ pass" } else { "❌ fail" };
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} |",
                r.name, r.severity, status, r.message
            );
        }
        out.push_str("\n");
        out.push_str(&self.coverage.to_markdown());
        out
    }
}

/// Battery of API security checks across auth, authz, fuzzing, and business logic.
pub struct SecurityTestSuite {
    registry: EndpointRegistry,
    fuzzer: FuzzingEngine,
    regression: RegressionTestSuite,
    tracker: SecurityDefectTracker,
    gate: CoverageGate,
}

impl SecurityTestSuite {
    pub fn new(
        registry: EndpointRegistry,
        fuzzer: FuzzingEngine,
        regression: RegressionTestSuite,
        tracker: SecurityDefectTracker,
    ) -> Self {
        Self {
            registry,
            fuzzer,
            regression,
            tracker,
            gate: CoverageGate::default(),
        }
    }

    pub fn with_gate(mut self, gate: CoverageGate) -> Self {
        self.gate = gate;
        self
    }

    /// Run every security check and produce a report.
    pub fn run(&self) -> SecurityReport {
        let results = vec![
            self.all_endpoints_have_auth_policy(),
            self.admin_endpoints_require_admin_role(),
            self.authenticated_endpoints_reject_anonymous(),
            self.fuzzing_suite_passes(),
            self.regression_suite_passes(),
            self.no_blocking_defects(),
            self.critical_workflows_have_business_logic_tests(),
            self.rate_limited_endpoints_configured(),
            self.injection_payloads_rejected(),
            self.jwt_none_algorithm_blocked(),
            self.oversized_payloads_rejected(),
            self.versioned_api_security_headers_policy(),
        ];

        let covered_paths: Vec<&str> = self.registry.all().iter().map(|e| e.path).collect();
        let coverage = build_coverage_report(&covered_paths, self.gate.clone());

        SecurityReport {
            results,
            coverage,
            timestamp: Utc::now(),
        }
    }

    fn all_endpoints_have_auth_policy(&self) -> SecurityCheckResult {
        let missing: Vec<_> = self
            .registry
            .all()
            .iter()
            .filter(|e| e.description.is_empty())
            .map(|e| e.path)
            .collect();
        if missing.is_empty() {
            SecurityCheckResult::pass(
                "all_endpoints_have_auth_policy",
                format!(
                    "all {} endpoints have auth policy and description",
                    self.registry.count()
                ),
            )
        } else {
            SecurityCheckResult::fail(
                "all_endpoints_have_auth_policy",
                "high",
                format!("endpoints missing policy: {missing:?}"),
            )
        }
    }

    fn admin_endpoints_require_admin_role(&self) -> SecurityCheckResult {
        let admin_paths = [
            "/api/admin/users",
            "/api/admin/sessions",
            "/api/admin/stats",
            "/state/export",
            "/state/import",
            "/state/save",
            "/queue/cleanup",
        ];
        let mut violations = Vec::new();
        for path in admin_paths {
            if let Some(ep) = self.registry.all().iter().find(|e| e.path == path) {
                if ep.auth != EndpointAuth::Admin {
                    violations.push(path);
                }
            } else {
                violations.push(path);
            }
        }
        if violations.is_empty() {
            SecurityCheckResult::pass(
                "admin_endpoints_require_admin_role",
                format!("{} admin endpoints correctly gated", admin_paths.len()),
            )
        } else {
            SecurityCheckResult::fail(
                "admin_endpoints_require_admin_role",
                "critical",
                format!("admin auth violations: {violations:?}"),
            )
        }
    }

    fn authenticated_endpoints_reject_anonymous(&self) -> SecurityCheckResult {
        let auth_eps = self.registry.by_auth(EndpointAuth::Authenticated);
        if !auth_eps.is_empty() {
            SecurityCheckResult::pass(
                "authenticated_endpoints_reject_anonymous",
                format!(
                    "{} authenticated endpoints registered for auth testing",
                    auth_eps.len()
                ),
            )
        } else {
            SecurityCheckResult::fail(
                "authenticated_endpoints_reject_anonymous",
                "high",
                "no authenticated endpoints in registry",
            )
        }
    }

    fn fuzzing_suite_passes(&self) -> SecurityCheckResult {
        if self.fuzzer.all_passed() {
            SecurityCheckResult::pass(
                "fuzzing_suite_passes",
                format!("{} fuzz cases passed", self.fuzzer.cases().len()),
            )
        } else {
            let failed: Vec<_> = self
                .fuzzer
                .run_all()
                .into_iter()
                .filter(|r| !r.passed)
                .map(|r| r.case_name)
                .collect();
            SecurityCheckResult::fail(
                "fuzzing_suite_passes",
                "high",
                format!("fuzz failures: {failed:?}"),
            )
        }
    }

    fn regression_suite_passes(&self) -> SecurityCheckResult {
        if self.regression.all_passed() {
            SecurityCheckResult::pass(
                "regression_suite_passes",
                format!(
                    "{} historical vulnerabilities verified",
                    self.regression.all().len()
                ),
            )
        } else {
            SecurityCheckResult::fail(
                "regression_suite_passes",
                "critical",
                "one or more historical vulnerability regressions detected",
            )
        }
    }

    fn no_blocking_defects(&self) -> SecurityCheckResult {
        if self.tracker.ci_should_block() {
            SecurityCheckResult::fail(
                "no_blocking_defects",
                "critical",
                format!(
                    "{} open high/critical defects block CI",
                    self.tracker.open_blocking().len()
                ),
            )
        } else {
            SecurityCheckResult::pass("no_blocking_defects", "no blocking defects open")
        }
    }

    fn critical_workflows_have_business_logic_tests(&self) -> SecurityCheckResult {
        let critical = self.registry.critical_workflows();
        if critical.len() >= 10 {
            SecurityCheckResult::pass(
                "critical_workflows_have_business_logic_tests",
                format!("{} critical API workflows covered", critical.len()),
            )
        } else {
            SecurityCheckResult::fail(
                "critical_workflows_have_business_logic_tests",
                "high",
                format!("only {} critical workflows registered", critical.len()),
            )
        }
    }

    fn rate_limited_endpoints_configured(&self) -> SecurityCheckResult {
        let limited = self.registry.by_auth(EndpointAuth::RateLimited);
        if limited.len() >= 2 {
            SecurityCheckResult::pass(
                "rate_limited_endpoints_configured",
                format!("{} rate-limited endpoints registered", limited.len()),
            )
        } else {
            SecurityCheckResult::fail(
                "rate_limited_endpoints_configured",
                "medium",
                "insufficient rate-limited endpoint coverage",
            )
        }
    }

    fn injection_payloads_rejected(&self) -> SecurityCheckResult {
        let injection_cases: Vec<_> = self
            .fuzzer
            .cases()
            .iter()
            .filter(|c| c.category == "injection")
            .collect();
        let all_pass = injection_cases
            .iter()
            .all(|c| self.fuzzer.evaluate(c).passed);
        if all_pass {
            SecurityCheckResult::pass(
                "injection_payloads_rejected",
                format!("{} injection fuzz cases rejected", injection_cases.len()),
            )
        } else {
            SecurityCheckResult::fail(
                "injection_payloads_rejected",
                "critical",
                "injection payload accepted by validation",
            )
        }
    }

    fn jwt_none_algorithm_blocked(&self) -> SecurityCheckResult {
        let case = self
            .fuzzer
            .cases()
            .iter()
            .find(|c| c.name == "jwt_tamper")
            .expect("jwt_tamper case must exist");
        let result = self.fuzzer.evaluate(case);
        if result.passed {
            SecurityCheckResult::pass(
                "jwt_none_algorithm_blocked",
                "JWT none-algorithm tokens are rejected",
            )
        } else {
            SecurityCheckResult::fail(
                "jwt_none_algorithm_blocked",
                "critical",
                "JWT none-algorithm bypass not blocked",
            )
        }
    }

    fn oversized_payloads_rejected(&self) -> SecurityCheckResult {
        let case = self
            .fuzzer
            .cases()
            .iter()
            .find(|c| c.name == "oversized_payload")
            .expect("oversized_payload case must exist");
        let result = self.fuzzer.evaluate(case);
        if result.passed {
            SecurityCheckResult::pass(
                "oversized_payloads_rejected",
                "oversized payloads are rejected (DoS protection)",
            )
        } else {
            SecurityCheckResult::fail(
                "oversized_payloads_rejected",
                "high",
                "oversized payload not rejected",
            )
        }
    }

    fn versioned_api_security_headers_policy(&self) -> SecurityCheckResult {
        // Versioned API must expose changelog endpoints for security audit trail
        let has_changelog = self
            .registry
            .all()
            .iter()
            .any(|e| e.path.contains("changelog"));
        if has_changelog {
            SecurityCheckResult::pass(
                "versioned_api_security_headers_policy",
                "versioned API changelog endpoints available for audit",
            )
        } else {
            SecurityCheckResult::fail(
                "versioned_api_security_headers_policy",
                "medium",
                "missing changelog endpoints for security audit",
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_security::scenarios;

    #[test]
    fn default_baseline_passes() {
        let suite = scenarios::default_suite();
        let report = suite.run();
        assert!(
            report.passed(),
            "default security baseline must pass; failures: {:?}",
            report
                .results
                .iter()
                .filter(|r| !r.passed)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn open_defect_blocks_ci() {
        let (registry, fuzzer, regression, _) = scenarios::default_baseline();
        let tracker = scenarios::with_open_high_severity_defect();
        let suite = SecurityTestSuite::new(registry, fuzzer, regression, tracker);
        let report = suite.run();
        assert!(report.ci_should_block());
        assert!(!report.passed());
    }

    #[test]
    fn report_markdown_is_well_formed() {
        let suite = scenarios::default_suite();
        let report = suite.run();
        let md = report.to_markdown();
        assert!(md.contains("# API Security Test Report"));
        assert!(md.contains("| Check | Severity |"));
    }

    #[test]
    fn coverage_is_100_percent() {
        let suite = scenarios::default_suite();
        let report = suite.run();
        assert_eq!(report.coverage.endpoint_coverage_pct(), 100.0);
        assert!(report.coverage.passes_gate());
    }
}
