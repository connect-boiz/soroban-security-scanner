//! Main scanner implementations for security and invariant checking

use crate::vulnerabilities::VulnerabilityType;
use crate::invariants::InvariantRule;
use crate::analysis::AnalysisResult;
use crate::emergency_stop::EmergencyStop;
use crate::{ScanResult, Severity};
use syn::{Item, ItemFn, ItemStruct, ItemEnum, Expr, ExprCall, ExprMethodCall, ExprPath};
use std::path::Path;
use std::fs;
use regex::Regex;
use anyhow::Result;
use log::{info, warn};

pub struct SecurityScanner {
    vulnerability_patterns: Vec<(VulnerabilityType, Regex)>,
    ignore_patterns: Vec<Regex>,
    pub emergency_stop: EmergencyStop,
}

pub struct InvariantScanner {
    invariant_rules: Vec<(InvariantRule, Regex)>,
    emergency_stop: EmergencyStop,
}

impl SecurityScanner {
    pub fn new() -> ScannerResult<Self> {
        let mut scanner = Self {
            vulnerability_patterns: Vec::new(),
            ignore_patterns: Vec::new(),
            emergency_stop: EmergencyStop::new()?,
        };
        
        scanner.initialize_patterns()?;
        Ok(scanner)
    }

    pub fn new_with_emergency_stop(emergency_stop: EmergencyStop) -> Result<Self> {
        let mut scanner = Self {
            vulnerability_patterns: Vec::new(),
            ignore_patterns: Vec::new(),
            emergency_stop,
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
            r"fn\s+burn.*\{[^}]*?(?!require_auth)[^}]*?balance.*-=")?;
        
        self.add_pattern(VulnerabilityType::InsufficientBalance,
            r"transfer.*\{[^}]*?(?!balance.*>=|require.*balance)[^}]*?balance.*-=")?;
        
        self.add_pattern(VulnerabilityType::BalanceUnderflow,
            r"balance.*-=.*(?!checked_|wrapping_|saturating_)")?;
        
        self.add_pattern(VulnerabilityType::BalanceOverflow,
            r"balance.*\+=.*(?!checked_|wrapping_|saturating_)")?;
        
        self.add_pattern(VulnerabilityType::TransferWithoutBalanceCheck,
            r"fn\s+transfer.*\{[^}]*?(?!require.*balance|balance.*>=)[^}]*?env\.invoke_contract|balance.*-=.*balance.*\+=")?;

        // Token Economics Vulnerabilities
        self.add_pattern(VulnerabilityType::InfiniteMint,
            r"mint.*balance.*\+=")?;
        
        self.add_pattern(VulnerabilityType::Reentrancy,
            r"env\.invoke_contract.*env\.invoke_contract")?;
        
        self.add_pattern(VulnerabilityType::IntegerOverflow,
            r"\+\s*=|-\s*=|\*\s*=|/\s*=.*(?!checked_|wrapping_|saturating_)")?;
        
        self.add_pattern(VulnerabilityType::IntegerUnderflow,
            r"balance.*-=[^}]*?(?!checked_|wrapping_|saturating_)")?;

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

        // Gas Limit Vulnerabilities
        self.add_pattern(VulnerabilityType::InsufficientGasLimitConsiderations,
            r"fn\s+(claim_reward|release_escrow|emergency_distribute).*\{[^}]*?(?!gas|limit|estimate)[^}]*?}")?;
        
        self.add_pattern(VulnerabilityType::ComplexOperationGasExhaustion,
            r"for.*\{[^}]*?env\.invoke_contract[^}]*?for.*\{[^}]*?env\.invoke_contract")?;
        
        self.add_pattern(VulnerabilityType::EscrowReleaseGasRisk,
            r"fn\s+release_escrow.*\{[^}]*?for.*in.*\{[^}]*?transfer[^}]*?\}")?;
        
        self.add_pattern(VulnerabilityType::EmergencyDistributionGasRisk,
            r"fn\s+emergency.*\{[^}]*?for.*in.*\{[^}]*?reward[^}]*?\}")?;
        
        self.add_pattern(VulnerabilityType::BatchOperationGasLimit,
            r"for.*\{[^}]*?env\.invoke_contract[^}]*?\}[^}]*?for.*\{[^}]*?env\.invoke_contract")?;

        // Event Logging Vulnerabilities
        self.add_pattern(VulnerabilityType::MissingCriticalEventLogging,
            r"fn\s+(transfer|withdraw|claim|approve|release).*\{[^}]*?(?!event|emit)[^}]*?balance.*=")?;
        
        self.add_pattern(VulnerabilityType::CriticalOperationWithoutEvents,
            r"fn\s+(transfer_funds|release_escrow|distribute_rewards).*\{[^}]*?(?!event|emit)[^}]*?\}")?;
        
        self.add_pattern(VulnerabilityType::IncompleteEventAuditTrail,
            r"event!\([^)]*\)[^}]*?balance.*=[^}]*?(?!event|emit)[^}]*?\}")?;
        
        self.add_pattern(VulnerabilityType::InsufficientEventMetadata,
            r"event!\([^)]*\)([^)]*\([^)]*\)){0,1}[^)]*\(,[^)]*\)[^)]*\(,[^)]*\)[^)]*\)")?;
        
        self.add_pattern(VulnerabilityType::EventLoggingBypass,
            r"if.*condition.*\{[^}]*?return[^}]*?\}[^}]*?event!\(")?;

        // Randomness and ID Generation Vulnerabilities
        self.add_pattern(VulnerabilityType::PredictableLedgerSequenceIds,
            r"env\.ledger\(\)\.sequence\(\).*id|id.*env\.ledger\(\)\.sequence\(\)")?;
        
        self.add_pattern(VulnerabilityType::WeakRandomnessInIdGeneration,
            r"generate.*id.*\{[^}]*?ledger\.sequence[^}]*?\}")?;
        
        self.add_pattern(VulnerabilityType::InsufficientEntropySources,
            r"fn.*generate.*random.*\{[^}]*?(timestamp|sequence)[^}]*?\}")?;
        
        self.add_pattern(VulnerabilityType::DeterministicNonceGeneration,
            r"generate.*nonce.*\{[^}]*?(format|concat)[^}]*?\}")?;
        
        self.add_pattern(VulnerabilityType::IdCollisionVulnerability,
            r"hash.*\{[^}]*?(simple|weak|predictable)[^}]*?\}.*id")?;

        Ok(())
    }

