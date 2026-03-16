use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRequest {
    pub code: String,
    pub deep_analysis: Option<bool>,
    pub check_invariants: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: String,
    pub title: String,
    pub severity: String,
    pub description: String,
    pub location: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub id: String,
    pub status: String,
    pub vulnerabilities: Vec<Vulnerability>,
    pub scan_time: DateTime<Utc>,
    pub contract_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub total_scans: i64,
    pub critical_vulnerabilities: i64,
    pub high_vulnerabilities: i64,
    pub medium_vulnerabilities: i64,
    pub low_vulnerabilities: i64,
    pub last_scan: Option<DateTime<Utc>>,
}
