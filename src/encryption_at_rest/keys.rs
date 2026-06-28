//! Encryption key management.
//!
//! A keyring of versioned AES-256 data keys with exactly one *active* key for
//! new encryptions and any number of *retired* keys retained so old ciphertext
//! still decrypts. Supports rotation, role-based access control with audit
//! logging, key escrow for disaster recovery, and compromise handling that
//! forces rotation.

use crate::encryption_at_rest::cipher::{generate_key, CryptoError, KEY_LEN};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Lifecycle state of a data key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyStatus {
    /// The current key used for new encryptions.
    Active,
    /// Superseded by rotation; kept only to decrypt old data.
    Retired,
    /// Known/suspected compromised; must not be used.
    Compromised,
}

/// Roles permitted to touch key material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyRole {
    /// May encrypt/decrypt via the service (use, not export).
    Service,
    /// May rotate and manage keys.
    KeyAdmin,
    /// May export escrowed keys for disaster recovery.
    EscrowOfficer,
}

/// What a principal is attempting to do with keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyOperation {
    /// Use a key to encrypt/decrypt.
    Use,
    /// Rotate keys.
    Rotate,
    /// Export an escrowed key.
    Escrow,
}

/// An audit record of a key-access attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyAccessRecord {
    /// When (unix seconds).
    pub at: i64,
    /// Acting principal.
    pub principal: String,
    /// Role presented.
    pub role: KeyRole,
    /// Operation attempted.
    pub operation: KeyOperation,
    /// Key id involved (0 when N/A).
    pub key_id: u32,
    /// Whether the access was permitted.
    pub granted: bool,
}

/// A versioned data key. The raw bytes are never serialized.
#[derive(Clone)]
pub struct DataKey {
    /// Key version id.
    pub id: u32,
    /// Raw 256-bit key material.
    pub(crate) bytes: [u8; KEY_LEN],
    /// Creation time (unix seconds).
    pub created_at: i64,
    /// Current status.
    pub status: KeyStatus,
}

/// Errors from key management.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyError {
    /// The principal's role does not permit the operation.
    AccessDenied,
    /// No such key id.
    UnknownKey,
    /// The key exists but is compromised and may not be used.
    KeyCompromised,
    /// Underlying crypto failure (e.g. RNG).
    Crypto(CryptoError),
}

/// The keyring and its policies.
pub struct KeyManager {
    keys: HashMap<u32, DataKey>,
    active_id: u32,
    next_id: u32,
    escrowed: HashMap<u32, [u8; KEY_LEN]>,
    audit: Vec<KeyAccessRecord>,
    /// Max key age (seconds) before rotation is considered overdue.
    rotation_period_secs: i64,
}

impl KeyManager {
    /// Creates a manager with an initial active key created at `now`.
    pub fn new(now: i64, rotation_period_secs: i64) -> Result<Self, KeyError> {
        let bytes = generate_key().map_err(KeyError::Crypto)?;
        let mut keys = HashMap::new();
        keys.insert(
            1,
            DataKey {
                id: 1,
                bytes,
                created_at: now,
                status: KeyStatus::Active,
            },
        );
        Ok(Self {
            keys,
            active_id: 1,
            next_id: 2,
            escrowed: HashMap::new(),
            audit: Vec::new(),
            rotation_period_secs,
        })
    }

    /// Returns whether a role may perform an operation.
    fn permits(role: KeyRole, op: KeyOperation) -> bool {
        match op {
            KeyOperation::Use => matches!(role, KeyRole::Service | KeyRole::KeyAdmin),
            KeyOperation::Rotate => role == KeyRole::KeyAdmin,
            KeyOperation::Escrow => role == KeyRole::EscrowOfficer,
        }
    }

    fn audit_access(
        &mut self,
        now: i64,
        principal: &str,
        role: KeyRole,
        op: KeyOperation,
        key_id: u32,
        granted: bool,
    ) {
        self.audit.push(KeyAccessRecord {
            at: now,
            principal: principal.to_string(),
            role,
            operation: op,
            key_id,
            granted,
        });
    }

    /// The active key id.
    pub fn active_id(&self) -> u32 {
        self.active_id
    }

