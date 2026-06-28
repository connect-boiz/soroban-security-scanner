//! Input sanitization and validation for contract-code uploads (issue #330).
//!
//! A defence-in-depth pipeline that validates an uploaded file before it ever
//! reaches the parser/analyzer. It sniffs the true file type from magic bytes,
//! enforces a content-type allowlist, structurally validates WASM, scans for
//! embedded malicious patterns and known malware, sanitizes accepted content,
//! and quarantines anything suspicious — all under tier-based size limits and
//! per-user / per-IP rate limiting.
//!
//! # Acceptance-criteria mapping
//!
//! | Requirement | Where |
//! |-------------|-------|
//! | File-type validation via magic numbers | [`magic::detect`] |
//! | Content-type allowlist (WASM, Rust, …) | [`content_type`] |
//! | Deep inspection for malicious patterns | [`deep_inspection::inspect`] |
//! | WASM structure validation (header/sections) | [`wasm::validate`] |
//! | Virus/malware scanning integration | [`malware::MalwareScanner`] |
//! | Content sanitization | [`sanitize::sanitize_content`] |
//! | Quarantine system for suspicious files | [`quarantine::Quarantine`] |
//! | Upload rate limiting (per user / per IP) | [`limits::UploadRateLimiter`] |
//! | Tier-based size limits (1/10/100 MB) | [`limits::UploadTier`] |
//! | Progress tracking & timeout handling | [`progress::UploadProgress`] |
//! | Comprehensive security testing | per-module tests + [`tests`] |
//!
//! # Example
//!
//! ```
//! use soroban_security_scanner::upload_sanitization::*;
//!
//! let scanner = SignatureScanner::new();
//! let req = UploadRequest {
//!     bytes: b"pub fn main() {}\n",
//!     declared_content_type: "text/x-rust",
//!     filename: "../../etc/lib.rs",
//!     tier: UploadTier::Free,
//! };
//! match validate_upload(&req, &scanner) {
//!     UploadVerdict::Accepted { filename, .. } => assert_eq!(filename, "lib.rs"),
//!     other => panic!("unexpected: {other:?}"),
//! }
//! ```

pub mod content_type;
pub mod deep_inspection;
pub mod limits;
pub mod magic;
pub mod malware;
pub mod pipeline;
pub mod progress;
pub mod quarantine;
pub mod sanitize;
pub mod wasm;

#[cfg(test)]
mod tests;

pub use content_type::{check as check_content_type, AllowedContentType, ContentTypeCheck};
pub use deep_inspection::{inspect, Finding, InspectionReport, Severity};
pub use limits::{RateScope, SizeError, UploadRateConfig, UploadRateLimiter, UploadTier};
pub use magic::{detect, FileKind};
pub use malware::{MalwareScanner, ScanVerdict, SignatureScanner};
pub use pipeline::{validate_upload, UploadRequest, UploadVerdict};
pub use progress::{UploadProgress, UploadState};
pub use quarantine::{Quarantine, QuarantineEntry, QuarantineStatus};
pub use sanitize::{sanitize_content, sanitize_filename, Sanitized};
pub use wasm::{validate as validate_wasm, WasmError, WasmInfo};
