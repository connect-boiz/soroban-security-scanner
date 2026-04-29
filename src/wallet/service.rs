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

use crate::wallet::{
    crypto::{
        compute_export_hmac, decrypt_seed, encrypt_seed, generate_hmac_key, verify_export_hmac,
    },
    types::{
        AssetBalance, CreateWalletRequest, ImportWalletRequest, RestoreWalletRequest, Wallet,
        WalletBalance, WalletError, WalletExport, WalletStatus, WalletSyncRecord, WalletType,
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
    async fn get_sync_records(&self, wallet_id: Uuid) -> Result<Vec<WalletSyncRecord>, WalletError>;

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
        if store.values().any(|w| w.stellar_address == wallet.stellar_address) {
            return Err(WalletError::DuplicateAddress(wallet.stellar_address.clone()));
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

    async fn get_sync_records(&self, wallet_id: Uuid) -> Result<Vec<WalletSyncRecord>, WalletError> {
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
// Wallet Service
// ---------------------------------------------------------------------------

pub struct WalletService {
    store: Arc<dyn WalletStore>,
}

impl WalletService {
    pub fn new(store: Arc<dyn WalletStore>) -> Self {
        Self { store }
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

    /// Fetch the current balance from Stellar Horizon (mock implementation).
    /// In production, replace with a real Horizon HTTP call.
    pub async fn get_balance(&self, wallet_id: Uuid, user_id: Uuid) -> Result<WalletBalance, WalletError> {
        let wallet = self.get_owned_wallet(wallet_id, user_id).await?;
        // TODO: replace with real Horizon API call
        Ok(WalletBalance {
            stellar_address: wallet.stellar_address.clone(),
            xlm_balance: wallet.balance_lumens,
            asset_balances: vec![],
            fetched_at: Utc::now(),
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

    pub async fn freeze_wallet(&self, wallet_id: Uuid, user_id: Uuid) -> Result<Wallet, WalletError> {
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

    async fn get_owned_wallet(&self, wallet_id: Uuid, user_id: Uuid) -> Result<Wallet, WalletError> {
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
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

        let mut seed = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut seed);

        // Encode as Stellar strkey: version byte + payload + checksum, base32
        let secret_address = stellar_strkey_encode(&seed, 0x90); // S... secret seed
        let key_pair = Ed25519KeyPair::from_seed_unchecked(&seed)
            .expect("valid 32-byte seed");
        let public_bytes: &[u8] = key_pair.public_key().as_ref();
        let public_address = stellar_strkey_encode(public_bytes, 0x30); // G... public key

        (public_address, secret_address)
    }

    /// Parse a Stellar secret seed (S...) and return (public_address, secret_seed).
    fn parse_keypair(&self, seed: &str) -> Result<(String, String), WalletError> {
        use ring::signature::{Ed25519KeyPair, KeyPair};

        let raw = stellar_strkey_decode(seed, 0x90)
            .map_err(|_| WalletError::InvalidSecretSeed)?;
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
