use crate::error::{ServiceResult};
use crate::models::{CodePatch, VulnerabilityReport, VerificationStatus};
use regex::Regex;
use std::collections::HashMap;

pub struct ConfidenceScorer {
    security_patterns: HashMap<String, f64>,
    quality_patterns: HashMap<String, f64>,
}

impl ConfidenceScorer {
    pub fn new() -> Self {
        let mut scorer = Self {
            security_patterns: HashMap::new(),
            quality_patterns: HashMap::new(),
        };
        
        scorer.init_patterns();
        scorer
    }
    
    fn init_patterns(&mut self) {
        // Security improvement patterns (positive weights)
        self.security_patterns.insert(
            r"require_auth\(\)|require_auth_for_admin\(\)".to_string(),
            0.15
        );
        self.security_patterns.insert(
            r"checked_add\(|checked_sub\(|checked_mul\(|checked_div\(".to_string(),
            0.10
        );
        self.security_patterns.insert(
            r"Result<.*, Error>|Ok\(|Err\(".to_string(),
            0.10
        );
        self.security_patterns.insert(
            r"env\.current_time\(\)|env\.ledger\(\)".to_string(),
            0.05
        );
        self.security_patterns.insert(
            r"#[contractclient]|#[contractimpl]".to_string(),
            0.05
        );
        
        // Code quality patterns (positive weights)
        self.quality_patterns.insert(
            r"///.*|//.*".to_string(),
            0.05
        );
        self.quality_patterns.insert(
            r"fn\s+\w+\s*\([^)]*\)\s*->\s*\w+".to_string(),
            0.05
        );
        self.quality_patterns.insert(
            r"let\s+mut\s+\w+|const\s+\w+|static\s+\w+".to_string(),
            0.03
        );
        self.quality_patterns.insert(
            r"if\s+.*\s*\{|match\s+.*\s*\{|for\s+.*\s+in".to_string(),
            0.03
        );
        
        // Negative patterns (reduce confidence)
        self.security_patterns.insert(
            r"panic!\(\)|unwrap\(\)|expect\(".to_string(),
            -0.20
        );
        self.security_patterns.insert(
            r"unsafe\s*\{".to_string(),
            -0.15
        );
        self.security_patterns.insert(
            r"\.clone\(\)|\.copy\(\)".to_string(),
            -0.05
        );
        self.security_patterns.insert(
            r"todo!\(\)|unimplemented!\(\)".to_string(),
            -0.30
        );
    }
    
    pub async fn calculate_confidence(
        &self,
        patch: &CodePatch,
        vulnerability: &VulnerabilityReport,
        verification_status: VerificationStatus,
    ) -> ServiceResult<f64> {
        let mut confidence = 0.5; // Base confidence
        
        // Factor 1: Verification status (30% weight)
        confidence += self.verification_score(verification_status) * 0.3;
        
        // Factor 2: Code quality analysis (25% weight)
        confidence += self.code_quality_score(&patch.patched_code) * 0.25;
        
        // Factor 3: Security improvements (20% weight)
        confidence += self.security_improvement_score(patch, vulnerability) * 0.2;
        
        // Factor 4: Explanation quality (15% weight)
        confidence += self.explanation_quality_score(&patch.explanation) * 0.15;
        
        // Factor 5: Vulnerability complexity (10% weight)
        confidence += self.vulnerability_complexity_score(vulnerability) * 0.1;
        
        // Ensure confidence is within [0.0, 1.0]
        confidence = confidence.max(0.0).min(1.0);
        
        Ok(confidence)
    }
    
    fn verification_score(&self, status: VerificationStatus) -> f64 {
        match status {
            VerificationStatus::Passed => 1.0,
            VerificationStatus::Failed => 0.0,
            VerificationStatus::Skipped => 0.5,
        }
    }
    
    fn code_quality_score(&self, patched_code: &str) -> f64 {
        let mut score = 0.5; // Base score
        
        // Check for positive patterns
        for (pattern, weight) in &self.quality_patterns {
            let regex = Regex::new(pattern).unwrap();
            let matches = regex.find_iter(patched_code).count();
            score += matches as f64 * weight;
        }
        
        // Check for negative patterns
        for (pattern, weight) in &self.security_patterns {
            if weight.is_negative() {
                let regex = Regex::new(pattern).unwrap();
                let matches = regex.find_iter(patched_code).count();
                score += matches as f64 * weight;
            }
        }
        
        // Bonus for proper error handling
        if patched_code.contains("Result<") && patched_code.contains("Ok(") && patched_code.contains("Err(") {
            score += 0.1;
        }
        
        // Bonus for comprehensive documentation
        let doc_lines = patched_code.lines()
            .filter(|line| line.trim().starts_with("///") || line.trim().starts_with("//"))
            .count();
        if doc_lines > 2 {
            score += 0.05;
        }
        
        score.max(0.0).min(1.0)
    }
    
