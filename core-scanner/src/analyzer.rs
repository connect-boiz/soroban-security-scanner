use std::collections::HashMap;
use syn::{Item, ItemFn, parse_str, visit::Visit};
use regex::Regex;
use anyhow::Result;

use crate::vulnerabilities::{Vulnerability, VulnerabilityType, Severity, SourceLocation};
use crate::patterns::get_vulnerability_patterns;
use crate::report::ScanReport;

pub struct SecurityAnalyzer {
    patterns: Vec<crate::vulnerabilities::VulnerabilityPattern>,
    deep_analysis: bool,
    check_invariants: bool,
}

impl SecurityAnalyzer {
    pub fn new(deep_analysis: bool, check_invariants: bool) -> Self {
        Self {
            patterns: get_vulnerability_patterns(),
            deep_analysis,
            check_invariants,
        }
    }

    pub fn analyze(&self, code: &str, filename: &str) -> Result<ScanReport> {
        tracing::info!("Starting analysis of file: {}", filename);

        let mut vulnerabilities = Vec::new();
        
        // Parse the code into AST
        let ast = parse_str::<syn::File>(code)
            .map_err(|e| anyhow::anyhow!("Failed to parse code: {}", e))?;

        // Perform pattern matching
        vulnerabilities.extend(self.pattern_match_analysis(code, filename)?);

        // Perform AST analysis
        vulnerabilities.extend(self.ast_analysis(&ast, filename)?);

        // Perform deep analysis if requested
        if self.deep_analysis {
            vulnerabilities.extend(self.deep_analysis_checks(&ast, filename)?);
        }

        // Perform invariant checking if requested
        if self.check_invariants {
            vulnerabilities.extend(self.invariant_analysis(&ast, filename)?);
        }

        // Calculate metrics
        let metrics = self.calculate_metrics(&vulnerabilities, code);

        Ok(ScanReport {
            id: uuid::Uuid::new_v4().to_string(),
            filename: filename.to_string(),
            vulnerabilities,
            metrics,
            scan_time: chrono::Utc::now(),
            code_hash: self.calculate_code_hash(code),
        })
    }

    fn pattern_match_analysis(&self, code: &str, filename: &str) -> Result<Vec<Vulnerability>> {
        let mut vulnerabilities = Vec::new();
        let lines: Vec<&str> = code.lines().collect();

        for pattern in &self.patterns {
            let regex = Regex::new(&pattern.pattern)
                .map_err(|e| anyhow::anyhow!("Invalid regex pattern {}: {}", pattern.id, e))?;

            for (line_num, line) in lines.iter().enumerate() {
                if let Some(mat) = regex.find(line) {
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
                    .with_cwe(pattern.cwe_id.clone().unwrap_or_default())
                    .with_references(vec![]);

                    vulnerabilities.push(vulnerability);
                }
            }
        }

        Ok(vulnerabilities)
    }

    fn ast_analysis(&self, ast: &syn::File, filename: &str) -> Result<Vec<Vulnerability>> {
        let mut vulnerabilities = Vec::new();
        let mut visitor = SecurityVisitor::new(filename, &mut vulnerabilities);
        visitor.visit_file(ast);
        Ok(vulnerabilities)
    }

    fn deep_analysis_checks(&self, ast: &syn::File, filename: &str) -> Result<Vec<Vulnerability>> {
        let mut vulnerabilities = Vec::new();

        // Check for complex control flow issues
        for item in &ast.items {
            if let Item::Fn(item_fn) = item {
                // Check for complex functions that might have hidden vulnerabilities
                if self.is_complex_function(item_fn) {
                    let vulnerability = Vulnerability::new(
                        VulnerabilityType::LogicVulnerability,
                        Severity::Medium,
                        "Complex Function Logic".to_string(),
                        "Function has complex control flow that may contain hidden vulnerabilities".to_string(),
                        SourceLocation {
                            file: filename.to_string(),
                            line: item_fn.sig.span().start().line,
                            column: item_fn.sig.span().start().column,
                            function: Some(item_fn.sig.ident.to_string()),
                        },
                        "Consider breaking down complex functions into smaller, testable units".to_string(),
                    );
                    vulnerabilities.push(vulnerability);
                }
            }
        }

        Ok(vulnerabilities)
    }

    fn invariant_analysis(&self, ast: &syn::File, filename: &str) -> Result<Vec<Vulnerability>> {
        let mut vulnerabilities = Vec::new();

        // Check for potential invariant violations
        for item in &ast.items {
            if let Item::Fn(item_fn) = item {
                // Look for state changes without proper validation
                if self.has_unvalidated_state_change(item_fn) {
                    let vulnerability = Vulnerability::new(
                        VulnerabilityType::LogicVulnerability,
                        Severity::High,
                        "Potential Invariant Violation".to_string(),
                        "State change without proper invariant validation".to_string(),
                        SourceLocation {
                            file: filename.to_string(),
                            line: item_fn.sig.span().start().line,
                            column: item_fn.sig.span().start().column,
                            function: Some(item_fn.sig.ident.to_string()),
                        },
                        "Add invariant checks before state changes".to_string(),
                    );
                    vulnerabilities.push(vulnerability);
                }
            }
        }

        Ok(vulnerabilities)
    }

