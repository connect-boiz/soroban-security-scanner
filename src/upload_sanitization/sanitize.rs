//! Content sanitization.
//!
//! Normalizes accepted uploads by stripping elements that are dangerous or
//! useless to the scanner: UTF-8 BOMs, NUL and most control bytes, and a
//! sanitized filename free of path components and traversal sequences. WASM
//! binaries are passed through untouched (their bytes are validated
//! structurally elsewhere); only text uploads are rewritten.

use crate::upload_sanitization::magic::FileKind;
use serde::{Deserialize, Serialize};

/// Outcome of sanitizing an upload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sanitized {
    /// The cleaned bytes.
    pub bytes: Vec<u8>,
    /// Whether any modification was made.
    pub modified: bool,
    /// Notes describing what was changed.
    pub notes: Vec<String>,
}

/// Sanitizes file content according to its detected kind.
pub fn sanitize_content(bytes: &[u8], kind: FileKind) -> Sanitized {
    match kind {
        // Binary WASM is validated structurally; do not mutate its bytes.
        FileKind::Wasm => Sanitized {
            bytes: bytes.to_vec(),
            modified: false,
            notes: vec![],
        },
        _ => sanitize_text(bytes),
    }
}

fn sanitize_text(bytes: &[u8]) -> Sanitized {
    let mut notes = Vec::new();
    let mut out = bytes.to_vec();

    // Strip a UTF-8 BOM.
    if out.starts_with(&[0xEF, 0xBB, 0xBF]) {
        out.drain(0..3);
        notes.push("removed UTF-8 BOM".to_string());
    }

    // Remove NUL and control bytes except tab (0x09), LF (0x0a), CR (0x0d).
    let before = out.len();
    out.retain(|b| *b >= 0x20 || *b == 0x09 || *b == 0x0a || *b == 0x0d);
    if out.len() != before {
        notes.push(format!("removed {} control byte(s)", before - out.len()));
    }

    // Normalize CRLF / lone CR to LF.
    if out.contains(&b'\r') {
        out = normalize_newlines(&out);
        notes.push("normalized line endings to LF".to_string());
    }

    let modified = !notes.is_empty();
    Sanitized {
        bytes: out,
        modified,
        notes,
    }
}

fn normalize_newlines(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\r' => {
                out.push(b'\n');
                // Skip a following LF (CRLF → single LF).
                if bytes.get(i + 1) == Some(&b'\n') {
                    i += 1;
                }
            }
            other => out.push(other),
        }
        i += 1;
    }
    out
}

/// Sanitizes a client-supplied filename: strips any directory components and
/// traversal sequences, keeping only a safe base name. Returns a fallback when
/// nothing safe remains.
pub fn sanitize_filename(name: &str) -> String {
    // Take the last path component under either separator.
    let base = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .replace("..", "");
    let cleaned: String = base
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect();
    let trimmed = cleaned.trim_matches('.').to_string();
    if trimmed.is_empty() {
        "upload.bin".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_is_untouched() {
        let wasm = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        let s = sanitize_content(&wasm, FileKind::Wasm);
        assert!(!s.modified);
        assert_eq!(s.bytes, wasm);
    }

    #[test]
    fn strips_bom_and_control_bytes() {
        let mut data = vec![0xEF, 0xBB, 0xBF];
        data.extend_from_slice(b"fn main() {}\x00\x07");
        let s = sanitize_content(&data, FileKind::SourceText);
        assert!(s.modified);
        assert!(!s.bytes.contains(&0));
        assert!(s.bytes.starts_with(b"fn main"));
    }

    #[test]
    fn normalizes_newlines() {
        let s = sanitize_content(b"a\r\nb\rc", FileKind::SourceText);
        assert_eq!(s.bytes, b"a\nb\nc");
        assert!(s.notes.iter().any(|n| n.contains("line endings")));
    }

    #[test]
    fn clean_text_is_unmodified() {
        let s = sanitize_content(b"fn main() {}\n", FileKind::SourceText);
        assert!(!s.modified);
    }

    #[test]
    fn filename_strips_paths_and_traversal() {
        assert_eq!(sanitize_filename("../../etc/passwd"), "passwd");
        assert_eq!(
            sanitize_filename("..\\..\\windows\\system32\\cmd.exe"),
            "cmd.exe"
        );
        assert_eq!(sanitize_filename("contract.wasm"), "contract.wasm");
        assert_eq!(sanitize_filename("..."), "upload.bin");
    }
}
