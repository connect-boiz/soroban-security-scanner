//! Gas Limit Analysis Module
//!
//! This module provides comprehensive analysis of gas limit considerations
//! for Soroban smart contracts, focusing on complex operations that
//! may exhaust transaction gas without proper validation.

use std::collections::HashMap;
use syn::{Item, ItemFn, Expr, Stmt, visit::Visit};
use regex::Regex;
use anyhow::Result;
use tracing::debug;

use crate::vulnerabilities::{Vulnerability, VulnerabilityType, Severity, SourceLocation};

/// Gas limit analyzer for detecting insufficient gas considerations
pub struct GasLimitAnalyzer {
    /// Patterns for detecting gas limit issues
    gas_limit_patterns: Vec<GasLimitPattern>,
}

/// Pattern for detecting gas limit vulnerabilities
struct GasLimitPattern {
    id: String,
    name: String,
    vulnerability_type: VulnerabilityType,
    severity: Severity,
    description: String,
    pattern: Regex,
    recommendation: String,
    cwe_id: Option<String>,
}

impl GasLimitAnalyzer {
    /// Creates a new gas limit analyzer
    pub fn new() -> Self {
        let patterns = vec![
            // Unbounded loop operations
            GasLimitPattern {
                id: "GAS-001".to_string(),
                name: "Unbounded Loop Operations".to_string(),
                vulnerability_type: VulnerabilityType::StellarSpecific,
                severity: Severity::High,
                description: "Loop operations without gas limit checks can exhaust transaction gas".to_string(),
                pattern: Regex::new(r"for\s+\w+\s+in\s+\w+\.\s*iter\(\)|while\s+\w+\s*\{").unwrap(),
                recommendation: "Add gas limit checks and consider batch processing for large operations".to_string(),
                cwe_id: Some("CWE-400".to_string()),
            },

            // Missing gas limit validation
            GasLimitPattern {
                id: "GAS-002".to_string(),
                name: "Missing Gas Limit Validation".to_string(),
                vulnerability_type: VulnerabilityType::StellarSpecific,
                severity: Severity::High,
                description: "Complex operations lack gas limit validation before execution".to_string(),
                pattern: Regex::new(r"env\.invoke_contract\s*\([^)]*\)|env\.storage\(\)\.get\([^)]*\)").unwrap(),
                recommendation: "Validate gas availability before complex operations and implement batch processing".to_string(),
                cwe_id: Some("CWE-400".to_string()),
            },

            // Emergency operations without gas checks
            GasLimitPattern {
                id: "GAS-003".to_string(),
                name: "Emergency Operations Without Gas Checks".to_string(),
                vulnerability_type: VulnerabilityType::StellarSpecific,
                severity: Severity::Critical,
                description: "Emergency reward/escrow release functions don't validate gas limits".to_string(),
                pattern: Regex::new(r"fn\s+(emergency_|release_|distribute_)[^}]*env\.transfer\(|env\.invoke_contract\(").unwrap(),
                recommendation: "Add gas limit validation to emergency operations and implement batch processing".to_string(),
                cwe_id: Some("CWE-400".to_string()),
            },

            // Large array operations without gas limits
            GasLimitPattern {
                id: "GAS-004".to_string(),
                name: "Large Array Operations Without Gas Limits".to_string(),
                vulnerability_type: VulnerabilityType::StellarSpecific,
                severity: Severity::High,
                description: "Large array operations without gas limit consideration can cause transaction failures".to_string(),
                pattern: Regex::new(r"Vec::new\([^)]*\)\.push_back\([^)]*\)|for\s+\w+\s+in\s+\w+\.\s*iter\(\)\s*\{[^}]*\.push_back").unwrap(),
                recommendation: "Implement chunked processing for large arrays and add gas limit checks".to_string(),
                cwe_id: Some("CWE-400".to_string()),
            },

            // Missing gas estimation
            GasLimitPattern {
                id: "GAS-005".to_string(),
                name: "Missing Gas Estimation".to_string(),
                vulnerability_type: VulnerabilityType::StellarSpecific,
                severity: Severity::Medium,
                description: "Functions don't estimate or track gas usage for complex operations".to_string(),
                pattern: Regex::new(r"fn\s+\w+\s*\([^)]*\)\s*->\s*[^{]*\{[^}]*env\.(budget|ledger)").unwrap(),
                recommendation: "Add gas estimation and tracking before executing complex operations".to_string(),
                cwe_id: Some("CWE-400".to_string()),
            },

            // Batch operations without optimization
            GasLimitPattern {
                id: "GAS-006".to_string(),
                name: "Inefficient Batch Processing".to_string(),
                vulnerability_type: VulnerabilityType::StellarSpecific,
                severity: Severity::Medium,
                description: "Batch operations process items individually without gas optimization".to_string(),
                pattern: Regex::new(r"for\s+\w+\s+in\s+\w+\s*\{[^}]*env\.invoke_contract").unwrap(),
                recommendation: "Implement efficient batch processing with proper gas estimation".to_string(),
                cwe_id: Some("CWE-400".to_string()),
            },
        ];

        Self { gas_limit_patterns: patterns }
    }

