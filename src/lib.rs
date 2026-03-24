//! Security and Invariant Scanners for Stellar Smart Contracts
//! 
//! This crate provides comprehensive security analysis tools for Stellar Soroban contracts,
//! including vulnerability detection, invariant checking, and best practices enforcement.

pub mod scanners;
pub mod vulnerabilities;
pub mod invariants;
pub mod analysis;
pub mod report;
pub mod config;
pub mod kubernetes;
pub mod bounty_marketplace;
pub mod scanner_registry;


pub use scanners::{SecurityScanner, InvariantScanner};
pub use vulnerabilities::VulnerabilityType;
pub use invariants::InvariantRule;
pub use analysis::AnalysisResult;
pub use report::{SecurityReport, ReportFormat};
pub use config::ScannerConfig;
pub use kubernetes::{K8sScanManager, ScanPodConfig, ScanAutoScaler};
pub use scanner_registry::{ScannerRegistry, ScannerVersion, VersionStatus};

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub file_path: String,
    pub vulnerabilities: Vec<VulnerabilityType>,
    pub invariant_violations: Vec<InvariantRule>,
    pub recommendations: Vec<String>,
}

impl ScanResult {
    pub fn new(file_path: String) -> Self {
        Self {
            file_path,
            vulnerabilities: Vec::new(),
            invariant_violations: Vec::new(),
            recommendations: Vec::new(),
        }
    }

    pub fn has_issues(&self) -> bool {
        !self.vulnerabilities.is_empty() || !self.invariant_violations.is_empty()
    }

    pub fn severity_count(&self) -> (usize, usize, usize) {
        let mut critical = 0;
        let mut high = 0;
        let mut medium = 0;

        for vuln in &self.vulnerabilities {
            match vuln.severity() {
                Severity::Critical => critical += 1,
                Severity::High => high += 1,
                Severity::Medium => medium += 1,
                Severity::Low => {}
            }
        }

        (critical, high, medium)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Critical => "CRITICAL",
            Severity::High => "HIGH",
            Severity::Medium => "MEDIUM",
            Severity::Low => "LOW",
        }
    }
}
