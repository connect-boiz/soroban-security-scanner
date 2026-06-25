//! Security testing in CI/CD pipeline helpers.
//!
//! Provides:
//! - SAST finding severity classification
//! - SCA (software composition analysis) gate
//! - Container image vulnerability summary
//! - CI gate: fail build when findings exceed thresholds

use serde::{Deserialize, Serialize};
use crate::dependency_audit::{AuditReport, Severity};

// ---------------------------------------------------------------------------
// SAST findings
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SastFinding {
    pub rule_id:   String,
    pub severity:  Severity,
    pub file:      String,
    pub line:      u32,
    pub message:   String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SastReport {
    pub findings: Vec<SastFinding>,
    pub tool:     String,
}

impl SastReport {
    pub fn critical_count(&self) -> usize {
        self.findings.iter().filter(|f| f.severity == Severity::Critical).count()
    }
    pub fn high_count(&self) -> usize {
        self.findings.iter().filter(|f| f.severity == Severity::High).count()
    }
}

// ---------------------------------------------------------------------------
// CI gate
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiGateConfig {
    pub max_sast_critical: usize,
    pub max_sast_high:     usize,
    pub max_sca_critical:  usize,
    pub block_on_critical: bool,
}

impl Default for CiGateConfig {
    fn default() -> Self {
        Self {
            max_sast_critical: 0,
            max_sast_high:     3,
            max_sca_critical:  0,
            block_on_critical: true,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CiGateResult {
    pub passed:   bool,
    pub reasons:  Vec<String>,
}

pub fn evaluate_ci_gate(
    sast:   &SastReport,
    sca:    &AuditReport,
    config: &CiGateConfig,
) -> CiGateResult {
    let mut reasons = Vec::new();

    if sast.critical_count() > config.max_sast_critical {
        reasons.push(format!("SAST: {} critical findings (max {})", sast.critical_count(), config.max_sast_critical));
    }
    if sast.high_count() > config.max_sast_high {
        reasons.push(format!("SAST: {} high findings (max {})", sast.high_count(), config.max_sast_high));
    }
    if config.block_on_critical && sca.has_severity_at_least(&Severity::Critical) {
        let crit = sca.filter_by_severity(&Severity::Critical).len();
        reasons.push(format!("SCA: {} critical CVEs in dependencies", crit));
    }

    CiGateResult { passed: reasons.is_empty(), reasons }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependency_audit::AuditReport;

    fn clean_sast() -> SastReport {
        SastReport { findings: vec![], tool: "clippy".into() }
    }
    fn clean_sca() -> AuditReport {
        AuditReport { vulnerabilities: vec![], warnings: vec![] }
    }

    #[test] fn all_clean_passes() {
        let r = evaluate_ci_gate(&clean_sast(), &clean_sca(), &CiGateConfig::default());
        assert!(r.passed);
    }
    #[test] fn critical_sast_blocks() {
        let sast = SastReport {
            findings: vec![SastFinding {
                rule_id: "S001".into(), severity: Severity::Critical,
                file: "lib.rs".into(), line: 1, message: "unsafe block".into(),
            }],
            tool: "clippy".into(),
        };
        let r = evaluate_ci_gate(&sast, &clean_sca(), &CiGateConfig::default());
        assert!(!r.passed);
    }
}
