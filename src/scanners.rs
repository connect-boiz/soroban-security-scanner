//! Main scanner implementations for security and invariant checking

use crate::vulnerabilities::VulnerabilityType;
use crate::invariants::InvariantRule;
use crate::analysis::AnalysisResult;
use crate::{ScanResult, Severity};
use crate::error::{ScannerError, ScannerResult, ErrorContext, IntoScannerError};
use syn::{Item, ItemFn, ItemStruct, ItemEnum, Expr, ExprCall, ExprMethodCall, ExprPath};
use std::path::Path;
use std::fs;
use regex::Regex;

pub struct SecurityScanner {
    vulnerability_patterns: Vec<(VulnerabilityType, Regex)>,
    ignore_patterns: Vec<Regex>,
}

pub struct InvariantScanner {
    invariant_rules: Vec<(InvariantRule, Regex)>,
}

impl SecurityScanner {
    pub fn new() -> ScannerResult<Self> {
        let context = ErrorContext::new("scanner_init", "security_scanner");
        
        let mut scanner = Self {
            vulnerability_patterns: Vec::new(),
            ignore_patterns: Vec::new(),
        };
        
        scanner.initialize_patterns()
            .map_err(|e| ScannerError::InitializationError {
                message: format!("Failed to initialize security scanner: {}", e),
            })?;
        Ok(scanner)
    }

    fn initialize_patterns(&mut self) -> ScannerResult<()> {
        // Access Control Vulnerabilities
        self.add_pattern(VulnerabilityType::MissingAccessControl, 
            r"pub fn \w+.*\{")?;
        
        self.add_pattern(VulnerabilityType::WeakAccessControl,
            r"require_auth\(.*\)|has_auth\(.*\)")?;
        
        self.add_pattern(VulnerabilityType::UnauthorizedMint,
            r"fn mint.*\{.*balance.*\+=")?;
        
        self.add_pattern(VulnerabilityType::UnauthorizedBurn,
            r"fn burn.*\{.*balance.*-=")?;

        // Token Economics Vulnerabilities
        self.add_pattern(VulnerabilityType::InfiniteMint,
            r"mint.*\{.*balance.*\+=")?;
        
        self.add_pattern(VulnerabilityType::Reentrancy,
            r"env\.invoke_contract.*\{.*balance.*=.*env\.invoke_contract")?;
        
        self.add_pattern(VulnerabilityType::IntegerOverflow,
            r"\+=|-=|\*=|/=")?;

        // Logic Vulnerabilities
        self.add_pattern(VulnerabilityType::FrozenFunds,
            r"transfer.*\{.*return")?;
        
        self.add_pattern(VulnerabilityType::RaceCondition,
            r"let mut \w+.*=.*env\.current_contract_address\(\).*env\.current_contract_address\(\)")?;

        // Stellar Specific Vulnerabilities
        self.add_pattern(VulnerabilityType::InsufficientFeeBump,
            r"env\.invoke_contract.*\{")?;
        
        self.add_pattern(VulnerabilityType::InvalidTimeBounds,
            r"env\.ledger\(\).*\{")?;
        
        self.add_pattern(VulnerabilityType::WeakSignatureVerification,
            r"verify.*\{")?;

        // Best Practices
        self.add_pattern(VulnerabilityType::UninitializedStorage,
            r"let mut \w+: \w+<.*>.*;.*\w+\.\w+\(")?;
        
        self.add_pattern(VulnerabilityType::MissingEventEmission,
            r"balance.*=.*\{")?;
        
        self.add_pattern(VulnerabilityType::HardcodedValues,
            r"const.*=.*".*(?:address|secret|key|password)|let.*=.*".*(?:address|secret|key|password)")?;

        // Security Issues
        self.add_pattern(VulnerabilityType::LackOfInputValidation,
            r"fn \w+\([^)]*\) -> \w+ \{")?;
        
        self.add_pattern(VulnerabilityType::DenialOfService,
            r"loop \{.*break.*\}")?;
        
        self.add_pattern(VulnerabilityType::InformationLeakage,
            r"event!\(.*(?:secret|private|password|key).*\)")?;

        Ok(())
    }

    fn add_pattern(&mut self, vuln: VulnerabilityType, pattern: &str) -> ScannerResult<()> {
        let regex = Regex::new(pattern)
            .map_err(|e| ScannerError::InitializationError {
                message: format!("Invalid regex pattern for {}: {}", vuln, e),
            })?;
        self.vulnerability_patterns.push((vuln, regex));
        Ok(())
    }

    pub fn scan_file(&self, file_path: &Path) -> ScannerResult<ScanResult> {
        let context = ErrorContext::new("scan_file", "security_scanner")
            .with_file_path(&file_path.to_string_lossy());
        
        // Validate file path
        if !file_path.exists() {
            return Err(ScannerError::FileError {
                path: file_path.to_string_lossy().to_string(),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "File not found"
                ),
            });
        }
        
        // Check file size to prevent memory exhaustion
        let metadata = match fs::metadata(file_path) {
            Ok(metadata) => metadata,
            Err(e) => {
                return Err(ScannerError::FileError {
                    path: file_path.to_string_lossy().to_string(),
                    source: e,
                });
            }
        };
        
        const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB limit
        if metadata.len() > MAX_FILE_SIZE {
            return Err(ScannerError::ValidationError {
                message: format!("File too large ({} bytes): {}", metadata.len(), file_path.display()),
            });
        }
        
        let content = fs::read_to_string(file_path)
            .with_context(context.clone())?;
        