    fn extract_function_name(&self, code: &str, line_num: usize) -> Option<String> {
        let lines: Vec<&str> = code.lines().collect();
        
        // Look backwards from the current line to find function definition
        for i in (0..=line_num).rev() {
            if let Some(line) = lines.get(i) {
                if line.trim().starts_with("pub fn ") || line.trim().starts_with("fn ") {
                    if let Some(fn_name) = line.split("fn ").nth(1) {
                        if let Some(name) = fn_name.split('(').next() {
                            return Some(name.trim().to_string());
                        }
                    }
                }
            }
        }
        None
    }

    fn is_complex_function(&self, item_fn: &ItemFn) -> bool {
        // Simple heuristic: count statements and nested blocks
        let mut complexity = 0;
        
        // Count statements
        for stmt in &item_fn.block.stmts {
            complexity += 1;
        }
        
        // Add complexity for nested blocks
        self.count_nested_blocks(&item_fn.block, &mut complexity);
        
        complexity > 10 // Threshold for "complex"
    }

    fn count_nested_blocks(&self, block: &syn::Block, complexity: &mut usize) {
        for stmt in &block.stmts {
            if let syn::Stmt::Item(item) = stmt {
                if let syn::Item::Fn(nested_fn) = item {
                    *complexity += 5; // Nested functions add complexity
                    self.count_nested_blocks(&nested_fn.block, complexity);
                }
            }
        }
    }

    fn has_unvalidated_state_change(&self, item_fn: &ItemFn) -> bool {
        // Simple heuristic: look for storage.set without validation
        let fn_str = quote::quote!(#item_fn).to_string();
        
        fn_str.contains("storage().set") && !fn_str.contains("require") && !fn_str.contains("assert")
    }

    fn calculate_metrics(&self, vulnerabilities: &[Vulnerability], code: &str) -> HashMap<String, serde_json::Value> {
        let mut metrics = HashMap::new();
        
        metrics.insert("total_vulnerabilities".to_string(), 
                       serde_json::Value::Number(vulnerabilities.len().into()));
        
        metrics.insert("lines_of_code".to_string(), 
                       serde_json::Value::Number(code.lines().count().into()));
        
        let critical_count = vulnerabilities.iter()
            .filter(|v| matches!(v.severity, Severity::Critical))
            .count();
        metrics.insert("critical_vulnerabilities".to_string(), 
                       serde_json::Value::Number(critical_count.into()));
        
        let high_count = vulnerabilities.iter()
            .filter(|v| matches!(v.severity, Severity::High))
            .count();
        metrics.insert("high_vulnerabilities".to_string(), 
                       serde_json::Value::Number(high_count.into()));
        
        let risk_score = vulnerabilities.iter()
            .map(|v| v.severity.score() as u32)
            .sum::<u32>();
        metrics.insert("risk_score".to_string(), 
                       serde_json::Value::Number(risk_score.into()));
        
        metrics
    }

    fn calculate_code_hash(&self, code: &str) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        code.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

struct SecurityVisitor<'a> {
    filename: &'a str,
    vulnerabilities: &'a mut Vec<Vulnerability>,
}

impl<'a> SecurityVisitor<'a> {
    fn new(filename: &'a str, vulnerabilities: &'a mut Vec<Vulnerability>) -> Self {
        Self { filename, vulnerabilities }
    }
}

impl<'a> Visit<'a> for SecurityVisitor<'a> {
    fn visit_item_fn(&mut self, item_fn: &'a ItemFn) {
        // Check for public functions without access control
        if item_fn.sig.vis.is_public() && !self.has_access_control(item_fn) {
            let vulnerability = Vulnerability::new(
                VulnerabilityType::AccessControl,
                Severity::Critical,
                "Public Function Without Access Control".to_string(),
                "Public function lacks access control checks".to_string(),
                SourceLocation {
                    file: self.filename.to_string(),
                    line: item_fn.sig.span().start().line,
                    column: item_fn.sig.span().start().column,
                    function: Some(item_fn.sig.ident.to_string()),
                },
                "Add access control checks to public functions".to_string(),
            );
            self.vulnerabilities.push(vulnerability);
        }

        syn::visit::visit_item_fn(self, item_fn);
    }
}

impl SecurityVisitor<'_> {
    fn has_access_control(&self, item_fn: &ItemFn) -> bool {
        let fn_str = quote::quote!(#item_fn).to_string();
        fn_str.contains("require_auth") || 
        fn_str.contains("env.authenticator") ||
        fn_str.contains("assert_eq")
    }
}
