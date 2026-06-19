//! Input validation and sanitization module
//!
//! Provides reusable validators for common input types with comprehensive
//! error handling and HTTP integration. All validators return `Result<T, ValidationError>`
//! which can be directly converted to HTTP 400 responses via the `IntoResponse` trait.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Validation error type that maps to HTTP 400 Bad Request
#[derive(Debug, Error, Clone)]
pub enum ValidationError {
    #[error("Invalid address format: {reason}")]
    InvalidAddress { reason: String },

    #[error("Invalid contract ID: {reason}")]
    InvalidContractId { reason: String },

    #[error("Invalid amount: {reason}")]
    InvalidAmount { reason: String },

    #[error("Invalid pagination parameters: {reason}")]
    InvalidPagination { reason: String },

    #[error("Invalid string input: {reason}")]
    InvalidString { reason: String },

    #[error("Input validation failed: {reason}")]
    ValidationFailed { reason: String },
}

/// HTTP error response body
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: String,
    pub status: u16,
}

impl IntoResponse for ValidationError {
    fn into_response(self) -> Response {
        let error_message = self.to_string();
        let details = match self {
            ValidationError::InvalidAddress { reason } => reason,
            ValidationError::InvalidContractId { reason } => reason,
            ValidationError::InvalidAmount { reason } => reason,
            ValidationError::InvalidPagination { reason } => reason,
            ValidationError::InvalidString { reason } => reason,
            ValidationError::ValidationFailed { reason } => reason,
        };

        let error_response = ErrorResponse {
            error: error_message,
            details,
            status: 400,
        };

        (StatusCode::BAD_REQUEST, Json(error_response)).into_response()
    }
}

// ============================================================================
// ADDRESS VALIDATION
// ============================================================================

/// Validates a Stellar address format
///
/// Stellar addresses are base32-encoded strings that:
/// - Start with 'G' (public key account)
/// - Are exactly 56 characters long
/// - Contain only valid base32 characters
///
/// # Examples
///
/// ```
/// use soroban_security_scanner::validation::validate_address;
///
/// // Valid Stellar address
/// let result = validate_address("GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQ");
/// assert!(result.is_ok());
///
/// // Invalid format
/// let result = validate_address("invalid");
/// assert!(result.is_err());
/// ```
pub fn validate_address(address: &str) -> Result<(), ValidationError> {
    if address.is_empty() {
        return Err(ValidationError::InvalidAddress {
            reason: "Address cannot be empty".to_string(),
        });
    }

    if address.len() != 56 {
        return Err(ValidationError::InvalidAddress {
            reason: format!(
                "Address must be exactly 56 characters, got {}",
                address.len()
            ),
        });
    }

    if !address.starts_with('G') {
        return Err(ValidationError::InvalidAddress {
            reason: "Address must start with 'G'".to_string(),
        });
    }

    // Check for valid base32 characters (A-Z, 2-7)
    if !address[1..].chars().all(|c| c.is_ascii_alphanumeric() && "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567".contains(c)) {
        return Err(ValidationError::InvalidAddress {
            reason: "Address contains invalid base32 characters".to_string(),
        });
    }

    Ok(())
}

// ============================================================================
// CONTRACT ID VALIDATION
// ============================================================================

/// Validates a contract ID (32-byte hash)
///
/// Contract IDs are byte arrays representing Soroban contract identifiers:
/// - Must be exactly 32 bytes
/// - No format restrictions beyond size
///
/// # Examples
///
/// ```
/// use soroban_security_scanner::validation::validate_contract_id;
///
/// let valid_id: [u8; 32] = [1; 32];
/// assert!(validate_contract_id(&valid_id).is_ok());
/// ```
pub fn validate_contract_id(id: &[u8; 32]) -> Result<(), ValidationError> {
    // Contract IDs must be exactly 32 bytes - this is guaranteed by the type signature
    // Additional validation could check for meaningful content
    if id.iter().all(|b| *b == 0) {
        return Err(ValidationError::InvalidContractId {
            reason: "Contract ID cannot be all zeros".to_string(),
        });
    }

    Ok(())
}

// ============================================================================
// AMOUNT VALIDATION
// ============================================================================

