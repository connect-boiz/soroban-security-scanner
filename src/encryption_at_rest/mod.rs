//! Data encryption at rest for sensitive information (issue #334).
//!
//! A self-contained encryption layer for the database: AES-256-GCM field-level
//! encryption with a versioned keyring (rotation-aware), separately-keyed
//! backup encryption, role-based key access with audit logging, key escrow for
//! disaster recovery, compromise handling, performance monitoring and
//! compliance reporting.
//!
//! # Acceptance-criteria mapping
//!
//! | Requirement | Where |
//! |-------------|-------|
//! | AES-256 encryption at rest | [`cipher`] (AES-256-GCM) |
//! | Field-level encryption (API keys, tokens, private keys) | [`field::EncryptedField`], [`service::EncryptionService`] |
//! | Key management with rotation | [`keys::KeyManager`] |
//! | Encrypted backups with separate keys | [`backup::BackupEncryptor`] |
//! | Key access controls & audit logging | [`keys::KeyRole`], [`keys::KeyAccessRecord`] |
//! | Key escrow for disaster recovery | [`keys::KeyManager::escrow`] |
//! | Encryption compliance reporting | [`compliance::report`] |
//! | Encryption performance monitoring | [`monitoring::PerfMonitor`] |
//! | Key-compromise detection & rotation | [`keys::KeyManager::mark_compromised`] |
//! | 100% sensitive-field coverage tracking | [`compliance::ComplianceReport::full_field_coverage`] |
//! | Comprehensive testing | per-module tests + [`tests`] |
//!
//! # Example
//!
//! ```
//! use soroban_security_scanner::encryption_at_rest::*;
//!
//! let keys = KeyManager::new(1_700_000_000, 30 * 24 * 3600).unwrap();
//! let mut svc = EncryptionService::new(keys, 30 * 24 * 3600);
//!
//! let stored = svc
//!     .encrypt_field("sk-live-secret", "users.api_key:1", 1_700_000_000, "svc", KeyRole::Service, 0)
//!     .unwrap();
//! assert!(!stored.contains("sk-live-secret"));
//!
//! let plain = svc
//!     .decrypt_field(&stored, "users.api_key:1", 1_700_000_000, "svc", KeyRole::Service, 0)
//!     .unwrap();
//! assert_eq!(plain, "sk-live-secret");
//! ```

pub mod backup;
pub mod cipher;
pub mod compliance;
pub mod field;
pub mod keys;
pub mod monitoring;
pub mod service;

#[cfg(test)]
mod tests;

pub use backup::{BackupEncryptor, BackupError, EncryptedBackup};
pub use cipher::{
    decrypt, encrypt, generate_key, generate_nonce, CryptoError, KEY_LEN, NONCE_BYTES,
};
pub use compliance::{report, ComplianceInput, ComplianceReport, ALGORITHM};
pub use field::EncryptedField;
pub use keys::{DataKey, KeyAccessRecord, KeyError, KeyManager, KeyOperation, KeyRole, KeyStatus};
pub use monitoring::{PerfMonitor, PerfStats};
pub use service::{EncryptionService, ServiceError};