    /// Borrows the active key's bytes for an authorized `Use`. Logs the access.
    pub fn use_active(
        &mut self,
        now: i64,
        principal: &str,
        role: KeyRole,
    ) -> Result<(u32, [u8; KEY_LEN]), KeyError> {
        let active = self.active_id;
        if !Self::permits(role, KeyOperation::Use) {
            self.audit_access(now, principal, role, KeyOperation::Use, active, false);
            return Err(KeyError::AccessDenied);
        }
        let key = self.keys.get(&active).ok_or(KeyError::UnknownKey)?;
        if key.status == KeyStatus::Compromised {
            self.audit_access(now, principal, role, KeyOperation::Use, active, false);
            return Err(KeyError::KeyCompromised);
        }
        let bytes = key.bytes;
        self.audit_access(now, principal, role, KeyOperation::Use, active, true);
        Ok((active, bytes))
    }

    /// Borrows a specific key's bytes for decrypting old data (retired keys are
    /// allowed; compromised keys are not).
    pub fn use_key(
        &mut self,
        key_id: u32,
        now: i64,
        principal: &str,
        role: KeyRole,
    ) -> Result<[u8; KEY_LEN], KeyError> {
        if !Self::permits(role, KeyOperation::Use) {
            self.audit_access(now, principal, role, KeyOperation::Use, key_id, false);
            return Err(KeyError::AccessDenied);
        }
        let key = self.keys.get(&key_id).ok_or(KeyError::UnknownKey)?;
        if key.status == KeyStatus::Compromised {
            self.audit_access(now, principal, role, KeyOperation::Use, key_id, false);
            return Err(KeyError::KeyCompromised);
        }
        let bytes = key.bytes;
        self.audit_access(now, principal, role, KeyOperation::Use, key_id, true);
        Ok(bytes)
    }

    /// Rotates to a fresh active key, retiring the previous one. Returns the new
    /// active key id.
    pub fn rotate(&mut self, now: i64, principal: &str, role: KeyRole) -> Result<u32, KeyError> {
        if !Self::permits(role, KeyOperation::Rotate) {
            self.audit_access(
                now,
                principal,
                role,
                KeyOperation::Rotate,
                self.active_id,
                false,
            );
            return Err(KeyError::AccessDenied);
        }
        // Retire the current active key (unless it was compromised).
        if let Some(prev) = self.keys.get_mut(&self.active_id) {
            if prev.status == KeyStatus::Active {
                prev.status = KeyStatus::Retired;
            }
        }
        let bytes = generate_key().map_err(KeyError::Crypto)?;
        let id = self.next_id;
        self.next_id += 1;
        self.keys.insert(
            id,
            DataKey {
                id,
                bytes,
                created_at: now,
                status: KeyStatus::Active,
            },
        );
        self.active_id = id;
        self.audit_access(now, principal, role, KeyOperation::Rotate, id, true);
        Ok(id)
    }

    /// Marks a key compromised. If it was the active key, immediately rotates to
    /// a clean key. Returns the (possibly new) active key id.
    pub fn mark_compromised(
        &mut self,
        key_id: u32,
        now: i64,
        principal: &str,
        role: KeyRole,
    ) -> Result<u32, KeyError> {
        if !Self::permits(role, KeyOperation::Rotate) {
            self.audit_access(now, principal, role, KeyOperation::Rotate, key_id, false);
            return Err(KeyError::AccessDenied);
        }
        let key = self.keys.get_mut(&key_id).ok_or(KeyError::UnknownKey)?;
        key.status = KeyStatus::Compromised;
        self.audit_access(now, principal, role, KeyOperation::Rotate, key_id, true);
        if key_id == self.active_id {
            // Active key compromised → mint a new active key right away.
            let bytes = generate_key().map_err(KeyError::Crypto)?;
            let id = self.next_id;
            self.next_id += 1;
            self.keys.insert(
                id,
                DataKey {
                    id,
                    bytes,
                    created_at: now,
                    status: KeyStatus::Active,
                },
            );
            self.active_id = id;
        }
        Ok(self.active_id)
    }

    /// Escrows a copy of a key for disaster recovery (EscrowOfficer only).
    pub fn escrow(
        &mut self,
        key_id: u32,
        now: i64,
        principal: &str,
        role: KeyRole,
    ) -> Result<(), KeyError> {
        if !Self::permits(role, KeyOperation::Escrow) {
            self.audit_access(now, principal, role, KeyOperation::Escrow, key_id, false);
            return Err(KeyError::AccessDenied);
        }
        let key = self.keys.get(&key_id).ok_or(KeyError::UnknownKey)?;
        self.escrowed.insert(key_id, key.bytes);
        self.audit_access(now, principal, role, KeyOperation::Escrow, key_id, true);
        Ok(())
    }

