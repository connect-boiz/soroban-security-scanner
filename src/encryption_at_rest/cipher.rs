//! AES-256-GCM authenticated encryption primitive.
//!
//! A thin, misuse-resistant wrapper over `ring`'s AEAD: callers supply a
//! 256-bit key, a 96-bit nonce and optional associated data (AAD). GCM provides
//! confidentiality *and* integrity — tampering is detected on decrypt. The
//! 16-byte authentication tag is appended to the ciphertext.

use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use ring::rand::{SecureRandom, SystemRandom};

/// AES-256 key length in bytes.
pub const KEY_LEN: usize = 32;
/// GCM nonce length in bytes.
pub const NONCE_BYTES: usize = NONCE_LEN; // 12
/// GCM authentication tag length in bytes.
pub const TAG_LEN: usize = 16;

/// Errors from the cipher layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    /// The supplied key was not 32 bytes / otherwise invalid.
    BadKey,
    /// Encryption failed.
    EncryptFailed,
    /// Decryption or authentication failed (wrong key, tampering, etc.).
    DecryptFailed,
    /// A random source failure.
    RandomFailure,
}

/// Generates a cryptographically-random 32-byte key.
pub fn generate_key() -> Result<[u8; KEY_LEN], CryptoError> {
    let rng = SystemRandom::new();
    let mut key = [0u8; KEY_LEN];
    rng.fill(&mut key).map_err(|_| CryptoError::RandomFailure)?;
    Ok(key)
}

/// Generates a cryptographically-random 12-byte nonce.
pub fn generate_nonce() -> Result<[u8; NONCE_BYTES], CryptoError> {
    let rng = SystemRandom::new();
    let mut nonce = [0u8; NONCE_BYTES];
    rng.fill(&mut nonce)
        .map_err(|_| CryptoError::RandomFailure)?;
    Ok(nonce)
}

/// Encrypts `plaintext` with AES-256-GCM, returning ciphertext || tag.
pub fn encrypt(
    key: &[u8; KEY_LEN],
    nonce: &[u8; NONCE_BYTES],
    aad: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let unbound = UnboundKey::new(&AES_256_GCM, key).map_err(|_| CryptoError::BadKey)?;
    let sealing = LessSafeKey::new(unbound);
    let nonce = Nonce::assume_unique_for_key(*nonce);

    let mut in_out = plaintext.to_vec();
    sealing
        .seal_in_place_append_tag(nonce, Aad::from(aad), &mut in_out)
        .map_err(|_| CryptoError::EncryptFailed)?;
    Ok(in_out)
}

/// Decrypts `ciphertext` (ciphertext || tag) with AES-256-GCM, verifying the
/// tag and AAD.
pub fn decrypt(
    key: &[u8; KEY_LEN],
    nonce: &[u8; NONCE_BYTES],
    aad: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    if ciphertext.len() < TAG_LEN {
        return Err(CryptoError::DecryptFailed);
    }
    let unbound = UnboundKey::new(&AES_256_GCM, key).map_err(|_| CryptoError::BadKey)?;
    let opening = LessSafeKey::new(unbound);
    let nonce = Nonce::assume_unique_for_key(*nonce);

    let mut in_out = ciphertext.to_vec();
    let plaintext = opening
        .open_in_place(nonce, Aad::from(aad), &mut in_out)
        .map_err(|_| CryptoError::DecryptFailed)?;
    Ok(plaintext.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> [u8; KEY_LEN] {
        // Deterministic key for reproducible round-trips.
        let mut k = [0u8; KEY_LEN];
        for (i, b) in k.iter_mut().enumerate() {
            *b = i as u8;
        }
        k
    }

    #[test]
    fn round_trips() {
        let nonce = [7u8; NONCE_BYTES];
        let ct = encrypt(&key(), &nonce, b"aad", b"secret data").unwrap();
        assert_ne!(ct, b"secret data");
        let pt = decrypt(&key(), &nonce, b"aad", &ct).unwrap();
        assert_eq!(pt, b"secret data");
    }

    #[test]
    fn ciphertext_includes_tag() {
        let nonce = [1u8; NONCE_BYTES];
        let ct = encrypt(&key(), &nonce, b"", b"hi").unwrap();
        // plaintext (2) + tag (16).
        assert_eq!(ct.len(), 2 + TAG_LEN);
    }

    #[test]
    fn wrong_key_fails_authentication() {
        let nonce = [2u8; NONCE_BYTES];
        let ct = encrypt(&key(), &nonce, b"", b"data").unwrap();
        let mut bad = key();
        bad[0] ^= 0xff;
        assert_eq!(
            decrypt(&bad, &nonce, b"", &ct),
            Err(CryptoError::DecryptFailed)
        );
    }

    #[test]
    fn tampering_is_detected() {
        let nonce = [3u8; NONCE_BYTES];
        let mut ct = encrypt(&key(), &nonce, b"", b"data").unwrap();
        ct[0] ^= 0x01; // flip a ciphertext bit
        assert_eq!(
            decrypt(&key(), &nonce, b"", &ct),
            Err(CryptoError::DecryptFailed)
        );
    }

    #[test]
    fn aad_mismatch_fails() {
        let nonce = [4u8; NONCE_BYTES];
        let ct = encrypt(&key(), &nonce, b"context-A", b"data").unwrap();
        assert_eq!(
            decrypt(&key(), &nonce, b"context-B", &ct),
            Err(CryptoError::DecryptFailed)
        );
    }

    #[test]
    fn generated_keys_and_nonces_are_right_size() {
        assert_eq!(generate_key().unwrap().len(), 32);
        assert_eq!(generate_nonce().unwrap().len(), 12);
        // Two generated keys should differ (overwhelmingly likely).
        assert_ne!(generate_key().unwrap(), generate_key().unwrap());
    }

    #[test]
    fn short_ciphertext_is_rejected() {
        assert_eq!(
            decrypt(&key(), &[0u8; NONCE_BYTES], b"", b"short"),
            Err(CryptoError::DecryptFailed)
        );
    }
}
