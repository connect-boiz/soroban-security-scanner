use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityReport {
    pub id: String,
    pub file_path: String,
    pub vulnerability_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub code_snippet: String,
    pub line_number: usize,
    pub sarif_report: Option<SarifReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifReport {
    pub version: String,
    pub runs: Vec<SarifRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifRun {
    pub tool: SarifTool,
    pub results: Vec<SarifResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifTool {
    pub driver: SarifDriver,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifDriver {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifResult {
    pub rule_id: String,
    pub message: SarifMessage,
    pub level: String,
    pub locations: Vec<SarifLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifMessage {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifLocation {
    pub physical_location: SarifPhysicalLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifPhysicalLocation {
    pub artifact_location: SarifArtifactLocation,
    pub region: SarifRegion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifArtifactLocation {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifRegion {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchRequest {
    pub vulnerability: VulnerabilityReport,
    pub original_code: String,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchResponse {
    pub id: String,
    pub vulnerability_id: String,
    pub patch: CodePatch,
    pub confidence_score: f64,
    pub verification_status: VerificationStatus,
    pub git_diff: String,
    pub fallback_provided: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodePatch {
    pub original_code: String,
    pub patched_code: String,
    pub explanation: String,
    pub security_improvements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationStatus {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationHistory {
    pub id: String,
    pub vulnerability_id: String,
    pub patch: CodePatch,
    pub confidence_score: f64,
    pub verification_status: VerificationStatus,
    pub applied: bool,
    pub success_rate: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub provider: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub host: String,
    pub port: u16,
    pub llm: LLMConfig,
    pub database: DatabaseConfig,
}