        // Check for empty file
        if content.trim().is_empty() {
            return Err(ScannerError::ValidationError {
                message: format!("File is empty: {}", file_path.display()),
            });
        }
        
        let mut result = ScanResult::new(file_path.to_string_lossy().to_string());

        // Parse the file for security analysis with enhanced error handling
        let syntax = match syn::parse_file(&content) {
            Ok(syntax) => syntax,
            Err(e) => {
                // Try to provide more helpful error messages
                let error_msg = if e.to_string().contains("unexpected token") {
                    format!("Syntax error in {}: Invalid Rust syntax", file_path.display())
                } else if e.to_string().contains("unclosed") {
                    format!("Syntax error in {}: Unclosed delimiter", file_path.display())
                } else {
                    format!("Failed to parse Rust file {}: {}", file_path.display(), e)
                };
                
                return Err(ScannerError::ParseError {
                    message: error_msg,
                    source: std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
                });
            }
        };

        // Check for vulnerabilities
        for item in syntax.items {
            self.check_item(&item, &mut result);
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
        let mut error_count = 0;
        const MAX_ERRORS: usize = 10; // Prevent error cascade
        
        // Validate directory path
        if !dir_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Directory not found: {}", dir_path.display())
            ));
        }
        
        if !dir_path.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Path is not a directory: {}", dir_path.display())
            ));
        }
        
        for entry in walkdir::WalkDir::new(dir_path).max_depth(50) { // Prevent infinite recursion
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    error_count += 1;
                    if error_count >= MAX_ERRORS {
                        eprintln!("⚠️  Too many errors encountered, stopping directory scan");
                        break;
                    }
                    eprintln!("⚠️  Error accessing directory entry: {}", e);
                    continue;
                }
            };
            
            let path = entry.path();
            
            // Skip hidden files and directories
            if path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with('.'))
                .unwrap_or(false) {
                continue;
            }
            
            // Only scan Rust files
            if path.extension().map_or(false, |ext| ext == "rs") {
                match self.scan_file(path) {
                    Ok(result) => results.push(result),
                    Err(e) => {
                        error_count += 1;
                        if error_count >= MAX_ERRORS {
                            eprintln!("⚠️  Too many file errors encountered, stopping directory scan");
                            break;
                        }
                        eprintln!("⚠️  Error scanning file {}: {}", path.display(), e);
                        // Continue scanning other files
                    }
                }
            }
        }
        
        if error_count > 0 {
            eprintln!("⚠️  Completed scan with {} errors", error_count);
        }
        
        Ok(results)
    }
}

impl InvariantScanner {
    pub fn new() -> ScannerResult<Self> {
        let context = ErrorContext::new("scanner_init", "invariant_scanner");
        
        let mut scanner = Self {
            invariant_rules: Vec::new(),
        };
        
        scanner.initialize_rules()
            .map_err(|e| ScannerError::InitializationError {
                message: format!("Failed to initialize invariant scanner: {}", e),
            })?;
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
            .map_err(|e| ScannerError::InitializationError {
                message: format!("Invalid regex pattern for {}: {}", rule, e),
            })?;
        self.invariant_rules.push((rule, regex));
        Ok(())
    }

    pub fn scan_file(&self, file_path: &Path) -> ScannerResult<ScanResult> {
        let context = ErrorContext::new("scan_file", "invariant_scanner")
            .with_file_path(&file_path.to_string_lossy());
        
        // Validate file path
        if !file_path.exists() {
            return Err(ScannerError::FileError {
                path: file_path.to_string_lossy().to_string(),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "File not found"
                ),
            });
        }
        
        // Check file size to prevent memory exhaustion
        let metadata = match fs::metadata(file_path) {
            Ok(metadata) => metadata,
            Err(e) => {
                return Err(ScannerError::FileError {
                    path: file_path.to_string_lossy().to_string(),
                    source: e,
                });
            }
        };
        
        const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB limit
        if metadata.len() > MAX_FILE_SIZE {
            return Err(ScannerError::ValidationError {
                message: format!("File too large ({} bytes): {}", metadata.len(), file_path.display()),
            });
        }
        
        let content = fs::read_to_string(file_path)
            .with_context(context.clone())?;
        
        // Check for empty file
        if content.trim().is_empty() {
            return Err(ScannerError::ValidationError {
                message: format!("File is empty: {}", file_path.display()),
            });
        }
        
        let mut result = ScanResult::new(file_path.to_string_lossy().to_string());

        for (rule, pattern) in &self.invariant_rules {
            if pattern.is_match(&content) {
                result.invariant_violations.push(rule.clone());
            }
        }

        Ok(result)
    }

    pub fn scan_directory(&self, dir_path: &Path) -> ScannerResult<Vec<ScanResult>> {
        let context = ErrorContext::new("scan_directory", "invariant_scanner")
            .with_file_path(&dir_path.to_string_lossy());
        
        let mut results = Vec::new();
        let mut error_count = 0;
        
        for entry in walkdir::WalkDir::new(dir_path) {
            let entry = entry.with_context(context.clone())?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "rs") {
                match self.scan_file(path) {
                    Ok(result) => results.push(result),
                    Err(e) => {
                        error_count += 1;
                        // Log error but continue scanning other files
                        eprintln!("Warning: Failed to scan {}: {}", 
                            crate::error::sanitize_path(&path.to_string_lossy()), 
                            e.user_message());
                    }
                }
            }
        }
        
        if error_count > 0 {
            eprintln!("Warning: Failed to scan {} files during directory scan", error_count);
        }
        
        Ok(results)
    }
}
