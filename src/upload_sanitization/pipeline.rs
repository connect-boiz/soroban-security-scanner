//! The end-to-end upload validation pipeline.
//!
//! Runs every stage in order and produces a single [`UploadVerdict`]:
//! size → magic sniff → content-type consistency → WASM structure →
//! deep inspection → malware scan → sanitize. The first hard failure rejects;
//! suspicious-but-not-malicious findings route to quarantine; otherwise the
//! sanitized bytes are accepted.

use crate::upload_sanitization::content_type::{check as check_content_type, ContentTypeCheck};
use crate::upload_sanitization::deep_inspection::{inspect, Severity};
use crate::upload_sanitization::limits::{SizeError, UploadTier};
use crate::upload_sanitization::magic::{detect, FileKind};
use crate::upload_sanitization::malware::{MalwareScanner, ScanVerdict};
use crate::upload_sanitization::sanitize::{sanitize_content, sanitize_filename};
use crate::upload_sanitization::wasm::validate as validate_wasm;
use serde::{Deserialize, Serialize};

/// Inputs describing one upload to validate.
#[derive(Debug, Clone)]
pub struct UploadRequest<'a> {
    /// Raw file bytes.
    pub bytes: &'a [u8],
    /// Client-declared MIME type.
    pub declared_content_type: &'a str,
    /// Client-supplied filename (untrusted).
    pub filename: &'a str,
    /// Uploader's tier (governs size limit).
    pub tier: UploadTier,
}

/// The terminal decision for an upload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UploadVerdict {
    /// Accepted; carries the sanitized bytes and a safe filename.
    Accepted {
        /// Cleaned file bytes.
        sanitized_bytes: Vec<u8>,
        /// Sanitized filename.
        filename: String,
        /// Detected kind.
        kind: FileKind,
    },
    /// Held for manual review.
    Quarantined {
        /// Why it was quarantined.
        reason: String,
        /// Safe filename for the quarantine record.
        filename: String,
    },
    /// Rejected outright.
    Rejected {
        /// Machine-readable stage/cause.
        stage: String,
        /// Human-readable reason.
        reason: String,
    },
}

impl UploadVerdict {
    /// Whether the upload was accepted.
    pub fn is_accepted(&self) -> bool {
        matches!(self, UploadVerdict::Accepted { .. })
    }

    fn reject(stage: &str, reason: impl Into<String>) -> Self {
        UploadVerdict::Rejected {
            stage: stage.to_string(),
            reason: reason.into(),
        }
    }
}

