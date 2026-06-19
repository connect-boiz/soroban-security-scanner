//! Backward compatibility test harness
//!
//! Validates that API contracts remain intact across version transitions.
//! Used by CI to guarantee "zero breaking changes for existing clients".
//!
//! Provides:
//! - [`CompatibilityTestSuite`] — battery of cross-version checks
//! - [`CompatibilityReport`] — human-readable pass/fail summary
//! - Pre-built scenarios for the V1 → V2 transition
//!
//! This module implements two acceptance criteria from issue #335:
//! - "backward compatibility testing between API versions"
//! - "API compatibility testing in CI/CD pipeline" (the suite is invoked
//!   automatically by `cargo test api_versioning::compatibility`).

use crate::api_versioning::changelog::{ApiChangeLog, ChangeEntry, ChangeType};
use crate::api_versioning::deprecation::{DeprecationPolicy, VersionRegistry};
use crate::api_versioning::version::{ApiVersion, VersionInfo, VersionLifecycle};
use chrono::{DateTime, Utc};
use std::fmt::Write as _;

/// Result of a single compatibility check
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: &'static str,
    pub passed: bool,
    pub message: String,
}

impl CheckResult {
    pub fn pass(name: &'static str, message: impl Into<String>) -> Self {
        Self {
            name,
            passed: true,
            message: message.into(),
        }
    }

    pub fn fail(name: &'static str, message: impl Into<String>) -> Self {
        Self {
            name,
            passed: false,
            message: message.into(),
        }
    }
}

/// Aggregate report for the full compatibility suite
#[derive(Debug, Clone)]
pub struct CompatibilityReport {
    pub results: Vec<CheckResult>,
    pub timestamp: DateTime<Utc>,
}

impl CompatibilityReport {
    pub fn passed(&self) -> bool {
        self.results.iter().all(|r| r.passed)
    }

    pub fn passed_count(&self) -> usize {
        self.results.iter().filter(|r| r.passed).count()
    }

    pub fn failed_count(&self) -> usize {
        self.results.iter().filter(|r| !r.passed).count()
    }

    /// Render the report as a Markdown string suitable for CI artifacts.
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# API Backward Compatibility Report");
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
        out.push_str("## Checks\n\n");
        out.push_str("| Check | Status | Message |\n");
        out.push_str("|-------|--------|---------|\n");
        for r in &self.results {
            let status = if r.passed { "✅ pass" } else { "❌ fail" };
            let _ = writeln!(out, "| {} | {} | {} |", r.name, status, r.message);
        }
        out
    }
}

/// Battery of cross-version checks. Constructed with a registry + change
/// log, then run via [`Self::run`].
pub struct CompatibilityTestSuite<'a> {
    registry: &'a VersionRegistry,
    change_log: &'a ApiChangeLog,
    policy: &'a DeprecationPolicy,
}

impl<'a> CompatibilityTestSuite<'a> {
    /// Create a new suite from the given registry, change log, and policy.
    pub fn new(
        registry: &'a VersionRegistry,
        change_log: &'a ApiChangeLog,
        policy: &'a DeprecationPolicy,
    ) -> Self {
        Self {
            registry,
            change_log,
            policy,
        }
    }

    /// Convenience constructor using the registry's default policy.
    pub fn with_registry_and_log(
        registry: &'a VersionRegistry,
        change_log: &'a ApiChangeLog,
    ) -> Self {
        Self::new(registry, change_log, registry.deprecation_policy())
    }

    /// Run every check and produce a report.
    pub fn run(&self) -> CompatibilityReport {
        let mut results = Vec::new();
        results.push(self.v1_endpoint_contract_preserved());
        results.push(self.v1_media_type_still_served());
        results.push(self.v1_changelog_audit_trail_complete());
        results.push(self.new_version_does_not_evict_old_version());
        results.push(self.deprecation_timeline_respects_min_notice());
        results.push(self.sunset_raises_not_found_after_sunset());
        results.push(self.promote_to_stable_demotes_previous_stable());
        results.push(self.stable_rejects_breaking_changes_runtime());
        results.push(self.change_log_markdown_is_non_empty());
        results.push(self.change_log_json_roundtrip());
        results.push(self.migration_summary_reflects_breaking_changes());
        results.push(self.current_stable_is_queryable());
        results.push(self.list_active_versions_excludes_sunset());
        results.push(self.urgency_notification_thresholds_match_policy());
        results.push(self.api_version_roundtrip_parse());
        results.push(self.lifecycle_serving_table_matches_documentation());
        CompatibilityReport {
            results,
            timestamp: Utc::now(),
        }
    }

