//! Security Analyzer for Soroban Smart Contracts
//!
//! This module provides comprehensive static analysis capabilities for detecting security
//! vulnerabilities in Stellar/Soroban smart contracts. It implements a multi-layered
//! analysis approach combining pattern matching, AST analysis, and invariant checking.
//!
//! # Security Model
//! The analyzer operates under the assumption that contract code may be adversarial or
//! contain unintentional security flaws. All analysis is performed on untrusted input.
//!
//! # Threat Model
//! - Malicious code designed to exploit token economics or access control
//! - Unintentional flaws causing reentrancy, integer overflow, or frozen funds
//! - Configuration errors including weak signature verification
//! - Best practice violations (missing event emission, poor error handling)
//!
//! # Audit Trail
//! Each scan generates: unique ID for tracking, code hash for change detection,
//! UTC timestamp, and complete vulnerability findings with source locations.

use std::collections::HashMap;
use syn::{Item, ItemFn, parse_str, visit::Visit};
use regex::Regex;
use anyhow::Result;
use tracing::{debug, error};

use crate::vulnerabilities::{Vulnerability, VulnerabilityType, Severity, SourceLocation};
use crate::patterns::get_vulnerability_patterns;
use crate::report::ScanReport;

/// Multi-layered security analyzer for smart contract vulnerability detection.
///
/// Performs four complementary analysis passes:
/// 1. Pattern matching for known vulnerability signatures
/// 2. Abstract Syntax Tree (AST) structural analysis
/// 3. Deep analysis for complex control flow issues (if enabled)
/// 4. Invariant violation detection (if enabled)
///
/// # Security Considerations
/// - **Input Validation**: No pre-validation of code input; handles parse errors gracefully
/// - **Resource Limits**: No hard limits on analysis time; consider timeouts in production
/// - **False Positives**: May report issues that are not actual vulnerabilities
/// - **Incomplete Coverage**: Does not detect all possible vulnerabilities
/// - **Time Complexity**: O(n*m) where n=code size, m=pattern count
pub struct SecurityAnalyzer {
    patterns: Vec<crate::vulnerabilities::VulnerabilityPattern>,
    deep_analysis: bool,
    check_invariants: bool,
}

impl SecurityAnalyzer {
    /// Creates a new SecurityAnalyzer instance.
    ///
    /// # Arguments
    /// * `deep_analysis` - Enable complex control flow analysis (slower, more thorough)
    /// * `check_invariants` - Enable invariant violation detection
    ///
    /// # Security Notes
    /// - Deep analysis increases resource consumption; use judiciously
    /// - Invariant checking uses heuristics with potential false positives/negatives
    /// - Patterns are loaded from static configuration at initialization
    pub fn new(deep_analysis: bool, check_invariants: bool) -> Self {
        debug!(
            "Initializing SecurityAnalyzer with deep_analysis={}, check_invariants={}",
            deep_analysis, check_invariants
        );

        Self {
            patterns: get_vulnerability_patterns(),
            deep_analysis,
            check_invariants,
        }
    }

