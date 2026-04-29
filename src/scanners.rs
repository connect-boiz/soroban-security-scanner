//! Main scanner implementations for security and invariant checking

use crate::vulnerabilities::VulnerabilityType;
use crate::invariants::InvariantRule;
use crate::analysis::AnalysisResult;
use crate::{ScanResult, Severity};
use syn::{Item, ItemFn, ItemStruct, ItemEnum, Expr, ExprCall, ExprMethodCall, ExprPath};
use std::path::Path;
use std::fs;
use regex::Regex;
use crate::error::{ScannerResult, ScannerError};

pub struct SecurityScanner {
    vulnerability_patterns: Vec<(VulnerabilityType, Regex)>,
    ignore_patterns: Vec<Regex>,
}

pub struct InvariantScanner {
    invariant_rules: Vec<(InvariantRule, Regex)>,
}

impl SecurityScanner {
    pub fn new() -> ScannerResult<Self> {
        let mut scanner = Self {
            vulnerability_patterns: Vec::new(),
            ignore_patterns: Vec::new(),
        };
        
        scanner.initialize_patterns()?;
        Ok(scanner)
    }

    fn initialize_patterns(&mut self) -> ScannerResult<()> {
        // Access Control Vulnerabilities
        self.add_pattern(VulnerabilityType::MissingAccessControl, 
            r"pub fn [^{]*}")?;
        
        self.add_pattern(VulnerabilityType::WeakAccessControl,
            r"require_auth\(\.\)|has_auth\(\.\)")?;
        
        self.add_pattern(VulnerabilityType::UnauthorizedMint,
            r"fn mint.*balance.*\+=")?;
        
        self.add_pattern(VulnerabilityType::UnauthorizedBurn,
            r"fn burn.*balance.*-=")?;

        // Token Economics Vulnerabilities
        self.add_pattern(VulnerabilityType::InfiniteMint,
            r"mint.*balance.*\+=")?;
        
        self.add_pattern(VulnerabilityType::Reentrancy,
            r"env\.invoke_contract.*env\.invoke_contract")?;
        
        self.add_pattern(VulnerabilityType::IntegerOverflow,
            r"\+=|-=|\*=|/=")?;

        // Logic Vulnerabilities
        self.add_pattern(VulnerabilityType::FrozenFunds,
            r"transfer.*return")?;
        
        self.add_pattern(VulnerabilityType::RaceCondition,
            r"env\.current_contract_address.*env\.current_contract_address")?;

        // Stellar Specific Vulnerabilities
        self.add_pattern(VulnerabilityType::InsufficientFeeBump,
            r"env\.invoke_contract")?;
        
        self.add_pattern(VulnerabilityType::InvalidTimeBounds,
            r"env\.ledger")?;
        
        self.add_pattern(VulnerabilityType::WeakSignatureVerification,
            r"verify")?;

        // Best Practices
        self.add_pattern(VulnerabilityType::UninitializedStorage,
            r"let mut.*:")?;
        
        self.add_pattern(VulnerabilityType::MissingEventEmission,
            r"balance.*=")?;
        
        self.add_pattern(VulnerabilityType::HardcodedValues,
            r"const.*=.*address|let.*=.*secret")?;

        // Security Issues
        self.add_pattern(VulnerabilityType::LackOfInputValidation,
            r"fn [^{]*}")?;
        
        self.add_pattern(VulnerabilityType::DenialOfService,
            r"loop")?;
        
        self.add_pattern(VulnerabilityType::InformationLeakage,
            r"event.*secret")?;

        Ok(())
    }

    fn add_pattern(&mut self, vuln_type: VulnerabilityType, pattern: &str) -> ScannerResult<()> {
        let regex = Regex::new(pattern)
            .map_err(|e| ScannerError::parsing_with_source("regex", pattern, "Invalid vulnerability pattern ", Box::new(e)))?;
        self.vulnerability_patterns.push((vuln_type, regex));
        Ok(())
    }

