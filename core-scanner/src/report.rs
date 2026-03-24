use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::vulnerabilities::Vulnerability;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReport {
    pub id: String,
    pub filename: String,
    pub vulnerabilities: Vec<Vulnerability>,
    pub metrics: HashMap<String, serde_json::Value>,
    pub scan_time: DateTime<Utc>,
    pub code_hash: String,
}

impl ScanReport {
    pub fn new(
        filename: String,
        vulnerabilities: Vec<Vulnerability>,
        metrics: HashMap<String, serde_json::Value>,
        code_hash: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            filename,
            vulnerabilities,
            metrics,
            scan_time: chrono::Utc::now(),
            code_hash,
        }
    }

    pub fn get_summary(&self) -> ScanSummary {
        let critical_count = self.vulnerabilities.iter()
            .filter(|v| matches!(v.severity, crate::vulnerabilities::Severity::Critical))
            .count();
        
        let high_count = self.vulnerabilities.iter()
            .filter(|v| matches!(v.severity, crate::vulnerabilities::Severity::High))
            .count();
        
        let medium_count = self.vulnerabilities.iter()
            .filter(|v| matches!(v.severity, crate::vulnerabilities::Severity::Medium))
            .count();
        
        let low_count = self.vulnerabilities.iter()
            .filter(|v| matches!(v.severity, crate::vulnerabilities::Severity::Low))
            .count();

        let risk_score = self.vulnerabilities.iter()
            .map(|v| v.severity.score() as u32)
            .sum::<u32>();

        ScanSummary {
            scan_id: self.id.clone(),
            filename: self.filename.clone(),
            total_vulnerabilities: self.vulnerabilities.len(),
            critical_count,
            high_count,
            medium_count,
            low_count,
            risk_score,
            scan_time: self.scan_time,
        }
    }

    pub fn get_vulnerabilities_by_severity(&self) -> HashMap<String, Vec<&Vulnerability>> {
        let mut by_severity: HashMap<String, Vec<&Vulnerability>> = HashMap::new();
        
        for vulnerability in &self.vulnerabilities {
            let severity_str = format!("{:?}", vulnerability.severity);
            by_severity.entry(severity_str).or_insert_with(Vec::new).push(vulnerability);
        }
        
        by_severity
    }

    pub fn get_vulnerabilities_by_type(&self) -> HashMap<String, Vec<&Vulnerability>> {
        let mut by_type: HashMap<String, Vec<&Vulnerability>> = HashMap::new();
        
        for vulnerability in &self.vulnerabilities {
            let type_str = format!("{:?}", vulnerability.vulnerability_type);
            by_type.entry(type_str).or_insert_with(Vec::new).push(vulnerability);
        }
        
        by_type
    }

    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn export_csv(&self) -> String {
        let mut csv = String::new();
        csv.push_str("ID,Type,Severity,Title,Description,File,Line,Column,Function,Recommendation\n");
        
        for vuln in &self.vulnerabilities {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{}\n",
                vuln.id,
                format!("{:?}", vuln.vulnerability_type),
                format!("{:?}", vuln.severity),
                vuln.title.replace(',', ";"),
                vuln.description.replace(',', ";"),
                vuln.location.file,
                vuln.location.line,
                vuln.location.column,
                vuln.location.function.as_ref().unwrap_or(&"None".to_string()),
                vuln.recommendation.replace(',', ";")
            ));
        }
        
        csv
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub scan_id: String,
    pub filename: String,
    pub total_vulnerabilities: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub risk_score: u32,
    pub scan_time: DateTime<Utc>,
}

impl ScanSummary {
    pub fn get_risk_level(&self) -> RiskLevel {
        match self.risk_score {
            score if score >= 50 => RiskLevel::Critical,
            score if score >= 30 => RiskLevel::High,
            score if score >= 15 => RiskLevel::Medium,
            _ => RiskLevel::Low,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateReport {
    pub id: String,
    pub scan_summaries: Vec<ScanSummary>,
    pub total_vulnerabilities: usize,
    pub total_files: usize,
    pub aggregate_metrics: HashMap<String, serde_json::Value>,
    pub generated_at: DateTime<Utc>,
}

impl AggregateReport {
    pub fn new(scan_summaries: Vec<ScanSummary>) -> Self {
        let total_vulnerabilities = scan_summaries.iter()
            .map(|s| s.total_vulnerabilities)
            .sum();
        
        let total_files = scan_summaries.len();
        
        let mut aggregate_metrics = HashMap::new();
        aggregate_metrics.insert("total_vulnerabilities".to_string(), 
                                serde_json::Value::Number(total_vulnerabilities.into()));
        aggregate_metrics.insert("total_files".to_string(), 
                                serde_json::Value::Number(total_files.into()));
        
        let critical_total = scan_summaries.iter()
            .map(|s| s.critical_count)
            .sum::<usize>();
        aggregate_metrics.insert("critical_vulnerabilities".to_string(), 
                                serde_json::Value::Number(critical_total.into()));
        
        let high_total = scan_summaries.iter()
            .map(|s| s.high_count)
            .sum::<usize>();
        aggregate_metrics.insert("high_vulnerabilities".to_string(), 
                                serde_json::Value::Number(high_total.into()));
        
        let avg_risk_score = if total_files > 0 {
            scan_summaries.iter()
                .map(|s| s.risk_score as f64)
                .sum::<f64>() / total_files as f64
        } else {
            0.0
        };
        aggregate_metrics.insert("average_risk_score".to_string(), 
                                serde_json::Value::Number(avg_risk_score.into()));

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            scan_summaries,
            total_vulnerabilities,
            total_files,
            aggregate_metrics,
            generated_at: chrono::Utc::now(),
        }
    }
}
