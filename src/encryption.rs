//! Encryption at rest for sensitive fields.
//!
//! Uses AES-256-GCM (authenticated encryption) to protect:
//! - Vulnerability report content
//! - Contract source code
//! - Personal data (email, addresses)
//!
//! The DEK (data encryption key) is loaded from the `ENCRYPTION_KEY`
//! environment variable (32-byte hex) or from a configured key-management
//! service. Each encrypted value is self-contained:
//! `base64(nonce || ciphertext || tag)`.

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use anyhow::Context;
use base64::{engine::general_purpose::STANDARD, Engine as _};

// ---------------------------------------------------------------------------
// Key loading
// ---------------------------------------------------------------------------

/// Load the AES-256 encryption key from the environment.
///
/// `ENCRYPTION_KEY` must be exactly 64 hex characters (32 bytes).
pub fn load_key_from_env() -> anyhow::Result<Key<Aes256Gcm>> {
    let hex = std::env::var("ENCRYPTION_KEY")
        .context("ENCRYPTION_KEY environment variable is required")?;
    if hex.len() != 64 {
        anyhow::bail!(
            "ENCRYPTION_KEY must be 64 hex characters (32 bytes), got {}",
            hex.len()
        );
    }
    let bytes = hex::decode(&hex).context("ENCRYPTION_KEY is not valid hex")?;
    Ok(*Key::<Aes256Gcm>::from_slice(&bytes))
}

// ---------------------------------------------------------------------------
// Encrypt / decrypt
// ---------------------------------------------------------------------------

/// Encrypts `plaintext` and returns a portable base64 blob:
/// `base64(12-byte-nonce || ciphertext+16-byte-tag)`.
pub fn encrypt(key: &Key<Aes256Gcm>, plaintext: &[u8]) -> anyhow::Result<String> {
    let cipher = Aes256Gcm::new(key);
    let nonce  = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| anyhow::anyhow!("encryption failed: {}", e))?;

    let mut blob = nonce.to_vec();
    blob.extend_from_slice(&ciphertext);
    Ok(STANDARD.encode(&blob))
}

/// Decrypts a blob produced by `encrypt`.
pub fn decrypt(key: &Key<Aes256Gcm>, blob: &str) -> anyhow::Result<Vec<u8>> {
    let raw = STANDARD.decode(blob).context("invalid base64 in encrypted blob")?;
    if raw.len() < 12 {
        anyhow::bail!("encrypted blob is too short");
    }
    let (nonce_bytes, ciphertext) = raw.split_at(12);
    let cipher = Aes256Gcm::new(key);
    let nonce  = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("decryption failed (wrong key or tampered data): {}", e))
}

// ---------------------------------------------------------------------------
// Typed wrappers for common fields
// ---------------------------------------------------------------------------

/// An encrypted string field stored as base64 ciphertext in the database.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EncryptedField(pub String);

impl EncryptedField {
    pub fn seal(key: &Key<Aes256Gcm>, plaintext: &str) -> anyhow::Result<Self> {
        Ok(Self(encrypt(key, plaintext.as_bytes())?))
    }
    pub fn open(&self, key: &Key<Aes256Gcm>) -> anyhow::Result<String> {
        let bytes = decrypt(key, &self.0)?;
        String::from_utf8(bytes).context("decrypted bytes are not valid UTF-8")
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use aes_gcm::{KeyInit, Key, Aes256Gcm};

    fn test_key() -> Key<Aes256Gcm> {
        let bytes = [0x42u8; 32];
        *Key::<Aes256Gcm>::from_slice(&bytes)
    }

    #[test]
    fn round_trip() {
        let key = test_key();
        let plaintext = b"sensitive contract source code";
        let blob = encrypt(&key, plaintext).unwrap();
        let recovered = decrypt(&key, &blob).unwrap();
        assert_eq!(recovered, plaintext);
    }

    #[test]
    fn wrong_key_fails() {
        let key1 = test_key();
        let key2 = *Key::<Aes256Gcm>::from_slice(&[0x13u8; 32]);
        let blob = encrypt(&key1, b"secret").unwrap();
        assert!(decrypt(&key2, &blob).is_err());
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let key = test_key();
        let mut blob = encrypt(&key, b"secret").unwrap();
        // Flip a character in the middle
        unsafe { blob.as_bytes_mut()[20] ^= 0xFF; }
        assert!(decrypt(&key, &blob).is_err());
    }

    #[test]
    fn encrypted_field_round_trip() {
        let key = test_key();
        let f = EncryptedField::seal(&key, "alice@example.com").unwrap();
        assert_eq!(f.open(&key).unwrap(), "alice@example.com");
    }
}
