//! Structural validation of WebAssembly modules.
//!
//! Verifies the 8-byte preamble (magic + version) and walks the section
//! framing (id + LEB128 length) to ensure the module is well-formed and free
//! of malformed/oversized section declarations that could exploit a downstream
//! parser. This is a structural sanity check, not a full Wasm validator.

use crate::upload_sanitization::magic::WASM_MAGIC;
use serde::{Deserialize, Serialize};

/// The only WebAssembly binary format version currently accepted.
pub const SUPPORTED_VERSION: u32 = 1;
/// Highest known section id (12 = data count, Wasm MVP + bulk-memory).
const MAX_KNOWN_SECTION_ID: u8 = 12;

/// Summary of a structurally validated module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmInfo {
    /// Binary-format version from the preamble.
    pub version: u32,
    /// Section ids encountered, in order.
    pub section_ids: Vec<u8>,
}

/// Why a WASM module failed structural validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmError {
    /// Shorter than the 8-byte preamble.
    TooShort,
    /// Missing `\0asm` magic.
    BadMagic,
    /// Unsupported binary-format version.
    UnsupportedVersion(u32),
    /// A section's declared length runs past the end of the file.
    SectionOverrun {
        /// The offending section id.
        section_id: u8,
    },
    /// A section uses an unknown id (possible parser-exploitation attempt).
    UnknownSection(u8),
    /// A LEB128 length field was malformed (overlong / truncated).
    BadLeb128,
}

/// Structurally validates a WebAssembly module.
pub fn validate(bytes: &[u8]) -> Result<WasmInfo, WasmError> {
    if bytes.len() < 8 {
        return Err(WasmError::TooShort);
    }
    if bytes[0..4] != WASM_MAGIC {
        return Err(WasmError::BadMagic);
    }
    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if version != SUPPORTED_VERSION {
        return Err(WasmError::UnsupportedVersion(version));
    }

    let mut section_ids = Vec::new();
    let mut offset = 8;
    while offset < bytes.len() {
        let section_id = bytes[offset];
        offset += 1;

        let (len, consumed) = read_leb128_u32(&bytes[offset..]).ok_or(WasmError::BadLeb128)?;
        offset += consumed;

        if section_id > MAX_KNOWN_SECTION_ID {
            return Err(WasmError::UnknownSection(section_id));
        }

        let end = offset
            .checked_add(len as usize)
            .ok_or(WasmError::SectionOverrun { section_id })?;
        if end > bytes.len() {
            return Err(WasmError::SectionOverrun { section_id });
        }
        section_ids.push(section_id);
        offset = end;
    }

    Ok(WasmInfo {
        version,
        section_ids,
    })
}

/// Reads an unsigned LEB128 u32, returning `(value, bytes_consumed)`.
fn read_leb128_u32(bytes: &[u8]) -> Option<(u32, usize)> {
    let mut result: u32 = 0;
    let mut shift = 0;
    for (i, &byte) in bytes.iter().enumerate() {
        // u32 needs at most 5 LEB128 bytes.
        if i >= 5 {
            return None;
        }
        let low = (byte & 0x7f) as u32;
        result |= low.checked_shl(shift)?;
        if byte & 0x80 == 0 {
            return Some((result, i + 1));
        }
        shift += 7;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a minimal valid module: preamble + one type section (id 1).
    fn minimal_module() -> Vec<u8> {
        let mut m = WASM_MAGIC.to_vec();
        m.extend_from_slice(&SUPPORTED_VERSION.to_le_bytes());
        m.push(1); // section id: type
        m.push(2); // section length: 2 bytes
        m.extend_from_slice(&[0x00, 0x00]); // payload
        m
    }

    #[test]
    fn valid_module_passes() {
        let info = validate(&minimal_module()).unwrap();
        assert_eq!(info.version, 1);
        assert_eq!(info.section_ids, vec![1]);
    }

    #[test]
    fn rejects_short_and_bad_magic() {
        assert_eq!(validate(b"\0asm").unwrap_err(), WasmError::TooShort);
        let mut bad = vec![0xde, 0xad, 0xbe, 0xef];
        bad.extend_from_slice(&[1, 0, 0, 0]);
        assert_eq!(validate(&bad).unwrap_err(), WasmError::BadMagic);
    }

    #[test]
    fn rejects_unsupported_version() {
        let mut m = WASM_MAGIC.to_vec();
        m.extend_from_slice(&2u32.to_le_bytes());
        assert_eq!(validate(&m).unwrap_err(), WasmError::UnsupportedVersion(2));
    }

    #[test]
    fn rejects_section_overrun() {
        let mut m = WASM_MAGIC.to_vec();
        m.extend_from_slice(&SUPPORTED_VERSION.to_le_bytes());
        m.push(1); // type section
        m.push(50); // claims 50 bytes but none follow
        assert_eq!(
            validate(&m).unwrap_err(),
            WasmError::SectionOverrun { section_id: 1 }
        );
    }

    #[test]
    fn rejects_unknown_section() {
        let mut m = WASM_MAGIC.to_vec();
        m.extend_from_slice(&SUPPORTED_VERSION.to_le_bytes());
        m.push(99); // unknown section id
        m.push(0);
        assert_eq!(validate(&m).unwrap_err(), WasmError::UnknownSection(99));
    }

    #[test]
    fn rejects_malformed_leb128() {
        let mut m = WASM_MAGIC.to_vec();
        m.extend_from_slice(&SUPPORTED_VERSION.to_le_bytes());
        m.push(1);
        // Five continuation bytes with no terminator → overlong.
        m.extend_from_slice(&[0x80, 0x80, 0x80, 0x80, 0x80]);
        assert_eq!(validate(&m).unwrap_err(), WasmError::BadLeb128);
    }

    #[test]
    fn leb128_multibyte_length() {
        // Section length 130 encodes as 0x82 0x01 in LEB128.
        let mut m = WASM_MAGIC.to_vec();
        m.extend_from_slice(&SUPPORTED_VERSION.to_le_bytes());
        m.push(1);
        m.extend_from_slice(&[0x82, 0x01]);
        m.extend_from_slice(&vec![0u8; 130]);
        let info = validate(&m).unwrap();
        assert_eq!(info.section_ids, vec![1]);
    }
}
