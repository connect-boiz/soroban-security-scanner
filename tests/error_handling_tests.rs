//! Tests for improved error handling functionality

use stellar_security_scanner::error::{ScannerError, ScannerResult, ErrorSeverity, ErrorContext};
use std::fs;

#[test]
fn test_error_creation_and_severity() {
    let error = ScannerError::validation("test_field", "test reason");
    assert_eq!(error.severity(), ErrorSeverity::Low);
    
    let critical_error = ScannerError::authentication("Invalid credentials");
    assert_eq!(critical_error.severity(), ErrorSeverity::Critical);
}

#[test]
fn test_user_message_sanitization() {
    let internal_error = ScannerError::internal("Detailed internal error with sensitive data");
    let user_msg = internal_error.user_message();
    
    // Should not expose internal details
    assert!(!user_msg.contains("Detailed internal error"));
    assert!(!user_msg.contains("sensitive data"));
    assert_eq!(user_msg, "An internal error occurred. Please try again or contact support.");
}

#[test]
fn test_error_recoverability() {
    let recoverable_error = ScannerError::config("Invalid configuration");
    assert!(recoverable_error.is_recoverable());
    
    let non_recoverable_error = ScannerError::file_operation(
        "read", 
        "nonexistent.txt", 
        std::io::Error::new(std::io::ErrorKind::NotFound, "File not found")
    );
    assert!(!non_recoverable_error.is_recoverable());
}

#[test]
fn test_error_context() {
    let base_error = ScannerError::execution("scanner", "Failed to parse");
    let context_error = ErrorContext::new("scan", "security_scanner")
        .add_context("file", "contract.rs")
        .add_context("line", "42")
        .build_error(base_error);
    
    assert!(context_error.to_string().contains("Context"));
    assert!(context_error.to_string().contains("security_scanner"));
}

#[test]
fn test_multiple_error_aggregation() {
    let errors = vec![
        ScannerError::validation("field1", "error1"),
        ScannerError::validation("field2", "error2"),
        ScannerError::config("config error"),
    ];
    
    let aggregated = ScannerError::multiple(errors);
    match aggregated {
        ScannerError::Multiple { count, .. } => {
            assert_eq!(count, 3);
        }
        _ => panic!("Expected Multiple error variant"),
    }
}

#[test]
fn test_file_operation_error_handling() {
    // Test with a non-existent file
    let result: ScannerResult<String> = fs::read_to_string("nonexistent_file.txt")
        .map_err(|e| ScannerError::file_operation("read", "nonexistent_file.txt", e));
    
    assert!(result.is_err());
    match result.unwrap_err() {
        ScannerError::FileOperation { operation, path, .. } => {
            assert_eq!(operation, "read");
            assert_eq!(path, "nonexistent_file.txt");
        }
        _ => panic!("Expected FileOperation error"),
    }
}

#[test]
fn test_error_severity_ordering() {
    assert!(ErrorSeverity::Critical > ErrorSeverity::High);
    assert!(ErrorSeverity::High > ErrorSeverity::Medium);
    assert!(ErrorSeverity::Medium > ErrorSeverity::Low);
}

#[test]
fn test_error_display_formatting() {
    let error = ScannerError::validation("contract_id", "Invalid format");
    let display_str = format!("{}", error);
    assert!(display_str.contains("Validation failed"));
    assert!(display_str.contains("contract_id"));
    assert!(display_str.contains("Invalid format"));
}

#[test]
fn test_parsing_error_with_source() {
    let json_error = serde_json::from_str::<serde_json::Value>("invalid json")
        .unwrap_err();
    
    let error = ScannerError::parsing_with_source(
        "JSON", 
        "test.json", 
        "Invalid JSON syntax", 
        Box::new(json_error)
    );
    
    match error {
        ScannerError::Parsing { file_type, path, details, .. } => {
            assert_eq!(file_type, "JSON");
            assert_eq!(path, "test.json");
            assert_eq!(details, "Invalid JSON syntax");
        }
        _ => panic!("Expected Parsing error"),
    }
}
