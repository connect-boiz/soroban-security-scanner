//! Contract-code input validation and sanitization.
//!
//! The first gate before any parsing: rejects input that is too large, not
//! valid UTF-8, contains NUL/control bytes, has pathologically long lines or
//! unbalanced/over-nested delimiters — the shapes that crash or hang naive
//! parsers — and strips a small set of dangerous patterns.

use crate::analysis_guard::limits::InputLimits;
use serde::{Deserialize, Serialize};

/// Why input validation rejected a contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputError {
    /// Empty input.
    Empty,
    /// Exceeds the byte-size limit.
    TooLarge {
        /// Actual size.
        size: usize,
        /// Allowed maximum.
        max: usize,
    },
    /// Not valid UTF-8.
    NotUtf8,
    /// Contains NUL or disallowed control bytes.
    ControlBytes,
    /// A line exceeds the maximum length.
    LineTooLong {
        /// The 1-based line number.
        line: usize,
    },
    /// Delimiters are unbalanced.
    UnbalancedDelimiters,
    /// Delimiter nesting exceeds the depth limit.
    DelimiterTooDeep {
        /// Allowed maximum depth.
        max: usize,
    },
}

/// A successfully validated (and lightly sanitized) input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatedInput {
    /// The sanitized source text.
    pub source: String,
    /// Notes describing any sanitization applied.
    pub notes: Vec<String>,
}

/// Validates raw bytes as contract source against `limits`. Returns the
/// sanitized input or the first error encountered.
pub fn validate(bytes: &[u8], limits: &InputLimits) -> Result<ValidatedInput, InputError> {
    if bytes.is_empty() {
        return Err(InputError::Empty);
    }
    if bytes.len() > limits.max_bytes {
        return Err(InputError::TooLarge {
            size: bytes.len(),
            max: limits.max_bytes,
        });
    }
    let text = std::str::from_utf8(bytes).map_err(|_| InputError::NotUtf8)?;

    // No NUL / disallowed control bytes (allow tab, LF, CR).
    if text
        .bytes()
        .any(|b| b == 0 || (b < 0x20 && b != b'\t' && b != b'\n' && b != b'\r'))
    {
        return Err(InputError::ControlBytes);
    }

    // Line length.
    for (i, line) in text.lines().enumerate() {
        if line.len() > limits.max_line_len {
            return Err(InputError::LineTooLong { line: i + 1 });
        }
    }

    // Delimiter balance and nesting depth.
    check_delimiters(text, limits.max_delimiter_depth)?;

    // Sanitize: normalize CRLF→LF and strip a UTF-8 BOM.
    let mut notes = Vec::new();
    let mut source = text.to_string();
    if source.starts_with('\u{feff}') {
        source.remove(0);
        notes.push("removed UTF-8 BOM".to_string());
    }
    if source.contains('\r') {
        source = source.replace("\r\n", "\n").replace('\r', "\n");
        notes.push("normalized line endings".to_string());
    }

    Ok(ValidatedInput { source, notes })
}

fn check_delimiters(text: &str, max_depth: usize) -> Result<(), InputError> {
    let mut stack: Vec<char> = Vec::new();
    let mut max_seen = 0usize;
    for c in text.chars() {
        match c {
            '(' | '[' | '{' => {
                stack.push(c);
                max_seen = max_seen.max(stack.len());
                if stack.len() > max_depth {
                    return Err(InputError::DelimiterTooDeep { max: max_depth });
                }
            }
            ')' | ']' | '}' => {
                let expected = match c {
                    ')' => '(',
                    ']' => '[',
                    _ => '{',
                };
                if stack.pop() != Some(expected) {
                    return Err(InputError::UnbalancedDelimiters);
                }
            }
            _ => {}
        }
    }
    if stack.is_empty() {
        Ok(())
    } else {
        Err(InputError::UnbalancedDelimiters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> InputLimits {
        InputLimits::default()
    }

    #[test]
    fn accepts_clean_source() {
        let r = validate(b"pub fn main() { let x = (1 + 2); }", &limits()).unwrap();
        assert!(r.notes.is_empty());
    }

    #[test]
    fn rejects_empty_and_oversize() {
        assert_eq!(validate(b"", &limits()).unwrap_err(), InputError::Empty);
        let small = InputLimits {
            max_bytes: 4,
            ..limits()
        };
        assert!(matches!(
            validate(b"abcdef", &small),
            Err(InputError::TooLarge { .. })
        ));
    }

    #[test]
    fn rejects_nul_and_control_bytes() {
        assert_eq!(
            validate(b"abc\0def", &limits()).unwrap_err(),
            InputError::ControlBytes
        );
        assert_eq!(
            validate(b"a\x07b", &limits()).unwrap_err(),
            InputError::ControlBytes
        );
    }

    #[test]
    fn rejects_overlong_line() {
        let small = InputLimits {
            max_line_len: 5,
            ..limits()
        };
        assert_eq!(
            validate(b"abcdefghij", &small).unwrap_err(),
            InputError::LineTooLong { line: 1 }
        );
    }

    #[test]
    fn rejects_unbalanced_and_too_deep() {
        assert_eq!(
            validate(b"fn x( {", &limits()).unwrap_err(),
            InputError::UnbalancedDelimiters
        );
        assert_eq!(
            validate(b")", &limits()).unwrap_err(),
            InputError::UnbalancedDelimiters
        );
        let shallow = InputLimits {
            max_delimiter_depth: 2,
            ..limits()
        };
        assert!(matches!(
            validate(b"((( )))", &shallow),
            Err(InputError::DelimiterTooDeep { .. })
        ));
    }

    #[test]
    fn sanitizes_bom_and_newlines() {
        let mut data = vec![0xEF, 0xBB, 0xBF];
        data.extend_from_slice(b"fn x() {}\r\n");
        let r = validate(&data, &limits()).unwrap();
        assert!(!r.source.contains('\r'));
        assert!(!r.source.starts_with('\u{feff}'));
        assert_eq!(r.notes.len(), 2);
    }
}