    fn security_improvement_score(&self, patch: &CodePatch, vulnerability: &VulnerabilityReport) -> f64 {
        let mut score = 0.5;
        
        // Check if security improvements are mentioned
        if !patch.security_improvements.is_empty() {
            score += 0.2;
        }
        
        // Check for specific security patterns in the patched code
        for (pattern, weight) in &self.security_patterns {
            if weight.is_positive() {
                let regex = Regex::new(pattern).unwrap();
                if regex.is_match(&patch.patched_code) {
                    score += weight;
                }
            }
        }
        
        // Vulnerability-specific scoring
        match vulnerability.vulnerability_type.to_lowercase().as_str() {
            "accesscontrol" => {
                if patch.patched_code.contains("require_auth") || 
                   patch.patched_code.contains("has_auth") {
                    score += 0.2;
                }
            },
            "tokeneconomics" => {
                if patch.patched_code.contains("checked_") ||
                   patch.patched_code.contains("Result<") {
                    score += 0.2;
                }
            },
            "reentrancy" => {
                if patch.patched_code.contains("env\.current_time") ||
                   patch.patched_code.contains("nonreentrant") {
                    score += 0.2;
                }
            },
            "integeroverflow" => {
                if patch.patched_code.contains("checked_") ||
                   patch.patched_code.contains("overflowing") {
                    score += 0.2;
                }
            },
            _ => {}
        }
        
        score.max(0.0).min(1.0)
    }
    
    fn explanation_quality_score(&self, explanation: &str) -> f64 {
        let mut score = 0.5;
        
        // Length and detail
        let word_count = explanation.split_whitespace().count();
        if word_count > 50 {
            score += 0.1;
        } else if word_count < 20 {
            score -= 0.1;
        }
        
        // Security-specific terminology
        let security_terms = vec![
            "vulnerability", "security", "attack", "mitigation", "protection",
            "validation", "authorization", "authentication", "safe", "secure",
            "prevent", "check", "verify", "ensure", "protect"
        ];
        
        let security_term_count = security_terms.iter()
            .map(|term| explanation.matches(term).count())
            .sum::<usize>();
        
        score += (security_term_count as f64 * 0.02).min(0.2);
        
        // Structure and clarity
        if explanation.contains(".") && explanation.len() > 100 {
            score += 0.05;
        }
        
        score.max(0.0).min(1.0)
    }
    
    fn vulnerability_complexity_score(&self, vulnerability: &VulnerabilityReport) -> f64 {
        match vulnerability.severity.to_lowercase().as_str() {
            "critical" => 0.3, // Harder to fix critical issues
            "high" => 0.5,
            "medium" => 0.7,
            "low" => 0.9, // Easier to fix simple issues
            _ => 0.5,
        }
    }
    
    pub fn get_confidence_level(&self, confidence: f64) -> String {
        match confidence {
            x if x >= 0.8 => "High".to_string(),
            x if x >= 0.6 => "Medium".to_string(),
            x if x >= 0.4 => "Low".to_string(),
            _ => "Very Low".to_string(),
        }
    }
    
    pub fn get_confidence_factors(&self, confidence: f64) -> Vec<String> {
        let mut factors = Vec::new();
        
        if confidence >= 0.8 {
            factors.push("Code compiles successfully".to_string());
            factors.push("Strong security improvements detected".to_string());
            factors.push("Comprehensive explanation provided".to_string());
        } else if confidence >= 0.6 {
            factors.push("Code compiles successfully".to_string());
            factors.push("Some security improvements detected".to_string());
        } else if confidence >= 0.4 {
            factors.push("Basic code structure maintained".to_string());
        } else {
            factors.push("Low confidence - manual review required".to_string());
        }
        
        factors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{VulnerabilityReport, SourceLocation};
    
    #[test]
    fn test_confidence_calculation() {
        let scorer = ConfidenceScorer::new();
        
        let patch = CodePatch {
            original_code: "fn vulnerable_code() { let x = 1 + 2; }".to_string(),
            patched_code: r#"
use soroban_sdk::Env;

fn secure_code(env: &Env, x: u64, y: u64) -> Result<u64, Error> {
    x.checked_add(y).ok_or(Error::Overflow)
}
"#.to_string(),
            explanation: "This patch fixes the integer overflow vulnerability by using checked_add instead of direct addition. This prevents overflow attacks and ensures the contract operates safely.".to_string(),
            security_improvements: vec![
                "Added overflow protection".to_string(),
                "Implemented proper error handling".to_string(),
            ],
        };
        
        let vulnerability = VulnerabilityReport {
            id: "test-1".to_string(),
            file_path: "test.rs".to_string(),
            vulnerability_type: "IntegerOverflow".to_string(),
            severity: "High".to_string(),
            title: "Integer Overflow".to_string(),
            description: "Potential integer overflow".to_string(),
            code_snippet: "1 + 2".to_string(),
            line_number: 1,
            sarif_report: None,
        };
        
        let confidence = futures::executor::block_on(
            scorer.calculate_confidence(&patch, &vulnerability, VerificationStatus::Passed)
        );
        
        assert!(confidence > 0.5);
        assert_eq!(scorer.get_confidence_level(confidence), "Medium");
    }
}
