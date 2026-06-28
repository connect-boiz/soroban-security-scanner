//! Field-level encryption envelope.
//!
//! Encrypts an individual database field into a self-describing [`EncryptedField`]
//! that records which key version produced it (so rotation works) plus the
//! nonce and ciphertext. The envelope serializes to a compact base64 string for
//! storage in a single text/blob column.

use crate::encryption_at_rest::cipher::{decrypt, encrypt, CryptoError, KEY_LEN, NONCE_BYTES};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde::{Deserialize, Serialize};

/// An encrypted field value with the metadata needed to decrypt it later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedField {
    /// Key version used (resolved against the key manager on decrypt).
    pub key_id: u32,
    /// Per-value nonce.
    pub nonce: [u8; NONCE_BYTES],
    /// Ciphertext || GCM tag.
    pub ciphertext: Vec<u8>,
}

impl EncryptedField {
    /// Encrypts `plaintext` with `key` (tagged as `key_id`). `aad` binds the
    /// ciphertext to a context (e.g. `"users.api_key:42"`) so a value cannot be
    /// transplanted to another row/column.
    pub fn seal(
        key_id: u32,
        key: &[u8; KEY_LEN],
        nonce: [u8; NONCE_BYTES],
        aad: &[u8],
        plaintext: &[u8],
    ) -> Result<Self, CryptoError> {
        let ciphertext = encrypt(key, &nonce, aad, plaintext)?;
        Ok(Self {
            key_id,
            nonce,
            ciphertext,
        })
    }

    /// Decrypts the field with the resolved `key` for `self.key_id`.
    pub fn open(&self, key: &[u8; KEY_LEN], aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
        decrypt(key, &self.nonce, aad, &self.ciphertext)
    }

    /// Serializes to a base64 string: `v1:<base64(key_id || nonce || ct)>`.
    pub fn to_storage_string(&self) -> String {
        let mut buf = Vec::with_capacity(4 + NONCE_BYTES + self.ciphertext.len());
        buf.extend_from_slice(&self.key_id.to_be_bytes());
        buf.extend_from_slice(&self.nonce);
        buf.extend_from_slice(&self.ciphertext);
        format!("v1:{}", B64.encode(buf))
    }

    /// Parses a value produced by [`to_storage_string`](Self::to_storage_string).
    pub fn from_storage_string(s: &str) -> Option<Self> {
        let b64 = s.strip_prefix("v1:")?;
        let buf = B64.decode(b64).ok()?;
        if buf.len() < 4 + NONCE_BYTES {
            return None;
        }
        let key_id = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let mut nonce = [0u8; NONCE_BYTES];
        nonce.copy_from_slice(&buf[4..4 + NONCE_BYTES]);
        let ciphertext = buf[4 + NONCE_BYTES..].to_vec();
        Some(Self {
            key_id,
            nonce,
            ciphertext,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encryption_at_rest::cipher::generate_nonce;

    fn key() -> [u8; KEY_LEN] {
        [42u8; KEY_LEN]
    }

    #[test]
    fn seal_open_round_trip() {
        let nonce = generate_nonce().unwrap();
        let field =
            EncryptedField::seal(1, &key(), nonce, b"users.api_key:42", b"sk-secret").unwrap();
        let pt = field.open(&key(), b"users.api_key:42").unwrap();
        assert_eq!(pt, b"sk-secret");
    }

    #[test]
    fn storage_string_round_trips() {
        let nonce = [9u8; NONCE_BYTES];
        let field = EncryptedField::seal(7, &key(), nonce, b"ctx", b"private-key-bytes").unwrap();
        let s = field.to_storage_string();
        assert!(s.starts_with("v1:"));
        let parsed = EncryptedField::from_storage_string(&s).unwrap();
        assert_eq!(parsed, field);
        assert_eq!(parsed.key_id, 7);
        assert_eq!(parsed.open(&key(), b"ctx").unwrap(), b"private-key-bytes");
    }

    #[test]
    fn ciphertext_is_not_plaintext() {
        let nonce = [1u8; NONCE_BYTES];
        let field = EncryptedField::seal(1, &key(), nonce, b"", b"plaintext-value").unwrap();
        let s = field.to_storage_string();
        assert!(!s.contains("plaintext-value"));
    }

    #[test]
    fn wrong_context_fails_to_open() {
        let nonce = [2u8; NONCE_BYTES];
        let field = EncryptedField::seal(1, &key(), nonce, b"row:1", b"v").unwrap();
        assert!(field.open(&key(), b"row:2").is_err());
    }

    #[test]
    fn malformed_storage_string_is_rejected() {
        assert!(EncryptedField::from_storage_string("nope").is_none());
        assert!(EncryptedField::from_storage_string("v1:!!!notbase64").is_none());
        assert!(EncryptedField::from_storage_string("v1:AAAA").is_none()); // too short
    }
}