/// Validates an upload through the full pipeline.
pub fn validate_upload(req: &UploadRequest, scanner: &dyn MalwareScanner) -> UploadVerdict {
    let safe_name = sanitize_filename(req.filename);

    // 1. Size / tier limit.
    match req.tier.check_size(req.bytes.len() as u64) {
        Ok(()) => {}
        Err(SizeError::Empty) => return UploadVerdict::reject("size", "upload is empty"),
        Err(SizeError::TooLarge { size, max }) => {
            return UploadVerdict::reject(
                "size",
                format!("{size} bytes exceeds tier limit of {max} bytes"),
            )
        }
    }

    // 2. Magic-number sniff.
    let kind = detect(req.bytes);
    if !kind.is_uploadable() {
        return UploadVerdict::reject("magic", format!("disallowed file kind: {}", kind.as_str()));
    }

    // 3. Declared content-type must be allowlisted and consistent.
    match check_content_type(req.declared_content_type, kind) {
        ContentTypeCheck::Ok(_) => {}
        ContentTypeCheck::NotAllowed => {
            return UploadVerdict::reject(
                "content-type",
                format!("content type '{}' is not allowed", req.declared_content_type),
            )
        }
        ContentTypeCheck::Mismatch { detected, expected } => {
            return UploadVerdict::reject(
                "content-type",
                format!(
                    "declared type implies {} but content is {}",
                    expected.as_str(),
                    detected.as_str()
                ),
            )
        }
    }

    // 4. WASM structural validation.
    if kind == FileKind::Wasm {
        if let Err(e) = validate_wasm(req.bytes) {
            return UploadVerdict::reject("wasm-structure", format!("malformed WASM: {e:?}"));
        }
    }

    // 5. Deep inspection.
    let report = inspect(req.bytes);
    if report.has_malicious() {
        let rules: Vec<_> = report.findings.iter().map(|f| f.rule.as_str()).collect();
        return UploadVerdict::reject(
            "deep-inspection",
            format!("malicious pattern(s): {}", rules.join(", ")),
        );
    }

    // 6. Malware scan.
    match scanner.scan(req.bytes) {
        ScanVerdict::Clean => {}
        ScanVerdict::Infected { signature } => {
            return UploadVerdict::reject("malware", format!("infected: {signature}"))
        }
        ScanVerdict::Indeterminate { reason } => {
            return UploadVerdict::Quarantined {
                reason: format!("malware scan inconclusive: {reason}"),
                filename: safe_name,
            }
        }
    }

    // 7. Suspicious (non-malicious) findings → quarantine.
    if report.max_severity() == Some(Severity::Suspicious) {
        let rules: Vec<_> = report.findings.iter().map(|f| f.rule.as_str()).collect();
        return UploadVerdict::Quarantined {
            reason: format!("suspicious pattern(s): {}", rules.join(", ")),
            filename: safe_name,
        };
    }

    // 8. Accept with sanitized content.
    let sanitized = sanitize_content(req.bytes, kind);
    UploadVerdict::Accepted {
        sanitized_bytes: sanitized.bytes,
        filename: safe_name,
        kind,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::upload_sanitization::malware::SignatureScanner;
    use crate::upload_sanitization::wasm::SUPPORTED_VERSION;

    fn scanner() -> SignatureScanner {
        SignatureScanner::new()
    }

    fn valid_wasm() -> Vec<u8> {
        let mut m = vec![0x00, 0x61, 0x73, 0x6d];
        m.extend_from_slice(&SUPPORTED_VERSION.to_le_bytes());
        m.push(1); // type section
        m.push(1); // length 1
        m.push(0); // payload
        m
    }

    #[test]
    fn accepts_clean_rust_source() {
        let req = UploadRequest {
            bytes: b"pub fn main() {}\n",
            declared_content_type: "text/x-rust",
            filename: "lib.rs",
            tier: UploadTier::Free,
        };
        let verdict = validate_upload(&req, &scanner());
        assert!(verdict.is_accepted());
    }

    #[test]
    fn accepts_valid_wasm() {
        let wasm = valid_wasm();
        let req = UploadRequest {
            bytes: &wasm,
            declared_content_type: "application/wasm",
            filename: "contract.wasm",
            tier: UploadTier::Free,
        };
        assert!(validate_upload(&req, &scanner()).is_accepted());
    }

    #[test]
    fn rejects_oversized_for_tier() {
        let big = vec![b'a'; 2 * 1024 * 1024];
        let req = UploadRequest {
            bytes: &big,
            declared_content_type: "text/plain",
            filename: "big.rs",
            tier: UploadTier::Free,
        };
        match validate_upload(&req, &scanner()) {
            UploadVerdict::Rejected { stage, .. } => assert_eq!(stage, "size"),
            other => panic!("expected size rejection, got {other:?}"),
        }
    }

    #[test]
    fn rejects_disguised_executable() {
        // ELF bytes declared as wasm.
        let req = UploadRequest {
            bytes: b"\x7fELF\x02\x01\x01\x00rest of binary",
            declared_content_type: "application/wasm",
            filename: "contract.wasm",
            tier: UploadTier::Free,
        };
        match validate_upload(&req, &scanner()) {
            UploadVerdict::Rejected { stage, .. } => assert_eq!(stage, "magic"),
            other => panic!("expected magic rejection, got {other:?}"),
        }
    }

    #[test]
    fn rejects_content_type_mismatch() {
        // Real Rust source but declared as wasm.
        let req = UploadRequest {
            bytes: b"pub fn main() {}\n",
            declared_content_type: "application/wasm",
            filename: "lib.rs",
            tier: UploadTier::Free,
        };
        match validate_upload(&req, &scanner()) {
            UploadVerdict::Rejected { stage, .. } => assert_eq!(stage, "content-type"),
            other => panic!("expected content-type rejection, got {other:?}"),
        }
    }

    #[test]
    fn rejects_malformed_wasm() {
        let mut wasm = vec![0x00, 0x61, 0x73, 0x6d];
        wasm.extend_from_slice(&SUPPORTED_VERSION.to_le_bytes());
        wasm.push(1); // section id
        wasm.push(200); // claims 200 bytes that aren't there
        let req = UploadRequest {
            bytes: &wasm,
            declared_content_type: "application/wasm",
            filename: "contract.wasm",
            tier: UploadTier::Free,
        };
        match validate_upload(&req, &scanner()) {
            UploadVerdict::Rejected { stage, .. } => assert_eq!(stage, "wasm-structure"),
            other => panic!("expected wasm-structure rejection, got {other:?}"),
        }
    }

    #[test]
    fn rejects_malicious_pattern() {
        let req = UploadRequest {
            bytes: b"fn x() { /* rm -rf / */ }",
            declared_content_type: "text/x-rust",
            filename: "evil.rs",
            tier: UploadTier::Free,
        };
        match validate_upload(&req, &scanner()) {
            UploadVerdict::Rejected { stage, .. } => assert_eq!(stage, "deep-inspection"),
            other => panic!("expected deep-inspection rejection, got {other:?}"),
        }
    }

    #[test]
    fn quarantines_suspicious_pattern() {
        let req = UploadRequest {
            bytes: b"let p = include_str!(\"../../secret.toml\");",
            declared_content_type: "text/x-rust",
            filename: "sneaky.rs",
            tier: UploadTier::Free,
        };
        match validate_upload(&req, &scanner()) {
            UploadVerdict::Quarantined { .. } => {}
            other => panic!("expected quarantine, got {other:?}"),
        }
    }

    #[test]
    fn detects_eicar_via_scanner() {
        use crate::upload_sanitization::malware::EICAR_TEST;
        let req = UploadRequest {
            bytes: EICAR_TEST,
            declared_content_type: "text/plain",
            filename: "test.txt",
            tier: UploadTier::Free,
        };
        match validate_upload(&req, &scanner()) {
            UploadVerdict::Rejected { stage, .. } => assert_eq!(stage, "malware"),
            other => panic!("expected malware rejection, got {other:?}"),
        }
    }

    #[test]
    fn accepted_filename_is_sanitized() {
        let req = UploadRequest {
            bytes: b"pub fn main() {}\n",
            declared_content_type: "text/x-rust",
            filename: "../../etc/lib.rs",
            tier: UploadTier::Free,
        };
        if let UploadVerdict::Accepted { filename, .. } = validate_upload(&req, &scanner()) {
            assert_eq!(filename, "lib.rs");
        } else {
            panic!("expected acceptance");
        }
    }
}
