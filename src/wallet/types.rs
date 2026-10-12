//! Wallet Management Types
//!
//! Core data structures for wallet creation, import/export, backup/restore,
//! and cross-device synchronization.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

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
    #[serde(default)]
    pub balance_stroops: i64,
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

/// Number of stroops in 1 Lumen (XLM). Stellar amounts have 7 decimal places.
pub const STROOPS_PER_LUMEN: i64 = 10_000_000;

/// Maximum number of decimal places supported by Stellar (7 decimal places).
pub const MAX_STROOPS_DECIMALS: usize = 7;

/// Parses a Stellar lumen amount string (e.g. "100.5000000") into exact stroops (1 XLM = 10^7 stroops).
/// Avoids floating point representations to eliminate IEEE 754 precision loss.
pub fn parse_stroops(s: &str) -> Result<i64, WalletError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(WalletError::InvalidAmount("Amount cannot be empty".to_string()));
    }

    let (is_negative, num_str) = if let Some(stripped) = s.strip_prefix('-') {
        (true, stripped)
    } else if let Some(stripped) = s.strip_prefix('+') {
        (false, stripped)
    } else {
        (false, s)
    };

    if num_str.is_empty() {
        return Err(WalletError::InvalidAmount(
            "Amount contains only a sign".to_string(),
        ));
    }

    let parts: Vec<&str> = num_str.split('.').collect();
    if parts.len() > 2 {
        return Err(WalletError::InvalidAmount(format!(
            "Too many decimal points in '{}'",
            s
        )));
    }

    let whole_part: i64 = if parts[0].is_empty() {
        0
    } else {
        parts[0].parse::<i64>().map_err(|e| {
            WalletError::InvalidAmount(format!("Invalid whole number '{}': {}", parts[0], e))
        })?
    };

    let frac_stroops: i64 = if parts.len() == 2 {
        let frac_str = parts[1];
        if frac_str.len() > MAX_STROOPS_DECIMALS {
            return Err(WalletError::InvalidAmount(format!(
                "Too many decimal places ({}) in '{}', maximum allowed is {}",
                frac_str.len(),
                s,
                MAX_STROOPS_DECIMALS
            )));
        }
        if !frac_str.chars().all(|c| c.is_ascii_digit()) {
            return Err(WalletError::InvalidAmount(format!(
                "Non-digit characters in fractional part '{}'",
                frac_str
            )));
        }
        // Pad fractional part with trailing zeros up to 7 places
        let padded = format!("{:0<7}", frac_str);
        padded.parse::<i64>().map_err(|e| {
            WalletError::InvalidAmount(format!("Failed to parse fractional part: {}", e))
        })?
    } else {
        0
    };

    let total_stroops = whole_part
        .checked_mul(STROOPS_PER_LUMEN)
        .and_then(|w| w.checked_add(frac_stroops))
        .ok_or_else(|| {
            WalletError::InvalidAmount(format!("Amount '{}' caused integer overflow", s))
        })?;

    Ok(if is_negative {
        -total_stroops
    } else {
        total_stroops
    })
}

/// Formats stroops into standard Stellar Horizon 7-decimal representation (e.g. 10_000_000 -> "1.0000000").
pub fn format_stroops(stroops: i64) -> String {
    let sign = if stroops < 0 { "-" } else { "" };
    let abs_stroops = stroops.unsigned_abs();
    let whole = abs_stroops / (STROOPS_PER_LUMEN as u64);
    let frac = abs_stroops % (STROOPS_PER_LUMEN as u64);
    format!("{}{}.{:07}", sign, whole, frac)
}

/// Converts stroops to f64 (for backward compatibility).
pub fn stroops_to_f64(stroops: i64) -> f64 {
    (stroops as f64) / (STROOPS_PER_LUMEN as f64)
}

/// Converts f64 lumens to stroops (rounding to nearest stroop).
pub fn f64_to_stroops(lumens: f64) -> Result<i64, WalletError> {
    if lumens.is_nan() || lumens.is_infinite() {
        return Err(WalletError::InvalidAmount(
            "NaN or infinite float cannot be converted to stroops".to_string(),
        ));
    }
    let scaled = lumens * (STROOPS_PER_LUMEN as f64);
    if scaled > (i64::MAX as f64) || scaled < (i64::MIN as f64) {
        return Err(WalletError::InvalidAmount(
            "Amount exceeds i64 range".to_string(),
        ));
    }
    Ok(scaled.round() as i64)
}

/// Wallet balance snapshot from Stellar Horizon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalance {
    pub stellar_address: String,
    pub xlm_balance: f64,
    #[serde(default)]
    pub xlm_balance_stroops: i64,
    #[serde(default)]
    pub xlm_balance_formatted: String,
    pub asset_balances: Vec<AssetBalance>,
    pub fetched_at: DateTime<Utc>,
}

/// Individual asset balance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetBalance {
    pub asset_code: String,
    pub asset_issuer: Option<String>,
    pub balance: f64,
    #[serde(default)]
    pub balance_stroops: i64,
    #[serde(default)]
    pub balance_formatted: String,
    #[serde(default)]
    pub limit_stroops: Option<i64>,
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

    #[error("Invalid amount: {0}")]
    InvalidAmount(String),

    #[error("Horizon API error: {0}")]
    HorizonError(String),
}