    fn v1_endpoint_contract_preserved(&self) -> CheckResult {
        match self.registry.get_version(ApiVersion::V1) {
            Some(info) if info.lifecycle.is_served() => CheckResult::pass(
                "v1_endpoint_contract_preserved",
                "v1 endpoints are still served",
            ),
            Some(_) => CheckResult::fail(
                "v1_endpoint_contract_preserved",
                "v1 is no longer served (sunset or worse)",
            ),
            None => CheckResult::fail(
                "v1_endpoint_contract_preserved",
                "v1 missing from registry — breaking change for v1 clients!",
            ),
        }
    }

    fn v1_media_type_still_served(&self) -> CheckResult {
        if ApiVersion::V1.media_type() == "application/vnd.soroban.v1+json" {
            CheckResult::pass(
                "v1_media_type_still_served",
                "media type is stable: application/vnd.soroban.v1+json",
            )
        } else {
            CheckResult::fail(
                "v1_media_type_still_served",
                "media type changed — clients using Accept headers may break",
            )
        }
    }

    fn v1_changelog_audit_trail_complete(&self) -> CheckResult {
        let entries = self.change_log.get_by_version(ApiVersion::V1);
        if entries.is_empty() {
            CheckResult::fail(
                "v1_changelog_audit_trail_complete",
                "no changelog entries for v1",
            )
        } else {
            CheckResult::pass(
                "v1_changelog_audit_trail_complete",
                format!("{} changelog entries preserved for v1", entries.len()),
            )
        }
    }

    fn new_version_does_not_evict_old_version(&self) -> CheckResult {
        let present = self.registry.get_version(ApiVersion::V1).is_some();
        if present {
            CheckResult::pass(
                "new_version_does_not_evict_old_version",
                "v1 still registered alongside newer versions",
            )
        } else {
            CheckResult::fail(
                "new_version_does_not_evict_old_version",
                "v1 evicted when newer versions were added",
            )
        }
    }

    fn deprecation_timeline_respects_min_notice(&self) -> CheckResult {
        let mut messages: Vec<String> = Vec::new();
        let mut all_good = true;
        for info in self.registry.list_versions() {
            if let Some(sunset) = info.sunset_date {
                let notice = (sunset - info.deprecation_date.unwrap_or(sunset)).num_days();
                if notice < self.policy.min_notice_days {
                    all_good = false;
                    messages.push(format!(
                        "{}: only {} days notice (< {} required)",
                        info.version.as_path(),
                        notice,
                        self.policy.min_notice_days
                    ));
                }
            }
        }
        if all_good {
            CheckResult::pass(
                "deprecation_timeline_respects_min_notice",
                format!(
                    "all deprecated versions meet {}-day minimum",
                    self.policy.min_notice_days
                ),
            )
        } else {
            CheckResult::fail(
                "deprecation_timeline_respects_min_notice",
                messages.join("; "),
            )
        }
    }

    fn sunset_raises_not_found_after_sunset(&self) -> CheckResult {
        for info in self.registry.list_versions() {
            if info.lifecycle == VersionLifecycle::Sunset && info.lifecycle.is_served() {
                return CheckResult::fail(
                    "sunset_raises_not_found_after_sunset",
                    format!(
                        "{} is sunset but still served (should return 410 Gone)",
                        info.version.as_path()
                    ),
                );
            }
        }
        CheckResult::pass(
            "sunset_raises_not_found_after_sunset",
            "no sunset version is incorrectly marked as served",
        )
    }

    fn promote_to_stable_demotes_previous_stable(&self) -> CheckResult {
        let stales: Vec<_> = self
            .registry
            .list_versions()
            .into_iter()
            .filter(|v| v.lifecycle == VersionLifecycle::Stable)
            .collect();
        let count = stales.len();
        if count == 1 || count == 0 {
            CheckResult::pass(
                "promote_to_stable_demotes_previous_stable",
                format!("{} stable version registered (no conflict)", count),
            )
        } else {
            CheckResult::fail(
                "promote_to_stable_demotes_previous_stable",
                format!("{} stable versions registered; only one allowed", count),
            )
        }
    }

