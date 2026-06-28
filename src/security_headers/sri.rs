//! Subresource Integrity (SRI).
//!
//! Computes the `integrity` attribute value for external scripts/styles so the
//! browser refuses to execute a resource whose bytes have been tampered with.
//! Supports SHA-256/384/512 (SHA-384 is the recommended default).

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha384, Sha512};

/// SRI hash algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SriAlgorithm {
    /// SHA-256.
    Sha256,
    /// SHA-384 (recommended).
    Sha384,
    /// SHA-512.
    Sha512,
}

impl SriAlgorithm {
    /// The `integrity` prefix token.
    pub fn prefix(&self) -> &'static str {
        match self {
            SriAlgorithm::Sha256 => "sha256",
            SriAlgorithm::Sha384 => "sha384",
            SriAlgorithm::Sha512 => "sha512",
        }
    }
}

/// Computes the SRI `integrity` value (e.g. `sha384-<base64>`) for `content`.
pub fn integrity(content: &[u8], algo: SriAlgorithm) -> String {
    let digest = match algo {
        SriAlgorithm::Sha256 => {
            let mut h = Sha256::new();
            h.update(content);
            B64.encode(h.finalize())
        }
        SriAlgorithm::Sha384 => {
            let mut h = Sha384::new();
            h.update(content);
            B64.encode(h.finalize())
        }
        SriAlgorithm::Sha512 => {
            let mut h = Sha512::new();
            h.update(content);
            B64.encode(h.finalize())
        }
    };
    format!("{}-{}", algo.prefix(), digest)
}

/// Verifies that `content` matches a previously-computed `integrity` value.
pub fn verify(content: &[u8], integrity_value: &str) -> bool {
    let algo = match integrity_value.split('-').next() {
        Some("sha256") => SriAlgorithm::Sha256,
        Some("sha384") => SriAlgorithm::Sha384,
        Some("sha512") => SriAlgorithm::Sha512,
        _ => return false,
    };
    // Constant-time-ish compare of the full token.
    let expected = integrity(content, algo);
    constant_time_eq(expected.as_bytes(), integrity_value.as_bytes())
}

/// Renders a `<script>` tag with the integrity + crossorigin attributes.
pub fn script_tag(src: &str, content: &[u8], algo: SriAlgorithm) -> String {
    format!(
        r#"<script src="{}" integrity="{}" crossorigin="anonymous"></script>"#,
        src,
        integrity(content, algo)
    )
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrity_has_algorithm_prefix() {
        let v = integrity(b"console.log(1)", SriAlgorithm::Sha384);
        assert!(v.starts_with("sha384-"));
    }

    #[test]
    fn verify_round_trips() {
        let content = b"alert('hi')";
        let v = integrity(content, SriAlgorithm::Sha256);
        assert!(verify(content, &v));
    }

    #[test]
    fn tampered_content_fails_verification() {
        let v = integrity(b"original", SriAlgorithm::Sha384);
        assert!(!verify(b"tampered", &v));
    }

    #[test]
    fn unknown_algorithm_rejected() {
        assert!(!verify(b"x", "md5-abcdef"));
    }

    #[test]
    fn script_tag_includes_integrity_and_crossorigin() {
        let tag = script_tag("https://cdn.x/app.js", b"code", SriAlgorithm::Sha384);
        assert!(tag.contains(r#"integrity="sha384-"#));
        assert!(tag.contains(r#"crossorigin="anonymous""#));
    }

    #[test]
    fn known_sha256_vector() {
        // SHA-256 of "" is base64 47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=
        assert_eq!(
            integrity(b"", SriAlgorithm::Sha256),
            "sha256-47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU="
        );
    }
}
