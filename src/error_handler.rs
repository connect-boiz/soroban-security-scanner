//! Centralized error handling for the Stellar Security Scanner
//! 
//! This module provides graceful error handling, logging, and user-friendly
//! error messages while preventing information leakage.

use crate::error::{ScannerError, ScannerResult, ErrorSeverity};
use colored::*;
use std::panic;

/// Centralized error handler for the application
pub struct ErrorHandler {
    enable_logging: bool,
    enable_verbose: bool,
}

impl ErrorHandler {
    /// Create a new error handler
    pub fn new(enable_logging: bool, enable_verbose: bool) -> Self {
        Self {
            enable_logging,
            enable_verbose,
        }
    }

    /// Handle a ScannerError and provide appropriate user feedback
    pub fn handle_error(&self, error: ScannerError) -> ! {
        let user_message = error.user_message();
        let severity = error.severity();
        
        // Log the detailed error for debugging
        if self.enable_logging {
            self.log_error(&error);
        }

        // Display user-friendly error message
        self.display_error(&user_message, severity);

        // Exit with appropriate error code
        std::process::exit(self.exit_code(severity));
    }

    /// Handle any Result and convert to ScannerError if needed
    pub fn handle_result<T>(&self, result: ScannerResult<T>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => self.handle_error(error),
        }
    }

    /// Set up panic handler to catch panics and convert to errors
    pub fn setup_panic_handler(&self) {
        panic::set_hook(Box::new(|panic_info| {
            let error = ScannerError::internal(format!(
                "Application panic: {}",
                panic_info.payload().downcast_ref::<&str>().unwrap_or(&"Unknown panic")
            ));
            
            let handler = ErrorHandler::new(true, false);
            handler.handle_error(error);
        }));
    }

    /// Log detailed error information
    fn log_error(&self, error: &ScannerError) {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        eprintln!(
            "{} [{}] {}",
            timestamp.to_string().dimmed(),
            error.severity().to_string().red(),
            error.log_message()
        );

        if self.enable_verbose {
            // Print error chain for debugging
            let mut source = error.source();
            let mut depth = 1;
            while let Some(err) = source {
                eprintln!("  {} -> {}", "  ".repeat(depth), err);
                source = err.source();
                depth += 1;
            }
        }
    }

    /// Display user-friendly error message
    fn display_error(&self, message: &str, severity: ErrorSeverity) {
        let (icon, color) = match severity {
            ErrorSeverity::Critical => ("", "red"),
            ErrorSeverity::High => ("", "yellow"),
            ErrorSeverity::Medium => ("", "blue"),
            ErrorSeverity::Low => ("", "white"),
        };

        eprintln!("\n{} {} {}", 
            "ERROR".color(color).bold(),
            icon,
            message
        );

        // Provide helpful suggestions based on error type
        self.suggest_help(severity);
    }

    /// Provide helpful suggestions based on error severity
    fn suggest_help(&self, severity: ErrorSeverity) {
        match severity {
            ErrorSeverity::Critical => {
                eprintln!("\n{} This is a critical error that prevents the scanner from running.", 
                    "=".repeat(50).red());
                eprintln!("Suggestions:");
                eprintln!("  1. Check your configuration files");
                eprintln!("  2. Verify file permissions");
                eprintln!("  3. Ensure all dependencies are installed");
                eprintln!("  4. Contact support if the issue persists");
            }
            ErrorSeverity::High => {
                eprintln!("\n{} This is a serious error that may affect scan results.", 
                    "=".repeat(50).yellow());
                eprintln!("Suggestions:");
                eprintln!("  1. Review the error message above");
                eprintln!("  2. Check input file formats");
                eprintln!("  3. Verify network connectivity");
            }
            ErrorSeverity::Medium => {
                eprintln!("\n{} This error may be recoverable.", 
                    "=".repeat(50).blue());
                eprintln!("Suggestions:");
                eprintln!("  1. Try the operation again");
                eprintln!("  2. Check file paths and names");
                eprintln!("  3. Use --verbose flag for more details");
            }
            ErrorSeverity::Low => {
                eprintln!("\n{} This is a minor issue.", 
                    "=".repeat(50).white());
                eprintln!("Suggestions:");
                eprintln!("  1. The operation may continue with reduced functionality");
                eprintln!("  2. Use --verbose flag for more details");
            }
        }

        eprintln!("\nFor more help, run: stellar-scanner --help");
        eprintln!("Or visit: https://github.com/connect-boiz/soroban-security-scanner\n");
    }

    /// Determine exit code based on error severity
    fn exit_code(&self, severity: ErrorSeverity) -> i32 {
        match severity {
            ErrorSeverity::Critical => 2,
            ErrorSeverity::High => 1,
            ErrorSeverity::Medium => 1,
            ErrorSeverity::Low => 0, // Low severity errors shouldn't cause failure
        }
    }

    /// Handle multiple errors and aggregate them
    pub fn handle_multiple_errors(&self, errors: Vec<ScannerError>) -> ! {
        let aggregated_error = ScannerError::multiple(errors);
        self.handle_error(aggregated_error);
    }

    /// Create a result wrapper that handles errors gracefully
    pub fn wrap_result<T, F>(&self, operation: F) -> ScannerResult<T>
    where
        F: FnOnce() -> ScannerResult<T>,
    {
        let result = operation();
        
        if let Err(ref error) = result {
            if self.enable_verbose {
                eprintln!("Operation failed: {}", error.user_message());
            }
        }
        
        result
    }
}

/// Macro for handling results with the global error handler
#[macro_export]
macro_rules! handle_result {
    ($result:expr) => {
        $crate::error_handler::ErrorHandler::new(true, false).handle_result($result)
    };
}

/// Macro for handling optional results that can continue on error
#[macro_export]
macro_rules! handle_optional_result {
    ($result:expr, $default:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => {
                eprintln!("Warning: {}", error.user_message());
                $default
            }
        }
    };
}

/// Utility function to set up global error handling
pub fn setup_global_error_handling(verbose: bool) -> ErrorHandler {
    let handler = ErrorHandler::new(true, verbose);
    handler.setup_panic_handler();
    handler
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_severity_exit_codes() {
        let handler = ErrorHandler::new(false, false);
        assert_eq!(handler.exit_code(ErrorSeverity::Critical), 2);
        assert_eq!(handler.exit_code(ErrorSeverity::High), 1);
        assert_eq!(handler.exit_code(ErrorSeverity::Medium), 1);
        assert_eq!(handler.exit_code(ErrorSeverity::Low), 0);
    }

    #[test]
    fn test_multiple_error_aggregation() {
        let errors = vec![
            ScannerError::validation("test", "test error"),
            ScannerError::config("config error"),
        ];
        
        let aggregated = ScannerError::multiple(errors);
        assert!(matches!(aggregated, ScannerError::Multiple { .. }));
    }
}