    pub fn scan_file(&self, file_path: &Path) -> ScannerResult<ScanResult> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ScannerError::file_operation("read", &file_path.to_string_lossy(), e))?;
        let mut result = ScanResult::new(file_path.to_string_lossy().to_string());

        // Skip if file matches ignore patterns
        if self.should_ignore(&content) {
            return Ok(result);
        }

        // Parse the file to get AST
        let syntax = syn::parse_file(&content)
            .map_err(|e| ScannerError::parsing_with_source("Rust", &file_path.to_string_lossy(), "Failed to parse AST ", Box::new(e)))?;
        
        // Check for vulnerabilities
        for (vuln_type, pattern) in &self.vulnerability_patterns {
            if let Some(matches) = pattern.find(&content) {
                // Additional context analysis
                if self.is_vulnerability_context_valid(&syntax, &content, matches.range()) {
                    result.vulnerabilities.push(vuln_type.clone());
                }
            }
        }

        // AST-based analysis
        self.analyze_ast(&syntax, &mut result);

        Ok(result)
    }

    fn should_ignore(&self, content: &str) -> bool {
        for pattern in &self.ignore_patterns {
            if pattern.is_match(content) {
                return true;
            }
        }
        false
    }

    fn is_vulnerability_context_valid(&self, _syntax: &syn::File, content: &str, range: std::ops::Range<usize>) -> bool {
        // Extract context around the match
        let start = range.start.saturating_sub(100);
        let end = (range.end + 100).min(content.len());
        let context = &content[start..end];

        // Check for false positive indicators
        !context.contains("// ignore-security") 
            && !context.contains("/* skip-security */")
            && !context.contains("test_")
    }

    fn analyze_ast(&self, syntax: &syn::File, result: &mut ScanResult) {
        for item in &syntax.items {
            match item {
                Item::Fn(func) => self.analyze_function(func, result),
                Item::Struct(struct_item) => self.analyze_struct(struct_item, result),
                Item::Enum(enum_item) => self.analyze_enum(enum_item, result),
                _ => {}
            }
        }
    }

    fn analyze_function(&self, func: &ItemFn, result: &mut ScanResult) {
        // Check for public functions without access control
        if func.vis == syn::Visibility::Public(crate::syn::Public(crate::syn::Token::Pub(None))) {
            if !self.has_access_control(&func.block) {
                result.vulnerabilities.push(VulnerabilityType::MissingAccessControl);
            }
        }

        // Check for unsafe operations
        self.check_unsafe_operations(&func.block, result);
    }

    fn analyze_struct(&self, _struct_item: &ItemStruct, _result: &mut ScanResult) {
        // Analyze struct definitions for security issues
    }

    fn analyze_enum(&self, _enum_item: &ItemEnum, _result: &mut ScanResult) {
        // Analyze enum definitions for security issues
    }

    fn has_access_control(&self, block: &syn::Block) -> bool {
        let content = quote::quote!(#block).to_string();
        content.contains("require_auth") || content.contains("has_auth")
    }

    fn check_unsafe_operations(&self, block: &syn::Block, result: &mut ScanResult) {
        let content = quote::quote!(#block).to_string();
        
        // Check for potential overflow/underflow
        if content.contains("+=") || content.contains("-=") {
            if !content.contains("checked_") && !content.contains("wrapping_") && !content.contains("saturating_") {
                result.vulnerabilities.push(VulnerabilityType::IntegerOverflow);
            }
        }
    }

    pub fn scan_directory(&self, dir_path: &Path) -> ScannerResult<Vec<ScanResult>> {
        let mut results = Vec::new();
        
        for entry in walkdir::WalkDir::new(dir_path) {
            let entry = entry
                .map_err(|e| ScannerError::file_operation("walk", &dir_path.to_string_lossy(), e))?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "rs") {
                match self.scan_file(path) {
                    Ok(result) => results.push(result),
                    Err(e) => {
                        // Log the error but continue scanning other files
                        eprintln!("Warning: Failed to scan file '{}': {}", path.display(), e.user_message());
                    }
                }
            }
        }
        
        Ok(results)
    }
}

impl InvariantScanner {
    pub fn new() -> ScannerResult<Self> {
        let mut scanner = Self {
            invariant_rules: Vec::new(),
        };
        
        scanner.initialize_rules()?;
        Ok(scanner)
    }

    fn initialize_rules(&mut self) -> ScannerResult<()> {
        // Token Invariants
        self.add_rule(InvariantRule::TotalSupplyConsistency,
            r"total_supply.*balance.*\+|balance.*total_supply.*\+")?;
        
        self.add_rule(InvariantRule::BalanceNonNegative,
            r"balance.*<.*0|balance.*-=.*balance")?;
        
        self.add_rule(InvariantRule::TransferConservation,
            r"transfer.*from.*to.*amount")?;

        // Mathematical Invariants
        self.add_rule(InvariantRule::SumOfBalancesEqualsSupply,
            r"sum.*balances.*total_supply|total_supply.*sum.*balances")?;
        
        self.add_rule(InvariantRule::OverflowProtection,
            r"checked_add|checked_sub|checked_mul|checked_div")?;

        // State Consistency
        self.add_rule(InvariantRule::StateTransitionValidity,
            r"require.*state|state.*require")?;
        
        self.add_rule(InvariantRule::EventStateConsistency,
            r"event.*emit.*state|state.*event.*emit")?;

        Ok(())
    }

    fn add_rule(&mut self, rule: InvariantRule, pattern: &str) -> ScannerResult<()> {
        let regex = Regex::new(pattern)
            .map_err(|e| ScannerError::parsing_with_source("regex", pattern, "Invalid invariant rule pattern", Box::new(e)))?;
        self.invariant_rules.push((rule, regex));
        Ok(())
    }

    pub fn scan_file(&self, file_path: &Path) -> ScannerResult<ScanResult> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ScannerError::file_operation("read", &file_path.to_string_lossy(), e))?;
        let mut result = ScanResult::new(file_path.to_string_lossy().to_string());

        for (rule, pattern) in &self.invariant_rules {
            if pattern.is_match(&content) {
                result.invariant_violations.push(rule.clone());
            }
        }

        Ok(result)
    }

    pub fn scan_directory(&self, dir_path: &Path) -> ScannerResult<Vec<ScanResult>> {
        let mut results = Vec::new();
        
        for entry in walkdir::WalkDir::new(dir_path) {
            let entry = entry
                .map_err(|e| ScannerError::file_operation("walk", &dir_path.to_string_lossy(), e))?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "rs") {
                match self.scan_file(path) {
                    Ok(result) => results.push(result),
                    Err(e) => {
                        // Log the error but continue scanning other files
                        eprintln!("Warning: Failed to scan file '{}': {}", path.display(), e.user_message());
                    }
                }
            }
        }
        
        Ok(results)
    }
}
