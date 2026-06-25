//! Input validation for the smart contract analysis engine.
//!
//! Prevents parser crashes, infinite loops, and memory exhaustion by
//! rejecting malformed or oversized inputs before they reach the analyser.

use thiserror::Error;

/// Hard limits for analysis inputs.
pub mod limits {
    /// Maximum source code size in bytes (512 KiB).
    pub const MAX_SOURCE_BYTES: usize = 512 * 1024;
    /// Maximum number of AST nodes.
    pub const MAX_AST_NODES: usize = 50_000;
    /// Maximum analysis timeout in seconds.
    pub const MAX_TIMEOUT_SECS: u64 = 300;
    /// Maximum nesting depth in AST structures.
    pub const MAX_NESTING_DEPTH: u32 = 128;
    /// Maximum number of functions in a single contract.
    pub const MAX_FUNCTIONS: usize = 1_000;
    /// Minimum source length (empty contracts are invalid).
    pub const MIN_SOURCE_BYTES: usize = 10;
}

/// Errors returned by analysis input validators.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AnalysisValidationError {
    #[error("source code is empty or too short (min {} bytes)", limits::MIN_SOURCE_BYTES)]
    SourceTooShort,
    #[error("source code exceeds maximum size of {} bytes", limits::MAX_SOURCE_BYTES)]
    SourceTooLarge,
    #[error("AST contains {0} nodes, exceeding limit of {}", limits::MAX_AST_NODES)]
    AstTooLarge(usize),
    #[error("AST nesting depth {0} exceeds limit of {}", limits::MAX_NESTING_DEPTH)]
    NestingTooDeep(u32),
    #[error("timeout {0}s exceeds maximum of {}s", limits::MAX_TIMEOUT_SECS)]
    TimeoutTooLong(u64),
    #[error("contract has {0} functions, exceeding limit of {}", limits::MAX_FUNCTIONS)]
    TooManyFunctions(usize),
    #[error("source contains disallowed binary content")]
    BinaryContent,
    #[error("source contains null bytes")]
    NullBytes,
}

// ---------------------------------------------------------------------------
// Source code validation
// ---------------------------------------------------------------------------

/// Validates raw contract source code before passing to the parser.
pub fn validate_source(source: &[u8]) -> Result<(), AnalysisValidationError> {
    if source.len() < limits::MIN_SOURCE_BYTES {
        return Err(AnalysisValidationError::SourceTooShort);
    }
    if source.len() > limits::MAX_SOURCE_BYTES {
        return Err(AnalysisValidationError::SourceTooLarge);
    }
    if source.contains(&0u8) {
        return Err(AnalysisValidationError::NullBytes);
    }
    // Reject files with a high ratio of non-UTF8 bytes (binary blobs).
    let text_ratio = source.iter().filter(|b| b.is_ascii()).count() as f64 / source.len() as f64;
    if text_ratio < 0.85 {
        return Err(AnalysisValidationError::BinaryContent);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// AST parameter validation
// ---------------------------------------------------------------------------

/// Validates parameters describing an AST structure.
pub fn validate_ast_params(
    node_count:    usize,
    nesting_depth: u32,
    function_count: usize,
) -> Result<(), AnalysisValidationError> {
    if node_count > limits::MAX_AST_NODES {
        return Err(AnalysisValidationError::AstTooLarge(node_count));
    }
    if nesting_depth > limits::MAX_NESTING_DEPTH {
        return Err(AnalysisValidationError::NestingTooDeep(nesting_depth));
    }
    if function_count > limits::MAX_FUNCTIONS {
        return Err(AnalysisValidationError::TooManyFunctions(function_count));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Analysis parameters validation
// ---------------------------------------------------------------------------

/// Validates user-supplied analysis parameters.
pub fn validate_analysis_params(timeout_secs: u64) -> Result<(), AnalysisValidationError> {
    if timeout_secs > limits::MAX_TIMEOUT_SECS {
        return Err(AnalysisValidationError::TimeoutTooLong(timeout_secs));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_source_rejected() {
        assert!(matches!(validate_source(b""), Err(AnalysisValidationError::SourceTooShort)));
    }

    #[test]
    fn oversized_source_rejected() {
        let big = vec![b'a'; limits::MAX_SOURCE_BYTES + 1];
        assert!(matches!(validate_source(&big), Err(AnalysisValidationError::SourceTooLarge)));
    }

    #[test]
    fn null_bytes_rejected() {
        let mut src = b"fn main() {}".to_vec();
        src.push(0);
        assert!(matches!(validate_source(&src), Err(AnalysisValidationError::NullBytes)));
    }

    #[test]
    fn binary_content_rejected() {
        let binary: Vec<u8> = (0u8..=255).cycle().take(200).collect();
        assert!(matches!(validate_source(&binary), Err(AnalysisValidationError::BinaryContent)));
    }

    #[test]
    fn valid_source_accepted() {
        assert!(validate_source(b"fn main() { println!(\"hello\"); }").is_ok());
    }

    #[test]
    fn ast_too_large_rejected() {
        assert!(validate_ast_params(limits::MAX_AST_NODES + 1, 10, 5).is_err());
    }

    #[test]
    fn nesting_too_deep_rejected() {
        assert!(validate_ast_params(100, limits::MAX_NESTING_DEPTH + 1, 5).is_err());
    }

    #[test]
    fn timeout_too_long_rejected() {
        assert!(validate_analysis_params(limits::MAX_TIMEOUT_SECS + 1).is_err());
    }

    #[test]
    fn valid_params_accepted() {
        assert!(validate_ast_params(1000, 20, 50).is_ok());
        assert!(validate_analysis_params(60).is_ok());
    }
}