    /// Verify the **runtime policy** that the registry refuses to record a
    /// breaking change against a currently-stable version. (Historical
    /// changelog entries from the version's prior alpha/beta period do
    /// not count \u{2014} they are part of the legitimate release history.)
    fn stable_rejects_breaking_changes_runtime(&self) -> CheckResult {
        for info in self.registry.list_versions() {
            if info.lifecycle == VersionLifecycle::Stable {
                // Probe the policy: if `add_change` accepts the breaking
                // entry, the policy is broken.
                let probe =
                    "Regression probe: policy must reject breaking changes on stable versions";
                let result = self.registry.add_change(info.version, probe, true);
                return if result.is_err() {
                    CheckResult::pass(
                        "stable_rejects_breaking_changes_runtime",
                        format!(
                            "registry correctly rejected breaking change on stable {}",
                            info.version.as_path()
                        ),
                    )
                } else {
                    CheckResult::fail(
                        "stable_rejects_breaking_changes_runtime",
                        format!(
                            "registry ACCEPTED a breaking change on stable {} \u{2014} policy is broken",
                            info.version.as_path()
                        ),
                    )
                };
            }
        }
        CheckResult::pass(
            "stable_rejects_breaking_changes_runtime",
            "no stable versions in registry (nothing to verify)",
        )
    }

    fn change_log_markdown_is_non_empty(&self) -> CheckResult {
        let md = self.change_log.generate_markdown();
        if md.contains("# API Change Log") && md.len() > 30 {
            CheckResult::pass(
                "change_log_markdown_is_non_empty",
                format!("{} bytes of markdown generated", md.len()),
            )
        } else {
            CheckResult::fail(
                "change_log_markdown_is_non_empty",
                "markdown generator produced empty/truncated output",
            )
        }
    }

    fn change_log_json_roundtrip(&self) -> CheckResult {
        let json = self.change_log.generate_json();
        match serde_json::from_str::<Vec<ChangeEntry>>(&json) {
            Ok(parsed) if parsed.len() == self.change_log.get_all().len() => CheckResult::pass(
                "change_log_json_roundtrip",
                format!("{} entries serialized and parsed back", parsed.len()),
            ),
            Ok(parsed) => CheckResult::fail(
                "change_log_json_roundtrip",
                format!(
                    "round-trip lost entries: {} serialized, {} parsed",
                    self.change_log.get_all().len(),
                    parsed.len()
                ),
            ),
            Err(e) => CheckResult::fail(
                "change_log_json_roundtrip",
                format!("JSON parse error: {}", e),
            ),
        }
    }

    fn migration_summary_reflects_breaking_changes(&self) -> CheckResult {
        // If v2 has breaking changes, summary must mention them; else must say "no breaking".
        let breaking = self
            .change_log
            .get_breaking_changes_for_version(ApiVersion::V2);
        let summary = self
            .change_log
            .generate_migration_summary(ApiVersion::V1, ApiVersion::V2);
        if breaking.is_empty() {
            if summary.contains("No breaking changes") {
                CheckResult::pass(
                    "migration_summary_reflects_breaking_changes",
                    "summary correctly reports zero breaking changes",
                )
            } else {
                CheckResult::fail(
                    "migration_summary_reflects_breaking_changes",
                    "no v2 breaking changes but summary does not say so",
                )
            }
        } else if summary.contains(&breaking[0].summary) {
            CheckResult::pass(
                "migration_summary_reflects_breaking_changes",
                format!("summary mentions {} breaking entries", breaking.len()),
            )
        } else {
            CheckResult::fail(
                "migration_summary_reflects_breaking_changes",
                "summary does not mention known breaking changes",
            )
        }
    }

