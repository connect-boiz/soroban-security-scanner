use crate::vulnerabilities::{VulnerabilityPattern, VulnerabilityType, Severity};

pub fn get_vulnerability_patterns() -> Vec<VulnerabilityPattern> {
    vec![
        // Access Control Patterns
        VulnerabilityPattern {
            id: "ACC-001".to_string(),
            name: "Missing Access Control".to_string(),
            vulnerability_type: VulnerabilityType::AccessControl,
            severity: Severity::Critical,
            description: "Function lacks proper access control checks".to_string(),
            pattern: r"pub\s+fn\s+\w+\s*\([^)]*\)\s*->\s*\w+\s*\{".to_string(),
            recommendation: "Add require_auth() or similar access control checks".to_string(),
            cwe_id: Some("CWE-284".to_string()),
        },
        
        VulnerabilityPattern {
            id: "ACC-002".to_string(),
            name: "Weak Access Control".to_string(),
            vulnerability_type: VulnerabilityType::AccessControl,
            severity: Severity::High,
            description: "Access control check can be bypassed".to_string(),
            pattern: r"env\.authenticator\(\)\s*==\s*[^;]+".to_string(),
            recommendation: "Use strict equality checks and validate permissions".to_string(),
            cwe_id: Some("CWE-284".to_string()),
        },

        // Token Economics Patterns
        VulnerabilityPattern {
            id: "TOKEN-001".to_string(),
            name: "Infinite Mint".to_string(),
            vulnerability_type: VulnerabilityType::TokenEconomics,
            severity: Severity::Critical,
            description: "Token can be minted without limits".to_string(),
            pattern: r"token\.mint\s*\([^)]*\)".to_string(),
            recommendation: "Implement total supply limits and minting controls".to_string(),
            cwe_id: Some("CWE-400".to_string()),
        },

        VulnerabilityPattern {
            id: "TOKEN-002".to_string(),
            name: "Integer Overflow".to_string(),
            vulnerability_type: VulnerabilityType::IntegerOverflow,
            severity: Severity::High,
            description: "Arithmetic operation may overflow".to_string(),
            pattern: r"\w+\s*\+\s*\w+|\w+\s*\*\s*\w+".to_string(),
            recommendation: "Use checked arithmetic or add overflow protection".to_string(),
            cwe_id: Some("CWE-190".to_string()),
        },

        // Logic Vulnerability Patterns
        VulnerabilityPattern {
            id: "LOGIC-001".to_string(),
            name: "Reentrancy".to_string(),
            vulnerability_type: VulnerabilityType::Reentrancy,
            severity: Severity::Critical,
            description: "Contract can be re-entered before state update".to_string(),
            pattern: r"env\.invoke_contract\s*\([^)]*\)".to_string(),
            recommendation: "Implement checks-effects-interactions pattern".to_string(),
            cwe_id: Some("CWE-841".to_string()),
        },

        VulnerabilityPattern {
            id: "LOGIC-002".to_string(),
            name: "Race Condition".to_string(),
            vulnerability_type: VulnerabilityType::RaceCondition,
            severity: Severity::Medium,
            description: "State can be modified between read and write".to_string(),
            pattern: r"let\s+\w+\s*=\s*env\.storage\(\)\.get\([^)]*\)".to_string(),
            recommendation: "Use atomic operations or proper locking".to_string(),
            cwe_id: Some("CWE-362".to_string()),
        },

        // Stellar-Specific Patterns
        VulnerabilityPattern {
            id: "STELLAR-001".to_string(),
            name: "Insufficient Fee Bump".to_string(),
            vulnerability_type: VulnerabilityType::StellarSpecific,
            severity: Severity::Medium,
            description: "Transaction fee handling is insufficient".to_string(),
            pattern: r"env\.get_contract_balance\(\)".to_string(),
            recommendation: "Implement proper fee bumping and balance checks".to_string(),
            cwe_id: Some("CWE-400".to_string()),
        },

        VulnerabilityPattern {
            id: "STELLAR-002".to_string(),
            name: "Invalid Time Bounds".to_string(),
            vulnerability_type: VulnerabilityType::StellarSpecific,
            severity: Severity:Low,
            description: "Time bounds validation is missing or weak".to_string(),
            pattern: r"env\.ledger\.timestamp\(\)".to_string(),
            recommendation: "Add proper time bounds validation".to_string(),
            cwe_id: Some("CWE-20".to_string()),
        },

        VulnerabilityPattern {
            id: "STELLAR-003".to_string(),
            name: "Weak Signature Verification".to_string(),
            vulnerability_type: VulnerabilityType::StellarSpecific,
            severity: Severity::High,
            description: "Signature verification can be bypassed".to_string(),
            pattern: r"env\.authenticator\(\)".to_string(),
            recommendation: "Implement robust signature verification".to_string(),
            cwe_id: Some("CWE-347".to_string()),
        },

        VulnerabilityPattern {
            id: "STELLAR-004".to_string(),
            name: "Stellar Asset Manipulation".to_string(),
            vulnerability_type: VulnerabilityType::StellarSpecific,
            severity: Severity::Critical,
            description: "Stellar asset operations can be manipulated".to_string(),
            pattern: r"token\.transfer\s*\([^)]*\)".to_string(),
            recommendation: "Add proper authorization checks for asset operations".to_string(),
            cwe_id: Some("CWE-20".to_string()),
        },
    ]
}