    /// Analyzes contract code for gas limit vulnerabilities
    pub fn analyze(&self, code: &str, filename: &str) -> Result<Vec<Vulnerability>> {
        debug!("Starting gas limit analysis on {}", filename);
        let mut vulnerabilities = Vec::new();
        let lines: Vec<&str> = code.lines().collect();

        for pattern in &self.gas_limit_patterns {
            debug!("Checking pattern {}: {}", pattern.id, pattern.name);
            
            for (line_num, line) in lines.iter().enumerate() {
                if let Some(mat) = pattern.pattern.find(line) {
                    let vulnerability = Vulnerability::new(
                        pattern.vulnerability_type.clone(),
                        pattern.severity.clone(),
                        pattern.name.clone(),
                        pattern.description.clone(),
                        SourceLocation {
                            file: filename.to_string(),
                            line: line_num + 1,
                            column: mat.start(),
                            function: self.extract_function_name(code, line_num),
                        },
                        pattern.recommendation.clone(),
                    )
                    .with_cwe(pattern.cwe_id.clone().unwrap_or_default());

                    vulnerabilities.push(vulnerability);
                }
            }
        }

        // Additional analysis for complex function patterns
        vulnerabilities.extend(self.analyze_complex_functions(code, filename)?);

        debug!("Gas limit analysis completed: found {} vulnerabilities", vulnerabilities.len());
        Ok(vulnerabilities)
    }

    /// Analyzes complex functions for gas limit issues
    fn analyze_complex_functions(&self, code: &str, filename: &str) -> Result<Vec<Vulnerability>> {
        let mut vulnerabilities = Vec::new();
        
        // Parse the code to analyze function structures
        if let Ok(ast) = syn::parse_str(code) {
            let mut visitor = GasLimitVisitor::new(filename, &mut vulnerabilities);
            visitor.visit_file(&ast);
        }

        Ok(vulnerabilities)
    }

    /// Extracts function name from code line
    fn extract_function_name(&self, code: &str, line_num: usize) -> Option<String> {
        let lines: Vec<&str> = code.lines().collect();
        
        // Look backwards from current line to find function declaration
        for i in (0..=line_num).rev() {
            if let Some(line) = lines.get(i) {
                if line.trim().starts_with("pub fn") || line.trim().starts_with("fn ") {
                    if let Some(fn_name) = line.split_whitespace().nth(1) {
                        if let Some(clean_name) = fn_name.split('(').next() {
                            return Some(clean_name.trim().to_string());
                        }
                    }
                }
            }
        }
        
        None
    }
}

