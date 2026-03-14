//! Main scanner implementations for security and invariant checking

use crate::vulnerabilities::VulnerabilityType;
use crate::invariants::InvariantRule;
use crate::analysis::AnalysisResult;
use crate::{ScanResult, Severity};
use syn::{Item, ItemFn, ItemStruct, ItemEnum, Expr, ExprCall, ExprMethodCall, ExprPath};
use std::path::Path;
use std::fs;
use regex::Regex;
use anyhow::Result;

pub struct SecurityScanner {
    vulnerability_patterns: Vec<(VulnerabilityType, Regex)>,
    ignore_patterns: Vec<Regex>,
}

pub struct InvariantScanner {
    invariant_rules: Vec<(InvariantRule, Regex)>,
}

impl SecurityScanner {
    pub fn new() -> Result<Self> {
        let mut scanner = Self {
            vulnerability_patterns: Vec::new(),
            ignore_patterns: Vec::new(),
        };
        
        scanner.initialize_patterns()?;
        Ok(scanner)
    }

    fn initialize_patterns(&mut self) -> Result<()> {
        // Access Control Vulnerabilities
        self.add_pattern(VulnerabilityType::MissingAccessControl, 
            r"(pub\s+fn\s+\w+.*\{[^}]*?(?!require_auth|has_auth)[^}]*?})")?;
        
        self.add_pattern(VulnerabilityType::WeakAccessControl,
            r"require_auth\(\s*\.\s*\)|has_auth\(\s*\.\s*\)")?;
        
        self.add_pattern(VulnerabilityType::UnauthorizedMint,
            r"fn\s+mint.*\{[^}]*?(?!require_auth)[^}]*?balance.*\+=")?;
        
        self.add_pattern(VulnerabilityType::UnauthorizedBurn,
            r"fn\s+burn.*\{[^}]*?(?!require_auth)[^}]*?balance.*-=")?;

        // Token Economics Vulnerabilities
        self.add_pattern(VulnerabilityType::InfiniteMint,
            r"mint.*\{[^}]*?balance.*\+=[^}]*?(?!limit|cap|max_supply)")?;
        
        self.add_pattern(VulnerabilityType::Reentrancy,
            r"env\.invoke_contract.*\{[^}]*?balance.*=.*[^}]*?env\.invoke_contract")?;
        
        self.add_pattern(VulnerabilityType::IntegerOverflow,
            r"\+\s*=|-\s*=|\*\s*=|/\s*=.*(?!checked_|wrapping_|saturating_)")?;

        // Logic Vulnerabilities
        self.add_pattern(VulnerabilityType::FrozenFunds,
            r"transfer.*\{[^}]*?(?!require|panic)[^}]*?return")?;
        
        self.add_pattern(VulnerabilityType::RaceCondition,
            r"let\s+mut\s+\w+.*=.*env\.current_contract_address\(\).*\{[^}]*?env\.current_contract_address\(\)")?;

        // Stellar Specific Vulnerabilities
        self.add_pattern(VulnerabilityType::InsufficientFeeBump,
            r"env\.invoke_contract.*\{[^}]*?(?!fee_bump)[^}]*?}")?;
        
        self.add_pattern(VulnerabilityType::InvalidTimeBounds,
            r"env\.ledger\(\).*\{[^}]*?(?!time_bounds|min_time|max_time)[^}]*?}")?;
        
        self.add_pattern(VulnerabilityType::WeakSignatureVerification,
            r"verify.*\{[^}]*?(?!ed25519|sha256)[^}]*?}")?;

        // Best Practices
        self.add_pattern(VulnerabilityType::UninitializedStorage,
            r"let\s+mut\s+\w+:\s+\w+<[^>]*>.*;[^;]*?\w+\.\w+\(")?;
        
        self.add_pattern(VulnerabilityType::MissingEventEmission,
            r"balance.*=.*\{[^}]*?(?!event|emit)[^}]*?}")?;
        
        self.add_pattern(VulnerabilityType::HardcodedValues,
            r"(const\s+|let\s+).*=\s*\"[^\"]*\".*\b(address|secret|key|password)\b")?;

        // Security Issues
        self.add_pattern(VulnerabilityType::LackOfInputValidation,
            r"fn\s+\w+\([^)]*\)\s*->\s*\w+\s*\{[^}]*?(?!require|assert|panic)[^}]*?}")?;
        
        self.add_pattern(VulnerabilityType::DenialOfService,
            r"loop\s*\{[^}]*?break[^}]*?\}")?;
        
        self.add_pattern(VulnerabilityType::InformationLeakage,
            r"event!\([^)]*\b(secret|private|password|key)\b")?;

        Ok(())
    }

    fn add_pattern(&mut self, vuln_type: VulnerabilityType, pattern: &str) -> Result<()> {
        let regex = Regex::new(pattern)?;
        self.vulnerability_patterns.push((vuln_type, regex));
        Ok(())
    }

    pub fn scan_file(&self, file_path: &Path) -> Result<ScanResult> {
        let content = fs::read_to_string(file_path)?;
        let mut result = ScanResult::new(file_path.to_string_lossy().to_string());

        // Skip if file matches ignore patterns
        if self.should_ignore(&content) {
            return Ok(result);
        }

        // Parse the file to get AST
        let syntax = syn::parse_file(&content)?;
        
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

    pub fn scan_directory(&self, dir_path: &Path) -> Result<Vec<ScanResult>> {
        let mut results = Vec::new();
        
        for entry in walkdir::WalkDir::new(dir_path) {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "rs") {
                if let Ok(result) = self.scan_file(path) {
                    results.push(result);
                }
            }
        }
        
        Ok(results)
    }
}

impl InvariantScanner {
    pub fn new() -> Result<Self> {
        let mut scanner = Self {
            invariant_rules: Vec::new(),
        };
        
        scanner.initialize_rules()?;
        Ok(scanner)
    }

    fn initialize_rules(&mut self) -> Result<()> {
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

    fn add_rule(&mut self, rule: InvariantRule, pattern: &str) -> Result<()> {
        let regex = Regex::new(pattern)?;
        self.invariant_rules.push((rule, regex));
        Ok(())
    }

    pub fn scan_file(&self, file_path: &Path) -> Result<ScanResult> {
        let content = fs::read_to_string(file_path)?;
        let mut result = ScanResult::new(file_path.to_string_lossy().to_string());

        for (rule, pattern) in &self.invariant_rules {
            if pattern.is_match(&content) {
                result.invariant_violations.push(rule.clone());
            }
        }

        Ok(result)
    }

    pub fn scan_directory(&self, dir_path: &Path) -> Result<Vec<ScanResult>> {
        let mut results = Vec::new();
        
        for entry in walkdir::WalkDir::new(dir_path) {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "rs") {
                if let Ok(result) = self.scan_file(path) {
                    results.push(result);
                }
            }
        }
        
        Ok(results)
    }
}
