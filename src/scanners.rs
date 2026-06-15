//! Main scanner implementations for security and invariant checking

use crate::vulnerabilities::VulnerabilityType;
use crate::invariants::InvariantRule;
use crate::analysis::AnalysisResult;
use crate::emergency_stop::{EmergencyStop, ScanWatchdog};
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
    pub watchdog: ScanWatchdog,

pub struct InvariantScanner {
    invariant_rules: Vec<(InvariantRule, Regex)>,
    emergency_stop: EmergencyStop,
    pub watchdog: ScanWatchdog,

impl SecurityScanner {    pub fn new() -> Result<Self> {
        let emergency_stop = EmergencyStop::new()?;
        let watchdog = ScanWatchdog::new().with_emergency_stop(emergency_stop.clone());
        let mut scanner = Self {
            vulnerability_patterns: Vec::new(),
            ignore_patterns: Vec::new(),
            emergency_stop,
            watchdog,
        };
        
        scanner.initialize_patterns()?;
        Ok(scanner)
    }

    pub fn new_with_emergency_stop(emergency_stop: EmergencyStop) -> Result<Self> {
        let watchdog = ScanWatchdog::new().with_emergency_stop(emergency_stop.clone());
        let mut scanner = Self {
            vulnerability_patterns: Vec::new(),
            ignore_patterns: Vec::new(),
            emergency_stop,
            watchdog,
        };
        
        scanner.initialize_patterns()?;
        Ok(scanner)
    }

    pub fn new_with_watchdog(emergency_stop: EmergencyStop, watchdog: ScanWatchdog) -> Result<Self> {
        let mut scanner = Self {
            vulnerability_patterns: Vec::new(),
            ignore_patterns: Vec::new(),
            emergency_stop,
            watchdog,
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
            r"mint.*\{[^}]*?balance.*\+=[^}]*?(?!limit|cap|max_supply)")?;
        
        self.add_pattern(VulnerabilityType::Reentrancy,
            r"env\.invoke_contract.*\{[^}]*?balance.*=.*[^}]*?env\.invoke_contract")?;
        
        self.add_pattern(VulnerabilityType::IntegerOverflow,
            r"\+\s*=|-\s*=|\*\s*=|/\s*=.*(?!checked_|wrapping_|saturating_)")?;
        
        self.add_pattern(VulnerabilityType::IntegerUnderflow,
            r"balance.*-=[^}]*?(?!checked_|wrapping_|saturating_)")?;

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

    fn add_pattern(&mut self, vuln_type: VulnerabilityType, pattern: &str) -> Result<()> {
        let regex = Regex::new(pattern)?;
        self.vulnerability_patterns.push((vuln_type, regex));
        Ok(())
    }

    pub fn scan_file(&self, file_path: &Path) -> Result<ScanResult> {
        // Check for emergency stop before processing
        if self.emergency_stop.is_stopped() || self.watchdog.has_timed_out() {
            info!("Scan cancelled due to emergency stop or watchdog timeout");
            return Ok(ScanResult::new(file_path.to_string_lossy().to_string()));
        }

        // Set current file and record heartbeat
        self.watchdog.heartbeat_with_file(&file_path.to_string_lossy());

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
            // Check for emergency stop during pattern matching
            if self.emergency_stop.is_stopped() || self.watchdog.has_timed_out() {
                info!("Scan interrupted during pattern matching");
                break;
            }
            
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

        // Record heartbeat after file completion
        self.watchdog.heartbeat();

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
        
        // Start the watchdog monitor thread
        self.watchdog.start_monitoring();
        self.watchdog.reset();
        
        for entry in walkdir::WalkDir::new(dir_path) {
            // Check for emergency stop or watchdog timeout before processing each file
            if self.emergency_stop.is_stopped() || self.watchdog.has_timed_out() {
                info!("Directory scan cancelled due to emergency stop or watchdog timeout");
                break;
            }
            
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "rs") {
                if let Ok(result) = self.scan_file(path) {
                    results.push(result);
                }
            }
        }
        
        self.watchdog.stop_monitoring();
        Ok(results)
    }
}

impl InvariantScanner {
    pub fn new() -> Result<Self> {
        let emergency_stop = EmergencyStop::new()?;
        let watchdog = ScanWatchdog::new().with_emergency_stop(emergency_stop.clone());
        let mut scanner = Self {
            invariant_rules: Vec::new(),
            emergency_stop,
            watchdog,
        };
        
        scanner.initialize_rules()?;
        Ok(scanner)
    }

    pub fn new_with_emergency_stop(emergency_stop: EmergencyStop) -> Result<Self> {
        let watchdog = ScanWatchdog::new().with_emergency_stop(emergency_stop.clone());
        let mut scanner = Self {
            invariant_rules: Vec::new(),
            emergency_stop,
            watchdog,
        };
        
        scanner.initialize_rules()?;
        Ok(scanner)
    }

    pub fn new_with_watchdog(emergency_stop: EmergencyStop, watchdog: ScanWatchdog) -> Result<Self> {
        let mut scanner = Self {
            invariant_rules: Vec::new(),
            emergency_stop,
            watchdog,
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

    fn add_rule(&mut self, rule: InvariantRule, pattern: &str) -> Result<()> {
        let regex = Regex::new(pattern)?;
        self.invariant_rules.push((rule, regex));
        Ok(())
    }

    pub fn scan_file(&self, file_path: &Path) -> Result<ScanResult> {
        // Check for emergency stop or watchdog timeout before processing
        if self.emergency_stop.is_stopped() || self.watchdog.has_timed_out() {
            info!("Invariant scan cancelled due to emergency stop or watchdog timeout");
            return Ok(ScanResult::new(file_path.to_string_lossy().to_string()));
        }

        // Set current file and record heartbeat
        self.watchdog.heartbeat_with_file(&file_path.to_string_lossy());

        let content = fs::read_to_string(file_path)?;
        let mut result = ScanResult::new(file_path.to_string_lossy().to_string());

        for (rule, pattern) in &self.invariant_rules {
            // Check for emergency stop during rule checking
            if self.emergency_stop.is_stopped() || self.watchdog.has_timed_out() {
                info!("Invariant scan interrupted during rule checking");
                break;
            }
            
            if pattern.is_match(&content) {
                result.invariant_violations.push(rule.clone());
            }
        }

        // Record heartbeat after file completion
        self.watchdog.heartbeat();

        Ok(result)
    }

    pub fn scan_directory(&self, dir_path: &Path) -> Result<Vec<ScanResult>> {
        let mut results = Vec::new();
        
        // Start the watchdog monitor thread
        self.watchdog.start_monitoring();
        self.watchdog.reset();
        
        for entry in walkdir::WalkDir::new(dir_path) {
            // Check for emergency stop or watchdog timeout before processing each file
            if self.emergency_stop.is_stopped() || self.watchdog.has_timed_out() {
                info!("Invariant directory scan cancelled due to emergency stop or watchdog timeout");
                break;
            }
            
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "rs") {
                if let Ok(result) = self.scan_file(path) {
                    results.push(result);
                }
            }
        }
        
        self.watchdog.stop_monitoring();
        Ok(results)
    }
}