/// Validates an amount (128-bit signed integer)
///
/// Amounts represent token quantities or transaction values:
/// - Must be non-negative
/// - Must not exceed maximum safe value
/// - Can be zero (for some operations)
///
/// # Examples
///
/// ```
/// use soroban_security_scanner::validation::validate_amount;
///
/// assert!(validate_amount(1000).is_ok());
/// assert!(validate_amount(0).is_ok());
/// assert!(validate_amount(-100).is_err());
/// ```
pub fn validate_amount(amount: i128) -> Result<(), ValidationError> {
    if amount < 0 {
        return Err(ValidationError::InvalidAmount {
            reason: "Amount cannot be negative".to_string(),
        });
    }

    // Check against maximum safe value (prevent overflow in downstream operations)
    const MAX_SAFE_AMOUNT: i128 = i128::MAX / 2;
    if amount > MAX_SAFE_AMOUNT {
        return Err(ValidationError::InvalidAmount {
            reason: format!("Amount exceeds maximum value of {}", MAX_SAFE_AMOUNT),
        });
    }

    Ok(())
}

// ============================================================================
// PAGINATION VALIDATION
// ============================================================================

/// Validates pagination parameters
///
/// Pagination controls query result limits and offsets:
/// - `limit`: Must be between 1 and 1000 (default: 50)
/// - `offset`: Must be non-negative (default: 0)
///
/// # Examples
///
/// ```
/// use soroban_security_scanner::validation::validate_pagination;
///
/// assert!(validate_pagination(50, 0).is_ok());
/// assert!(validate_pagination(100, 200).is_ok());
/// assert!(validate_pagination(0, 0).is_err());     // limit too small
/// assert!(validate_pagination(2000, 0).is_err());  // limit too large
/// assert!(validate_pagination(50, u32::MAX).is_err()); // offset too large
/// ```
pub fn validate_pagination(limit: u32, offset: u32) -> Result<(), ValidationError> {
    const MIN_LIMIT: u32 = 1;
    const MAX_LIMIT: u32 = 1000;
    const MAX_OFFSET: u32 = 1_000_000; // Prevent scanning entire database

    if limit < MIN_LIMIT {
        return Err(ValidationError::InvalidPagination {
            reason: format!("Limit must be at least {}", MIN_LIMIT),
        });
    }

    if limit > MAX_LIMIT {
        return Err(ValidationError::InvalidPagination {
            reason: format!("Limit cannot exceed {}", MAX_LIMIT),
        });
    }

    if offset > MAX_OFFSET {
        return Err(ValidationError::InvalidPagination {
            reason: format!("Offset cannot exceed {}", MAX_OFFSET),
        });
    }

    Ok(())
}

// ============================================================================
// STRING SANITIZATION
// ============================================================================

/// Sanitizes a string by truncating to max length and removing control characters
///
/// This function provides basic input sanitization for string fields:
/// - Truncates to specified maximum length
/// - Removes ASCII control characters (except newline and tab)
/// - Validates UTF-8 encoding (implicit via Rust strings)
/// - Preserves Unicode characters
///
/// # Arguments
/// * `input` - The string to sanitize
/// * `max_len` - Maximum allowed length in bytes
///
/// # Returns
/// * `Ok(String)` - Sanitized string
/// * `Err(ValidationError)` - If string is empty or contains only whitespace
///
/// # Examples
///
/// ```
/// use soroban_security_scanner::validation::sanitize_string;
///
/// let input = "  valid_name_123  ";
/// let result = sanitize_string(input, 100).unwrap();
/// assert_eq!(result, "valid_name_123");
///
/// let long_input = "a".repeat(200);
/// let result = sanitize_string(&long_input, 100).unwrap();
/// assert_eq!(result.len(), 100);
/// ```
pub fn sanitize_string(input: &str, max_len: usize) -> Result<String, ValidationError> {
    if input.is_empty() {
        return Err(ValidationError::InvalidString {
            reason: "String cannot be empty".to_string(),
        });
    }

    // Trim whitespace
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Err(ValidationError::InvalidString {
            reason: "String cannot be only whitespace".to_string(),
        });
    }

    // Remove control characters except newline and tab
    let cleaned: String = trimmed
        .chars()
        .filter(|c| {
            let code = *c as u32;
            // Allow printable ASCII, newline, tab, and all Unicode chars
            code >= 32 || *c == '\n' || *c == '\t' || code > 127
        })
        .collect();

    // Truncate to max length (in bytes)
    let truncated = if cleaned.len() > max_len {
        cleaned.char_indices()
            .take_while(|(byte_idx, _)| *byte_idx < max_len)
            .map(|(_, c)| c)
            .collect()
    } else {
        cleaned
    };

    if truncated.is_empty() {
        return Err(ValidationError::InvalidString {
            reason: "String contains only control characters".to_string(),
        });
    }

    Ok(truncated)
}

