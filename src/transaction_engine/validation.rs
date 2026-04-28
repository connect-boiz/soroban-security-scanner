//! Transaction validation and security checks

use crate::transaction_engine::{
    Transaction, TransactionType, TransactionFailure, TransactionPriority,
    ValidationResult as EngineValidationResult
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub score: ValidationScore,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
    pub severity: ValidationSeverity,
}

/// Validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
    pub recommendation: Option<String>,
}

/// Validation severity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Validation score (0-100)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationScore {
    pub overall: u8,
    pub security: u8,
    pub performance: u8,
    pub reliability: u8,
}

/// Validation configuration
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Enable strict validation
    pub strict_mode: bool,
    /// Maximum transaction data size (bytes)
    pub max_data_size: usize,
    /// Maximum transaction age (minutes)
    pub max_age_minutes: u64,
    /// Required fields for each transaction type
    pub required_fields: HashMap<TransactionType, Vec<String>>,
    /// Blacklisted submitters
    pub blacklisted_submitters: Vec<String>,
    /// Rate limiting per submitter
    pub rate_limit_per_minute: u32,
    /// Enable security scanning
    pub enable_security_scan: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        let mut required_fields = HashMap::new();
        required_fields.insert(TransactionType::Payment, vec!["amount".to_string(), "recipient".to_string()]);
        required_fields.insert(TransactionType::MultiSignature, vec!["signers".to_string(), "threshold".to_string()]);
        required_fields.insert(TransactionType::ContractDeployment, vec!["contract_code".to_string()]);
        required_fields.insert(TransactionType::ContractCall, vec!["contract_id".to_string(), "function".to_string()]);
        required_fields.insert(TransactionType::BatchOperation, vec!["operations".to_string()]);
        
        ValidationConfig {
            strict_mode: false,
            max_data_size: 1024 * 1024, // 1MB
            max_age_minutes: 60,
            required_fields,
            blacklisted_submitters: Vec::new(),
            rate_limit_per_minute: 100,
            enable_security_scan: true,
        }
    }
}

/// Transaction validator
pub struct TransactionValidator {
    config: ValidationConfig,
    rate_limiter: HashMap<String, Vec<chrono::DateTime<chrono::Utc>>>,
}

impl TransactionValidator {
    /// Create a new validator
    pub fn new(config: ValidationConfig) -> Self {
        Self {
            config,
            rate_limiter: HashMap::new(),
        }
    }

    /// Validate a transaction
    pub async fn validate_transaction(&mut self, transaction: &Transaction) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut score = ValidationScore {
            overall: 100,
            security: 100,
            performance: 100,
            reliability: 100,
        };

        // Basic structure validation
        self.validate_basic_structure(transaction, &mut errors, &mut warnings, &mut score);

        // Data validation
        self.validate_transaction_data(transaction, &mut errors, &mut warnings, &mut score);

        // Submitter validation
        self.validate_submitter(transaction, &mut errors, &mut warnings, &mut score);

        // Transaction type specific validation
        self.validate_transaction_type(transaction, &mut errors, &mut warnings, &mut score);

        // Security validation
        if self.config.enable_security_scan {
            self.validate_security(transaction, &mut errors, &mut warnings, &mut score);
        }

        // Performance validation
        self.validate_performance(transaction, &mut errors, &mut warnings, &mut score);

        // Calculate final score
        score.overall = (score.security + score.performance + score.reliability) / 3;

        let is_valid = errors.is_empty() || (!self.config.strict_mode && !errors.iter().any(|e| e.severity == ValidationSeverity::Critical));

