//! Wallet Cryptography Utilities
//!
//! Handles key generation, encryption/decryption of secret seeds,
//! and integrity verification for wallet export bundles.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use ring::{
    aead::{
        self, Aad, BoundKey, Nonce, NonceSequence, OpeningKey, SealingKey, UnboundKey, AES_256_GCM,
    },
    error::Unspecified,
    hmac, pbkdf2,
};
use std::num::NonZeroU32;

use crate::wallet::types::{WalletError, WalletExport};

const PBKDF2_ITERATIONS: u32 = 100_000;
const SALT_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32; // AES-256

/// Single-use nonce wrapper for ring's AEAD API
struct SingleNonce([u8; NONCE_LEN]);

impl NonceSequence for SingleNonce {
    fn advance(&mut self) -> Result<Nonce, Unspecified> {
        Ok(Nonce::assume_unique_for_key(self.0))
    }
}

/// Derive a 256-bit key from a password using PBKDF2-HMAC-SHA256
fn derive_key(password: &str, salt: &[u8], iterations: u32) -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        NonZeroU32::new(iterations).expect("iterations must be > 0"),
        salt,
        password.as_bytes(),
        &mut key,
    );
    key
}

/// Encrypt a secret seed with a user-supplied password.
/// Returns (encrypted_seed_b64, nonce_b64, salt_b64).
pub fn encrypt_seed(
    secret_seed: &str,
    password: &str,
) -> Result<(String, String, String), WalletError> {
    let mut rng = rand::thread_rng();

    let mut salt = [0u8; SALT_LEN];
    rng.fill_bytes(&mut salt);

    let mut nonce_bytes = [0u8; NONCE_LEN];
    rng.fill_bytes(&mut nonce_bytes);

    let key_bytes = derive_key(password, &salt, PBKDF2_ITERATIONS);

    let unbound = UnboundKey::new(&AES_256_GCM, &key_bytes)
        .map_err(|_| WalletError::EncryptionError("key creation failed".into()))?;
    let mut sealing = SealingKey::new(unbound, SingleNonce(nonce_bytes));

    let mut in_out = secret_seed.as_bytes().to_vec();
    sealing
        .seal_in_place_append_tag(Aad::empty(), &mut in_out)
        .map_err(|_| WalletError::EncryptionError("seal failed".into()))?;

    Ok((
        BASE64.encode(&in_out),
        BASE64.encode(nonce_bytes),
        BASE64.encode(salt),
    ))
}

/// Decrypt a secret seed from an export bundle using the user's password.
pub fn decrypt_seed(export: &WalletExport, password: &str) -> Result<String, WalletError> {
    let encrypted = BASE64
        .decode(&export.encrypted_seed)
        .map_err(|_| WalletError::DecryptionError)?;
    let nonce_bytes: [u8; NONCE_LEN] = BASE64
        .decode(&export.encryption_nonce)
        .map_err(|_| WalletError::DecryptionError)?
        .try_into()
        .map_err(|_| WalletError::DecryptionError)?;
    let salt = BASE64
        .decode(&export.kdf_salt)
        .map_err(|_| WalletError::DecryptionError)?;

    let key_bytes = derive_key(password, &salt, export.kdf_iterations);

    let unbound =
        UnboundKey::new(&AES_256_GCM, &key_bytes).map_err(|_| WalletError::DecryptionError)?;
    let mut opening = OpeningKey::new(unbound, SingleNonce(nonce_bytes));

    let mut in_out = encrypted;
    let plaintext = opening
        .open_in_place(Aad::empty(), &mut in_out)
        .map_err(|_| WalletError::DecryptionError)?;

    String::from_utf8(plaintext.to_vec()).map_err(|_| WalletError::DecryptionError)
}

/// Compute HMAC-SHA256 over the canonical export fields (excluding the hmac field itself).
/// Used to detect tampering of export bundles.
pub fn compute_export_hmac(export: &WalletExport, hmac_key: &[u8]) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, hmac_key);
    // Canonical message: deterministic concatenation of all fields
    let message = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}",
        export.version,
        export.wallet_id,
        export.stellar_address,
        export.wallet_name,
        export.wallet_type,
        export.encrypted_seed,
        export.encryption_nonce,
        export.kdf_salt,
        export.kdf_iterations,
    );
    let tag = hmac::sign(&key, message.as_bytes());
    BASE64.encode(tag.as_ref())
}

/// Verify the HMAC on an export bundle.
pub fn verify_export_hmac(export: &WalletExport, hmac_key: &[u8]) -> Result<(), WalletError> {
    let expected = compute_export_hmac(export, hmac_key);
    if expected != export.integrity_hmac {
        return Err(WalletError::IntegrityError);
    }
    Ok(())
}

/// Generate a random 32-byte HMAC key (used per-user, stored server-side).
pub fn generate_hmac_key() -> Vec<u8> {
    let mut key = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    key
}
