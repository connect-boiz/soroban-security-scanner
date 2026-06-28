//! The encryption-at-rest service.
//!
//! The high-level API the database layer calls: it encrypts a sensitive field
//! with the active key (always producing a self-describing envelope) and
//! decrypts a stored value by resolving the key version it was sealed with —
//! so encryption survives key rotation. Every operation is access-controlled,
//! audited (via the key manager) and performance-monitored.

use crate::encryption_at_rest::cipher::generate_nonce;
use crate::encryption_at_rest::compliance::{report, ComplianceInput, ComplianceReport};
use crate::encryption_at_rest::field::EncryptedField;
use crate::encryption_at_rest::keys::{KeyError, KeyManager, KeyRole, KeyStatus};
use crate::encryption_at_rest::monitoring::{PerfMonitor, PerfStats};

/// Errors surfaced by the service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceError {
    /// Key access/management failure.
    Key(KeyError),
    /// Cryptographic failure.
    Crypto(crate::encryption_at_rest::cipher::CryptoError),
    /// A stored value could not be parsed as an envelope.
    MalformedCiphertext,
}

/// Field-level encryption service over a key manager.
pub struct EncryptionService {
    keys: KeyManager,
    monitor: PerfMonitor,
    rotation_period_secs: i64,
}

impl EncryptionService {
    /// Creates a service over a key manager. `rotation_period_secs` is surfaced
    /// in compliance reporting.
    pub fn new(keys: KeyManager, rotation_period_secs: i64) -> Self {
        Self {
            keys,
            monitor: PerfMonitor::new(),
            rotation_period_secs,
        }
    }

    /// Mutable access to the key manager (rotation, escrow, compromise).
    pub fn keys_mut(&mut self) -> &mut KeyManager {
        &mut self.keys
    }

    /// Read access to the key manager.
    pub fn keys(&self) -> &KeyManager {
        &self.keys
    }

    /// Performance stats.
    pub fn perf(&self) -> PerfStats {
        self.monitor.stats()
    }

    /// Encrypts a sensitive value with the active key, returning a storage
    /// string. `context` is bound as AAD so a value can't be moved between
    /// fields/rows. `elapsed_us` lets the caller feed real timing into the perf
    /// monitor (pass 0 if not measuring).
    pub fn encrypt_field(
        &mut self,
        plaintext: &str,
        context: &str,
        now: i64,
        principal: &str,
        role: KeyRole,
        elapsed_us: u64,
    ) -> Result<String, ServiceError> {
        let (key_id, key) = self
            .keys
            .use_active(now, principal, role)
            .map_err(ServiceError::Key)?;
        let nonce = generate_nonce().map_err(ServiceError::Crypto)?;
        let field = EncryptedField::seal(
            key_id,
            &key,
            nonce,
            context.as_bytes(),
            plaintext.as_bytes(),
        )
        .map_err(ServiceError::Crypto)?;
        self.monitor.record_encrypt(plaintext.len(), elapsed_us);
        Ok(field.to_storage_string())
    }

    /// Decrypts a stored value, resolving its key version. Returns the plaintext
    /// as a UTF-8 string.
    pub fn decrypt_field(
        &mut self,
        stored: &str,
        context: &str,
        now: i64,
        principal: &str,
        role: KeyRole,
        elapsed_us: u64,
    ) -> Result<String, ServiceError> {
        let field =
            EncryptedField::from_storage_string(stored).ok_or(ServiceError::MalformedCiphertext)?;
        let key = self
            .keys
            .use_key(field.key_id, now, principal, role)
            .map_err(ServiceError::Key)?;
        let bytes = field
            .open(&key, context.as_bytes())
            .map_err(ServiceError::Crypto)?;
        self.monitor.record_decrypt(bytes.len(), elapsed_us);
        String::from_utf8(bytes).map_err(|_| ServiceError::MalformedCiphertext)
    }

    /// Re-encrypts a stored value under the current active key (used during a
    /// rotation/compromise re-encryption sweep).
    pub fn reencrypt_field(
        &mut self,
        stored: &str,
        context: &str,
        now: i64,
        principal: &str,
        role: KeyRole,
    ) -> Result<String, ServiceError> {
        let plaintext = self.decrypt_field(stored, context, now, principal, role, 0)?;
        self.encrypt_field(&plaintext, context, now, principal, role, 0)
    }