    /// Analyzes contract code for security vulnerabilities.
    ///
    /// Performs a comprehensive multi-stage security analysis including parsing,
    /// pattern matching, AST analysis, optional deep analysis, and invariant checking.
    ///
    /// # Arguments
    /// * `code` - The contract source code to analyze (must be valid Rust syntax)
    /// * `filename` - The source filename for error reporting and tracking
    ///
    /// # Returns
    /// `ScanReport` containing:
    /// - Unique scan ID (UUID v4) for audit trail tracking
    /// - All detected vulnerabilities with precise source locations (line, column, function)
    /// - Risk metrics: vulnerability counts, total risk score, lines of code
    /// - Code hash (non-cryptographic, for change detection)
    /// - Scan timestamp (UTC)
    ///
    /// # Errors
    /// - Invalid Rust syntax (parse_str fails)
    /// - Invalid regex patterns (should not occur with static patterns)
    /// - Internal analysis errors
    ///
    /// # Security Considerations
    /// - **Input Size**: No limits on code size; extremely large files may exhaust memory
    /// - **Parse Errors**: Invalid syntax fails entire scan; consider partial parsing
    /// - **False Positives**: Require developer review and context
    /// - **False Negatives**: Unknown patterns and variants are not detected
    /// - **Time Complexity**: O(n*m) where n=code size, m=pattern count
    /// - **Information Disclosure**: Code hash and filename in results require protection
    ///
    /// # Audit Trail
    /// - Unique scan ID generated per invocation for end-to-end tracking
    /// - Code hash enables detection of modifications between scans
    /// - UTC timestamp for chronological ordering
    /// - All findings include line/column/function for reproducibility
    /// - Recommend correlating scan ID with system audit logs
    pub fn analyze(&self, code: &str, filename: &str) -> Result<ScanReport> {
        let scan_id = uuid::Uuid::new_v4().to_string();
        debug!(
            "Starting security analysis: scan_id={}, filename={}, code_size={}",
            scan_id,
            filename,
            code.len()
        );

        let mut vulnerabilities = Vec::new();

        // Parse the code into AST - SECURITY: Fails on invalid syntax
        let ast = parse_str::<syn::File>(code).map_err(|e| {
            error!(
                "Failed to parse code (scan_id={}): {}",
                scan_id, e
            );
            anyhow::anyhow!("Failed to parse code: {}", e)
        })?;

        // Perform pattern matching - SECURITY: Runs regex on each line
        debug!(\"Running pattern matching analysis (scan_id={})\", scan_id);
        vulnerabilities.extend(self.pattern_match_analysis(code, filename)?);

        // Perform AST analysis - SECURITY: Traverses entire AST
        debug!(\"Running AST analysis (scan_id={})\", scan_id);
        vulnerabilities.extend(self.ast_analysis(&ast, filename)?);

        // Perform deep analysis if requested - SECURITY: Expensive, use with caution
        if self.deep_analysis {
            debug!(\"Running deep control flow analysis (scan_id={})\", scan_id);
            vulnerabilities.extend(self.deep_analysis_checks(&ast, filename)?);
        }

        // Perform invariant checking if requested - SECURITY: Uses heuristics
        if self.check_invariants {
            debug!(\"Running invariant violation analysis (scan_id={})\", scan_id);
            vulnerabilities.extend(self.invariant_analysis(&ast, filename)?);
        }

        // Calculate metrics - SECURITY: Scores based on severity classification
        let metrics = self.calculate_metrics(&vulnerabilities, code);

        let risk_score = metrics
            .get(\"risk_score\")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        debug!(
            \"Scan completed (scan_id={}): found {} vulnerabilities, risk_score={}\",
            scan_id,
            vulnerabilities.len(),
            risk_score
        );

        Ok(ScanReport {
            id: scan_id,
            filename: filename.to_string(),
            vulnerabilities,
            metrics,
            scan_time: chrono::Utc::now(),
            code_hash: self.calculate_code_hash(code),
        })
    }

    /// Pattern matching analysis for known vulnerability signatures.
    ///
    /// Scans code against pre-defined vulnerability patterns using regex matching.
    /// This is the fastest analysis phase but only detects known signatures.
    ///
    /// # Security Considerations
    /// - **Limited Coverage**: Only detects explicitly defined patterns
    /// - **False Positives**: Regex patterns may match innocuous code
    /// - **Regex Complexity**: Static patterns from config, not user-supplied
    /// - **Line-Based Analysis**: Multi-line constructs may be missed
    ///
    /// # Performance
    /// O(n * m) where n = number of lines, m = number of patterns
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

    /// AST-based structural analysis.
    ///
    /// Walks the Abstract Syntax Tree to detect structural security issues
    /// that pattern matching cannot find. Detects missing checks, unvalidated state changes.
    ///
    /// # Security Considerations
    /// - **Limited Type Info**: Syntactic analysis only; cannot verify types
    /// - **Context Insensitive**: Analysis is local; cross-function data flow not understood
    /// - **False Positives**: May flag legitimate patterns as suspicious
    ///
    /// # Audit Trail
    /// All findings include exact line/column/function location for reproducibility
    fn ast_analysis(&self, ast: &syn::File, filename: &str) -> Result<Vec<Vulnerability>> {
        debug!(\"Performing AST analysis on {}\", filename);
        let mut vulnerabilities = Vec::new();
        let mut visitor = SecurityVisitor::new(filename, &mut vulnerabilities);
        visitor.visit_file(ast);
        Ok(vulnerabilities)
    }

    /// Deep control flow analysis for complex vulnerabilities.
    ///
    /// Performs expensive analysis looking for complex control flow issues,
    /// nested functions, and other indicators of potential vulnerabilities.
    ///
    /// # Security Considerations
    /// - **Expensive**: Dominates analysis time for large files
    /// - **Heuristic-Based**: Complexity metrics may not correlate with actual risk
    /// - **False Positives**: Complex functions are not necessarily vulnerable
    /// - **Incomplete**: Does not analyze semantic correctness
    ///
    /// # Performance
    /// O(n) where n = total statements; can dominate analysis time
    fn deep_analysis_checks(&self, ast: &syn::File, filename: &str) -> Result<Vec<Vulnerability>> {
        debug!(\"Performing deep control flow analysis on {}\", filename);
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

    /// Invariant violation detection.
    ///
    /// Checks for state changes without proper validation, which could indicate
    /// violations of contract invariants. Uses pattern matching on storage operations.
    ///
    /// # Security Considerations
    /// - **Heuristic-Based**: Simple heuristics (storage.set without require/assert)
    /// - **False Positives**: Many legitimate patterns will be flagged
    /// - **False Negatives**: Complex invariant violations won't be detected
    /// - **No Semantic Analysis**: Cannot understand contract intent
    ///
    /// # Limitations
    /// - Requires explicit require/assert calls to avoid false positives
    /// - Cannot detect cross-function invariant violations
    /// - Does not understand conditional logic
    fn invariant_analysis(&self, ast: &syn::File, filename: &str) -> Result<Vec<Vulnerability>> {
        debug!(\"Performing invariant violation analysis on {}\", filename);
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

    /// Calculates security metrics for the scanned code.
    ///
    /// Computes risk metrics including vulnerability counts, risk score, and code size.
    ///
    /// # Metrics Computed
    /// - `total_vulnerabilities`: Count of all findings
    /// - `lines_of_code`: Total lines in the scanned code
    /// - `critical_vulnerabilities`: Count of CRITICAL severity findings
    /// - `high_vulnerabilities`: Count of HIGH severity findings
    /// - `risk_score`: Sum of all vulnerability severity scores (higher = more risk)
    ///
    /// # Security Notes
    /// - **Risk Score**: Sum of severity values; not a probabilistic risk assessment
    /// - **False Positive Impact**: Presence of false positives inflates metrics
    /// - **Comparison Bias**: Metrics should not be compared between analyzers
    ///
    /// # Audit Trail
    /// All metrics are deterministic and reproducible for the same code
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

    /// Computes a hash of the contract code for change detection.
    ///
    /// Uses Rust's DefaultHasher to generate a hash of the code content.
    /// Can be used to detect if contract code changes between scans.
    ///
    /// # Security Considerations
    /// - **Non-Cryptographic Hash**: DefaultHasher is NOT a cryptographic hash
    /// - **Collision Risk**: Hash collisions are possible and should be expected
    /// - **Not Suitable for Security**: Should not be used for integrity verification
    /// - **Intended Use**: Change detection and cache invalidation only
    ///
    /// # Recommendations
    /// - For integrity verification, use SHA-256 or other cryptographic hash
    /// - For critical applications, include cryptographic hash alongside this
    /// - Do not use as a signature; use actual cryptographic signatures
    fn calculate_code_hash(&self, code: &str) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        code.hash(&mut hasher);
        // WARNING: Non-cryptographic hash; do not use for security-critical operations
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
