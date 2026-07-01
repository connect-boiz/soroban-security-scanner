//! Minimal RFC 4648 base32 codec (no padding) for TOTP shared secrets.
//!
//! Authenticator apps (Google Authenticator, Authy, 1Password, …) expect the
//! shared secret in `otpauth://` URIs to be base32-encoded, so we need a small
//! dependency-free codec rather than the base64 already in the tree.

const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// Encodes bytes as uppercase base32 without padding.
pub fn encode(data: &[u8]) -> String {
    let mut out = String::new();
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;
    for &byte in data {
        buffer = (buffer << 8) | byte as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            let idx = ((buffer >> bits) & 0x1f) as usize;
            out.push(ALPHABET[idx] as char);
        }
    }
    if bits > 0 {
        let idx = ((buffer << (5 - bits)) & 0x1f) as usize;
        out.push(ALPHABET[idx] as char);
    }
    out
}

/// Decodes a base32 string, ignoring case, padding (`=`) and spaces.
///
/// Returns `None` if a non-alphabet character is encountered.
pub fn decode(input: &str) -> Option<Vec<u8>> {
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;
    let mut out = Vec::new();
    for c in input.chars() {
        if c == '=' || c == ' ' {
            continue;
        }
        let val = match c.to_ascii_uppercase() {
            'A'..='Z' => c.to_ascii_uppercase() as u32 - 'A' as u32,
            '2'..='7' => c as u32 - '2' as u32 + 26,
            _ => return None,
        };
        buffer = (buffer << 5) | val;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        for case in [&b""[..], b"f", b"fo", b"foo", b"foob", b"fooba", b"foobar"] {
            let encoded = encode(case);
            assert_eq!(decode(&encoded).unwrap(), case);
        }
    }

    #[test]
    fn known_vector() {
        // RFC 4648: "foobar" -> "MZXW6YTBOI"
        assert_eq!(encode(b"foobar"), "MZXW6YTBOI");
        assert_eq!(decode("MZXW6YTBOI").unwrap(), b"foobar");
    }

    #[test]
    fn tolerates_lowercase_spaces_padding() {
        assert_eq!(decode("mzxw 6ytb oi==").unwrap(), b"foobar");
    }

    #[test]
    fn rejects_invalid_chars() {
        assert!(decode("0189!").is_none());
    }
}
