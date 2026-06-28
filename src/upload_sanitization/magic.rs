//! Magic-number / content sniffing for uploaded files.
//!
//! Determines the *actual* type of a file from its bytes rather than trusting
//! the client-declared content type or extension. This is the first line of
//! defence against an attacker uploading an executable disguised as a `.wasm`
//! or `.rs` file.

use serde::{Deserialize, Serialize};

/// The detected kind of an uploaded file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileKind {
    /// WebAssembly binary (`\0asm` magic).
    Wasm,
    /// Plausible UTF-8 source text (e.g. Rust source / TOML manifest).
    SourceText,
    /// ELF executable (Linux binary) — disallowed.
    Elf,
    /// PE/COFF executable (`MZ`, Windows) — disallowed.
    PeExecutable,
    /// Mach-O executable (macOS) — disallowed.
    MachO,
    /// Shell script / shebang — disallowed.
    Script,
    /// Archive (ZIP/gzip/etc.) — disallowed (could smuggle content).
    Archive,
    /// Unknown / unrecognized binary content.
    Unknown,
}

impl FileKind {
    /// Whether this kind is ever acceptable as a contract-code upload.
    pub fn is_uploadable(&self) -> bool {
        matches!(self, FileKind::Wasm | FileKind::SourceText)
    }

    /// Stable label.
    pub fn as_str(&self) -> &'static str {
        match self {
            FileKind::Wasm => "wasm",
            FileKind::SourceText => "source-text",
            FileKind::Elf => "elf-executable",
            FileKind::PeExecutable => "pe-executable",
            FileKind::MachO => "mach-o-executable",
            FileKind::Script => "script",
            FileKind::Archive => "archive",
            FileKind::Unknown => "unknown",
        }
    }
}

/// The WebAssembly magic header: `\0asm` followed by a u32 version.
pub const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6d];

/// Detects the file kind from a content prefix.
pub fn detect(bytes: &[u8]) -> FileKind {
    if bytes.starts_with(&WASM_MAGIC) {
        return FileKind::Wasm;
    }
    if bytes.starts_with(b"\x7fELF") {
        return FileKind::Elf;
    }
    if bytes.starts_with(b"MZ") {
        return FileKind::PeExecutable;
    }
    // Mach-O (32/64-bit, both endiannesses) and universal binaries.
    if matches!(
        bytes.get(0..4),
        Some([0xfe, 0xed, 0xfa, 0xce])
            | Some([0xfe, 0xed, 0xfa, 0xcf])
            | Some([0xcf, 0xfa, 0xed, 0xfe])
            | Some([0xce, 0xfa, 0xed, 0xfe])
            | Some([0xca, 0xfe, 0xba, 0xbe])
    ) {
        return FileKind::MachO;
    }
    if bytes.starts_with(b"#!") {
        return FileKind::Script;
    }
    if bytes.starts_with(b"PK\x03\x04") // zip
        || bytes.starts_with(&[0x1f, 0x8b]) // gzip
        || bytes.starts_with(b"7z\xbc\xaf") // 7z
        || bytes.starts_with(b"Rar!")
    {
        return FileKind::Archive;
    }
    if is_probably_text(bytes) {
        return FileKind::SourceText;
    }
    FileKind::Unknown
}

/// Heuristic: a buffer is "text" if it is valid UTF-8 (over the inspected
/// prefix) and contains no NUL bytes or excessive control characters.
pub fn is_probably_text(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    // NUL bytes never appear in source text.
    if bytes.contains(&0) {
        return false;
    }
    // Inspect a bounded prefix for control-character density.
    let prefix = &bytes[..bytes.len().min(8192)];
    if std::str::from_utf8(prefix).is_err() {
        // Allow a trailing truncated multi-byte char only on the full buffer.
        if std::str::from_utf8(bytes).is_err() {
            return false;
        }
    }
    let control = prefix
        .iter()
        .filter(|b| **b < 0x09 || (**b > 0x0d && **b < 0x20))
        .count();
    // Fewer than ~1% control bytes.
    control * 100 <= prefix.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_wasm() {
        let mut wasm = WASM_MAGIC.to_vec();
        wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // version 1
        assert_eq!(detect(&wasm), FileKind::Wasm);
        assert!(detect(&wasm).is_uploadable());
    }

    #[test]
    fn detects_rust_source() {
        let src = b"pub fn main() {\n    println!(\"hi\");\n}\n";
        assert_eq!(detect(src), FileKind::SourceText);
        assert!(detect(src).is_uploadable());
    }

    #[test]
    fn detects_executables() {
        assert_eq!(detect(b"\x7fELF\x02\x01\x01\x00"), FileKind::Elf);
        assert_eq!(detect(b"MZ\x90\x00"), FileKind::PeExecutable);
        assert_eq!(detect(&[0xfe, 0xed, 0xfa, 0xcf, 0, 0]), FileKind::MachO);
        for kind in [FileKind::Elf, FileKind::PeExecutable, FileKind::MachO] {
            assert!(!kind.is_uploadable());
        }
    }

    #[test]
    fn detects_scripts_and_archives() {
        assert_eq!(detect(b"#!/bin/bash\nrm -rf /\n"), FileKind::Script);
        assert_eq!(detect(b"PK\x03\x04rest"), FileKind::Archive);
        assert_eq!(detect(&[0x1f, 0x8b, 0x08]), FileKind::Archive);
    }

    #[test]
    fn binary_with_nul_is_not_text() {
        assert!(!is_probably_text(b"abc\0def"));
        assert_eq!(detect(b"abc\0\0\0\xff\xfe"), FileKind::Unknown);
    }

    #[test]
    fn empty_is_unknown() {
        assert_eq!(detect(b""), FileKind::Unknown);
    }
}
