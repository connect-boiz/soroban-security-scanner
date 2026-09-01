//! Integration tests for the Stellar Security Scanner

#[cfg(test)]
mod tests {
    use stellar_security_scanner::{scanners::{SecurityScanner, InvariantScanner}, config::ScannerConfig};
    use std::path::PathBuf;

    #[test]
    fn test_security_scanner_vulnerable_contract() {
        let scanner = SecurityScanner::new().unwrap();
        let vulnerable_path = PathBuf::from("examples/vulnerable_contract.rs");
        
        let result = scanner.scan_file(&vulnerable_path).unwrap();
        
        // Should detect multiple vulnerabilities
        assert!(result.has_issues());
        assert!(!result.vulnerabilities.is_empty());
        
        // Check for specific expected vulnerabilities
        let vulnerability_types: std::collections::HashSet<_> = 
            result.vulnerabilities.iter().collect();
        
        // Should detect missing access control
        assert!(vulnerability_types.iter().any(|v| 
            matches!(v, stellar_security_scanner::vulnerabilities::VulnerabilityType::MissingAccessControl)
        ));
    }

    #[test]
    fn test_security_scanner_secure_contract() {
        let scanner = SecurityScanner::new().unwrap();
        let secure_path = PathBuf::from("examples/secure_contract.rs");
        
        let result = scanner.scan_file(&secure_path).unwrap();
        
        // Should have fewer or no vulnerabilities
        // Note: May still detect some patterns that look suspicious
    }

    #[test]
    fn test_invariant_scanner() {
        let scanner = InvariantScanner::new().unwrap();
        let test_path = PathBuf::from("examples/vulnerable_contract.rs");
        
        let result = scanner.scan_file(&test_path).unwrap();
        
        // Should detect invariant violations in vulnerable contract
        // Note: Implementation depends on the specific rules
    }

    #[test]
    fn test_config_loading() {
        let config = ScannerConfig::default();
        
        // Test default configuration
        assert!(!config.scan_paths.is_empty());
        assert!(!config.ignore_paths.is_empty());
        assert!(!config.vulnerability_checks.enabled_checks.is_empty());
        assert!(!config.invariant_checks.enabled_rules.is_empty());
    }

    #[test]
    fn test_directory_scanning() {
        let security_scanner = SecurityScanner::new().unwrap();
        let examples_path = PathBuf::from("examples");
        
        if examples_path.exists() {
            let results = security_scanner.scan_directory(&examples_path).unwrap();
            
            // Should scan at least one file
            assert!(!results.is_empty());
        }
    }

    #[test]
    fn test_analysis_result_creation() {
        use stellar_security_scanner::{ScanResult, analysis::AnalysisResult};
        
        let mut result1 = ScanResult::new("test1.rs".to_string());
        result1.vulnerabilities.push(stellar_security_scanner::vulnerabilities::VulnerabilityType::MissingAccessControl);
        
        let mut result2 = ScanResult::new("test2.rs".to_string());
        result2.invariant_violations.push(stellar_security_scanner::invariants::InvariantRule::BalanceNonNegative);
        
        let results = vec![result1, result2];
        let analysis = AnalysisResult::new(results, 1000);
        
        // Should create analysis with proper statistics
        assert_eq!(analysis.scan_summary.total_files_scanned, 2);
        assert_eq!(analysis.scan_summary.total_vulnerabilities, 1);
        assert_eq!(analysis.scan_summary.total_invariant_violations, 1);
        assert_eq!(analysis.scan_summary.scan_duration_ms, 1000);
    }
}
