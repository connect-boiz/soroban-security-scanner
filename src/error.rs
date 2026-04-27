//! Error handling and management for the security scanner

use std::collections::HashMap;
use thiserror::Error;
use colored::*;

/// Custom error types for the security scanner
#[derive(Error, Debug)]
pub enum ScannerError {
    #[error("File operation failed: {path}")]
    FileError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Configuration error: {message}")]
    ConfigError { message: String },

    #[error("Network operation failed: {operation}")]
    NetworkError {
        operation: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("Kubernetes operation failed: {operation}")]
    K8sError {
        operation: String,
        #[source]
        source: kube::Error,
    },

    #[error("Parse error: {message}")]
    ParseError {
        message: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("Scanner initialization failed: {message}")]
    InitializationError { message: String },

    #[error("Scan timeout after {seconds} seconds")]
    TimeoutError { seconds: u64 },

    #[error("Invalid input: {message}")]
    ValidationError { message: String },

    #[error("Internal error: {message}")]
    InternalError { message: String },

    #[error("Resource limit exceeded: {resource} (limit: {limit})")]
    ResourceLimitExceeded {
        resource: String,
        limit: String,
    },

    #[error("Partial scan completed: {scanned}/{total} files processed")]
    PartialScan {
        scanned: usize,
        total: usize,
        errors: Vec<String>,
    },

    #[error("Recovery operation failed: {operation}")]
    RecoveryError {
        operation: String,
        attempt: u32,
        max_attempts: u32,
    },

    #[error("Corrupted data detected: {data_type}")]
    DataCorruptionError {
        data_type: String,
        location: Option<String>,
    },

    #[error("Concurrency error: {message}")]
    ConcurrencyError { message: String },

    #[error("External service unavailable: {service}")]
    ServiceUnavailable {
        service: String,
        retry_after: Option<u64>,
    },
}

