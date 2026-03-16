//! Analysis and result aggregation for security scans

use crate::{ScanResult, VulnerabilityType, InvariantRule, Severity};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub scan_summary: ScanSummary,
    pub vulnerability_analysis: VulnerabilityAnalysis,
    pub invariant_analysis: InvariantAnalysis,
    pub recommendations: Vec<Recommendation>,
    pub risk_score: RiskScore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub total_files_scanned: usize,
    pub files_with_issues: usize,
    pub total_vulnerabilities: usize,
    pub total_invariant_violations: usize,
    pub scan_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityAnalysis {
    pub vulnerabilities_by_type: HashMap<VulnerabilityType, usize>,
    pub vulnerabilities_by_severity: HashMap<Severity, usize>,
    pub most_common_vulnerabilities: Vec<(VulnerabilityType, usize)>,
    pub critical_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantAnalysis {
    pub violations_by_rule: HashMap<InvariantRule, usize>,
    pub violations_by_severity: HashMap<Severity, usize>,
    pub most_violated_invariants: Vec<(InvariantRule, usize)>,
    pub critical_invariants: Vec<InvariantRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub category: String,
    pub priority: Severity,
    pub description: String,
    pub affected_files: Vec<String>,
    pub implementation_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskScore {
    pub overall_score: f64,
    pub security_score: f64,
    pub invariant_score: f64,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl AnalysisResult {
    pub fn new(results: Vec<ScanResult>, scan_duration_ms: u64) -> Self {
        let scan_summary = Self::create_scan_summary(&results, scan_duration_ms);
        let vulnerability_analysis = Self::analyze_vulnerabilities(&results);
        let invariant_analysis = Self::analyze_invariants(&results);
        let recommendations = Self::generate_recommendations(&results, &vulnerability_analysis, &invariant_analysis);
        let risk_score = Self::calculate_risk_score(&vulnerability_analysis, &invariant_analysis);

        Self {
            scan_summary,
            vulnerability_analysis,
            invariant_analysis,
            recommendations,
            risk_score,
        }
    }

    fn create_scan_summary(results: &[ScanResult], scan_duration_ms: u64) -> ScanSummary {
        let total_files_scanned = results.len();
        let files_with_issues = results.iter().filter(|r| r.has_issues()).count();
        let total_vulnerabilities = results.iter().map(|r| r.vulnerabilities.len()).sum();
        let total_invariant_violations = results.iter().map(|r| r.invariant_violations.len()).sum();

        ScanSummary {
            total_files_scanned,
            files_with_issues,
            total_vulnerabilities,
            total_invariant_violations,
            scan_duration_ms,
        }
    }

    fn analyze_vulnerabilities(results: &[ScanResult]) -> VulnerabilityAnalysis {
        let mut vulnerabilities_by_type = HashMap::new();
        let mut vulnerabilities_by_severity = HashMap::new();
        let mut critical_files = Vec::new();

        for result in results {
            if !result.vulnerabilities.is_empty() {
                let (critical, high, _) = result.severity_count();
                if critical > 0 || high > 0 {
                    critical_files.push(result.file_path.clone());
                }
            }

            for vulnerability in &result.vulnerabilities {
                *vulnerabilities_by_type.entry(vulnerability.clone()).or_insert(0) += 1;
                *vulnerabilities_by_severity.entry(vulnerability.severity()).or_insert(0) += 1;
            }
        }

        let mut most_common_vulnerabilities: Vec<_> = vulnerabilities_by_type.iter().collect();
        most_common_vulnerabilities.sort_by(|a, b| b.1.cmp(a.1));
        let most_common_vulnerabilities = most_common_vulnerabilities.into_iter()
            .take(10)
            .map(|(vuln, count)| (vuln.clone(), *count))
            .collect();

        VulnerabilityAnalysis {
            vulnerabilities_by_type,
            vulnerabilities_by_severity,
            most_common_vulnerabilities,
            critical_files,
        }
    }

    fn analyze_invariants(results: &[ScanResult]) -> InvariantAnalysis {
        let mut violations_by_rule = HashMap::new();
        let mut violations_by_severity = HashMap::new();

        for result in results {
            for invariant in &result.invariant_violations {
                *violations_by_rule.entry(invariant.clone()).or_insert(0) += 1;
                *violations_by_severity.entry(invariant.severity()).or_insert(0) += 1;
            }
        }

        let mut most_violated_invariants: Vec<_> = violations_by_rule.iter().collect();
        most_violated_invariants.sort_by(|a, b| b.1.cmp(a.1));
        let most_violated_invariants = most_violated_invariants.into_iter()
            .take(10)
            .map(|(rule, count)| (rule.clone(), *count))
            .collect();

        let critical_invariants = violations_by_rule.keys()
            .filter(|rule| rule.severity() == Severity::Critical || rule.severity() == Severity::High)
            .cloned()
            .collect();

        InvariantAnalysis {
            violations_by_rule,
            violations_by_severity,
            most_violated_invariants,
            critical_invariants,
        }
    }

    fn generate_recommendations(
        results: &[ScanResult],
        vuln_analysis: &VulnerabilityAnalysis,
        inv_analysis: &InvariantAnalysis,
    ) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();

        // Access Control Recommendations
        if vuln_analysis.vulnerabilities_by_type.contains_key(&VulnerabilityType::MissingAccessControl) {
            let affected_files: Vec<String> = results.iter()
                .filter(|r| r.vulnerabilities.contains(&VulnerabilityType::MissingAccessControl))
                .map(|r| r.file_path.clone())
                .collect();

            recommendations.push(Recommendation {
                category: "Access Control".to_string(),
                priority: Severity::Critical,
                description: "Implement proper access control mechanisms for all public functions".to_string(),
                affected_files,
                implementation_hint: "Use require_auth() or custom authorization logic to protect sensitive functions".to_string(),
            });
        }

        // Token Economics Recommendations
        if vuln_analysis.vulnerabilities_by_type.contains_key(&VulnerabilityType::InfiniteMint) {
            let affected_files: Vec<String> = results.iter()
                .filter(|r| r.vulnerabilities.contains(&VulnerabilityType::InfiniteMint))
                .map(|r| r.file_path.clone())
                .collect();

            recommendations.push(Recommendation {
                category: "Token Economics".to_string(),
                priority: Severity::Critical,
                description: "Implement supply limits and proper minting controls".to_string(),
                affected_files,
                implementation_hint: "Add max_supply constant and validate minting against limits".to_string(),
            });
        }

        // Reentrancy Recommendations
        if vuln_analysis.vulnerabilities_by_type.contains_key(&VulnerabilityType::Reentrancy) {
            let affected_files: Vec<String> = results.iter()
                .filter(|r| r.vulnerabilities.contains(&VulnerabilityType::Reentrancy))
                .map(|r| r.file_path.clone())
                .collect();

            recommendations.push(Recommendation {
                category: "Security".to_string(),
                priority: Severity::High,
                description: "Implement reentrancy protection using checks-effects-interactions pattern".to_string(),
                affected_files,
                implementation_hint: "Use reentrancy guards and ensure state changes happen before external calls".to_string(),
            });
        }

        // Invariant Recommendations
        if !inv_analysis.critical_invariants.is_empty() {
            let affected_files: Vec<String> = results.iter()
                .filter(|r| r.invariant_violations.iter().any(|inv| inv.severity() == Severity::Critical))
                .map(|r| r.file_path.clone())
                .collect();

            recommendations.push(Recommendation {
                category: "Invariants".to_string(),
                priority: Severity::High,
                description: "Implement proper invariant checking and enforcement".to_string(),
                affected_files,
                implementation_hint: "Add invariant checks at function boundaries and state transitions".to_string(),
            });
        }

        // General Best Practices
        if vuln_analysis.vulnerabilities_by_type.contains_key(&VulnerabilityType::LackOfInputValidation) {
            recommendations.push(Recommendation {
                category: "Best Practices".to_string(),
                priority: Severity::Medium,
                description: "Add comprehensive input validation for all external inputs".to_string(),
                affected_files: vec!["Multiple files".to_string()],
                implementation_hint: "Validate ranges, types, and business logic constraints for all inputs".to_string(),
            });
        }

        recommendations
    }

    fn calculate_risk_score(vuln_analysis: &VulnerabilityAnalysis, inv_analysis: &InvariantAnalysis) -> RiskScore {
        let security_score = Self::calculate_security_score(vuln_analysis);
        let invariant_score = Self::calculate_invariant_score(inv_analysis);
        let overall_score = (security_score + invariant_score) / 2.0;

        let risk_level = match overall_score {
            score if score >= 8.0 => RiskLevel::Critical,
            score if score >= 6.0 => RiskLevel::High,
            score if score >= 4.0 => RiskLevel::Medium,
            _ => RiskLevel::Low,
        };

        RiskScore {
            overall_score,
            security_score,
            invariant_score,
            risk_level,
        }
    }

    fn calculate_security_score(analysis: &VulnerabilityAnalysis) -> f64 {
        let critical_count = analysis.vulnerabilities_by_severity.get(&Severity::Critical).unwrap_or(&0);
        let high_count = analysis.vulnerabilities_by_severity.get(&Severity::High).unwrap_or(&0);
        let medium_count = analysis.vulnerabilities_by_severity.get(&Severity::Medium).unwrap_or(&0);
        let low_count = analysis.vulnerabilities_by_severity.get(&Severity::Low).unwrap_or(&0);

        let total_issues = critical_count + high_count + medium_count + low_count;
        if total_issues == 0 {
            return 10.0; // Perfect score
        }

        let weighted_score = (critical_count * 10 + high_count * 7 + medium_count * 4 + low_count * 1) as f64;
        let max_possible_score = (total_issues * 10) as f64;
        
        10.0 - (weighted_score / max_possible_score * 10.0)
    }

    fn calculate_invariant_score(analysis: &InvariantAnalysis) -> f64 {
        let critical_count = analysis.violations_by_severity.get(&Severity::Critical).unwrap_or(&0);
        let high_count = analysis.violations_by_severity.get(&Severity::High).unwrap_or(&0);
        let medium_count = analysis.violations_by_severity.get(&Severity::Medium).unwrap_or(&0);
        let low_count = analysis.violations_by_severity.get(&Severity::Low).unwrap_or(&0);

        let total_issues = critical_count + high_count + medium_count + low_count;
        if total_issues == 0 {
            return 10.0; // Perfect score
        }

        let weighted_score = (critical_count * 10 + high_count * 7 + medium_count * 4 + low_count * 1) as f64;
        let max_possible_score = (total_issues * 10) as f64;
        
        10.0 - (weighted_score / max_possible_score * 10.0)
    }
}
