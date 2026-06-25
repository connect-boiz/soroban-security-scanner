//! Input validation and sanitisation module.
//!
//! Provides reusable validators for all API endpoints.
//! Every validator returns a typed `ValidationError` on failure,
//! which maps directly to `AppError::ValidationError`.

use thiserror::Error;

/// A validation failure with a field name and human-readable message.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("field `{field}`: {message}")]
pub struct ValidationError {
    pub field:   &'static str,
    pub message: String,
}

impl ValidationError {
    fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self { field, message: message.into() }
    }
}

// ---------------------------------------------------------------------------
// Address validation
// ---------------------------------------------------------------------------

/// Validates a Stellar / Soroban account or contract address.
///
/// Stellar addresses are base32-encoded 56-character strings starting
/// with `G` (account) or `C` (contract).
pub fn validate_address(address: &str) -> Result<(), ValidationError> {
    if address.is_empty() {
        return Err(ValidationError::new("address", "must not be empty"));
    }
    if address.len() != 56 {
        return Err(ValidationError::new(
            "address",
            format!("must be 56 characters, got {}", address.len()),
        ));
    }
    let first = address.chars().next().unwrap();
    if first != 'G' && first != 'C' {
        return Err(ValidationError::new(
            "address",
            "must start with 'G' (account) or 'C' (contract)",
        ));
    }
    if !address.chars().all(|c| c.is_ascii_alphanumeric() && c.is_ascii_uppercase()) {
        return Err(ValidationError::new(
            "address",
            "must contain only uppercase alphanumeric characters",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Contract ID validation
// ---------------------------------------------------------------------------

/// Validates a 32-byte Soroban contract ID.
pub fn validate_contract_id(id: &[u8; 32]) -> Result<(), ValidationError> {
    // All-zero contract IDs are invalid (null address).
    if id.iter().all(|b| *b == 0) {
        return Err(ValidationError::new("contract_id", "must not be the zero address"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Amount validation
// ---------------------------------------------------------------------------

/// Validates a Stellar token amount.
/// Amounts must be positive and fit within the Stellar i128 range.
pub fn validate_amount(amount: i128) -> Result<(), ValidationError> {
    if amount <= 0 {
        return Err(ValidationError::new("amount", "must be greater than zero"));
    }
    // Stellar stores amounts as stroops (1 XLM = 10^7 stroops).
    // Practical upper bound: 100 billion tokens * 10^7 stroops.
    const MAX: i128 = 100_000_000_000 * 10_000_000;
    if amount > MAX {
        return Err(ValidationError::new(
            "amount",
            format!("exceeds maximum allowed amount of {}", MAX),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Pagination validation
// ---------------------------------------------------------------------------

/// Validates pagination parameters.
/// `limit` must be 1–100; `offset` must be 0–100_000.
pub fn validate_pagination(limit: u32, offset: u32) -> Result<(), ValidationError> {
    if limit == 0 || limit > 100 {
        return Err(ValidationError::new("limit", "must be between 1 and 100"));
    }
    if offset > 100_000 {
        return Err(ValidationError::new("offset", "must not exceed 100,000"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// String sanitisation
// ---------------------------------------------------------------------------

/// Sanitises a user-supplied string.
/// Trims whitespace, rejects empty strings, enforces `max_len`.
/// Does **not** allow NUL bytes or ASCII control characters.
pub fn sanitize_string(input: &str, max_len: usize) -> Result<String, ValidationError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ValidationError::new("input", "must not be empty or whitespace-only"));
    }
    if trimmed.len() > max_len {
        return Err(ValidationError::new(
            "input",
            format!("exceeds maximum length of {} characters", max_len),
        ));
    }
    if trimmed.chars().any(|c| c.is_ascii_control() && c != '\t') {
        return Err(ValidationError::new(
            "input",
            "must not contain ASCII control characters",
        ));
    }
    Ok(trimmed.to_owned())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_account_address() {
        let addr = "G".to_owned() + &"A".repeat(55);
        assert!(validate_address(&addr).is_ok());
    }

    #[test]
    fn address_wrong_length() {
        assert!(validate_address("GABC").is_err());
    }

    #[test]
    fn address_bad_prefix() {
        let addr = "X".to_owned() + &"A".repeat(55);
        assert!(validate_address(&addr).is_err());
    }

    #[test]
    fn zero_amount_rejected() {
        assert!(validate_amount(0).is_err());
    }

    #[test]
    fn negative_amount_rejected() {
        assert!(validate_amount(-1).is_err());
    }

    #[test]
    fn positive_amount_allowed() {
        assert!(validate_amount(1_000_000).is_ok());
    }

    #[test]
    fn pagination_limit_zero_rejected() {
        assert!(validate_pagination(0, 0).is_err());
    }

    #[test]
    fn pagination_limit_over_100_rejected() {
        assert!(validate_pagination(101, 0).is_err());
    }

    #[test]
    fn sanitize_trims_whitespace() {
        assert_eq!(sanitize_string("  hello  ", 100).unwrap(), "hello");
    }

    #[test]
    fn sanitize_rejects_over_max_len() {
        assert!(sanitize_string("hello world", 5).is_err());
    }

    #[test]
    fn sanitize_rejects_control_chars() {
        assert!(sanitize_string("hello\x01world", 100).is_err());
    }
}