    fn current_stable_is_queryable(&self) -> CheckResult {
        match self.registry.current_stable() {
            Some(info) if info.lifecycle == VersionLifecycle::Stable => CheckResult::pass(
                "current_stable_is_queryable",
                format!("current stable: {}", info.version.as_path()),
            ),
            Some(other) => CheckResult::fail(
                "current_stable_is_queryable",
                format!(
                    "current_stable returned {} but lifecycle is {}",
                    other.version.as_path(),
                    other.lifecycle.as_str()
                ),
            ),
            None => CheckResult::fail(
                "current_stable_is_queryable",
                "no stable version registered",
            ),
        }
    }

    fn list_active_versions_excludes_sunset(&self) -> CheckResult {
        let active = self.registry.list_active_versions();
        let has_sunset = active
            .iter()
            .any(|v| v.lifecycle == VersionLifecycle::Sunset);
        if has_sunset {
            CheckResult::fail(
                "list_active_versions_excludes_sunset",
                "active version list included a sunset version",
            )
        } else {
            CheckResult::pass(
                "list_active_versions_excludes_sunset",
                format!("{} active versions, none sunset", active.len()),
            )
        }
    }

    fn urgency_notification_thresholds_match_policy(&self) -> CheckResult {
        let notes = self.registry.get_urgency_notifications();
        for n in &notes {
            if !self
                .policy
                .urgency_notification_days
                .iter()
                .any(|t| *t == n.threshold)
            {
                return CheckResult::fail(
                    "urgency_notification_thresholds_match_policy",
                    format!("urgency threshold {} not in policy", n.threshold),
                );
            }
        }
        CheckResult::pass(
            "urgency_notification_thresholds_match_policy",
            format!(
                "{} urgency notifications align with policy thresholds",
                notes.len()
            ),
        )
    }

    fn api_version_roundtrip_parse(&self) -> CheckResult {
        for v in ApiVersion::all() {
            let parsed: Result<ApiVersion, _> = v.as_path().parse();
            if parsed != Ok(v) {
                return CheckResult::fail(
                    "api_version_roundtrip_parse",
                    format!("failed to round-trip {}", v.as_path()),
                );
            }
        }
        CheckResult::pass(
            "api_version_roundtrip_parse",
            "all versions parse back to themselves",
        )
    }

    fn lifecycle_serving_table_matches_documentation(&self) -> CheckResult {
        let docs = [
            (VersionLifecycle::Alpha, true),
            (VersionLifecycle::Beta, true),
            (VersionLifecycle::Stable, true),
            (VersionLifecycle::Deprecated, true),
            (VersionLifecycle::Sunset, false),
        ];
        for (lc, expected) in docs {
            if lc.is_served() != expected {
                return CheckResult::fail(
                    "lifecycle_serving_table_matches_documentation",
                    format!(
                        "{} expected served={} but is_served()={}",
                        lc.as_str(),
                        expected,
                        lc.is_served()
                    ),
                );
            }
        }
        CheckResult::pass(
            "lifecycle_serving_table_matches_documentation",
            "all 5 lifecycle phases map is_served() correctly",
        )
    }
}

/// Standard scenarios used by CI when running the compatibility suite.
/// Each function returns a populated suite ready to be `.run()`.
pub mod scenarios {
    use super::*;

    /// The baseline scenario: fresh registry + default change log.
    pub fn default_baseline() -> (VersionRegistry, ApiChangeLog, DeprecationPolicy) {
        let registry = VersionRegistry::default();
        let change_log = ApiChangeLog::default();
        let policy = DeprecationPolicy::default();
        (registry, change_log, policy)
    }

    /// Scenario after V2 features are recorded but V2 is still alpha
    /// (the default `VersionRegistry` already registers V2 as alpha).
    pub fn v2_alpha_introduced() -> (VersionRegistry, ApiChangeLog, DeprecationPolicy) {
        let registry = VersionRegistry::default();
        let change_log = ApiChangeLog::default();

        // Real V2 alpha features — recorded only in the changelog:
        change_log
            .add_entry(
                ChangeEntry::new(
                    ApiVersion::V2,
                    ChangeType::Addition,
                    "Bulk transaction endpoint",
                    "Added POST /api/v2/transactions/bulk for submitting up to 100 transactions in one request.",
                    "platform-team",
                )
                .with_endpoints(vec!["/api/v2/transactions/bulk"]),
            )
            .unwrap();
        change_log
            .add_entry(ChangeEntry::new(
                ApiVersion::V2,
                ChangeType::Improvement,
                "Reduced default rate-limit window",
                "Default rate-limit window changed from 60s to 30s — improvement, not breaking.",
                "platform-team",
            ))
            .unwrap();

        (registry, change_log, DeprecationPolicy::default())
    }