    /// Builds a compliance report for the current posture. `sensitive_fields`
    /// and `encrypted_fields` come from the data inventory; `backups_encrypted`
    /// from the backup subsystem.
    pub fn compliance_report(
        &self,
        now: i64,
        sensitive_fields: u64,
        encrypted_fields: u64,
        backups_encrypted: bool,
    ) -> ComplianceReport {
        let compromised_key_present = (1..=self.keys.active_id())
            .any(|id| self.keys.status(id) == Some(KeyStatus::Compromised));
        report(ComplianceInput {
            sensitive_fields,
            encrypted_fields,
            backups_encrypted,
            active_key_age_secs: self.keys.active_key_age(now),
            rotation_overdue: self.keys.rotation_overdue(now),
            compromised_key_present,
        })
    }

    /// The configured rotation period, in seconds.
    pub fn rotation_period_secs(&self) -> i64 {
        self.rotation_period_secs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> EncryptionService {
        let period = 30 * 24 * 3600;
        EncryptionService::new(KeyManager::new(1000, period).unwrap(), period)
    }

    #[test]
    fn encrypt_decrypt_round_trip() {
        let mut s = service();
        let stored = s
            .encrypt_field(
                "sk-live-secret",
                "users.api_key:1",
                1000,
                "svc",
                KeyRole::Service,
                50,
            )
            .unwrap();
        assert!(stored.starts_with("v1:"));
        assert!(!stored.contains("sk-live-secret"));

        let pt = s
            .decrypt_field(
                &stored,
                "users.api_key:1",
                1000,
                "svc",
                KeyRole::Service,
                40,
            )
            .unwrap();
        assert_eq!(pt, "sk-live-secret");

        let perf = s.perf();
        assert_eq!(perf.encrypt_ops, 1);
        assert_eq!(perf.decrypt_ops, 1);
    }

    #[test]
    fn decrypt_works_across_rotation() {
        let mut s = service();
        let stored = s
            .encrypt_field("private-key", "wallet:1", 1000, "svc", KeyRole::Service, 0)
            .unwrap();
        // Rotate the active key.
        s.keys_mut()
            .rotate(2000, "admin", KeyRole::KeyAdmin)
            .unwrap();
        // Old value still decrypts via its retired key version.
        let pt = s
            .decrypt_field(&stored, "wallet:1", 2000, "svc", KeyRole::Service, 0)
            .unwrap();
        assert_eq!(pt, "private-key");
    }

    #[test]
    fn reencrypt_moves_value_to_active_key() {
        let mut s = service();
        let stored_v1 = s
            .encrypt_field("token", "sessions:1", 1000, "svc", KeyRole::Service, 0)
            .unwrap();
        let old_id = EncryptedField::from_storage_string(&stored_v1)
            .unwrap()
            .key_id;
        s.keys_mut()
            .rotate(2000, "admin", KeyRole::KeyAdmin)
            .unwrap();

        let stored_v2 = s
            .reencrypt_field(&stored_v1, "sessions:1", 2000, "svc", KeyRole::Service)
            .unwrap();
        let new_id = EncryptedField::from_storage_string(&stored_v2)
            .unwrap()
            .key_id;
        assert!(new_id > old_id, "re-encrypted under the new active key");
        // Still decrypts.
        assert_eq!(
            s.decrypt_field(&stored_v2, "sessions:1", 2000, "svc", KeyRole::Service, 0)
                .unwrap(),
            "token"
        );
    }

    #[test]
    fn unauthorized_role_is_denied() {
        let mut s = service();
        let err = s
            .encrypt_field("x", "ctx", 1000, "intruder", KeyRole::EscrowOfficer, 0)
            .unwrap_err();
        assert_eq!(err, ServiceError::Key(KeyError::AccessDenied));
    }

    #[test]
    fn malformed_ciphertext_is_rejected() {
        let mut s = service();
        let err = s
            .decrypt_field("not-an-envelope", "ctx", 1000, "svc", KeyRole::Service, 0)
            .unwrap_err();
        assert_eq!(err, ServiceError::MalformedCiphertext);
    }

    #[test]
    fn compliance_report_reflects_state() {
        let mut s = service();
        // Healthy report.
        let r = s.compliance_report(1000, 10, 10, true);
        assert!(r.compliant);
        // Compromise the active key → report flags it.
        let active = s.keys().active_id();
        s.keys_mut()
            .mark_compromised(active, 1000, "admin", KeyRole::KeyAdmin)
            .unwrap();
        let r2 = s.compliance_report(1000, 10, 10, true);
        assert!(!r2.compliant);
        assert!(r2.findings.iter().any(|f| f.contains("compromised")));
    }
}