    /// Recovers an escrowed key (disaster recovery).
    pub fn recover_escrow(
        &mut self,
        key_id: u32,
        now: i64,
        principal: &str,
        role: KeyRole,
    ) -> Result<[u8; KEY_LEN], KeyError> {
        if !Self::permits(role, KeyOperation::Escrow) {
            self.audit_access(now, principal, role, KeyOperation::Escrow, key_id, false);
            return Err(KeyError::AccessDenied);
        }
        let bytes = self
            .escrowed
            .get(&key_id)
            .copied()
            .ok_or(KeyError::UnknownKey)?;
        self.audit_access(now, principal, role, KeyOperation::Escrow, key_id, true);
        Ok(bytes)
    }

    /// Status of a key.
    pub fn status(&self, key_id: u32) -> Option<KeyStatus> {
        self.keys.get(&key_id).map(|k| k.status)
    }

    /// Age (seconds) of the active key at `now`.
    pub fn active_key_age(&self, now: i64) -> i64 {
        self.keys
            .get(&self.active_id)
            .map(|k| (now - k.created_at).max(0))
            .unwrap_or(0)
    }

    /// Whether the active key is past its rotation period.
    pub fn rotation_overdue(&self, now: i64) -> bool {
        self.active_key_age(now) > self.rotation_period_secs
    }

    /// The access audit log.
    pub fn audit_log(&self) -> &[KeyAccessRecord] {
        &self.audit
    }

    /// Number of live (non-compromised) keys retained.
    pub fn live_key_count(&self) -> usize {
        self.keys
            .values()
            .filter(|k| k.status != KeyStatus::Compromised)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mgr() -> KeyManager {
        KeyManager::new(1000, 30 * 24 * 3600).unwrap()
    }

    #[test]
    fn starts_with_one_active_key() {
        let m = mgr();
        assert_eq!(m.active_id(), 1);
        assert_eq!(m.status(1), Some(KeyStatus::Active));
    }

    #[test]
    fn service_can_use_admin_can_rotate() {
        let mut m = mgr();
        assert!(m.use_active(1000, "svc", KeyRole::Service).is_ok());
        // Service cannot rotate.
        assert_eq!(
            m.rotate(1000, "svc", KeyRole::Service).unwrap_err(),
            KeyError::AccessDenied
        );
        let new_id = m.rotate(1000, "admin", KeyRole::KeyAdmin).unwrap();
        assert_eq!(new_id, 2);
        assert_eq!(m.active_id(), 2);
        assert_eq!(m.status(1), Some(KeyStatus::Retired));
    }

    #[test]
    fn retired_key_still_decrypts() {
        let mut m = mgr();
        m.rotate(1000, "admin", KeyRole::KeyAdmin).unwrap();
        // Old key (id 1) is retired but usable for decryption.
        assert!(m.use_key(1, 1000, "svc", KeyRole::Service).is_ok());
    }

    #[test]
    fn compromised_active_key_forces_rotation() {
        let mut m = mgr();
        let new_active = m
            .mark_compromised(1, 2000, "admin", KeyRole::KeyAdmin)
            .unwrap();
        assert_ne!(new_active, 1);
        assert_eq!(m.active_id(), new_active);
        assert_eq!(m.status(1), Some(KeyStatus::Compromised));
        // The compromised key can no longer be used.
        assert_eq!(
            m.use_key(1, 2000, "svc", KeyRole::Service).unwrap_err(),
            KeyError::KeyCompromised
        );
    }

    #[test]
    fn escrow_requires_officer_and_round_trips() {
        let mut m = mgr();
        assert_eq!(
            m.escrow(1, 1000, "admin", KeyRole::KeyAdmin).unwrap_err(),
            KeyError::AccessDenied
        );
        m.escrow(1, 1000, "officer", KeyRole::EscrowOfficer)
            .unwrap();
        let recovered = m
            .recover_escrow(1, 1000, "officer", KeyRole::EscrowOfficer)
            .unwrap();
        assert_eq!(recovered.len(), KEY_LEN);
    }

    #[test]
    fn rotation_overdue_by_age() {
        let m = mgr(); // 30-day period, created at 1000
        assert!(!m.rotation_overdue(1000 + 10));
        assert!(m.rotation_overdue(1000 + 31 * 24 * 3600));
    }

    #[test]
    fn access_attempts_are_audited() {
        let mut m = mgr();
        let _ = m.use_active(1000, "svc", KeyRole::Service);
        let _ = m.rotate(1000, "svc", KeyRole::Service); // denied
        let log = m.audit_log();
        assert_eq!(log.len(), 2);
        assert!(log[0].granted);
        assert!(!log[1].granted);
    }
}
