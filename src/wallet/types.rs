//! Wallet Management Types
//!
//! Core data structures for wallet creation, import/export, backup/restore,
//! and cross-device synchronization.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Wallet type classification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WalletType {
    Standard,
    Hardware,
    MultiSig,
    WatchOnly,
    Imported,
}

impl std::fmt::Display for WalletType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WalletType::Standard => write!(f, "standard"),
            WalletType::Hardware => write!(f, "hardware"),
            WalletType::MultiSig => write!(f, "multisig"),
            WalletType::WatchOnly => write!(f, "watch_only"),
            WalletType::Imported => write!(f, "imported"),
        }
    }
}

/// Wallet operational status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WalletStatus {
    Active,
    Inactive,
    Frozen,
    Compromised,
}

/// Wallet record (mirrors the DB wallets table)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub id: Uuid,
    pub user_id: Uuid,
    pub stellar_address: String,
    pub wallet_name: String,
    pub description: Option<String>,
    pub wallet_type: WalletType,
    pub status: WalletStatus,
    pub balance_lumens: f64,
    pub is_primary: bool,
    pub is_verified: bool,
    pub verification_level: i32,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_transaction_at: Option<DateTime<Utc>>,
    pub transaction_count: i32,
    pub security_score: i32,
}

/// Request to create a new wallet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWalletRequest {
    pub user_id: Uuid,
    pub wallet_name: String,
    pub description: Option<String>,
    pub wallet_type: WalletType,
    /// If provided, import this existing Stellar keypair (secret seed)
    pub secret_seed: Option<String>,
    pub set_as_primary: bool,
}

/// Request to import a wallet from a secret seed or mnemonic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportWalletRequest {
    pub user_id: Uuid,
    pub wallet_name: String,
    pub description: Option<String>,
    /// Stellar secret seed (S...) or BIP-39 mnemonic phrase
    pub secret_seed: String,
    pub set_as_primary: bool,
}

/// Encrypted wallet export bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletExport {
    /// Schema version for forward compatibility
    pub version: u8,
    pub wallet_id: Uuid,
    pub stellar_address: String,
    pub wallet_name: String,
    pub wallet_type: WalletType,
    /// AES-256-GCM encrypted secret seed, base64-encoded
    pub encrypted_seed: String,
    /// Random nonce used for encryption, base64-encoded
    pub encryption_nonce: String,
    /// PBKDF2 salt for key derivation, base64-encoded
    pub kdf_salt: String,
    /// PBKDF2 iteration count
    pub kdf_iterations: u32,
    pub exported_at: DateTime<Utc>,
    /// HMAC-SHA256 over all fields (excluding this one), base64-encoded
    pub integrity_hmac: String,
}

/// Request to restore a wallet from an export bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreWalletRequest {
    pub user_id: Uuid,
    pub export_bundle: WalletExport,
    /// Password used to decrypt the export bundle
    pub password: String,
    pub set_as_primary: bool,
}

/// Sync record for cross-device wallet synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSyncRecord {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub user_id: Uuid,
    pub device_id: String,
    pub device_name: Option<String>,
    pub last_synced_at: DateTime<Utc>,
    /// Encrypted wallet state delta, base64-encoded
    pub encrypted_state: String,
    pub sync_version: i64,
}

/// Wallet balance snapshot from Stellar Horizon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalance {
    pub stellar_address: String,
    pub xlm_balance: f64,
    pub asset_balances: Vec<AssetBalance>,
    pub fetched_at: DateTime<Utc>,
}

/// Individual asset balance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetBalance {
    pub asset_code: String,
    pub asset_issuer: Option<String>,
    pub balance: f64,
}

/// Errors specific to wallet operations
#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    #[error("Wallet not found: {0}")]
    NotFound(Uuid),

    #[error("Invalid Stellar address: {0}")]
    InvalidAddress(String),

    #[error("Invalid secret seed")]
    InvalidSecretSeed,

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Decryption error: wrong password or corrupted data")]
    DecryptionError,

    #[error("Integrity check failed: export bundle may be tampered")]
    IntegrityError,

    #[error("Wallet already exists for address: {0}")]
    DuplicateAddress(String),

    #[error("User already has a primary wallet")]
    PrimaryWalletExists,

    #[error("Sync conflict: local version {local} vs remote version {remote}")]
    SyncConflict { local: i64, remote: i64 },

    #[error("Stellar network error: {0}")]
    NetworkError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Unauthorized: user {0} does not own wallet {1}")]
    Unauthorized(Uuid, Uuid),
}
