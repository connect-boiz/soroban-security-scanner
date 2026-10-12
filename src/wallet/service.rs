//! Wallet Management Service
//!
//! Provides wallet creation, import/export, backup/restore, and
//! cross-device synchronization for Stellar wallets.

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::wallet::{
    crypto::{
        compute_export_hmac, decrypt_seed, encrypt_seed, generate_hmac_key, verify_export_hmac,
    },
    types::{
        format_stroops, parse_stroops, stroops_to_f64, AssetBalance, CreateWalletRequest,
        ImportWalletRequest, RestoreWalletRequest, Wallet, WalletBalance, WalletError,
        WalletExport, WalletStatus, WalletSyncRecord, WalletType,
    },
};

// ---------------------------------------------------------------------------
// Storage trait — swap in a real DB implementation without changing service logic
// ---------------------------------------------------------------------------

#[async_trait]
pub trait WalletStore: Send + Sync {
    async fn create(&self, wallet: &Wallet) -> Result<(), WalletError>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Wallet>, WalletError>;
    async fn get_by_address(&self, address: &str) -> Result<Option<Wallet>, WalletError>;
    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<Wallet>, WalletError>;
    async fn update(&self, wallet: &Wallet) -> Result<(), WalletError>;
    async fn delete(&self, id: Uuid) -> Result<(), WalletError>;
    async fn clear_primary(&self, user_id: Uuid) -> Result<(), WalletError>;

    // Sync records
    async fn upsert_sync_record(&self, record: &WalletSyncRecord) -> Result<(), WalletError>;
    async fn get_sync_records(&self, wallet_id: Uuid)
        -> Result<Vec<WalletSyncRecord>, WalletError>;

    // Per-user HMAC key for export integrity
    async fn get_hmac_key(&self, user_id: Uuid) -> Result<Vec<u8>, WalletError>;
    async fn set_hmac_key(&self, user_id: Uuid, key: Vec<u8>) -> Result<(), WalletError>;
}

// ---------------------------------------------------------------------------
// In-memory store (for testing / development)
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct InMemoryWalletStore {
    wallets: RwLock<HashMap<Uuid, Wallet>>,
    sync_records: RwLock<HashMap<Uuid, Vec<WalletSyncRecord>>>,
    hmac_keys: RwLock<HashMap<Uuid, Vec<u8>>>,
}

