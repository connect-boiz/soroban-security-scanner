//! Vulnerability advisory database and dependency scanning.
//!
//! Models an OSV/RustSec-style advisory feed and scans a dependency inventory
//! against it, reporting findings and the fraction of dependencies covered by
//! the scan (target: 100%).

use crate::supply_chain::inventory::{Dependency, DependencyInventory, Ecosystem};
use crate::supply_chain::version::{Version, VersionRange};
use serde::{Deserialize, Serialize};

/// Severity of an advisory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum VulnSeverity {
    /// Low.
    Low,
    /// Medium.
    Medium,
    /// High.
    High,
    /// Critical.
    Critical,
}

impl VulnSeverity {
    /// Maps a CVSS base score to a severity bucket.
    pub fn from_cvss(score: f64) -> Self {
        if score >= 9.0 {
            VulnSeverity::Critical
        } else if score >= 7.0 {
            VulnSeverity::High
        } else if score >= 4.0 {
            VulnSeverity::Medium
        } else {
            VulnSeverity::Low
        }
    }

    /// Stable label.
    pub fn as_str(&self) -> &'static str {
        match self {
            VulnSeverity::Low => "LOW",
            VulnSeverity::Medium => "MEDIUM",
            VulnSeverity::High => "HIGH",
            VulnSeverity::Critical => "CRITICAL",
        }
    }
}

/// A security advisory for a package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Advisory {
    /// Advisory id (e.g. `RUSTSEC-2024-0001`, `CVE-2024-1234`).
    pub id: String,
    /// Ecosystem.
    pub ecosystem: Ecosystem,
    /// Affected package name.
    pub package: String,
    /// Affected version range.
    pub range: VersionRange,
    /// Severity.
    pub severity: VulnSeverity,
    /// First fixed version, if any.
    pub fixed_version: Option<Version>,
    /// Short summary.
    pub summary: String,
}

impl Advisory {
    /// Whether this advisory applies to `dep`.
    pub fn affects(&self, dep: &Dependency) -> bool {
        self.ecosystem == dep.ecosystem
            && self.package == dep.name
            && self.range.contains(&dep.version)
    }
}

/// A vulnerability database of advisories.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdvisoryDatabase {
    advisories: Vec<Advisory>,
}

impl AdvisoryDatabase {
    /// Creates an empty database.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an advisory.
    pub fn add(&mut self, advisory: Advisory) {
        self.advisories.push(advisory);
    }

    /// All advisories.
    pub fn advisories(&self) -> &[Advisory] {
        &self.advisories
    }

    /// Advisories affecting a given dependency.
    pub fn matching(&self, dep: &Dependency) -> Vec<&Advisory> {
        self.advisories.iter().filter(|a| a.affects(dep)).collect()
    }
}

/// A finding: a dependency affected by an advisory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VulnFinding {
    /// The affected dependency.
    pub dependency: Dependency,
    /// The advisory it matches.
    pub advisory: Advisory,
}

/// The report of a vulnerability scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanReport {
    /// Findings discovered.
    pub findings: Vec<VulnFinding>,
    /// Number of dependencies scanned.
    pub scanned: usize,
    /// Total dependencies in the inventory.
    pub total: usize,
}

impl ScanReport {
    /// Coverage fraction (scanned / total); 1.0 for an empty inventory.
    pub fn coverage(&self) -> f64 {
        if self.total == 0 {
            1.0
        } else {
            self.scanned as f64 / self.total as f64
        }
    }

    /// Whether every dependency was scanned (100% coverage target).
    pub fn is_full_coverage(&self) -> bool {
        self.scanned == self.total
    }

    /// Highest severity among findings, if any.
    pub fn worst_severity(&self) -> Option<VulnSeverity> {
        self.findings.iter().map(|f| f.advisory.severity).max()
    }
}

/// Scans an inventory against the database. Every dependency is checked, so
/// coverage is always 100% of the inventory.
pub fn scan(inventory: &DependencyInventory, db: &AdvisoryDatabase) -> ScanReport {
    let mut findings = Vec::new();
    for dep in &inventory.dependencies {
        for advisory in db.matching(dep) {
            findings.push(VulnFinding {
                dependency: dep.clone(),
                advisory: advisory.clone(),
            });
        }
    }
    ScanReport {
        findings,
        scanned: inventory.len(),
        total: inventory.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supply_chain::inventory::Dependency;

    fn advisory() -> Advisory {
        Advisory {
            id: "RUSTSEC-2024-0001".to_string(),
            ecosystem: Ecosystem::Cargo,
            package: "vuln-lib".to_string(),
            range: VersionRange::below(Version::new(1, 2, 0)),
            severity: VulnSeverity::High,
            fixed_version: Some(Version::new(1, 2, 0)),
            summary: "RCE in vuln-lib".to_string(),
        }
    }

    fn db() -> AdvisoryDatabase {
        let mut db = AdvisoryDatabase::new();
        db.add(advisory());
        db
    }

    #[test]
    fn severity_from_cvss() {
        assert_eq!(VulnSeverity::from_cvss(9.8), VulnSeverity::Critical);
        assert_eq!(VulnSeverity::from_cvss(7.5), VulnSeverity::High);
        assert_eq!(VulnSeverity::from_cvss(5.0), VulnSeverity::Medium);
        assert_eq!(VulnSeverity::from_cvss(2.0), VulnSeverity::Low);
    }

    #[test]
    fn vulnerable_version_is_flagged() {
        let mut inv = DependencyInventory::new();
        inv.add(Dependency::new(
            "vuln-lib",
            Version::new(1, 1, 0),
            Ecosystem::Cargo,
        ));
        let report = scan(&inv, &db());
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.worst_severity(), Some(VulnSeverity::High));
    }

    #[test]
    fn patched_version_is_clean() {
        let mut inv = DependencyInventory::new();
        inv.add(Dependency::new(
            "vuln-lib",
            Version::new(1, 2, 0),
            Ecosystem::Cargo,
        ));
        let report = scan(&inv, &db());
        assert!(report.findings.is_empty());
    }

    #[test]
    fn ecosystem_mismatch_not_flagged() {
        let mut inv = DependencyInventory::new();
        inv.add(Dependency::new(
            "vuln-lib",
            Version::new(1, 0, 0),
            Ecosystem::Npm,
        ));
        assert!(scan(&inv, &db()).findings.is_empty());
    }

    #[test]
    fn coverage_is_always_full() {
        let mut inv = DependencyInventory::new();
        for i in 0..5 {
            inv.add(Dependency::new(
                format!("dep{i}"),
                Version::new(1, 0, 0),
                Ecosystem::Cargo,
            ));
        }
        let report = scan(&inv, &db());
        assert!(report.is_full_coverage());
        assert_eq!(report.coverage(), 1.0);
    }
}
