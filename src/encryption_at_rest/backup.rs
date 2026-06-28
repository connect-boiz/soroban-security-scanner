//! Database backup encryption.
//!
//! Backups are encrypted with a key from a **separate** keyring than the
//! field-encryption keys, so compromise of one does not expose the other and
//! the two can be rotated on independent schedules. Produces a self-describing
//! [`EncryptedBackup`] envelope.

use crate::encryption_at_rest::cipher::{
    decrypt, encrypt, generate_nonce, CryptoError, NONCE_BYTES,
};
use crate::encryption_at_rest::keys::{KeyError, KeyManager, KeyRole};
use serde::{Deserialize, Serialize};

/// An encrypted backup blob with the metadata to restore it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedBackup {
    /// Backup-keyring key version used.
    pub key_id: u32,
    /// Nonce.
    pub nonce: [u8; NONCE_BYTES],
    /// Ciphertext || tag.
    pub data: Vec<u8>,
    /// Plaintext length (for integrity sanity checks on restore).
    pub plaintext_len: usize,
}

/// Encrypts backups using a dedicated key manager.
pub struct BackupEncryptor {
    keys: KeyManager,
}

impl BackupEncryptor {
    /// Wraps a dedicated backup keyring.
    pub fn new(keys: KeyManager) -> Self {
        Self { keys }
    }

    /// Mutable access to the backup keyring (for rotation/escrow).
    pub fn keys_mut(&mut self) -> &mut KeyManager {
        &mut self.keys
    }

    /// Encrypts a backup with the active backup key.
    pub fn encrypt_backup(
        &mut self,
        now: i64,
        principal: &str,
        role: KeyRole,
        plaintext: &[u8],
    ) -> Result<EncryptedBackup, BackupError> {
        let (key_id, key) = self
            .keys
            .use_active(now, principal, role)
            .map_err(BackupError::Key)?;
        let nonce = generate_nonce().map_err(BackupError::Crypto)?;
        let data = encrypt(&key, &nonce, b"db-backup", plaintext).map_err(BackupError::Crypto)?;
        Ok(EncryptedBackup {
            key_id,
            nonce,
            data,
            plaintext_len: plaintext.len(),
        })
    }

    /// Decrypts a backup, resolving the key version it was sealed with.
    pub fn decrypt_backup(
        &mut self,
        backup: &EncryptedBackup,
        now: i64,
        principal: &str,
        role: KeyRole,
    ) -> Result<Vec<u8>, BackupError> {
        let key = self
            .keys
            .use_key(backup.key_id, now, principal, role)
            .map_err(BackupError::Key)?;
        let pt = decrypt(&key, &backup.nonce, b"db-backup", &backup.data)
            .map_err(BackupError::Crypto)?;
        if pt.len() != backup.plaintext_len {
            return Err(BackupError::LengthMismatch);
        }
        Ok(pt)
    }
}

/// Errors from backup encryption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackupError {
    /// Key access/management error.
    Key(KeyError),
    /// Cryptographic error.
    Crypto(CryptoError),
    /// Restored length did not match the recorded length.
    LengthMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encryptor() -> BackupEncryptor {
        BackupEncryptor::new(KeyManager::new(1000, 7 * 24 * 3600).unwrap())
    }

    #[test]
    fn backup_round_trips() {
        let mut e = encryptor();
        let plaintext = b"full database dump bytes...";
        let backup = e
            .encrypt_backup(1000, "backup-job", KeyRole::Service, plaintext)
            .unwrap();
        assert_ne!(backup.data, plaintext);
        let restored = e
            .decrypt_backup(&backup, 1000, "restore-job", KeyRole::Service)
            .unwrap();
        assert_eq!(restored, plaintext);
    }

    #[test]
    fn backup_uses_separate_keyring_from_fields() {
        // A field keyring and a backup keyring are independent instances; a key
        // id from one must not decrypt the other's data.
        let mut backup_enc = encryptor();
        let backup = backup_enc
            .encrypt_backup(1000, "job", KeyRole::Service, b"data")
            .unwrap();

        let mut other_ring = encryptor(); // distinct random key for id 1
        let result = other_ring.decrypt_backup(&backup, 1000, "job", KeyRole::Service);
        assert!(matches!(result, Err(BackupError::Crypto(_))));
    }

    #[test]
    fn rotated_backup_key_still_restores_old_backup() {
        let mut e = encryptor();
        let backup = e
            .encrypt_backup(1000, "job", KeyRole::Service, b"old")
            .unwrap();
        // Rotate the backup key; old backup must still restore via retired key.
        e.keys_mut()
            .rotate(2000, "admin", KeyRole::KeyAdmin)
            .unwrap();
        let restored = e
            .decrypt_backup(&backup, 2000, "job", KeyRole::Service)
            .unwrap();
        assert_eq!(restored, b"old");
    }

    #[test]
    fn unauthorized_role_cannot_encrypt() {
        let mut e = encryptor();
        let result = e.encrypt_backup(1000, "intruder", KeyRole::EscrowOfficer, b"x");
        assert_eq!(
            result.unwrap_err(),
            BackupError::Key(KeyError::AccessDenied)
        );
    }
}