/// AST visitor for detecting gas limit issues in function structures
struct GasLimitVisitor<'a> {
    filename: &'a str,
    vulnerabilities: &'a mut Vec<Vulnerability>,
}

impl<'a> GasLimitVisitor<'a> {
    fn new(filename: &'a str, vulnerabilities: &'a mut Vec<Vulnerability>) -> Self {
        Self { filename, vulnerabilities }
    }

    /// Analyzes function items for gas limit issues
    fn analyze_function(&mut self, item_fn: &ItemFn) {
        let fn_name = item_fn.sig.ident.to_string();
        
        // Check for functions that perform multiple operations without gas checks
        if self.has_multiple_operations(item_fn) && !self.has_gas_check(item_fn) {
            let vulnerability = Vulnerability::new(
                VulnerabilityType::StellarSpecific,
                Severity::High,
                "Multiple Operations Without Gas Validation".to_string(),
                format!("Function {} performs multiple operations without gas limit validation", fn_name),
                SourceLocation {
                    file: self.filename.to_string(),
                    line: item_fn.sig.ident.span().start().line,
                    column: item_fn.sig.ident.span().start().column,
                    function: Some(fn_name),
                },
                "Add gas limit validation before performing multiple operations".to_string(),
            )
            .with_cwe("CWE-400".to_string());

            self.vulnerabilities.push(vulnerability);
        }

        // Check for emergency functions without proper gas handling
        if self.is_emergency_function(&fn_name) && !self.has_emergency_gas_handling(item_fn) {
            let vulnerability = Vulnerability::new(
                VulnerabilityType::StellarSpecific,
                Severity::Critical,
                "Emergency Function Without Gas Consideration".to_string(),
                format!("Emergency function {} lacks proper gas limit consideration", fn_name),
                SourceLocation {
                    file: self.filename.to_string(),
                    line: item_fn.sig.ident.span().start().line,
                    column: item_fn.sig.ident.span().start().column,
                    function: Some(fn_name),
                },
                "Implement gas limit validation and batch processing for emergency operations".to_string(),
            )
            .with_cwe("CWE-400".to_string());

            self.vulnerabilities.push(vulnerability);
        }
    }

    /// Checks if function performs multiple operations
    fn has_multiple_operations(&self, item_fn: &ItemFn) -> bool {
        let fn_str = quote::quote!(item_fn).to_string();
        
        // Count operations that could consume significant gas
        let operation_count = fn_str.matches("env.").count();
        let loop_count = fn_str.matches("for ").count() + fn_str.matches("while ").count();
        
        operation_count > 2 || loop_count > 0
    }

    /// Checks if function has gas limit validation
    fn has_gas_check(&self, item_fn: &ItemFn) -> bool {
        let fn_str = quote::quote!(item_fn).to_string();
        
        fn_str.contains("budget") || 
        fn_str.contains("gas_left") ||
        fn_str.contains("gas_limit") ||
        fn_str.contains("MAX_BATCH_SIZE") ||
        fn_str.contains("DEFAULT_GAS")
    }

    /// Checks if function is an emergency-related function
    fn is_emergency_function(&self, fn_name: &str) -> bool {
        fn_name.contains("emergency") ||
        fn_name.contains("distribute") ||
        fn_name.contains("release_") ||
        fn_name.contains("batch_")
    }

    /// Checks if emergency function has proper gas handling
    fn has_emergency_gas_handling(&self, item_fn: &ItemFn) -> bool {
        let fn_str = quote::quote!(item_fn).to_string();
        
        fn_str.contains("gas_left") ||
        fn_str.contains("budget") ||
        fn_str.contains("chunk") ||
        fn_str.contains("batch_size")
    }
}

impl<'a> Visit<'a> for GasLimitVisitor<'a> {
    fn visit_item_fn(&mut self, item_fn: &ItemFn) {
        self.analyze_function(item_fn);
        syn::visit::visit_item_fn(self, item_fn);
    }
}