        ValidationResult {
            is_valid,
            errors,
            warnings,
            score,
        }
    }

    /// Validate basic transaction structure
    fn validate_basic_structure(
        &self,
        transaction: &Transaction,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
        score: &mut ValidationScore,
    ) {
        // Check transaction ID
        if transaction.id.is_nil() {
            errors.push(ValidationError {
                code: "INVALID_ID".to_string(),
                message: "Transaction ID cannot be nil".to_string(),
                field: Some("id".to_string()),
                severity: ValidationSeverity::Critical,
            });
            score.reliability -= 30;
        }

        // Check data size
        if transaction.data.is_empty() {
            errors.push(ValidationError {
                code: "EMPTY_DATA".to_string(),
                message: "Transaction data cannot be empty".to_string(),
                field: Some("data".to_string()),
                severity: ValidationSeverity::High,
            });
            score.reliability -= 20;
        } else if transaction.data.len() > self.config.max_data_size {
            errors.push(ValidationError {
                code: "DATA_TOO_LARGE".to_string(),
                message: format!("Transaction data exceeds maximum size of {} bytes", self.config.max_data_size),
                field: Some("data".to_string()),
                severity: ValidationSeverity::High,
            });
            score.performance -= 15;
        }

        // Check transaction age
        let age = chrono::Utc::now() - transaction.metadata.created_at;
        if age.num_minutes() > self.config.max_age_minutes as i64 {
            warnings.push(ValidationWarning {
                code: "OLD_TRANSACTION".to_string(),
                message: format!("Transaction is {} minutes old", age.num_minutes()),
                field: Some("created_at".to_string()),
                recommendation: Some("Consider creating a fresh transaction".to_string()),
            });
            score.reliability -= 10;
        }

        // Check submitter
        if transaction.metadata.submitter.is_empty() {
            errors.push(ValidationError {
                code: "EMPTY_SUBMITTER".to_string(),
                message: "Submitter cannot be empty".to_string(),
                field: Some("submitter".to_string()),
                severity: ValidationSeverity::Critical,
            });
            score.reliability -= 25;
        }

        // Check network
        if transaction.metadata.network.is_empty() {
            errors.push(ValidationError {
                code: "EMPTY_NETWORK".to_string(),
                message: "Network cannot be empty".to_string(),
                field: Some("network".to_string()),
                severity: ValidationSeverity::High,
            });
            score.reliability -= 15;
        }
    }

    /// Validate transaction data
    fn validate_transaction_data(
        &self,
        transaction: &Transaction,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
        score: &mut ValidationScore,
    ) {
        // Try to parse transaction data as JSON for validation
        if let Ok(data_str) = std::str::from_utf8(&transaction.data) {
            if let Ok(data_value) = serde_json::from_str::<serde_json::Value>(data_str) {
                // Validate JSON structure
                self.validate_json_structure(&data_value, transaction.transaction_type.clone(), errors, warnings, score);
            } else {
                warnings.push(ValidationWarning {
                    code: "INVALID_JSON".to_string(),
                    message: "Transaction data is not valid JSON".to_string(),
                    field: Some("data".to_string()),
                    recommendation: Some("Ensure transaction data is valid JSON".to_string()),
                });
                score.reliability -= 5;
            }
        }
    }

    /// Validate JSON structure based on transaction type
    fn validate_json_structure(
        &self,
        data: &serde_json::Value,
        transaction_type: TransactionType,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
        score: &mut ValidationScore,
    ) {
        if let Some(required_fields) = self.config.required_fields.get(&transaction_type) {
            for field in required_fields {
                if !data.get(field).is_some() {
                    errors.push(ValidationError {
                        code: "MISSING_REQUIRED_FIELD".to_string(),
                        message: format!("Required field '{}' is missing", field),
                        field: Some(field.clone()),
                        severity: ValidationSeverity::High,
                    });
                    score.reliability -= 10;
                }
            }
        }

        // Type-specific validations
        match transaction_type {
            TransactionType::Payment => {
                self.validate_payment_data(data, errors, warnings, score);
            }
            TransactionType::MultiSignature => {
                self.validate_multisig_data(data, errors, warnings, score);
            }
            TransactionType::ContractDeployment => {
                self.validate_contract_deployment_data(data, errors, warnings, score);
            }
            TransactionType::ContractCall => {
                self.validate_contract_call_data(data, errors, warnings, score);
            }
            TransactionType::BatchOperation => {
                self.validate_batch_operation_data(data, errors, warnings, score);
            }
            _ => {} // No specific validation for other types
        }
    }

    /// Validate payment transaction data
    fn validate_payment_data(
        &self,
        data: &serde_json::Value,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
        score: &mut ValidationScore,
    ) {
        // Validate amount
        if let Some(amount) = data.get("amount") {
            if let Some(amount_str) = amount.as_str() {
                if let Ok(amount_val) = amount_str.parse::<f64>() {
                    if amount_val <= 0.0 {
                        errors.push(ValidationError {
                            code: "INVALID_AMOUNT".to_string(),
                            message: "Payment amount must be positive".to_string(),
                            field: Some("amount".to_string()),
                            severity: ValidationSeverity::High,
                        });
                        score.reliability -= 15;
                    } else if amount_val > 1000000.0 {
                        warnings.push(ValidationWarning {
                            code: "LARGE_AMOUNT".to_string(),
                            message: "Large payment amount detected".to_string(),
                            field: Some("amount".to_string()),
                            recommendation: Some("Consider additional verification for large amounts".to_string()),
                        });
                        score.security -= 5;
                    }
                } else {
                    errors.push(ValidationError {
                        code: "INVALID_AMOUNT_FORMAT".to_string(),
                        message: "Amount must be a valid number".to_string(),
                        field: Some("amount".to_string()),
                        severity: ValidationSeverity::High,
                    });
                    score.reliability -= 10;
                }
            }
        }

        // Validate recipient
        if let Some(recipient) = data.get("recipient") {
            if let Some(recipient_str) = recipient.as_str() {
                if !self.is_valid_stellar_address(recipient_str) {
                    errors.push(ValidationError {
                        code: "INVALID_RECIPIENT".to_string(),
                        message: "Invalid Stellar address format".to_string(),
                        field: Some("recipient".to_string()),
                        severity: ValidationSeverity::High,
                    });
                    score.reliability -= 15;
                }
            }
        }
    }

    /// Validate multi-signature transaction data
    fn validate_multisig_data(
        &self,
        data: &serde_json::Value,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
        score: &mut ValidationScore,
    ) {
        // Validate threshold
        if let Some(threshold) = data.get("threshold") {
            if let Some(threshold_val) = threshold.as_u64() {
                if threshold_val == 0 {
                    errors.push(ValidationError {
                        code: "INVALID_THRESHOLD".to_string(),
                        message: "Threshold must be greater than 0".to_string(),
                        field: Some("threshold".to_string()),
                        severity: ValidationSeverity::Critical,
                    });
                    score.reliability -= 25;
                } else if threshold_val > 20 {
                    warnings.push(ValidationWarning {
                        code: "HIGH_THRESHOLD".to_string(),
                        message: "High threshold may be difficult to achieve".to_string(),
                        field: Some("threshold".to_string()),
                        recommendation: Some("Consider a more reasonable threshold".to_string()),
                    });
                    score.performance -= 5;
                }
            }
        }

        // Validate signers
        if let Some(signers) = data.get("signers") {
            if let Some(signers_array) = signers.as_array() {
                if signers_array.is_empty() {
                    errors.push(ValidationError {
                        code: "NO_SIGNERS".to_string(),
                        message: "At least one signer is required".to_string(),
                        field: Some("signers".to_string()),
                        severity: ValidationSeverity::Critical,
                    });
                    score.reliability -= 30;
                } else if signers_array.len() > 20 {
                    errors.push(ValidationError {
                        code: "TOO_MANY_SIGNERS".to_string(),
                        message: "Maximum 20 signers allowed".to_string(),
                        field: Some("signers".to_string()),
                        severity: ValidationSeverity::High,
                    });
                    score.performance -= 10;
                }

                // Validate each signer
                for (i, signer) in signers_array.iter().enumerate() {
                    if let Some(signer_obj) = signer.as_object() {
                        if let Some(public_key) = signer_obj.get("public_key") {
                            if let Some(pk_str) = public_key.as_str() {
                                if !self.is_valid_stellar_address(pk_str) {
                                    errors.push(ValidationError {
                                        code: "INVALID_SIGNER_KEY".to_string(),
                                        message: format!("Invalid public key for signer {}", i),
                                        field: Some(format!("signers[{}].public_key", i)),
                                        severity: ValidationSeverity::High,
                                    });
                                    score.reliability -= 10;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Validate contract deployment data
    fn validate_contract_deployment_data(
        &self,
        data: &serde_json::Value,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
        score: &mut ValidationScore,
    ) {
        if let Some(contract_code) = data.get("contract_code") {
            if let Some(code_str) = contract_code.as_str() {
                if code_str.is_empty() {
                    errors.push(ValidationError {
                        code: "EMPTY_CONTRACT_CODE".to_string(),
                        message: "Contract code cannot be empty".to_string(),
                        field: Some("contract_code".to_string()),
                        severity: ValidationSeverity::Critical,
                    });
                    score.reliability -= 30;
                } else if code_str.len() > 100000 {
                    warnings.push(ValidationWarning {
                        code: "LARGE_CONTRACT".to_string(),
                        message: "Large contract code may impact performance".to_string(),
                        field: Some("contract_code".to_string()),
                        recommendation: Some("Consider optimizing contract code".to_string()),
                    });
                    score.performance -= 10;
                }
            }
        }
    }

    /// Validate contract call data
    fn validate_contract_call_data(
        &self,
        data: &serde_json::Value,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
        score: &mut ValidationScore,
    ) {
        // Validate contract ID
        if let Some(contract_id) = data.get("contract_id") {
            if let Some(id_str) = contract_id.as_str() {
                if !self.is_valid_contract_id(id_str) {
                    errors.push(ValidationError {
                        code: "INVALID_CONTRACT_ID".to_string(),
                        message: "Invalid contract ID format".to_string(),
                        field: Some("contract_id".to_string()),
                        severity: ValidationSeverity::High,
                    });
                    score.reliability -= 15;
                }
            }
        }

        // Validate function name
        if let Some(function) = data.get("function") {
            if let Some(func_str) = function.as_str() {
                if func_str.is_empty() {
                    errors.push(ValidationError {
                        code: "EMPTY_FUNCTION".to_string(),
                        message: "Function name cannot be empty".to_string(),
                        field: Some("function".to_string()),
                        severity: ValidationSeverity::High,
                    });
                    score.reliability -= 10;
                } else if !func_str.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    errors.push(ValidationError {
                        code: "INVALID_FUNCTION_NAME".to_string(),
                        message: "Function name contains invalid characters".to_string(),
                        field: Some("function".to_string()),
                        severity: ValidationSeverity::Medium,
                    });
                    score.reliability -= 5;
                }
            }
        }
    }

    /// Validate batch operation data
    fn validate_batch_operation_data(
        &self,
        data: &serde_json::Value,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
        score: &mut ValidationScore,
    ) {
        if let Some(operations) = data.get("operations") {
            if let Some(ops_array) = operations.as_array() {
                if ops_array.is_empty() {
                    errors.push(ValidationError {
                        code: "EMPTY_OPERATIONS".to_string(),
                        message: "Batch operation must contain at least one operation".to_string(),
                        field: Some("operations".to_string()),
                        severity: ValidationSeverity::Critical,
                    });
                    score.reliability -= 25;
                } else if ops_array.len() > 100 {
                    warnings.push(ValidationWarning {
                        code: "LARGE_BATCH".to_string(),
                        message: "Large batch operation may impact performance".to_string(),
                        field: Some("operations".to_string()),
                        recommendation: Some("Consider splitting into smaller batches".to_string()),
                    });
                    score.performance -= 15;
                }
            }
        }
    }

    /// Validate submitter
    fn validate_submitter(
        &mut self,
        transaction: &Transaction,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
        score: &mut ValidationScore,
    ) {
        // Check blacklist
        if self.config.blacklisted_submitters.contains(&transaction.metadata.submitter) {
            errors.push(ValidationError {
                code: "BLACKLISTED_SUBMITTER".to_string(),
                message: "Submitter is blacklisted".to_string(),
                field: Some("submitter".to_string()),
                severity: ValidationSeverity::Critical,
            });
            score.security -= 50;
        }

        // Check rate limiting
        let now = chrono::Utc::now();
        let submitter_requests = self.rate_limiter.entry(transaction.metadata.submitter.clone()).or_insert_with(Vec::new);
        
        // Clean old requests (older than 1 minute)
        submitter_requests.retain(|&timestamp| now - timestamp < chrono::Duration::minutes(1));
        
        if submitter_requests.len() >= self.config.rate_limit_per_minute as usize {
            errors.push(ValidationError {
                code: "RATE_LIMIT_EXCEEDED".to_string(),
                message: "Submitter has exceeded rate limit".to_string(),
                field: Some("submitter".to_string()),
                severity: ValidationSeverity::High,
            });
            score.reliability -= 20;
        } else {
            submitter_requests.push(now);
        }
    }

    /// Validate transaction type specific rules
    fn validate_transaction_type(
        &self,
        transaction: &Transaction,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
        score: &mut ValidationScore,
    ) {
        match transaction.transaction_type {
            TransactionType::Payment => {
                // Payment specific validations
                if transaction.priority == TransactionPriority::Low {
                    warnings.push(ValidationWarning {
                        code: "LOW_PRIORITY_PAYMENT".to_string(),
                        message: "Low priority may delay payment processing".to_string(),
                        field: Some("priority".to_string()),
                        recommendation: Some("Consider higher priority for time-sensitive payments".to_string()),
                    });
                    score.performance -= 5;
                }
            }
            TransactionType::SecurityScan => {
                // Security scan specific validations
                if transaction.priority == TransactionPriority::Emergency {
                    warnings.push(ValidationWarning {
                        code: "UNNECESSARY_EMERGENCY".to_string(),
                        message: "Emergency priority for security scan may be unnecessary".to_string(),
                        field: Some("priority".to_string()),
                        recommendation: Some("Consider using High priority instead".to_string()),
                    });
                    score.performance -= 3;
                }
            }
            _ => {} // No specific validations for other types
        }
    }

    /// Validate security aspects
    fn validate_security(
        &self,
        transaction: &Transaction,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
        score: &mut ValidationScore,
    ) {
        // Check for suspicious patterns in data
        let data_str = String::from_utf8_lossy(&transaction.data);
        
        // Check for potential SQL injection patterns
        if data_str.to_lowercase().contains("drop table") || 
           data_str.to_lowercase().contains("union select") ||
           data_str.to_lowercase().contains("exec(") {
            errors.push(ValidationError {
                code: "SUSPICIOUS_PATTERN".to_string(),
                message: "Suspicious pattern detected in transaction data".to_string(),
                field: Some("data".to_string()),
                severity: ValidationSeverity::Critical,
            });
            score.security -= 40;
        }

        // Check for large repeated patterns (potential DoS)
        if data_str.len() > 1000 {
            let chunks: Vec<&str> = data_str.chars().collect::<Vec<char>>().chunks(100).map(|chunk| chunk.iter().collect::<String>()).collect();
            let mut repeated_patterns = 0;
            
            for i in 1..chunks.len() {
                if chunks[i] == chunks[i-1] {
                    repeated_patterns += 1;
                }
            }
            
            if repeated_patterns > chunks.len() / 2 {
                warnings.push(ValidationWarning {
                    code: "REPEATED_PATTERNS".to_string(),
                    message: "Large repeated patterns detected".to_string(),
                    field: Some("data".to_string()),
                    recommendation: Some("Consider data compression".to_string()),
                });
                score.security -= 10;
            }
        }
    }

    /// Validate performance aspects
    fn validate_performance(
        &self,
        transaction: &Transaction,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
        score: &mut ValidationScore,
    ) {
        // Check data size impact on performance
        let data_size_kb = transaction.data.len() / 1024;
        
        if data_size_kb > 500 {
            warnings.push(ValidationWarning {
                code: "LARGE_TRANSACTION".to_string(),
                message: format!("Large transaction ({}KB) may impact performance", data_size_kb),
                field: Some("data".to_string()),
                recommendation: Some("Consider optimizing data size".to_string()),
            });
            score.performance -= 15;
        } else if data_size_kb > 100 {
            warnings.push(ValidationWarning {
                code: "MODERATE_TRANSACTION_SIZE".to_string(),
                message: format!("Moderate transaction size ({}KB)", data_size_kb),
                field: Some("data".to_string()),
                recommendation: Some("Monitor processing performance".to_string()),
            });
            score.performance -= 5;
        }

        // Check priority vs transaction type mismatch
        match (transaction.transaction_type, transaction.priority) {
            (TransactionType::Payment, TransactionPriority::Emergency) => {
                warnings.push(ValidationWarning {
                    code: "PRIORITY_MISMATCH".to_string(),
                    message: "Emergency priority for simple payment may be unnecessary".to_string(),
                    field: Some("priority".to_string()),
                    recommendation: Some("Consider using High priority".to_string()),
                });
                score.performance -= 3;
            }
            (TransactionType::SecurityScan, TransactionPriority::Low) => {
                warnings.push(ValidationWarning {
                    code: "PRIORITY_MISMATCH".to_string(),
                    message: "Low priority for security scan may delay important results".to_string(),
                    field: Some("priority".to_string()),
                    recommendation: Some("Consider using Normal or High priority".to_string()),
                });
                score.performance -= 5;
            }
            _ => {} // No mismatch
        }
    }

    /// Check if address is valid Stellar address
    fn is_valid_stellar_address(&self, address: &str) -> bool {
        // Stellar addresses start with 'G' and are 56 characters long
        address.len() == 56 && 
        address.starts_with('G') && 
        address.chars().all(|c| c.is_ascii_alphanumeric())
    }

    /// Check if contract ID is valid
    fn is_valid_contract_id(&self, contract_id: &str) -> bool {
        // Contract IDs are typically hex strings
        contract_id.len() == 64 && 
        contract_id.chars().all(|c| c.is_ascii_hexdigit())
    }
}

/// Security scanner for transactions
pub struct TransactionSecurityScanner {
    enabled_patterns: Vec<SecurityPattern>,
}

/// Security pattern
#[derive(Debug, Clone)]
pub struct SecurityPattern {
    pub name: String,
    pub pattern: String,
    pub severity: ValidationSeverity,
    pub description: String,
}

impl TransactionSecurityScanner {
    /// Create a new security scanner
    pub fn new() -> Self {
        let enabled_patterns = vec![
            SecurityPattern {
                name: "SQL_INJECTION".to_string(),
                pattern: "(?i)(drop|union|exec|insert|update|delete)".to_string(),
                severity: ValidationSeverity::Critical,
                description: "Potential SQL injection pattern".to_string(),
            },
            SecurityPattern {
                name: "XSS_PATTERN".to_string(),
                pattern: "(?i)(<script|javascript:|onload=|onerror=)".to_string(),
                severity: ValidationSeverity::High,
                description: "Potential XSS pattern".to_string(),
            },
            SecurityPattern {
                name: "COMMAND_INJECTION".to_string(),
                pattern: "(?i)(;|&&|\\|\\||`|\\$\\()".to_string(),
                severity: ValidationSeverity::Critical,
                description: "Potential command injection pattern".to_string(),
            },
        ];

        Self { enabled_patterns }
    }

    /// Scan transaction for security issues
    pub fn scan_transaction(&self, transaction: &Transaction) -> Vec<ValidationError> {
        let mut security_errors = Vec::new();
        let data_str = String::from_utf8_lossy(&transaction.data);

        for pattern in &self.enabled_patterns {
            if let Ok(regex) = regex::Regex::new(&pattern.pattern) {
                if regex.is_match(&data_str) {
                    security_errors.push(ValidationError {
                        code: pattern.name.clone(),
                        message: pattern.description.clone(),
                        field: Some("data".to_string()),
                        severity: pattern.severity.clone(),
                    });
                }
            }
        }

        security_errors
    }
}

impl Default for TransactionSecurityScanner {
    fn default() -> Self {
        Self::new()
    }
}
