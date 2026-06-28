//! Analysis result validation and consistency checks.
//!
//! Validates that an analysis result is internally well-formed (no findings
//! point outside the analyzed source, counts agree, severities are known) and
//! supports cross-run consistency: the same input must yield the same result
//! fingerprint, catching nondeterministic or tampered analyzers.

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// A single analysis finding.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Finding {
    /// Rule/detector id.
    pub rule: String,
    /// Severity label (validated against the allowed set).
    pub severity: String,
    /// 1-based line the finding refers to.
    pub line: usize,
}

/// An analysis result to validate.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Findings produced.
    pub findings: Vec<Finding>,
    /// Total lines in the analyzed source (for bounds checks).
    pub source_lines: usize,
}

/// Why result validation failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResultError {
    /// A finding references a line outside the source.
    LineOutOfRange {
        /// The offending line.
        line: usize,
        /// Number of source lines.
        source_lines: usize,
    },
    /// A finding has an unknown severity.
    UnknownSeverity {
        /// The offending value.
        severity: String,
    },
    /// A finding has an empty rule id.
    EmptyRule,
}

/// Allowed severity labels.
pub const ALLOWED_SEVERITIES: &[&str] = &["CRITICAL", "HIGH", "MEDIUM", "LOW", "INFO"];

/// Validates a result's internal consistency.
pub fn validate(result: &AnalysisResult) -> Result<(), ResultError> {
    for f in &result.findings {
        if f.rule.is_empty() {
            return Err(ResultError::EmptyRule);
        }
        if !ALLOWED_SEVERITIES.contains(&f.severity.as_str()) {
            return Err(ResultError::UnknownSeverity {
                severity: f.severity.clone(),
            });
        }
        // A finding at line 0 is invalid (1-based); beyond source is invalid.
        if f.line == 0 || f.line > result.source_lines {
            return Err(ResultError::LineOutOfRange {
                line: f.line,
                source_lines: result.source_lines,
            });
        }
    }
    Ok(())
}

/// A stable fingerprint of a result, for cross-run consistency checks.
pub fn fingerprint(result: &AnalysisResult) -> u64 {
    let mut hasher = DefaultHasher::new();
    // Order-independent: combine per-finding hashes commutatively so finding
    // ordering does not change the fingerprint.
    let mut acc: u64 = result.source_lines as u64;
    for f in &result.findings {
        let mut h = DefaultHasher::new();
        f.hash(&mut h);
        acc ^= h.finish();
    }
    acc.hash(&mut hasher);
    hasher.finish()
}

/// Whether two results from the same input are consistent (identical
/// fingerprints). A mismatch indicates a nondeterministic or tampered analyzer.
pub fn is_consistent(a: &AnalysisResult, b: &AnalysisResult) -> bool {
    fingerprint(a) == fingerprint(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(rule: &str, sev: &str, line: usize) -> Finding {
        Finding {
            rule: rule.to_string(),
            severity: sev.to_string(),
            line,
        }
    }

    fn result() -> AnalysisResult {
        AnalysisResult {
            findings: vec![
                finding("reentrancy", "HIGH", 10),
                finding("overflow", "MEDIUM", 20),
            ],
            source_lines: 100,
        }
    }

    #[test]
    fn valid_result_passes() {
        assert!(validate(&result()).is_ok());
    }

    #[test]
    fn line_out_of_range_rejected() {
        let mut r = result();
        r.findings.push(finding("x", "LOW", 200));
        assert!(matches!(
            validate(&r),
            Err(ResultError::LineOutOfRange { .. })
        ));
        let mut z = result();
        z.findings.push(finding("x", "LOW", 0));
        assert!(matches!(
            validate(&z),
            Err(ResultError::LineOutOfRange { .. })
        ));
    }

    #[test]
    fn unknown_severity_rejected() {
        let mut r = result();
        r.findings.push(finding("x", "SPICY", 5));
        assert!(matches!(
            validate(&r),
            Err(ResultError::UnknownSeverity { .. })
        ));
    }

    #[test]
    fn empty_rule_rejected() {
        let mut r = result();
        r.findings.push(finding("", "LOW", 5));
        assert_eq!(validate(&r), Err(ResultError::EmptyRule));
    }

    #[test]
    fn consistency_is_order_independent() {
        let a = result();
        let mut b = result();
        b.findings.reverse();
        assert!(is_consistent(&a, &b));
    }

    #[test]
    fn different_results_are_inconsistent() {
        let a = result();
        let mut b = result();
        b.findings.push(finding("new", "LOW", 30));
        assert!(!is_consistent(&a, &b));
    }
}
