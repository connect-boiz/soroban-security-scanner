//! Comprehensive error handling for the Stellar Security Scanner
//! 
//! This module provides structured error types and handling mechanisms
//! to prevent information leakage and provide graceful error recovery.

use std::fmt;
use thiserror::Error;

/// Comprehensive error types for the security scanner
#[derive(Error, Debug)]
pub enum ScannerError {
    /// Configuration-related errors
    #[error("Configuration error: {message}")]
    Config { message: String, source: Option<Box<dyn std::error::Error + Send + Sync>> },

    /// File I/O related errors
    #[error("File operation failed: {operation} on '{path}'")]
    FileOperation { 
        operation: String, 
        path: String, 
        #[source] 
        source: std::io::Error 
    },

    /// Parsing errors (AST, TOML, JSON, etc.)
    #[error("Parsing error in {file_type} file '{path}': {details}")]
    Parsing { 
        file_type: String, 
        path: String, 
        details: String,
        #[source] 
        source: Option<Box<dyn std::error::Error + Send + Sync>> 
    },

    /// Network-related errors
    #[error("Network operation failed: {operation}")]
    Network { 
        operation: String, 
        details: String,
        #[source] 
        source: Option<Box<dyn std::error::Error + Send + Sync>> 
    },

    /// Kubernetes-related errors
    #[error("Kubernetes operation failed: {operation} in namespace '{namespace}'")]
    Kubernetes { 
        operation: String, 
        namespace: String, 
        details: String,
        #[source] 
        source: Option<Box<dyn std::error::Error + Send + Sync>> 
    },

    /// Validation errors
    #[error("Validation failed: {field} - {reason}")]
    Validation { field: String, reason: String },

    /// Scanner execution errors
    #[error("Scanner execution failed: {component} - {details}")]
    Execution { 
        component: String, 
        details: String,
        #[source] 
        source: Option<Box<dyn std::error::Error + Send + Sync>> 
    },

    /// Resource limit errors
    #[error("Resource limit exceeded: {resource_type} limit {limit} reached")]
    ResourceLimit { resource_type: String, limit: String },

    /// Authentication/Authorization errors
    #[error("Authentication failed: {reason}")]
    Authentication { reason: String },

    /// Timeout errors
    #[error("Operation timed out: {operation} after {duration}s")]
    Timeout { operation: String, duration: u64 },

    /// Database/storage errors
    #[error("Storage operation failed: {operation} on {key}")]
    Storage { 
        operation: String, 
        key: String,
        #[source] 
        source: Option<Box<dyn std::error::Error + Send + Sync>> 
    },

    /// Contract analysis errors
    #[error("Contract analysis failed: {contract_id} - {reason}")]
    ContractAnalysis { contract_id: String, reason: String },

    /// Internal errors (for unexpected conditions)
    #[error("Internal error: {message}")]
    Internal { message: String },

    /// Multiple errors aggregated
    #[error("Multiple errors occurred: {count} issues")]
    Multiple { count: usize, errors: Vec<ScannerError> },
}