// ============================================================================
// COMBINED VALIDATION STRUCTURES
// ============================================================================

/// Represents pagination parameters with validated values
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ValidatedPagination {
    pub limit: u32,
    pub offset: u32,
}

impl ValidatedPagination {
    /// Creates and validates pagination parameters
    pub fn new(limit: u32, offset: u32) -> Result<Self, ValidationError> {
        validate_pagination(limit, offset)?;
        Ok(Self { limit, offset })
    }
}

/// Represents a validated Stellar address
#[derive(Debug, Clone)]
pub struct ValidatedAddress(String);

impl ValidatedAddress {
    /// Creates and validates a Stellar address
    pub fn new(address: &str) -> Result<Self, ValidationError> {
        validate_address(address)?;
        Ok(Self(address.to_string()))
    }

    /// Returns the validated address as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes self and returns the inner String
    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for ValidatedAddress {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // ADDRESS VALIDATION TESTS
    // ========================================================================

    #[test]
    fn test_valid_stellar_address() {
        let valid_address = "GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQ";
        assert!(validate_address(valid_address).is_ok());
    }

    #[test]
    fn test_empty_address() {
        assert!(validate_address("").is_err());
    }

    #[test]
    fn test_address_wrong_length() {
        assert!(validate_address("GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFD").is_err());
        assert!(validate_address("GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQX").is_err());
    }

    #[test]
    fn test_address_invalid_prefix() {
        // Addresses must start with 'G'
        assert!(validate_address("ABBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQ").is_err());
    }

    #[test]
    fn test_address_invalid_base32() {
        // Contains invalid characters like 'O' and '0'
        assert!(validate_address("GBBO47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFD0FQ").is_err());
    }

    #[test]
    fn test_validated_address_newtype() {
        let addr = ValidatedAddress::new("GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQ").unwrap();
        assert_eq!(addr.as_str(), "GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQ");
    }

    // ========================================================================
    // CONTRACT ID VALIDATION TESTS
    // ========================================================================

    #[test]
    fn test_valid_contract_id() {
        let valid_id: [u8; 32] = [42; 32];
        assert!(validate_contract_id(&valid_id).is_ok());
    }

    #[test]
    fn test_zero_contract_id() {
        let zero_id: [u8; 32] = [0; 32];
        assert!(validate_contract_id(&zero_id).is_err());
    }

    #[test]
    fn test_contract_id_with_mixed_bytes() {
        let mut id: [u8; 32] = [0; 32];
        id[0] = 255;
        id[15] = 128;
        assert!(validate_contract_id(&id).is_ok());
    }

    // ========================================================================
    // AMOUNT VALIDATION TESTS
    // ========================================================================

    #[test]
    fn test_valid_positive_amount() {
        assert!(validate_amount(1).is_ok());
        assert!(validate_amount(1000).is_ok());
        assert!(validate_amount(1_000_000).is_ok());
    }

    #[test]
    fn test_zero_amount() {
        assert!(validate_amount(0).is_ok());
    }

    #[test]
    fn test_negative_amount() {
        assert!(validate_amount(-1).is_err());
        assert!(validate_amount(-1000).is_err());
    }

    #[test]
    fn test_amount_overflow() {
        let too_large = i128::MAX - 1;
        assert!(validate_amount(too_large).is_err());
    }

    #[test]
    fn test_amount_at_max_safe() {
        let max_safe = i128::MAX / 2;
        assert!(validate_amount(max_safe).is_ok());
        assert!(validate_amount(max_safe + 1).is_err());
    }

    // ========================================================================
    // PAGINATION VALIDATION TESTS
    // ========================================================================

    #[test]
    fn test_valid_pagination() {
        assert!(validate_pagination(50, 0).is_ok());
        assert!(validate_pagination(1, 0).is_ok());
        assert!(validate_pagination(1000, 0).is_ok());
        assert!(validate_pagination(100, 500_000).is_ok());
    }

    #[test]
    fn test_pagination_limit_too_small() {
        assert!(validate_pagination(0, 0).is_err());
    }

    #[test]
    fn test_pagination_limit_too_large() {
        assert!(validate_pagination(1001, 0).is_err());
        assert!(validate_pagination(u32::MAX, 0).is_err());
    }

    #[test]
    fn test_pagination_offset_too_large() {
        assert!(validate_pagination(50, 1_000_001).is_err());
    }

    #[test]
    fn test_validated_pagination() {
        let pag = ValidatedPagination::new(50, 0).unwrap();
        assert_eq!(pag.limit, 50);
        assert_eq!(pag.offset, 0);
    }

    // ========================================================================
    // STRING SANITIZATION TESTS
    // ========================================================================

    #[test]
    fn test_sanitize_valid_string() {
        let result = sanitize_string("hello", 100).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_sanitize_string_with_whitespace() {
        let result = sanitize_string("  hello  ", 100).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_sanitize_empty_string() {
        assert!(sanitize_string("", 100).is_err());
    }

    #[test]
    fn test_sanitize_whitespace_only() {
        assert!(sanitize_string("   ", 100).is_err());
    }

    #[test]
    fn test_sanitize_truncate_to_max_length() {
        let long_string = "a".repeat(200);
        let result = sanitize_string(&long_string, 100).unwrap();
        assert_eq!(result.len(), 100);
    }

    #[test]
    fn test_sanitize_removes_control_characters() {
        // String with embedded control characters
        let input = "hello\x00world\x01test";
        let result = sanitize_string(input, 100).unwrap();
        assert_eq!(result, "helloworld test");
    }

    #[test]
    fn test_sanitize_preserves_newlines_and_tabs() {
        let input = "hello\nworld\ttest";
        let result = sanitize_string(input, 100).unwrap();
        assert_eq!(result, "hello\nworld\ttest");
    }

    #[test]
    fn test_sanitize_unicode_characters() {
        let input = "hello 世界 mùndo";
        let result = sanitize_string(input, 100).unwrap();
        assert_eq!(result, "hello 世界 mùndo");
    }

    #[test]
    fn test_sanitize_unicode_truncation() {
        // Multi-byte unicode characters should not be cut in the middle
        let input = "hello世界world";
        let result = sanitize_string(input, 10).unwrap();
        // Should truncate at character boundaries
        assert!(result.len() <= 10);
        assert!(result.is_char_boundary(result.len()));
    }

    // ========================================================================
    // ERROR RESPONSE TESTS
    // ========================================================================

    #[test]
    fn test_validation_error_display() {
        let error = ValidationError::InvalidAddress {
            reason: "test error".to_string(),
        };
        assert!(error.to_string().contains("Invalid address format"));
    }

    #[test]
    fn test_error_response_structure() {
        let error = ValidationError::InvalidAmount {
            reason: "too large".to_string(),
        };
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // ========================================================================
    // PROPERTY-BASED TESTS
    // ========================================================================
    // These tests verify invariants that should hold across all valid inputs
    // They use strategic examples to test edge cases and boundaries

    #[test]
    fn test_valid_amount_is_always_non_negative() {
        // Test a range of valid amounts
        for amount in [0i128, 1, 100, 1000, 1_000_000, i128::MAX / 2] {
            assert!(validate_amount(amount).is_ok(), "Valid amount {} should pass", amount);
        }
    }

    #[test]
    fn test_valid_pagination_limits_are_bounded() {
        // Test the full valid range of pagination limits
        for limit in [1u32, 50, 100, 500, 1000] {
            for offset in [0u32, 1, 100, 10_000, 500_000] {
                assert!(
                    validate_pagination(limit, offset).is_ok(),
                    "Valid pagination ({}, {}) should pass",
                    limit,
                    offset
                );
            }
        }
    }

    #[test]
    fn test_sanitized_string_never_empty_when_valid() {
        // Test that sanitization of valid inputs never produces empty strings
        let test_cases = vec![
            ("test", 100),
            ("hello world", 100),
            ("  spaces  ", 100),
            ("x", 1),
            ("很好", 100),
        ];

        for (input, max_len) in test_cases {
            if let Ok(result) = sanitize_string(input, max_len) {
                assert!(!result.is_empty(), "Sanitized result should never be empty");
            }
        }
    }

    #[test]
    fn test_sanitized_string_respects_max_length() {
        // Test that truncation always respects max length boundaries
        let test_inputs = vec![
            ("a".repeat(200), 100),
            ("test long string here please truncate me", 20),
            ("abcdefghijklmnopqrstuvwxyz", 10),
        ];

        for (input, max_len) in test_inputs {
            if let Ok(result) = sanitize_string(&input, max_len) {
                assert!(
                    result.len() <= max_len,
                    "Sanitized string length {} exceeds max {}",
                    result.len(),
                    max_len
                );
            }
        }
    }

    #[test]
    fn test_invalid_amounts_all_negative() {
        // All negative amounts should be rejected
        for amount in [-1i128, -100, -1000, i128::MIN] {
            assert!(
                validate_amount(amount).is_err(),
                "Negative amount {} should be rejected",
                amount
            );
        }
    }

    #[test]
    fn test_invalid_pagination_limits_boundaries() {
        // Test boundary conditions
        assert!(validate_pagination(0, 0).is_err(), "Limit 0 should be invalid");
        assert!(validate_pagination(1001, 0).is_err(), "Limit > 1000 should be invalid");

        // Test large offsets
        assert!(
            validate_pagination(50, 1_000_001).is_err(),
            "Large offset should be invalid"
        );
    }

    #[test]
    fn test_invalid_addresses_all_rejected() {
        let invalid_addresses = vec![
            "",                                                    // empty
            "invalid",                                             // too short
            "GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFD", // too short (55 chars)
            "GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQX", // too long (57 chars)
            "ABBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQ", // invalid prefix
            "GBBO47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFD0FQ", // invalid chars (0,O)
        ];

        for addr in invalid_addresses {
            assert!(
                validate_address(addr).is_err(),
                "Invalid address '{}' should be rejected",
                addr
            );
        }
    }

    #[test]
    fn test_contract_id_validation_only_rejects_zeros() {
        // Contract IDs with any non-zero byte should be valid
        for i in 0..32 {
            let mut id: [u8; 32] = [0; 32];
            id[i] = 1; // Set one byte to non-zero
            assert!(validate_contract_id(&id).is_ok());
        }

        // Only all-zero contract ID should fail
        assert!(validate_contract_id(&[0; 32]).is_err());
    }

    #[test]
    fn test_string_sanitization_idempotent_for_valid_input() {
        // Test that sanitizing an already-sanitized string is idempotent
        let input = "hello world";
        let result1 = sanitize_string(input, 100).unwrap();
        let result2 = sanitize_string(&result1, 100).unwrap();
        assert_eq!(result1, result2, "Sanitization should be idempotent");
    }

    #[test]
    fn test_validation_consistency_across_boundary_values() {
        // Test consistency around important boundary values

        // Amount boundaries
        assert!(validate_amount(i128::MAX / 2).is_ok());
        assert!(validate_amount(i128::MAX / 2 + 1).is_err());

        // Pagination boundaries
        assert!(validate_pagination(1, 0).is_ok());
        assert!(validate_pagination(0, 0).is_err());
        assert!(validate_pagination(1000, 0).is_ok());
        assert!(validate_pagination(1001, 0).is_err());

        // Address length boundaries
        assert!(validate_address("GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQ").is_ok());
        assert!(validate_address("GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7F").is_err());
        assert!(validate_address("GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQA").is_err());
    }

    #[test]
    fn test_sanitization_handles_edge_case_strings() {
        // Test various edge cases that might cause issues
        let edge_cases = vec![
            ("a", 1),                      // minimum single char
            ("a", 0),                      // truncate to zero (should still fail?)
            ("\u{200B}", 100),             // zero-width space
            ("test\u{0000}test", 100),     // null byte embedded
            ("   \t\n   ", 100),           // only whitespace and control chars
            ("😀😁😂", 100),                // emoji characters
        ];

        for (input, max_len) in edge_cases {
            // Just verify these don't panic - results may vary
            let _ = sanitize_string(input, max_len);
        }
    }

    #[test]
    fn test_error_types_are_clone_and_debug() {
        // Verify error types have required traits
        let error = ValidationError::InvalidAddress {
            reason: "test".to_string(),
        };
        let _cloned = error.clone();
        println!("{:?}", error); // Verify Debug trait works
    }

    #[test]
    fn test_validated_address_from_valid_string() {
        let valid = "GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQ";
        let addr = ValidatedAddress::new(valid).unwrap();
        assert_eq!(addr.as_str(), valid);
        assert_eq!(addr.as_ref(), valid);
    }

    #[test]
    fn test_validated_address_rejects_invalid() {
        assert!(ValidatedAddress::new("invalid").is_err());
        assert!(ValidatedAddress::new("").is_err());
    }

    #[test]
    fn test_validated_pagination_accessors() {
        let pag = ValidatedPagination::new(100, 500).unwrap();
        assert_eq!(pag.limit, 100);
        assert_eq!(pag.offset, 500);
    }

    #[test]
    fn test_large_string_truncation_preserves_integrity() {
        // Test that truncating very large strings maintains UTF-8 validity
        let large_string = "test ".repeat(10000) + "end";
        let result = sanitize_string(&large_string, 500).unwrap();
        assert!(result.is_char_boundary(result.len()));
        assert!(result.len() <= 500);
        // Ensure we can iterate over characters without panicking
        for _ in result.chars() {}
    }
}

