//! Dependency vulnerability management.
//!
//! Provides utilities to:
//! - Parse `cargo audit` JSON output into typed structs
//! - Classify advisories by severity
//! - Generate a Software Bill of Materials (SBOM) summary
//! - Block builds that contain critical CVEs

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Severity
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    None,
    Low,
    Medium,
    High,
    Critical,
}

// ---------------------------------------------------------------------------
// Advisory
// ---------------------------------------------------------------------------

/// A security advisory for a Cargo dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Advisory {
    pub id:          String,
    pub package:     String,
    pub version:     String,
    pub title:       String,
    pub severity:    Severity,
    pub url:         Option<String>,
    pub patched:     Vec<String>,
}

/// Result of running `cargo audit --json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub vulnerabilities: Vec<Advisory>,
    pub warnings:        Vec<Advisory>,
}

impl AuditReport {
    /// Parse from `cargo audit --json` output.
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        let raw: serde_json::Value = serde_json::from_str(json)?;

        let vulns = raw["vulnerabilities"]["list"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        let vulnerabilities = vulns.iter().filter_map(|v| {
            Some(Advisory {
                id:      v["advisory"]["id"].as_str()?.to_owned(),
                package: v["package"]["name"].as_str()?.to_owned(),
                version: v["package"]["version"].as_str()?.to_owned(),
                title:   v["advisory"]["title"].as_str()?.to_owned(),
                severity: match v["advisory"]["cvss"]["score"].as_f64().unwrap_or(0.0) {
                    s if s >= 9.0 => Severity::Critical,
                    s if s >= 7.0 => Severity::High,
                    s if s >= 4.0 => Severity::Medium,
                    s if s > 0.0  => Severity::Low,
                    _             => Severity::None,
                },
                url:     v["advisory"]["url"].as_str().map(str::to_owned),
                patched: v["versions"]["patched"]
                    .as_array().cloned().unwrap_or_default()
                    .iter().filter_map(|p| p.as_str().map(str::to_owned)).collect(),
            })
        }).collect();

        Ok(Self { vulnerabilities, warnings: vec![] })
    }

    /// Returns true if any advisory meets or exceeds `threshold`.
    pub fn has_severity_at_least(&self, threshold: &Severity) -> bool {
        self.vulnerabilities.iter().any(|a| &a.severity >= threshold)
    }

    /// Advisories at or above a severity level.
    pub fn filter_by_severity(&self, min: &Severity) -> Vec<&Advisory> {
        self.vulnerabilities.iter().filter(|a| &a.severity >= min).collect()
    }

    /// Generate a compact SBOM summary string.
    pub fn sbom_summary(&self) -> String {
        if self.vulnerabilities.is_empty() {
            return "No vulnerabilities found.".to_owned();
        }
        let mut lines = vec![format!("{} vulnerabilities:", self.vulnerabilities.len())];
        for a in &self.vulnerabilities {
            lines.push(format!("  [{:?}] {} {} — {}", a.severity, a.package, a.version, a.title));
        }
        lines.join("\n")
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn sample_report() -> AuditReport {
        AuditReport {
            vulnerabilities: vec![
                Advisory {
                    id: "RUSTSEC-2023-0001".into(),
                    package: "openssl".into(),
                    version: "0.10.40".into(),
                    title: "Use after free".into(),
                    severity: Severity::Critical,
                    url: Some("https://rustsec.org".into()),
                    patched: vec![">=0.10.48".into()],
                },
                Advisory {
                    id: "RUSTSEC-2023-0002".into(),
                    package: "hyper".into(),
                    version: "0.14.0".into(),
                    title: "DoS via oversized headers".into(),
                    severity: Severity::Medium,
                    url: None,
                    patched: vec![">=0.14.26".into()],
                },
            ],
            warnings: vec![],
        }
    }

    #[test]
    fn has_critical_detected() {
        assert!(sample_report().has_severity_at_least(&Severity::Critical));
    }

    #[test]
    fn filter_critical_returns_one() {
        assert_eq!(sample_report().filter_by_severity(&Severity::Critical).len(), 1);
    }

    #[test]
    fn filter_medium_returns_both() {
        assert_eq!(sample_report().filter_by_severity(&Severity::Medium).len(), 2);
    }

    #[test]
    fn sbom_summary_contains_package_name() {
        assert!(sample_report().sbom_summary().contains("openssl"));
    }

    #[test]
    fn empty_report_sbom_is_clean() {
        let r = AuditReport { vulnerabilities: vec![], warnings: vec![] };
        assert_eq!(r.sbom_summary(), "No vulnerabilities found.");
    }
}