#[async_trait]
impl WalletStore for InMemoryWalletStore {
    async fn create(&self, wallet: &Wallet) -> Result<(), WalletError> {
        let mut store = self.wallets.write().await;
        if store
            .values()
            .any(|w| w.stellar_address == wallet.stellar_address)
        {
            return Err(WalletError::DuplicateAddress(
                wallet.stellar_address.clone(),
            ));
        }
        store.insert(wallet.id, wallet.clone());
        Ok(())
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Wallet>, WalletError> {
        Ok(self.wallets.read().await.get(&id).cloned())
    }

    async fn get_by_address(&self, address: &str) -> Result<Option<Wallet>, WalletError> {
        Ok(self
            .wallets
            .read()
            .await
            .values()
            .find(|w| w.stellar_address == address)
            .cloned())
    }

    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<Wallet>, WalletError> {
        Ok(self
            .wallets
            .read()
            .await
            .values()
            .filter(|w| w.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn update(&self, wallet: &Wallet) -> Result<(), WalletError> {
        self.wallets.write().await.insert(wallet.id, wallet.clone());
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), WalletError> {
        self.wallets.write().await.remove(&id);
        Ok(())
    }

    async fn clear_primary(&self, user_id: Uuid) -> Result<(), WalletError> {
        let mut store = self.wallets.write().await;
        for wallet in store.values_mut().filter(|w| w.user_id == user_id) {
            wallet.is_primary = false;
        }
        Ok(())
    }

    async fn upsert_sync_record(&self, record: &WalletSyncRecord) -> Result<(), WalletError> {
        let mut store = self.sync_records.write().await;
        let records = store.entry(record.wallet_id).or_default();
        if let Some(existing) = records.iter_mut().find(|r| r.device_id == record.device_id) {
            *existing = record.clone();
        } else {
            records.push(record.clone());
        }
        Ok(())
    }

    async fn get_sync_records(
        &self,
        wallet_id: Uuid,
    ) -> Result<Vec<WalletSyncRecord>, WalletError> {
        Ok(self
            .sync_records
            .read()
            .await
            .get(&wallet_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn get_hmac_key(&self, user_id: Uuid) -> Result<Vec<u8>, WalletError> {
        Ok(self
            .hmac_keys
            .read()
            .await
            .get(&user_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn set_hmac_key(&self, user_id: Uuid, key: Vec<u8>) -> Result<(), WalletError> {
        self.hmac_keys.write().await.insert(user_id, key);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Stellar Horizon Client
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonBalanceItem {
    pub balance: String,
    pub asset_type: String,
    #[serde(default)]
    pub asset_code: Option<String>,
    #[serde(default)]
    pub asset_issuer: Option<String>,
    #[serde(default)]
    pub limit: Option<String>,
    #[serde(default)]
    pub buying_liabilities: Option<String>,
    #[serde(default)]
    pub selling_liabilities: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonAccountBalances {
    pub balances: Vec<HorizonBalanceItem>,
}

#[async_trait]
pub trait HorizonClient: Send + Sync {
    async fn get_account_balances(
        &self,
        stellar_address: &str,
    ) -> Result<Option<HorizonAccountBalances>, WalletError>;
}

/// HTTP client for Stellar Horizon API
pub struct HttpHorizonClient {
    base_url: String,
    client: reqwest::Client,
}

impl HttpHorizonClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    pub fn testnet() -> Self {
        Self::new("https://horizon-testnet.stellar.org")
    }

    pub fn public() -> Self {
        Self::new("https://horizon.stellar.org")
    }
}

#[async_trait]
impl HorizonClient for HttpHorizonClient {
    async fn get_account_balances(
        &self,
        stellar_address: &str,
    ) -> Result<Option<HorizonAccountBalances>, WalletError> {
        let url = format!("{}/accounts/{}", self.base_url, stellar_address);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| WalletError::NetworkError(e.to_string()))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read response body".to_string());
            return Err(WalletError::HorizonError(format!("HTTP {}: {}", status, body)));
        }

        let account: HorizonAccountBalances = resp
            .json()
            .await
            .map_err(|e| WalletError::HorizonError(format!("Failed to parse Horizon response: {}", e)))?;

        Ok(Some(account))
    }
}

/// In-memory mock Horizon client for testing and offline development
#[derive(Default)]
pub struct MockHorizonClient {
    accounts: RwLock<HashMap<String, HorizonAccountBalances>>,
}

impl MockHorizonClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn set_account_balances(&self, address: &str, balances: HorizonAccountBalances) {
        let mut map = self.accounts.write().await;
        map.insert(address.to_string(), balances);
    }

    pub async fn set_xlm_balance(&self, address: &str, stroops: i64) {
        let balances = HorizonAccountBalances {
            balances: vec![HorizonBalanceItem {
                balance: format_stroops(stroops),
                asset_type: "native".to_string(),
                asset_code: None,
                asset_issuer: None,
                limit: None,
                buying_liabilities: None,
                selling_liabilities: None,
            }],
        };
        self.set_account_balances(address, balances).await;
    }

    pub async fn remove_account(&self, address: &str) {
        let mut map = self.accounts.write().await;
        map.remove(address);
    }
}

#[async_trait]
impl HorizonClient for MockHorizonClient {
    async fn get_account_balances(
        &self,
        stellar_address: &str,
    ) -> Result<Option<HorizonAccountBalances>, WalletError> {
        let map = self.accounts.read().await;
        Ok(map.get(stellar_address).cloned())
    }
}

// ---------------------------------------------------------------------------
// Wallet Service
// ---------------------------------------------------------------------------

pub struct WalletService {
    store: Arc<dyn WalletStore>,
    horizon_client: Arc<dyn HorizonClient>,
}

impl WalletService {
    pub fn new(store: Arc<dyn WalletStore>) -> Self {
        Self::with_horizon(store, Arc::new(HttpHorizonClient::testnet()))
    }

    pub fn with_horizon(store: Arc<dyn WalletStore>, horizon_client: Arc<dyn HorizonClient>) -> Self {
        Self {
            store,
            horizon_client,
        }
    }

    // -----------------------------------------------------------------------
    // Wallet Creation
    // -----------------------------------------------------------------------

    /// Create a brand-new Stellar keypair and register it as a wallet.
    pub async fn create_wallet(&self, req: CreateWalletRequest) -> Result<Wallet, WalletError> {
        let (stellar_address, _secret) = match &req.secret_seed {
            Some(seed) => self.parse_keypair(seed)?,
            None => self.generate_keypair(),
        };

        self.build_and_store_wallet(
            req.user_id,
            stellar_address,
            req.wallet_name,
            req.description,
            req.wallet_type,
            req.set_as_primary,
        )
        .await
    }

    /// Import a wallet from an existing secret seed or mnemonic.
    pub async fn import_wallet(&self, req: ImportWalletRequest) -> Result<Wallet, WalletError> {
        let (stellar_address, _secret) = self.parse_keypair(&req.secret_seed)?;

        self.build_and_store_wallet(
            req.user_id,
            stellar_address,
            req.wallet_name,
            req.description,
            WalletType::Imported,
            req.set_as_primary,
        )
        .await
    }

    // -----------------------------------------------------------------------
    // Export / Backup
    // -----------------------------------------------------------------------

    /// Export a wallet as an encrypted bundle suitable for backup or transfer.
    /// The caller must supply the raw secret seed (obtained at creation/import time).
    pub async fn export_wallet(
        &self,
        wallet_id: Uuid,
        user_id: Uuid,
        secret_seed: &str,
        password: &str,
    ) -> Result<WalletExport, WalletError> {
        let wallet = self.get_owned_wallet(wallet_id, user_id).await?;

        let (encrypted_seed, nonce, salt) = encrypt_seed(secret_seed, password)?;

        // Ensure the user has an HMAC key
        let hmac_key = self.get_or_create_hmac_key(user_id).await?;

        let mut export = WalletExport {
            version: 1,
            wallet_id: wallet.id,
            stellar_address: wallet.stellar_address.clone(),
            wallet_name: wallet.wallet_name.clone(),
            wallet_type: wallet.wallet_type.clone(),
            encrypted_seed,
            encryption_nonce: nonce,
            kdf_salt: salt,
            kdf_iterations: 100_000,
            exported_at: Utc::now(),
            integrity_hmac: String::new(), // filled below
        };

        export.integrity_hmac = compute_export_hmac(&export, &hmac_key);
        Ok(export)
    }

    // -----------------------------------------------------------------------
    // Restore / Import from backup
    // -----------------------------------------------------------------------

    /// Restore a wallet from an encrypted export bundle.
    pub async fn restore_wallet(&self, req: RestoreWalletRequest) -> Result<Wallet, WalletError> {
        let hmac_key = self.get_or_create_hmac_key(req.user_id).await?;
        verify_export_hmac(&req.export_bundle, &hmac_key)?;

        let secret_seed = decrypt_seed(&req.export_bundle, &req.password)?;
        let (stellar_address, _) = self.parse_keypair(&secret_seed)?;

        // Verify the decrypted address matches the bundle
        if stellar_address != req.export_bundle.stellar_address {
            return Err(WalletError::IntegrityError);
        }

        self.build_and_store_wallet(
            req.user_id,
            stellar_address,
            req.export_bundle.wallet_name.clone(),
            None,
            req.export_bundle.wallet_type.clone(),
            req.set_as_primary,
        )
        .await
    }

    // -----------------------------------------------------------------------
    // Wallet Synchronization
    // -----------------------------------------------------------------------

    /// Record a sync event from a device. The encrypted_state is an opaque
    /// blob (e.g., encrypted wallet metadata delta) managed by the client.
    pub async fn sync_wallet(
        &self,
        wallet_id: Uuid,
        user_id: Uuid,
        device_id: String,
        device_name: Option<String>,
        encrypted_state: String,
        client_version: i64,
    ) -> Result<WalletSyncRecord, WalletError> {
        // Ownership check
        self.get_owned_wallet(wallet_id, user_id).await?;

        // Conflict detection: reject if client is behind the latest known version
        let existing = self.store.get_sync_records(wallet_id).await?;
        let latest_version = existing.iter().map(|r| r.sync_version).max().unwrap_or(0);

        if client_version < latest_version {
            return Err(WalletError::SyncConflict {
                local: client_version,
                remote: latest_version,
            });
        }

        let record = WalletSyncRecord {
            id: Uuid::new_v4(),
            wallet_id,
            user_id,
            device_id,
            device_name,
            last_synced_at: Utc::now(),
            encrypted_state,
            sync_version: latest_version + 1,
        };

        self.store.upsert_sync_record(&record).await?;
        Ok(record)
    }

    /// List all sync records for a wallet (one per device).
    pub async fn list_sync_records(
        &self,
        wallet_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<WalletSyncRecord>, WalletError> {
        self.get_owned_wallet(wallet_id, user_id).await?;
        self.store.get_sync_records(wallet_id).await
    }

    // -----------------------------------------------------------------------
    // Balance
    // -----------------------------------------------------------------------

    /// Fetch the current balance from Stellar Horizon, update the wallet record
    /// in the store with the latest balance, and return the balance breakdown.
    pub async fn get_balance(
        &self,
        wallet_id: Uuid,
        user_id: Uuid,
    ) -> Result<WalletBalance, WalletError> {
        let mut wallet = self.get_owned_wallet(wallet_id, user_id).await?;

        let account_info = self
            .horizon_client
            .get_account_balances(&wallet.stellar_address)
            .await?;

        let (xlm_stroops, asset_balances) = match account_info {
            Some(info) => {
                let mut xlm_stroops = 0i64;
                let mut assets = Vec::new();

                for item in info.balances {
                    let balance_stroops = parse_stroops(&item.balance)?;
                    let balance_f64 = stroops_to_f64(balance_stroops);

                    if item.asset_type == "native" {
                        xlm_stroops = balance_stroops;
                    } else {
                        let limit_stroops = match &item.limit {
                            Some(l) => Some(parse_stroops(l)?),
                            None => None,
                        };
                        assets.push(AssetBalance {
                            asset_code: item.asset_code.unwrap_or_else(|| "UNKNOWN".to_string()),
                            asset_issuer: item.asset_issuer,
                            balance: balance_f64,
                            balance_stroops,
                            balance_formatted: item.balance.clone(),
                            limit_stroops,
                        });
                    }
                }
                (xlm_stroops, assets)
            }
            None => {
                // Account not found / unfunded on Stellar Horizon
                (0i64, Vec::new())
            }
        };

        let now = Utc::now();
        wallet.balance_stroops = xlm_stroops;
        wallet.balance_lumens = stroops_to_f64(xlm_stroops);
        wallet.updated_at = now;
        self.store.update(&wallet).await?;

        Ok(WalletBalance {
            stellar_address: wallet.stellar_address.clone(),
            xlm_balance: wallet.balance_lumens,
            xlm_balance_stroops: xlm_stroops,
            xlm_balance_formatted: format_stroops(xlm_stroops),
            asset_balances,
            fetched_at: now,
        })
    }

    // -----------------------------------------------------------------------
    // CRUD helpers
    // -----------------------------------------------------------------------

    pub async fn get_wallet(&self, wallet_id: Uuid, user_id: Uuid) -> Result<Wallet, WalletError> {
        self.get_owned_wallet(wallet_id, user_id).await
    }

    pub async fn list_wallets(&self, user_id: Uuid) -> Result<Vec<Wallet>, WalletError> {
        self.store.list_for_user(user_id).await
    }

    pub async fn freeze_wallet(
        &self,
        wallet_id: Uuid,
        user_id: Uuid,
    ) -> Result<Wallet, WalletError> {
        let mut wallet = self.get_owned_wallet(wallet_id, user_id).await?;
        wallet.status = WalletStatus::Frozen;
        wallet.updated_at = Utc::now();
        self.store.update(&wallet).await?;
        Ok(wallet)
    }

    pub async fn delete_wallet(&self, wallet_id: Uuid, user_id: Uuid) -> Result<(), WalletError> {
        self.get_owned_wallet(wallet_id, user_id).await?;
        self.store.delete(wallet_id).await
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    async fn build_and_store_wallet(
        &self,
        user_id: Uuid,
        stellar_address: String,
        wallet_name: String,
        description: Option<String>,
        wallet_type: WalletType,
        set_as_primary: bool,
    ) -> Result<Wallet, WalletError> {
        if set_as_primary {
            self.store.clear_primary(user_id).await?;
        }

        let wallet = Wallet {
            id: Uuid::new_v4(),
            user_id,
            stellar_address,
            wallet_name,
            description,
            wallet_type,
            status: WalletStatus::Active,
            balance_lumens: 0.0,
            balance_stroops: 0,
            is_primary: set_as_primary,
            is_verified: false,
            verification_level: 0,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_transaction_at: None,
            transaction_count: 0,
            security_score: 0,
        };

        self.store.create(&wallet).await?;
        Ok(wallet)
    }

    async fn get_owned_wallet(
        &self,
        wallet_id: Uuid,
        user_id: Uuid,
    ) -> Result<Wallet, WalletError> {
        let wallet = self
            .store
            .get_by_id(wallet_id)
            .await?
            .ok_or(WalletError::NotFound(wallet_id))?;

        if wallet.user_id != user_id {
            return Err(WalletError::Unauthorized(user_id, wallet_id));
        }
        Ok(wallet)
    }

    async fn get_or_create_hmac_key(&self, user_id: Uuid) -> Result<Vec<u8>, WalletError> {
        let key = self.store.get_hmac_key(user_id).await?;
        if key.is_empty() {
            let new_key = generate_hmac_key();
            self.store.set_hmac_key(user_id, new_key.clone()).await?;
            Ok(new_key)
        } else {
            Ok(key)
        }
    }

    /// Generate a fresh Stellar keypair. Returns (public_address, secret_seed).
    /// Uses ring's Ed25519 key generation and encodes with Stellar's base32 strkey format.
    fn generate_keypair(&self) -> (String, String) {
        use rand::RngCore;
        use ring::signature::{Ed25519KeyPair, KeyPair};

        let mut seed = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut seed);

        // Encode as Stellar strkey: version byte + payload + checksum, base32
        let secret_address = stellar_strkey_encode(&seed, 0x90); // S... secret seed
        let key_pair = Ed25519KeyPair::from_seed_unchecked(&seed).expect("valid 32-byte seed");
        let public_bytes: &[u8] = key_pair.public_key().as_ref();
        let public_address = stellar_strkey_encode(public_bytes, 0x30); // G... public key

        (public_address, secret_address)
    }

    /// Parse a Stellar secret seed (S...) and return (public_address, secret_seed).
    fn parse_keypair(&self, seed: &str) -> Result<(String, String), WalletError> {
        use ring::signature::{Ed25519KeyPair, KeyPair};

        let raw = stellar_strkey_decode(seed, 0x90).map_err(|_| WalletError::InvalidSecretSeed)?;
        if raw.len() != 32 {
            return Err(WalletError::InvalidSecretSeed);
        }
        let key_pair = Ed25519KeyPair::from_seed_unchecked(&raw)
            .map_err(|_| WalletError::InvalidSecretSeed)?;
        let public_bytes: &[u8] = key_pair.public_key().as_ref();
        let public_address = stellar_strkey_encode(public_bytes, 0x30);
        Ok((public_address, seed.to_string()))
    }
}

// ---------------------------------------------------------------------------
// Stellar strkey encoding helpers (RFC-compliant base32 with CRC-16/XMODEM)
// ---------------------------------------------------------------------------

/// Encode raw bytes as a Stellar strkey string (base32 with version byte + CRC).
fn stellar_strkey_encode(payload: &[u8], version: u8) -> String {
    let mut data = Vec::with_capacity(1 + payload.len() + 2);
    data.push(version);
    data.extend_from_slice(payload);
    let crc = crc16_xmodem(&data);
    data.push((crc & 0xFF) as u8);
    data.push((crc >> 8) as u8);
    base32_encode_stellar(&data)
}

/// Decode a Stellar strkey string, verifying the version byte and CRC.
fn stellar_strkey_decode(s: &str, expected_version: u8) -> Result<Vec<u8>, ()> {
    let data = base32_decode_stellar(s).ok_or(())?;
    if data.len() < 3 {
        return Err(());
    }
    if data[0] != expected_version {
        return Err(());
    }
    let payload = &data[1..data.len() - 2];
    let stored_crc = (data[data.len() - 2] as u16) | ((data[data.len() - 1] as u16) << 8);
    let computed_crc = crc16_xmodem(&data[..data.len() - 2]);
    if stored_crc != computed_crc {
        return Err(());
    }
    Ok(payload.to_vec())
}

/// CRC-16/XMODEM used by Stellar strkey
fn crc16_xmodem(data: &[u8]) -> u16 {
    let mut crc: u16 = 0x0000;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

const BASE32_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

fn base32_encode_stellar(data: &[u8]) -> String {
    let mut result = String::new();
    let mut buffer: u64 = 0;
    let mut bits_left: u32 = 0;
    for &byte in data {
        buffer = (buffer << 8) | byte as u64;
        bits_left += 8;
        while bits_left >= 5 {
            bits_left -= 5;
            result.push(BASE32_ALPHABET[((buffer >> bits_left) & 0x1F) as usize] as char);
        }
    }
    if bits_left > 0 {
        result.push(BASE32_ALPHABET[((buffer << (5 - bits_left)) & 0x1F) as usize] as char);
    }
    result
}

fn base32_decode_stellar(s: &str) -> Option<Vec<u8>> {
    let mut buffer: u64 = 0;
    let mut bits_left: u32 = 0;
    let mut result = Vec::new();
    for ch in s.chars() {
        let val = BASE32_ALPHABET.iter().position(|&c| c == ch as u8)? as u64;
        buffer = (buffer << 5) | val;
        bits_left += 5;
        if bits_left >= 8 {
            bits_left -= 8;
            result.push(((buffer >> bits_left) & 0xFF) as u8);
        }
    }
    Some(result)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::types::{f64_to_stroops, format_stroops, parse_stroops, STROOPS_PER_LUMEN};

    #[test]
    fn test_parse_and_format_stroops_standard() {
        // Standard 7-decimal Stellar string
        let stroops = parse_stroops("100.5000000").expect("should parse 100.5000000");
        assert_eq!(stroops, 1_005_000_000);
        assert_eq!(format_stroops(stroops), "100.5000000");

        // Smallest unit (1 stroop)
        let one_stroop = parse_stroops("0.0000001").expect("should parse 1 stroop");
        assert_eq!(one_stroop, 1);
        assert_eq!(format_stroops(one_stroop), "0.0000001");

        // Zero
        let zero = parse_stroops("0.0000000").expect("should parse zero with decimals");
        assert_eq!(zero, 0);
        assert_eq!(format_stroops(zero), "0.0000000");

        let zero_int = parse_stroops("0").expect("should parse zero integer");
        assert_eq!(zero_int, 0);
        assert_eq!(format_stroops(zero_int), "0.0000000");

        // Exactly 1 Lumen
        let one_lumen = parse_stroops("1").expect("should parse 1 lumen");
        assert_eq!(one_lumen, STROOPS_PER_LUMEN);
        assert_eq!(format_stroops(one_lumen), "1.0000000");

        // Padding partial decimals to 7 places
        let partial = parse_stroops("12.34").expect("should parse 12.34");
        assert_eq!(partial, 123_400_000);
        assert_eq!(format_stroops(partial), "12.3400000");

        // Large number
        let large = parse_stroops("999999999.9999999").expect("should parse large number");
        assert_eq!(large, 9_999_999_999_999_999);
        assert_eq!(format_stroops(large), "999999999.9999999");
    }

    #[test]
    fn test_parse_negative_stroops() {
        let neg = parse_stroops("-10.5000000").expect("should parse negative");
        assert_eq!(neg, -105_000_000);
        assert_eq!(format_stroops(neg), "-10.5000000");

        let neg_one = parse_stroops("-0.0000001").expect("should parse negative 1 stroop");
        assert_eq!(neg_one, -1);
        assert_eq!(format_stroops(neg_one), "-0.0000001");
    }

    #[test]
    fn test_parse_stroops_invalid_inputs() {
        assert!(parse_stroops("").is_err());
        assert!(parse_stroops("   ").is_err());
        assert!(parse_stroops("abc").is_err());
        assert!(parse_stroops("10.12.34").is_err());
        assert!(parse_stroops("-").is_err());
        assert!(parse_stroops("+").is_err());
        assert!(parse_stroops("10.12345678").is_err()); // 8 decimal places exceeds max 7
    }

    #[test]
    fn test_precision_loss_elimination() {
        // In IEEE 754 float: 0.1 + 0.2 = 0.30000000000000004 != 0.3
        let float_sum = 0.1f64 + 0.2f64;
        assert_ne!(float_sum, 0.3f64);

        // In stroops (integer): 1,000,000 + 2,000,000 = 3,000,000 == exact 0.3
        let s1 = parse_stroops("0.1").unwrap();
        let s2 = parse_stroops("0.2").unwrap();
        let s3 = parse_stroops("0.3").unwrap();
        assert_eq!(s1 + s2, s3);
        assert_eq!(format_stroops(s1 + s2), "0.3000000");
    }

    #[test]
    fn test_stroops_f64_conversions() {
        assert_eq!(stroops_to_f64(10_000_000), 1.0);
        assert_eq!(stroops_to_f64(1_005_000_000), 100.5);

        assert_eq!(f64_to_stroops(100.5).unwrap(), 1_005_000_000);
        assert!(f64_to_stroops(f64::NAN).is_err());
        assert!(f64_to_stroops(f64::INFINITY).is_err());
    }

    #[tokio::test]
    async fn test_wallet_creation_initial_balance_zero() {
        let store = Arc::new(InMemoryWalletStore::default());
        let service = WalletService::new(store);

        let user_id = Uuid::new_v4();
        let wallet = service
            .create_wallet(CreateWalletRequest {
                user_id,
                wallet_name: "Test Wallet".to_string(),
                description: None,
                wallet_type: WalletType::Standard,
                secret_seed: None,
                set_as_primary: true,
            })
            .await
            .expect("should create wallet");

        assert_eq!(wallet.balance_stroops, 0);
        assert_eq!(wallet.balance_lumens, 0.0);
        assert!(wallet.is_primary);
    }

    #[tokio::test]
    async fn test_horizon_mock_sync_unfunded_account() {
        let store = Arc::new(InMemoryWalletStore::default());
        let mock_horizon = Arc::new(MockHorizonClient::new());
        let service = WalletService::with_horizon(store.clone(), mock_horizon);

        let user_id = Uuid::new_v4();
        let wallet = service
            .create_wallet(CreateWalletRequest {
                user_id,
                wallet_name: "Unfunded Wallet".to_string(),
                description: None,
                wallet_type: WalletType::Standard,
                secret_seed: None,
                set_as_primary: false,
            })
            .await
            .unwrap();

        // Account does not exist in Horizon mock (simulates 404 unfunded)
        let balance = service
            .get_balance(wallet.id, user_id)
            .await
            .expect("should handle unfunded account gracefully");

        assert_eq!(balance.xlm_balance_stroops, 0);
        assert_eq!(balance.xlm_balance, 0.0);
        assert_eq!(balance.xlm_balance_formatted, "0.0000000");
        assert!(balance.asset_balances.is_empty());

        // Verify stored wallet in store reflects 0
        let stored = service.get_wallet(wallet.id, user_id).await.unwrap();
        assert_eq!(stored.balance_stroops, 0);
        assert_eq!(stored.balance_lumens, 0.0);
    }

    #[tokio::test]
    async fn test_horizon_mock_sync_funded_account_updates_store() {
        let store = Arc::new(InMemoryWalletStore::default());
        let mock_horizon = Arc::new(MockHorizonClient::new());
        let service = WalletService::with_horizon(store.clone(), mock_horizon.clone());

        let user_id = Uuid::new_v4();
        let wallet = service
            .create_wallet(CreateWalletRequest {
                user_id,
                wallet_name: "Funded Wallet".to_string(),
                description: None,
                wallet_type: WalletType::Standard,
                secret_seed: None,
                set_as_primary: false,
            })
            .await
            .unwrap();

        // Initially wallet has 0 balance
        assert_eq!(wallet.balance_stroops, 0);

        // Fund account on Horizon with 250.75 XLM (2,507,500,000 stroops)
        mock_horizon
            .set_xlm_balance(&wallet.stellar_address, 2_507_500_000)
            .await;

        // Fetch balance via service
        let balance = service
            .get_balance(wallet.id, user_id)
            .await
            .expect("should fetch balance from Horizon");

        assert_eq!(balance.xlm_balance_stroops, 2_507_500_000);
        assert_eq!(balance.xlm_balance, 250.75);
        assert_eq!(balance.xlm_balance_formatted, "250.7500000");

        // Verify that wallet record in the store has been updated and is no longer stale!
        let stored_wallet = service
            .get_wallet(wallet.id, user_id)
            .await
            .expect("wallet should exist");

        assert_eq!(stored_wallet.balance_stroops, 2_507_500_000);
        assert_eq!(stored_wallet.balance_lumens, 250.75);
    }

    #[tokio::test]
    async fn test_horizon_mock_sync_multi_asset_account() {
        let store = Arc::new(InMemoryWalletStore::default());
        let mock_horizon = Arc::new(MockHorizonClient::new());
        let service = WalletService::with_horizon(store.clone(), mock_horizon.clone());

        let user_id = Uuid::new_v4();
        let wallet = service
            .create_wallet(CreateWalletRequest {
                user_id,
                wallet_name: "Multi-Asset Wallet".to_string(),
                description: None,
                wallet_type: WalletType::Standard,
                secret_seed: None,
                set_as_primary: false,
            })
            .await
            .unwrap();

        let account_balances = HorizonAccountBalances {
            balances: vec![
                HorizonBalanceItem {
                    balance: "100.0000000".to_string(),
                    asset_type: "native".to_string(),
                    asset_code: None,
                    asset_issuer: None,
                    limit: None,
                    buying_liabilities: None,
                    selling_liabilities: None,
                },
                HorizonBalanceItem {
                    balance: "50.1234567".to_string(),
                    asset_type: "credit_alphanum4".to_string(),
                    asset_code: Some("USDC".to_string()),
                    asset_issuer: Some("GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5".to_string()),
                    limit: Some("1000.0000000".to_string()),
                    buying_liabilities: None,
                    selling_liabilities: None,
                },
            ],
        };

        mock_horizon
            .set_account_balances(&wallet.stellar_address, account_balances)
            .await;

        let balance = service
            .get_balance(wallet.id, user_id)
            .await
            .expect("should fetch multi-asset balance");

        assert_eq!(balance.xlm_balance_stroops, 1_000_000_000);
        assert_eq!(balance.xlm_balance, 100.0);
        assert_eq!(balance.asset_balances.len(), 1);

        let usdc = &balance.asset_balances[0];
        assert_eq!(usdc.asset_code, "USDC");
        assert_eq!(usdc.balance_stroops, 501_234_567);
        assert_eq!(usdc.balance_formatted, "50.1234567");
        assert_eq!(usdc.limit_stroops, Some(10_000_000_000));
    }

    #[tokio::test]
    async fn test_balance_update_propagation() {
        let store = Arc::new(InMemoryWalletStore::default());
        let mock_horizon = Arc::new(MockHorizonClient::new());
        let service = WalletService::with_horizon(store.clone(), mock_horizon.clone());

        let user_id = Uuid::new_v4();
        let wallet = service
            .create_wallet(CreateWalletRequest {
                user_id,
                wallet_name: "Dynamic Wallet".to_string(),
                description: None,
                wallet_type: WalletType::Standard,
                secret_seed: None,
                set_as_primary: false,
            })
            .await
            .unwrap();

        // 1st fetch: 100 XLM
        mock_horizon
            .set_xlm_balance(&wallet.stellar_address, 1_000_000_000)
            .await;
        let b1 = service.get_balance(wallet.id, user_id).await.unwrap();
        assert_eq!(b1.xlm_balance_stroops, 1_000_000_000);

        // 2nd fetch after new deposit: 500.5 XLM
        mock_horizon
            .set_xlm_balance(&wallet.stellar_address, 5_005_000_000)
            .await;
        let b2 = service.get_balance(wallet.id, user_id).await.unwrap();
        assert_eq!(b2.xlm_balance_stroops, 5_005_000_000);

        // Verify stored wallet is updated to 500.5 XLM
        let stored = service.get_wallet(wallet.id, user_id).await.unwrap();
        assert_eq!(stored.balance_stroops, 5_005_000_000);
        assert_eq!(stored.balance_lumens, 500.5);
    }

    #[tokio::test]
    async fn test_unauthorized_balance_access() {
        let store = Arc::new(InMemoryWalletStore::default());
        let service = WalletService::new(store);

        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();

        let wallet = service
            .create_wallet(CreateWalletRequest {
                user_id: user1,
                wallet_name: "User 1 Wallet".to_string(),
                description: None,
                wallet_type: WalletType::Standard,
                secret_seed: None,
                set_as_primary: false,
            })
            .await
            .unwrap();

        let err = service.get_balance(wallet.id, user2).await.unwrap_err();
        match err {
            WalletError::Unauthorized(u, w) => {
                assert_eq!(u, user2);
                assert_eq!(w, wallet.id);
            }
            other => panic!("expected Unauthorized error, got {:?}", other),
        }
    }
}
