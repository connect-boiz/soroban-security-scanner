//! Input sanitisation for contract code uploads.
//!
//! Validates uploaded contract files before they reach the parser or
//! storage layer. Checks:
//! - Content-Type header vs. allowed MIME types
//! - File extension whitelist
//! - Magic-byte detection for binary disguised as text
//! - Maximum file size
//! - Null byte injection
//! - Path traversal in filename

use thiserror::Error;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum accepted upload size in bytes (1 MiB).
pub const MAX_UPLOAD_BYTES: usize = 1 * 1024 * 1024;

/// Allowed source file extensions.
pub const ALLOWED_EXTENSIONS: &[&str] = &["rs", "ts", "js", "sol", "wasm", "json", "toml"];

/// Allowed MIME types.
pub const ALLOWED_MIME_TYPES: &[&str] = &[
    "text/plain",
    "text/x-rust",
    "application/json",
    "application/wasm",
    "application/octet-stream", // .wasm fallback
];

/// Known binary magic bytes that must not appear in text uploads.
const BINARY_MAGIC: &[&[u8]] = &[
    b"\x7fELF",        // ELF binary
    b"MZ",            // Windows PE
    b"\xca\xfe\xba\xbe", // Mach-O
    b"\x50\x4b\x03\x04", // ZIP
    b"\x1f\x8b",       // gzip
];

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UploadError {
    #[error("file exceeds maximum size of {} bytes", MAX_UPLOAD_BYTES)]
    FileTooLarge,
    #[error("file is empty")]
    EmptyFile,
    #[error("file extension '{0}' is not allowed")]
    DisallowedExtension(String),
    #[error("MIME type '{0}' is not allowed")]
    DisallowedMimeType(String),
    #[error("filename contains path traversal characters")]
    PathTraversal,
    #[error("filename contains null bytes")]
    NullBytesInFilename,
    #[error("file content contains null bytes")]
    NullBytesInContent,
    #[error("file appears to be binary content (magic bytes detected)")]
    BinaryContent,
    #[error("text content ratio too low — possible binary disguised as text")]
    SuspiciousContent,
}

// ---------------------------------------------------------------------------
// Validators
// ---------------------------------------------------------------------------

/// Validate an uploaded filename.
pub fn validate_filename(name: &str) -> Result<(), UploadError> {
    if name.contains('\0') {
        return Err(UploadError::NullBytesInFilename);
    }
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err(UploadError::PathTraversal);
    }
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(UploadError::DisallowedExtension(ext));
    }
    Ok(())
}

/// Validate a Content-Type header value.
pub fn validate_mime_type(content_type: &str) -> Result<(), UploadError> {
    let ct = content_type.split(';').next().unwrap_or("").trim().to_lowercase();
    if !ALLOWED_MIME_TYPES.contains(&ct.as_str()) {
        return Err(UploadError::DisallowedMimeType(ct));
    }
    Ok(())
}

/// Validate the raw bytes of an uploaded file.
pub fn validate_content(bytes: &[u8]) -> Result<(), UploadError> {
    if bytes.is_empty() {
        return Err(UploadError::EmptyFile);
    }
    if bytes.len() > MAX_UPLOAD_BYTES {
        return Err(UploadError::FileTooLarge);
    }
    if bytes.contains(&0u8) {
        return Err(UploadError::NullBytesInContent);
    }
    // Magic-byte binary detection
    for magic in BINARY_MAGIC {
        if bytes.starts_with(magic) {
            return Err(UploadError::BinaryContent);
        }
    }
    // Text ratio check (>85% ASCII required)
    let ascii_ratio = bytes.iter().filter(|b| b.is_ascii()).count() as f64 / bytes.len() as f64;
    if ascii_ratio < 0.85 {
        return Err(UploadError::SuspiciousContent);
    }
    Ok(())
}

/// Run all three checks together.
pub fn validate_upload(
    filename:     &str,
    content_type: &str,
    bytes:        &[u8],
) -> Result<(), UploadError> {
    validate_filename(filename)?;
    validate_mime_type(content_type)?;
    validate_content(bytes)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_rust_upload_passes() {
        assert!(validate_upload("contract.rs", "text/plain", b"fn main() {}").is_ok());
    }

    #[test]
    fn path_traversal_rejected() {
        assert!(matches!(validate_filename("../etc/passwd"), Err(UploadError::PathTraversal)));
    }

    #[test]
    fn disallowed_extension_rejected() {
        assert!(matches!(validate_filename("evil.exe"), Err(UploadError::DisallowedExtension(_))));
    }

    #[test]
    fn disallowed_mime_rejected() {
        assert!(matches!(
            validate_mime_type("application/x-executable"),
            Err(UploadError::DisallowedMimeType(_))
        ));
    }

    #[test]
    fn null_bytes_in_filename_rejected() {
        assert!(matches!(validate_filename("file\0.rs"), Err(UploadError::NullBytesInFilename)));
    }

    #[test]
    fn null_bytes_in_content_rejected() {
        let mut content = b"fn main() {}".to_vec();
        content.push(0);
        assert!(matches!(validate_content(&content), Err(UploadError::NullBytesInContent)));
    }

    #[test]
    fn elf_binary_rejected() {
        let elf = b"\x7fELFxxxx";
        assert!(matches!(validate_content(elf), Err(UploadError::BinaryContent)));
    }

    #[test]
    fn oversized_file_rejected() {
        let big = vec![b'a'; MAX_UPLOAD_BYTES + 1];
        assert!(matches!(validate_content(&big), Err(UploadError::FileTooLarge)));
    }
}