impl ScannerError {
    /// Create a configuration error
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config { 
            message: message.into(), 
            source: None 
        }
    }

    /// Create a configuration error with source
    pub fn config_with_source<S: Into<String>, E: Into<Box<dyn std::error::Error + Send + Sync>>>(message: S, source: E) -> Self {
        Self::Config { 
            message: message.into(), 
            source: Some(source.into()) 
        }
    }

    /// Create a file operation error
    pub fn file_operation<S: Into<String>>(operation: S, path: S, source: std::io::Error) -> Self {
        Self::FileOperation { 
            operation: operation.into(), 
            path: path.into(), 
            source 
        }
    }

    /// Create a parsing error
    pub fn parsing<S: Into<String>>(file_type: S, path: S, details: S) -> Self {
        Self::Parsing { 
            file_type: file_type.into(), 
            path: path.into(), 
            details: details.into(),
            source: None 
        }
    }

    /// Create a parsing error with source
    pub fn parsing_with_source<S: Into<String>, E: Into<Box<dyn std::error::Error + Send + Sync>>>(
        file_type: S, 
        path: S, 
        details: S, 
        source: E
    ) -> Self {
        Self::Parsing { 
            file_type: file_type.into(), 
            path: path.into(), 
            details: details.into(),
            source: Some(source.into()) 
        }
    }

    /// Create a network error
    pub fn network<S: Into<String>>(operation: S, details: S) -> Self {
        Self::Network { 
            operation: operation.into(), 
            details: details.into(),
            source: None 
        }
    }

    /// Create a kubernetes error
    pub fn kubernetes<S: Into<String>>(operation: S, namespace: S, details: S) -> Self {
        Self::Kubernetes { 
            operation: operation.into(), 
            namespace: namespace.into(), 
            details: details.into(),
            source: None 
        }
    }

    /// Create a validation error
    pub fn validation<S: Into<String>>(field: S, reason: S) -> Self {
        Self::Validation { 
            field: field.into(), 
            reason: reason.into() 
        }
    }

    /// Create an execution error
    pub fn execution<S: Into<String>>(component: S, details: S) -> Self {
        Self::Execution { 
            component: component.into(), 
            details: details.into(),
            source: None 
        }
    }

    /// Create a resource limit error
    pub fn resource_limit<S: Into<String>>(resource_type: S, limit: S) -> Self {
        Self::ResourceLimit { 
            resource_type: resource_type.into(), 
            limit: limit.into() 
        }
    }

    /// Create an authentication error
    pub fn authentication<S: Into<String>>(reason: S) -> Self {
        Self::Authentication { reason: reason.into() }
    }

    /// Create a timeout error
    pub fn timeout<S: Into<String>>(operation: S, duration: u64) -> Self {
        Self::Timeout { operation: operation.into(), duration }
    }

    /// Create a storage error
    pub fn storage<S: Into<String>>(operation: S, key: S) -> Self {
        Self::Storage { 
            operation: operation.into(), 
            key: key.into(),
            source: None 
        }
    }

    /// Create a contract analysis error
    pub fn contract_analysis<S: Into<String>>(contract_id: S, reason: S) -> Self {
        Self::ContractAnalysis { 
            contract_id: contract_id.into(), 
            reason: reason.into() 
        }
    }

    /// Create an internal error
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal { message: message.into() }
    }

    /// Create a multiple errors aggregation
    pub fn multiple(errors: Vec<ScannerError>) -> Self {
        let count = errors.len();
        Self::Multiple { count, errors }
    }

    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Config { .. } => true,
            Self::FileOperation { .. } => false,
            Self::Parsing { .. } => true,
            Self::Network { .. } => true,
            Self::Kubernetes { .. } => true,
            Self::Validation { .. } => true,
            Self::Execution { .. } => true,
            Self::ResourceLimit { .. } => true,
            Self::Authentication { .. } => false,
            Self::Timeout { .. } => true,
            Self::Storage { .. } => false,
            Self::ContractAnalysis { .. } => true,
            Self::Internal { .. } => false,
            Self::Multiple { .. } => false,
        }
    }

    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::Config { .. } => ErrorSeverity::Medium,
            Self::FileOperation { .. } => ErrorSeverity::High,
            Self::Parsing { .. } => ErrorSeverity::Medium,
            Self::Network { .. } => ErrorSeverity::Medium,
            Self::Kubernetes { .. } => ErrorSeverity::High,
            Self::Validation { .. } => ErrorSeverity::Low,
            Self::Execution { .. } => ErrorSeverity::Medium,
            Self::ResourceLimit { .. } => ErrorSeverity::Medium,
            Self::Authentication { .. } => ErrorSeverity::Critical,
            Self::Timeout { .. } => ErrorSeverity::Medium,
            Self::Storage { .. } => ErrorSeverity::High,
            Self::ContractAnalysis { .. } => ErrorSeverity::Medium,
            Self::Internal { .. } => ErrorSeverity::Critical,
            Self::Multiple { .. } => ErrorSeverity::High,
        }
    }

    /// Get user-friendly error message (sanitized for security)
    pub fn user_message(&self) -> String {
        match self {
            Self::Config { message, .. } => format!("Configuration issue: {}", message),
            Self::FileOperation { operation, path, .. } => {
                format!("Unable to {} file '{}'. Please check file permissions and paths.", operation, path)
            }
            Self::Parsing { file_type, path, details, .. } => {
                format!("Invalid {} file '{}': {}", file_type, path, details)
            }
            Self::Network { operation, details, .. } => {
                format!("Network error during {}: {}. Please check your connection.", operation, details)
            }
            Self::Kubernetes { operation, namespace, details, .. } => {
                format!("Kubernetes error in namespace '{}' during {}: {}", namespace, operation, details)
            }
            Self::Validation { field, reason } => {
                format!("Invalid {}: {}", field, reason)
            }
            Self::Execution { component, details, .. } => {
                format!("Scanner error in {}: {}", component, details)
            }
            Self::ResourceLimit { resource_type, limit } => {
                format!("Resource limit reached: {} limit of {} exceeded", resource_type, limit)
            }
            Self::Authentication { reason } => {
                format!("Authentication failed: {}", reason)
            }
            Self::Timeout { operation, duration } => {
                format!("Operation '{}' timed out after {} seconds", operation, duration)
            }
            Self::Storage { operation, .. } => {
                format!("Storage operation failed: {}. Please try again.", operation)
            }
            Self::ContractAnalysis { contract_id, reason } => {
                format!("Unable to analyze contract '{}': {}", contract_id, reason)
            }
            Self::Internal { message } => {
                // Don't expose internal details to users
                "An internal error occurred. Please try again or contact support.".to_string()
            }
            Self::Multiple { count, .. } => {
                format!("Multiple errors occurred ({} issues). Please check the logs for details.", count)
            }
        }
    }

    /// Get detailed error message for logging
    pub fn log_message(&self) -> String {
        format!("{} [Severity: {:?}] - {}", self, self.severity(), self)
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "LOW"),
            Self::Medium => write!(f, "MEDIUM"),
            Self::High => write!(f, "HIGH"),
            Self::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Result type alias for convenience
