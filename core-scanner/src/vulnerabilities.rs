use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

impl Severity {
    pub fn score(&self) -> u8 {
        match self {
            Severity::Critical => 10,
            Severity::High => 7,
            Severity::Medium => 4,
            Severity::Low => 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VulnerabilityType {
    AccessControl,
    TokenEconomics,
    LogicVulnerability,
    StellarSpecific,
    Reentrancy,
    IntegerOverflow,
    FrontRunning,
    RaceCondition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: String,
    pub vulnerability_type: VulnerabilityType,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub location: SourceLocation,
    pub recommendation: String,
    pub cwe_id: Option<String>,
    pub references: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub function: Option<String>,
}

impl Vulnerability {
    pub fn new(
        vulnerability_type: VulnerabilityType,
        severity: Severity,
        title: String,
        description: String,
        location: SourceLocation,
        recommendation: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            vulnerability_type,
            severity,
            title,
            description,
            location,
            recommendation,
            cwe_id: None,
            references: Vec::new(),
        }
    }

    pub fn with_cwe(mut self, cwe_id: String) -> Self {
        self.cwe_id = Some(cwe_id);
        self
    }

    pub fn with_references(mut self, references: Vec<String>) -> Self {
        self.references = references;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityPattern {
    pub id: String,
    pub name: String,
    pub vulnerability_type: VulnerabilityType,
    pub severity: Severity,
    pub description: String,
    pub pattern: String,
    pub recommendation: String,
    pub cwe_id: Option<String>,
}
