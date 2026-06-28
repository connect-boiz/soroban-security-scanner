//! Encryption compliance reporting.
//!
//! Produces a point-in-time compliance report: the fraction of sensitive
//! fields actually encrypted (target 100%), key age vs. rotation policy, and
//! the algorithm in use — the evidence a security platform needs for audits.

use serde::{Deserialize, Serialize};

/// The encryption algorithm in use (reported for audits).
pub const ALGORITHM: &str = "AES-256-GCM";

/// Inputs describing the current encryption posture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComplianceInput {
    /// Total number of fields classified as sensitive.
    pub sensitive_fields: u64,
    /// Of those, how many are actually encrypted.
    pub encrypted_fields: u64,
    /// Whether database backups are encrypted.
    pub backups_encrypted: bool,
    /// Age of the active data key, in seconds.
    pub active_key_age_secs: i64,
    /// Whether key rotation is overdue.
    pub rotation_overdue: bool,
    /// Whether any key is currently flagged compromised.
    pub compromised_key_present: bool,
}

/// A compliance report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComplianceReport {
    /// Algorithm in use.
    pub algorithm: String,
    /// Percentage of sensitive fields encrypted (0–100).
    pub encrypted_percent: f64,
    /// Whether 100% field-encryption coverage is met.
    pub full_field_coverage: bool,
    /// Whether backups are encrypted.
    pub backups_encrypted: bool,
    /// Active key age in days.
    pub active_key_age_days: f64,
    /// Whether rotation is overdue.
    pub rotation_overdue: bool,
    /// Overall: is the deployment compliant?
    pub compliant: bool,
    /// Findings that block compliance (empty when compliant).
    pub findings: Vec<String>,
}

/// Builds a compliance report from current posture inputs.
pub fn report(input: ComplianceInput) -> ComplianceReport {
    let encrypted_percent = if input.sensitive_fields == 0 {
        100.0
    } else {
        (input.encrypted_fields as f64 / input.sensitive_fields as f64) * 100.0
    };
    let full_field_coverage = input.encrypted_fields >= input.sensitive_fields;

    let mut findings = Vec::new();
    if !full_field_coverage {
        findings.push(format!(
            "{} of {} sensitive fields are unencrypted",
            input.sensitive_fields - input.encrypted_fields,
            input.sensitive_fields
        ));
    }
    if !input.backups_encrypted {
        findings.push("database backups are not encrypted".to_string());
    }
    if input.rotation_overdue {
        findings.push("encryption key rotation is overdue".to_string());
    }
    if input.compromised_key_present {
        findings.push("a compromised key is present and must be retired".to_string());
    }

    ComplianceReport {
        algorithm: ALGORITHM.to_string(),
        encrypted_percent,
        full_field_coverage,
        backups_encrypted: input.backups_encrypted,
        active_key_age_days: input.active_key_age_secs as f64 / 86_400.0,
        rotation_overdue: input.rotation_overdue,
        compliant: findings.is_empty(),
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compliant_input() -> ComplianceInput {
        ComplianceInput {
            sensitive_fields: 100,
            encrypted_fields: 100,
            backups_encrypted: true,
            active_key_age_secs: 5 * 86_400,
            rotation_overdue: false,
            compromised_key_present: false,
        }
    }

    #[test]
    fn fully_encrypted_is_compliant() {
        let r = report(compliant_input());
        assert_eq!(r.encrypted_percent, 100.0);
        assert!(r.full_field_coverage);
        assert!(r.compliant);
        assert!(r.findings.is_empty());
        assert_eq!(r.algorithm, "AES-256-GCM");
        assert_eq!(r.active_key_age_days, 5.0);
    }

    #[test]
    fn partial_coverage_is_noncompliant() {
        let mut input = compliant_input();
        input.encrypted_fields = 90;
        let r = report(input);
        assert_eq!(r.encrypted_percent, 90.0);
        assert!(!r.compliant);
        assert!(r.findings.iter().any(|f| f.contains("unencrypted")));
    }

    #[test]
    fn unencrypted_backups_flagged() {
        let mut input = compliant_input();
        input.backups_encrypted = false;
        let r = report(input);
        assert!(!r.compliant);
        assert!(r.findings.iter().any(|f| f.contains("backups")));
    }

    #[test]
    fn overdue_rotation_and_compromise_flagged() {
        let mut input = compliant_input();
        input.rotation_overdue = true;
        input.compromised_key_present = true;
        let r = report(input);
        assert!(!r.compliant);
        assert_eq!(r.findings.len(), 2);
    }

    #[test]
    fn no_sensitive_fields_is_trivially_full_coverage() {
        let mut input = compliant_input();
        input.sensitive_fields = 0;
        input.encrypted_fields = 0;
        let r = report(input);
        assert_eq!(r.encrypted_percent, 100.0);
        assert!(r.full_field_coverage);
    }
}