pub type ScannerResult<T> = Result<T, ScannerError>;

/// Error context builder for adding additional information
pub struct ErrorContext {
    operation: String,
    component: String,
    context: Vec<(String, String)>,
}

impl ErrorContext {
    pub fn new<S: Into<String>>(operation: S, component: S) -> Self {
        Self {
            operation: operation.into(),
            component: component.into(),
            context: Vec::new(),
        }
    }

    pub fn add_context<S: Into<String>, K: Into<String>>(mut self, key: K, value: S) -> Self {
        self.context.push((key.into(), value.into()));
        self
    }

    pub fn build_error(self, base_error: ScannerError) -> ScannerError {
        let context_str = if self.context.is_empty() {
            String::new()
        } else {
            format!(" | Context: {} in {}", 
                self.context.iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join(", "),
                self.component
            )
        };

        match base_error {
            ScannerError::Execution { component, details, source } => {
                ScannerError::Execution {
                    component: format!("{}{}{}", component, context_str, details),
                    details: format!("{} during {}", self.operation, component),
                    source,
                }
            }
            _ => base_error
        }
    }
}

/// Macro for creating errors with context
#[macro_export]
macro_rules! scanner_error {
    (validation: $field:expr, $reason:expr) => {
        $crate::error::ScannerError::validation($field, $reason)
    };
    (config: $message:expr) => {
        $crate::error::ScannerError::config($message)
    };
    (file: $operation:expr, $path:expr, $source:expr) => {
        $crate::error::ScannerError::file_operation($operation, $path, $source)
    };
    (parsing: $file_type:expr, $path:expr, $details:expr) => {
        $crate::error::ScannerError::parsing($file_type, $path, $details)
    };
    (network: $operation:expr, $details:expr) => {
        $crate::error::ScannerError::network($operation, $details)
    };
    (k8s: $operation:expr, $namespace:expr, $details:expr) => {
        $crate::error::ScannerError::kubernetes($operation, $namespace, $details)
    };
    (execution: $component:expr, $details:expr) => {
        $crate::error::ScannerError::execution($component, $details)
    };
    (timeout: $operation:expr, $duration:expr) => {
        $crate::error::ScannerError::timeout($operation, $duration)
    };
    (internal: $message:expr) => {
        $crate::error::ScannerError::internal($message)
    };
}

/// Macro for handling Result conversions with context
#[macro_export]
macro_rules! scanner_result {
    ($result:expr, $operation:expr, $component:expr) => {
        $result.map_err(|e| {
            ErrorContext::new($operation, $component)
                .build_error($crate::error::ScannerError::execution($component, format!("{}", e)))
        })
    };
    ($result:expr, $operation:expr, $component:expr, $($key:ident = $value:expr),*) => {
        $result.map_err(|e| {
            let mut ctx = ErrorContext::new($operation, $component);
            $(
                ctx = ctx.add_context(stringify!($key), $value);
            )*
            ctx.build_error($crate::error::ScannerError::execution($component, format!("{}", e)))
        })
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = ScannerError::validation("contract_id", "Invalid format");
        assert!(matches!(error, ScannerError::Validation { .. }));
        assert_eq!(error.severity(), ErrorSeverity::Low);
    }

    #[test]
    fn test_error_recoverability() {
        assert!(ScannerError::config("test").is_recoverable());
        assert!(!ScannerError::file_operation("read", "test.txt", std::io::Error::new(std::io::ErrorKind::NotFound, "not found")).is_recoverable());
    }

    #[test]
    fn test_user_message_sanitization() {
        let internal_error = ScannerError::internal("Detailed internal error");
        let user_msg = internal_error.user_message();
        assert!(!user_msg.contains("Detailed internal error"));
        assert_eq!(user_msg, "An internal error occurred. Please try again or contact support.");
    }

    #[test]
    fn test_error_context() {
        let base_error = ScannerError::execution("scanner", "Failed to parse");
        let context_error = ErrorContext::new("scan", "security_scanner")
            .add_context("file", "contract.rs")
            .add_context("line", "42")
            .build_error(base_error);
        
        assert!(context_error.to_string().contains("Context"));
    }
}