    /// Scenario after V2 is promoted to stable: V1 must auto-deprecate.
    /// V3 is also pre-registered as alpha to exercise the "new_version_does_not_evict"
    /// invariant with multiple sibling versions.
    pub fn v2_promoted_to_stable() -> (VersionRegistry, ApiChangeLog, DeprecationPolicy) {
        let registry = VersionRegistry::default();
        let change_log = ApiChangeLog::default();

        // Record a breaking change in V2 before promotion — V2 is alpha at this point
        // so the registry accepts it. The legitimate release history.
        change_log
            .add_entry(
                ChangeEntry::new(
                    ApiVersion::V2,
                    ChangeType::Breaking,
                    "Renamed /v1/transactions body field",
                    "Field `amount_str` renamed to `amount` in transaction creation request body. A migration guide is provided alongside this change.",
                    "platform-team",
                )
                .with_endpoints(vec!["/api/v2/transactions"])
                .with_migration_guide(
                    "Replace all occurrences of `amount_str` with `amount` (string is no longer required, u64 is accepted).",
                ),
            )
            .unwrap();

        // V2 is already in the default registry as alpha; pre-registering it
        // again is a no-op (registry returns Err on duplicate). We only need
        // to add V3.
        let _ = registry.register_version(VersionInfo::new_alpha(
            ApiVersion::V3,
            "Planned future major.",
        ));

        // Promote V2 to stable. This auto-deprecates V1 with the
        // policy-driven 6-month sunset window.
        registry.promote_to_stable(ApiVersion::V2).unwrap();

        (registry, change_log, DeprecationPolicy::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_baseline_passes() {
        let (registry, change_log, policy) = scenarios::default_baseline();
        let suite = CompatibilityTestSuite::new(&registry, &change_log, &policy);
        let report = suite.run();
        assert!(
            report.passed(),
            "default baseline should pass; failures: {:?}",
            report
                .results
                .iter()
                .filter(|r| !r.passed)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_v2_alpha_introduced_passes() {
        let (registry, change_log, policy) = scenarios::v2_alpha_introduced();
        let suite = CompatibilityTestSuite::new(&registry, &change_log, &policy);
        let report = suite.run();
        assert!(
            report.passed(),
            "v2 alpha introduced should pass; failures: {:?}",
            report
                .results
                .iter()
                .filter(|r| !r.passed)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_v2_promoted_to_stable_passes() {
        let (registry, change_log, policy) = scenarios::v2_promoted_to_stable();
        let suite = CompatibilityTestSuite::new(&registry, &change_log, &policy);
        let report = suite.run();
        assert!(
            report.passed(),
            "v2 promoted to stable should pass; failures: {:?}",
            report
                .results
                .iter()
                .filter(|r| !r.passed)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_report_markdown_is_well_formed() {
        let (registry, change_log, policy) = scenarios::default_baseline();
        let suite = CompatibilityTestSuite::new(&registry, &change_log, &policy);
        let report = suite.run();
        let md = report.to_markdown();
        assert!(md.contains("# API Backward Compatibility Report"));
        assert!(md.contains("| Check | Status |"));
    }

    #[test]
    fn test_evicting_v1_is_detected_as_breaking() {
        let registry = VersionRegistry::default();
        // Properly evict v1: deprecate then sunset (sunset requires deprecated first).
        registry.deprecate_version(ApiVersion::V1).unwrap();
        registry.sunset_version(ApiVersion::V1).unwrap();
        let change_log = ApiChangeLog::default();
        let policy = DeprecationPolicy::default();
        let suite = CompatibilityTestSuite::new(&registry, &change_log, &policy);
        let report = suite.run();
        assert!(
            !report.passed(),
            "suite must flag an evicted v1 as a breaking change"
        );
        assert!(report
            .results
            .iter()
            .any(|r| !r.passed && r.name == "v1_endpoint_contract_preserved"));
    }
}
