//! Content-type allowlisting and consistency checks.
//!
//! Only a small set of declared content types is accepted, and the declared
//! type must be *consistent* with the kind sniffed from the bytes — a request
//! that claims `application/wasm` but whose bytes are an ELF binary is
//! rejected.

use crate::upload_sanitization::magic::FileKind;
use serde::{Deserialize, Serialize};

/// A declared, allowlisted content type for contract-code uploads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllowedContentType {
    /// `application/wasm`.
    Wasm,
    /// `text/rust` / `text/x-rust` — Rust source.
    RustSource,
    /// `text/x-toml` / `application/toml` — Cargo manifest.
    Toml,
    /// `text/plain` — generic source text.
    PlainText,
}

impl AllowedContentType {
    /// Parses a declared MIME type (case-insensitive, parameters ignored).
    pub fn parse(mime: &str) -> Option<Self> {
        let base = mime.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
        match base.as_str() {
            "application/wasm" => Some(Self::Wasm),
            "text/rust" | "text/x-rust" | "application/rust" => Some(Self::RustSource),
            "text/x-toml" | "application/toml" | "text/toml" => Some(Self::Toml),
            "text/plain" => Some(Self::PlainText),
            _ => None,
        }
    }

    /// The file kind the declared type implies.
    pub fn expected_kind(&self) -> FileKind {
        match self {
            AllowedContentType::Wasm => FileKind::Wasm,
            AllowedContentType::RustSource
            | AllowedContentType::Toml
            | AllowedContentType::PlainText => FileKind::SourceText,
        }
    }
}

/// Result of validating a declared content type against sniffed bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentTypeCheck {
    /// Declared type is allowlisted and matches the detected kind.
    Ok(AllowedContentType),
    /// Declared type is not on the allowlist.
    NotAllowed,
    /// Declared type is allowlisted but contradicts the detected bytes.
    Mismatch {
        /// What the bytes actually look like.
        detected: FileKind,
        /// What the declaration implied.
        expected: FileKind,
    },
}

/// Validates a declared MIME type against the detected file kind.
pub fn check(declared_mime: &str, detected: FileKind) -> ContentTypeCheck {
    let Some(allowed) = AllowedContentType::parse(declared_mime) else {
        return ContentTypeCheck::NotAllowed;
    };
    let expected = allowed.expected_kind();
    if expected == detected {
        ContentTypeCheck::Ok(allowed)
    } else {
        ContentTypeCheck::Mismatch { detected, expected }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_parsing() {
        assert_eq!(AllowedContentType::parse("application/wasm"), Some(AllowedContentType::Wasm));
        assert_eq!(
            AllowedContentType::parse("text/x-rust; charset=utf-8"),
            Some(AllowedContentType::RustSource)
        );
        assert_eq!(AllowedContentType::parse("application/octet-stream"), None);
        assert_eq!(AllowedContentType::parse("image/png"), None);
    }

    #[test]
    fn consistent_declaration_passes() {
        assert_eq!(
            check("application/wasm", FileKind::Wasm),
            ContentTypeCheck::Ok(AllowedContentType::Wasm)
        );
        assert_eq!(
            check("text/x-rust", FileKind::SourceText),
            ContentTypeCheck::Ok(AllowedContentType::RustSource)
        );
    }

    #[test]
    fn disallowed_type_is_rejected() {
        assert_eq!(check("application/x-msdownload", FileKind::Wasm), ContentTypeCheck::NotAllowed);
    }

    #[test]
    fn mismatch_is_detected() {
        // Claims WASM but bytes are an ELF executable.
        assert_eq!(
            check("application/wasm", FileKind::Elf),
            ContentTypeCheck::Mismatch {
                detected: FileKind::Elf,
                expected: FileKind::Wasm,
            }
        );
    }
}