impl ScannerError {
    /// Get error severity for logging purposes
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            ScannerError::ValidationError { .. } => ErrorSeverity::Warning,
            ScannerError::ConfigError { .. } => ErrorSeverity::Error,
            ScannerError::FileError { .. } => ErrorSeverity::Error,
            ScannerError::NetworkError { .. } => ErrorSeverity::Error,
            ScannerError::K8sError { .. } => ErrorSeverity::Error,
            ScannerError::ParseError { .. } => ErrorSeverity::Error,
            ScannerError::InitializationError { .. } => ErrorSeverity::Critical,
            ScannerError::TimeoutError { .. } => ErrorSeverity::Warning,
            ScannerError::InternalError { .. } => ErrorSeverity::Critical,
            ScannerError::ResourceLimitExceeded { .. } => ErrorSeverity::Error,
            ScannerError::PartialScan { .. } => ErrorSeverity::Warning,
            ScannerError::RecoveryError { .. } => ErrorSeverity::Error,
            ScannerError::DataCorruptionError { .. } => ErrorSeverity::Critical,
            ScannerError::ConcurrencyError { .. } => ErrorSeverity::Error,
            ScannerError::ServiceUnavailable { .. } => ErrorSeverity::Error,
        }
    }

    /// Get user-friendly error message (without sensitive information)
    pub fn user_message(&self) -> String {
        match self {
            ScannerError::FileError { path, .. } => {
                format!("Unable to access file: {}", sanitize_path(path))
            },
            ScannerError::ConfigError { message } => {
                format!("Configuration error: {}", message)
            },
            ScannerError::NetworkError { operation, .. } => {
                format!("Network operation failed: {}", operation)
            },
            ScannerError::K8sError { operation, .. } => {
                format!("Kubernetes operation failed: {}", operation)
            },
            ScannerError::ParseError { message, .. } => {
                format!("Parse error: {}", message)
            },
            ScannerError::InitializationError { message } => {
                format!("Scanner initialization failed: {}", message)
            },
            ScannerError::TimeoutError { seconds } => {
                format!("Operation timed out after {} seconds", seconds)
            },
            ScannerError::ValidationError { message } => {
                format!("Invalid input: {}", message)
            },
            ScannerError::InternalError { message } => {
                format!("Internal error occurred: {}", message)
            },
            ScannerError::ResourceLimitExceeded { resource, limit } => {
                format!("Resource limit exceeded: {} (limit: {})", resource, limit)
            },
            ScannerError::PartialScan { scanned, total, .. } => {
                format!("Partial scan completed: {} of {} files processed", scanned, total)
            },
            ScannerError::RecoveryError { operation, attempt, max_attempts } => {
                format!("Recovery operation failed: {} (attempt {} of {})", operation, attempt, max_attempts)
            },
            ScannerError::DataCorruptionError { data_type, .. } => {
                format!("Data corruption detected: {}", data_type)
            },
            ScannerError::ConcurrencyError { message } => {
                format!("Concurrency error: {}", message)
            },
            ScannerError::ServiceUnavailable { service, .. } => {
                format!("External service unavailable: {}", service)
            },
        }
    }

    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            ScannerError::ValidationError { .. } => true,
            ScannerError::TimeoutError { .. } => true,
            ScannerError::NetworkError { .. } => true,
            ScannerError::K8sError { .. } => true,
            ScannerError::FileError { .. } => true,
            ScannerError::ParseError { .. } => true,
            ScannerError::ResourceLimitExceeded { .. } => true,
            ScannerError::PartialScan { .. } => true,
            ScannerError::ServiceUnavailable { .. } => true,
            ScannerError::RecoveryError { .. } => false,
            ScannerError::DataCorruptionError { .. } => false,
            ScannerError::ConcurrencyError { .. } => false,
            ScannerError::ConfigError { .. } => false,
            ScannerError::InitializationError { .. } => false,
            ScannerError::InternalError { .. } => false,
        }
    }

    /// Get suggested retry delay in milliseconds
    pub fn retry_delay_ms(&self) -> Option<u64> {
        match self {
            ScannerError::NetworkError { .. } => Some(1000),
            ScannerError::ServiceUnavailable { retry_after, .. } => {
                retry_after.map(|seconds| seconds * 1000)
            }
            ScannerError::TimeoutError { .. } => Some(5000),
            ScannerError::ResourceLimitExceeded { .. } => Some(10000),
            ScannerError::FileError { source, .. } => {
                match source.kind() {
                    std::io::ErrorKind::PermissionDenied => Some(30000),
                    std::io::ErrorKind::ConnectionRefused => Some(5000),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Get maximum retry attempts
    pub fn max_retries(&self) -> u32 {
        match self {
            ScannerError::NetworkError { .. } => 3,
            ScannerError::ServiceUnavailable { .. } => 5,
            ScannerError::TimeoutError { .. } => 2,
            ScannerError::ResourceLimitExceeded { .. } => 1,
            ScannerError::FileError { .. } => 2,
            _ => 0,
        }
    }

    /// Check if error should trigger graceful degradation
    pub fn should_degrade(&self) -> bool {
        matches!(
            self,
            ScannerError::PartialScan { .. }
                | ScannerError::NetworkError { .. }
                | ScannerError::TimeoutError { .. }
                | ScannerError::ResourceLimitExceeded { .. }
        )
    }
}

/// Error severity levels for logging
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Warning,
    Error,
    Critical,
}

impl ErrorSeverity {
    pub fn color(&self) -> &dyn colored::Color {
        match self {
            ErrorSeverity::Warning => &colored::Yellow,
            ErrorSeverity::Error => &colored::Red,
            ErrorSeverity::Critical => &colored::Purple,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorSeverity::Warning => "WARNING",
            ErrorSeverity::Error => "ERROR",
            ErrorSeverity::Critical => "CRITICAL",
        }
    }
}

pub type ScannerResult<T> = Result<T, ScannerError>;

/// Error context for better error reporting
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub operation: String,
    pub component: String,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
}

impl ErrorContext {
    pub fn new(operation: &str, component: &str) -> Self {
        Self {
            operation: operation.to_string(),
            component: component.to_string(),
            file_path: None,
            line_number: None,
        }
    }

    pub fn with_file_path(mut self, file_path: &str) -> Self {
        self.file_path = Some(file_path.to_string());
        self
    }

    pub fn with_line_number(mut self, line_number: u32) -> Self {
        self.line_number = Some(line_number);
        self
    }
}

/// Error handler for logging and user-friendly error messages
pub struct ErrorHandler {
    verbose: bool,
}

impl ErrorHandler {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    pub fn handle_error(&self, error: &ScannerError) {
        let severity = error.severity();
        let message = error.user_message();
        
        match severity {
            ErrorSeverity::Warning => {
                eprintln!("{} {}: {}", "⚠️".yellow(), severity.as_str().yellow(), message);
            }
            ErrorSeverity::Error => {
                eprintln!("{} {}: {}", "❌".red(), severity.as_str().red(), message);
            }
            ErrorSeverity::Critical => {
                eprintln!("{} {}: {}", "🔴".purple(), severity.as_str().purple(), message);
            }
        }

        if self.verbose {
            eprintln!("{} Full error: {:?}", "📋".blue(), error);
        }
    }
}

/// Sanitize file paths to prevent information leakage
pub fn sanitize_path(path: &str) -> String {
    // Only show the last 2 components of the path to prevent information leakage
    let components: Vec<&str> = path
        .split(['/', '\\'])
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    
    if components.len() > 2 {
        components[components.len()-2..].join("/")
    } else {
        components.join("/")
    }
}

/// Input validation and sanitization utilities
pub struct InputValidator;

impl InputValidator {
    /// Validate file path for security
    pub fn validate_file_path(path: &str) -> ScannerResult<()> {
        if path.is_empty() {
            return Err(ScannerError::ValidationError {
                message: "File path cannot be empty".to_string(),
            });
        }

        // Check for path traversal attempts
        if path.contains("..") || path.contains("~") {
            return Err(ScannerError::ValidationError {
                message: "Path contains potentially dangerous components".to_string(),
            });
        }

        // Check for extremely long paths
        if path.len() > 4096 {
            return Err(ScannerError::ValidationError {
                message: "Path too long".to_string(),
            });
        }

        // Check for invalid characters
        let invalid_chars = ['\0', '\u{FFFD}'];
        for &ch in &invalid_chars {
            if path.contains(ch) {
                return Err(ScannerError::ValidationError {
                    message: "Path contains invalid characters".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Validate and sanitize string input
    pub fn sanitize_string(input: &str, max_length: usize) -> ScannerResult<String> {
        if input.is_empty() {
            return Ok(input.to_string());
        }

        if input.len() > max_length {
            return Err(ScannerError::ValidationError {
                message: format!("Input too long (max {} characters)", max_length),
            });
        }

        // Remove potentially dangerous characters
        let sanitized = input
            .chars()
            .filter(|c| c.is_ascii() && !c.is_control())
            .collect::<String>();

        if sanitized.len() != input.len() {
            return Err(ScannerError::ValidationError {
                message: "Input contains invalid characters".to_string(),
            });
        }

        Ok(sanitized)
    }

    /// Validate network URL
    pub fn validate_url(url: &str) -> ScannerResult<()> {
        if url.is_empty() {
            return Err(ScannerError::ValidationError {
                message: "URL cannot be empty".to_string(),
            });
        }

        // Basic URL validation
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ScannerError::ValidationError {
                message: "URL must start with http:// or https://".to_string(),
            });
        }

        // Check for suspicious URLs
        let suspicious_patterns = ["localhost", "127.0.0.1", "0.0.0.0", "::1"];
        for pattern in &suspicious_patterns {
            if url.contains(pattern) {
                return Err(ScannerError::ValidationError {
                    message: "URL contains potentially dangerous host".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Validate numeric input within range
    pub fn validate_numeric_range<T>(value: T, min: T, max: T) -> ScannerResult<()>
    where
        T: PartialOrd + std::fmt::Display,
    {
        if value < min || value > max {
            return Err(ScannerError::ValidationError {
                message: format!("Value {} must be between {} and {}", value, min, max),
            });
        }
        Ok(())
    }
}

/// Convert common error types to ScannerError with context
pub trait IntoScannerError<T> {
    fn with_context(self, context: ErrorContext) -> ScannerResult<T>;
}

impl<T> IntoScannerError<T> for Result<T, std::io::Error> {
    fn with_context(self, context: ErrorContext) -> ScannerResult<T> {
        self.map_err(|e| ScannerError::FileError {
            path: context.file_path.unwrap_or_else(|| "unknown".to_string()),
            source: e,
        })
    }
}

impl<T> IntoScannerError<T> for Result<T, serde_json::Error> {
    fn with_context(self, context: ErrorContext) -> ScannerResult<T> {
        self.map_err(|e| ScannerError::ParseError {
            message: format!("JSON parsing failed in {}", context.operation),
            source: e,
        })
    }
}

impl<T> IntoScannerError<T> for Result<T, toml::de::Error> {
    fn with_context(self, context: ErrorContext) -> ScannerResult<T> {
        self.map_err(|e| ScannerError::ParseError {
            message: format!("TOML parsing failed in {}", context.operation),
            source: serde_json::Error::custom(e.to_string()),
        })
    }
}

impl<T> IntoScannerError<T> for Result<T, regex::Error> {
    fn with_context(self, context: ErrorContext) -> ScannerResult<T> {
        self.map_err(|e| ScannerError::InitializationError {
            message: format!("Regex compilation failed in {}: {}", context.operation, e),
        })
    }
}

impl<T> IntoScannerError<T> for Result<T, reqwest::Error> {
    fn with_context(self, context: ErrorContext) -> ScannerResult<T> {
        self.map_err(|e| ScannerError::NetworkError {
            operation: context.operation,
            source: e,
        })
    }
}

impl<T> IntoScannerError<T> for Result<T, kube::Error> {
    fn with_context(self, context: ErrorContext) -> ScannerResult<T> {
        self.map_err(|e| ScannerError::K8sError {
            operation: context.operation,
            source: e,
        })
    }
}