    fn add_pattern(&mut self, vuln_type: VulnerabilityType, pattern: &str) -> ScannerResult<()> {
        let regex = Regex::new(pattern)
            .map_err(|e| ScannerError::parsing_with_source("regex", pattern, "Invalid vulnerability pattern ", Box::new(e)))?;
        self.vulnerability_patterns.push((vuln_type, regex));
        Ok(())
    }

    pub fn scan_file(&self, file_path: &Path) -> Result<ScanResult> {
        // Check for emergency stop before processing
        if self.emergency_stop.is_stopped() {
            info!("Scan cancelled due to emergency stop");
            return Ok(ScanResult::new(file_path.to_string_lossy().to_string()));
        }

        let content = fs::read_to_string(file_path)?;
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
            // Check for emergency stop during pattern matching
            check_emergency_stop!(self.emergency_stop);
            
            if let Some(matches) = pattern.find(&content) {
                // Additional context analysis
                if self.is_vulnerability_context_valid(&syntax, &content, matches.range()) {
                    result.vulnerabilities.push(vuln_type.clone());
                    
                    // Trigger emergency stop for critical vulnerabilities
                    if vuln_type.severity() == Severity::Critical {
                        warn!("Critical vulnerability detected in {}: {}", 
                              file_path.display(), vuln_type.to_string());
                        self.emergency_stop.stop_on_critical_vulnerability(
                            &file_path.to_string_lossy(),
                            &vuln_type.to_string()
                        )?;
                    }
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
            // Check for emergency stop before processing each file
            if self.emergency_stop.is_stopped() {
                info!("Directory scan cancelled due to emergency stop");
                break;
            }
            
            let entry = entry?;
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
            emergency_stop: EmergencyStop::new()?,
        };
        
        scanner.initialize_rules()?;
        Ok(scanner)
    }

    pub fn new_with_emergency_stop(emergency_stop: EmergencyStop) -> Result<Self> {
        let mut scanner = Self {
            invariant_rules: Vec::new(),
            emergency_stop,
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
        
        self.add_rule(InvariantRule::SufficientBalanceCheck,
            r"transfer.*\{[^}]*?(?!balance.*>=|require.*balance)[^}]*?balance.*-=")?;
        
        self.add_rule(InvariantRule::BalanceBoundsCheck,
            r"balance.*\+=.*(?!max_balance|limit)|balance.*-=.*(?!min_balance)")?;
        
        self.add_rule(InvariantRule::TransferAtomicity,
            r"transfer.*\{[^}]*?balance.*-=.*balance.*\+=")?;
        
        self.add_rule(InvariantRule::BalanceIntegrity,
            r"balance.*=.*(?!checked_|safe_)")?;
        
        self.add_rule(InvariantRule::TransferConservation,
            r"transfer.*from.*to.*amount")?;

        // Mathematical Invariants
        self.add_rule(InvariantRule::SumOfBalancesEqualsSupply,
            r"sum.*balances.*total_supply|total_supply.*sum.*balances")?;
        
        self.add_rule(InvariantRule::OverflowProtection,
            r"checked_add|checked_sub|checked_mul|checked_div")?;
        
        self.add_rule(InvariantRule::BalanceBoundsCheck,
            r"balance.*<=.*max_balance|max_balance.*>=.*balance")?;

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

    pub fn scan_file(&self, file_path: &Path) -> Result<ScanResult> {
        // Check for emergency stop before processing
        if self.emergency_stop.is_stopped() {
            info!("Invariant scan cancelled due to emergency stop");
            return Ok(ScanResult::new(file_path.to_string_lossy().to_string()));
        }

        let content = fs::read_to_string(file_path)?;
        let mut result = ScanResult::new(file_path.to_string_lossy().to_string());

        for (rule, pattern) in &self.invariant_rules {
            // Check for emergency stop during rule checking
            check_emergency_stop!(self.emergency_stop);
            
            if pattern.is_match(&content) {
                result.invariant_violations.push(rule.clone());
            }
        }

        Ok(result)
    }

    pub fn scan_directory(&self, dir_path: &Path) -> ScannerResult<Vec<ScanResult>> {
        let mut results = Vec::new();
        
        for entry in walkdir::WalkDir::new(dir_path) {
            // Check for emergency stop before processing each file
            if self.emergency_stop.is_stopped() {
                info!("Invariant directory scan cancelled due to emergency stop");
                break;
            }
            
            let entry = entry?;
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
