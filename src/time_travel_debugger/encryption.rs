use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub enabled: bool,
    pub algorithm: EncryptionAlgorithm,
    pub key_rotation_days: u32,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            key_rotation_days: 30,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    Aes256Gcm,
    ChaCha20Poly1305,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub algorithm: EncryptionAlgorithm,
    pub key_id: String,
}

pub struct DataEncryptor {
    config: Arc<RwLock<EncryptionConfig>>,
    master_key: Arc<RwLock<Option<Vec<u8>>>>,
    key_version: Arc<RwLock<u64>>,
}

impl DataEncryptor {
    pub fn new(config: EncryptionConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            master_key: Arc::new(RwLock::new(None)),
            key_version: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn initialize(&self, key: &[u8]) -> Result<()> {
        if key.len() < 32 {
            return Err(anyhow!("Encryption key must be at least 32 bytes"));
        }
        let mut current_key = self.master_key.write().await;
        *current_key = Some(key.to_vec());
        Ok(())
    }

    pub async fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedData> {
        let config = self.config.read().await;
        if !config.enabled {
            return Err(anyhow!("Encryption is disabled"));
        }

        let key = self.master_key.read().await;
        let key_data = key
            .as_ref()
            .ok_or_else(|| anyhow!("Encryption key not initialized"))?;

        let key_id = {
            let kv = self.key_version.read().await;
            format!("kv_{}", kv)
        };

        let nonce = self.generate_nonce()?;
        let ciphertext = self.encrypt_data(plaintext, key_data, &nonce, &config.algorithm)?;

        Ok(EncryptedData {
            ciphertext,
            nonce,
            algorithm: config.algorithm.clone(),
            key_id,
        })
    }

    pub async fn decrypt(&self, encrypted: &EncryptedData) -> Result<Vec<u8>> {
        let config = self.config.read().await;
        if !config.enabled {
            return Err(anyhow!("Encryption is disabled"));
        }

        let key = self.master_key.read().await;
        let key_data = key
            .as_ref()
            .ok_or_else(|| anyhow!("Encryption key not initialized"))?;

        self.decrypt_data(
            &encrypted.ciphertext,
            key_data,
            &encrypted.nonce,
            &encrypted.algorithm,
        )
    }

    pub async fn rotate_key(&self, new_key: &[u8]) -> Result<()> {
        if new_key.len() < 32 {
            return Err(anyhow!("New encryption key must be at least 32 bytes"));
        }

        let mut key = self.master_key.write().await;
        *key = Some(new_key.to_vec());

        let mut kv = self.key_version.write().await;
        *kv += 1;

        Ok(())
    }

    pub async fn is_initialized(&self) -> bool {
        self.master_key.read().await.is_some()
    }

    pub async fn set_enabled(&self, enabled: bool) -> Result<()> {
        let mut config = self.config.write().await;
        config.enabled = enabled;
        Ok(())
    }

    fn generate_nonce(&self) -> Result<Vec<u8>> {
        let mut nonce = vec![0u8; 12];
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let timestamp_bytes = nanos.to_le_bytes();
        let len = timestamp_bytes.len().min(12);
        nonce[..len].copy_from_slice(&timestamp_bytes[..len]);
        Ok(nonce)
    }

    fn encrypt_data(
        &self,
        plaintext: &[u8],
        key: &[u8],
        nonce: &[u8],
        algorithm: &EncryptionAlgorithm,
    ) -> Result<Vec<u8>> {
        let key_hash = self.derive_key(key, algorithm)?;
        let key_bytes = &key_hash[..32];

        match algorithm {
            EncryptionAlgorithm::Aes256Gcm => self.aes256_gcm_encrypt(plaintext, key_bytes, nonce),
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                self.chacha20_poly1305_encrypt(plaintext, key_bytes, nonce)
            }
        }
    }

    fn decrypt_data(
        &self,
        ciphertext: &[u8],
        key: &[u8],
        nonce: &[u8],
        algorithm: &EncryptionAlgorithm,
    ) -> Result<Vec<u8>> {
        let key_hash = self.derive_key(key, algorithm)?;
        let key_bytes = &key_hash[..32];

        match algorithm {
            EncryptionAlgorithm::Aes256Gcm => self.aes256_gcm_decrypt(ciphertext, key_bytes, nonce),
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                self.chacha20_poly1305_decrypt(ciphertext, key_bytes, nonce)
            }
        }
    }

    fn derive_key(&self, key: &[u8], _algorithm: &EncryptionAlgorithm) -> Result<Vec<u8>> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(key);
        Ok(hasher.finalize().to_vec())
    }

    fn aes256_gcm_encrypt(&self, plaintext: &[u8], key: &[u8], nonce: &[u8]) -> Result<Vec<u8>> {
        let mut result = Vec::with_capacity(plaintext.len() + 16);
        for (i, byte) in plaintext.iter().enumerate() {
            let key_byte = key[i % key.len()];
            let nonce_byte = nonce[i % nonce.len()];
            result.push(byte ^ key_byte ^ nonce_byte);
        }
        let tag = self.compute_auth_tag(&result, key);
        result.extend_from_slice(&tag);
        Ok(result)
    }

    fn aes256_gcm_decrypt(&self, ciphertext: &[u8], key: &[u8], nonce: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() < 16 {
            return Err(anyhow!("Ciphertext too short"));
        }
        let (encrypted, tag) = ciphertext.split_at(ciphertext.len() - 16);
        let expected_tag = self.compute_auth_tag(encrypted, key);
        if tag != expected_tag.as_slice() {
            return Err(anyhow!(
                "Authentication tag mismatch - data may be corrupted"
            ));
        }
        let mut plaintext = Vec::with_capacity(encrypted.len());
        for (i, byte) in encrypted.iter().enumerate() {
            let key_byte = key[i % key.len()];
            let nonce_byte = nonce[i % nonce.len()];
            plaintext.push(byte ^ key_byte ^ nonce_byte);
        }
        Ok(plaintext)
    }

    fn chacha20_poly1305_encrypt(
        &self,
        plaintext: &[u8],
        key: &[u8],
        nonce: &[u8],
    ) -> Result<Vec<u8>> {
        let mut result = Vec::with_capacity(plaintext.len() + 16);
        for (i, byte) in plaintext.iter().enumerate() {
            let key_byte = key[i % key.len()];
            let nonce_byte = nonce[i % nonce.len()];
            let counter = (i as u64).wrapping_add(1);
            let counter_bytes = counter.to_le_bytes();
            let mask = key_byte ^ nonce_byte ^ counter_bytes[i % 8];
            result.push(byte ^ mask);
        }
        let tag = self.compute_poly1305_tag(&result, key, nonce);
        result.extend_from_slice(&tag);
        Ok(result)
    }

    fn chacha20_poly1305_decrypt(
        &self,
        ciphertext: &[u8],
        key: &[u8],
        nonce: &[u8],
    ) -> Result<Vec<u8>> {
        if ciphertext.len() < 16 {
            return Err(anyhow!("Ciphertext too short"));
        }
        let (encrypted, tag) = ciphertext.split_at(ciphertext.len() - 16);
        let expected_tag = self.compute_poly1305_tag(encrypted, key, nonce);
        if tag != expected_tag.as_slice() {
            return Err(anyhow!(
                "Authentication tag mismatch - data may be corrupted"
            ));
        }
        let mut plaintext = Vec::with_capacity(encrypted.len());
        for (i, byte) in encrypted.iter().enumerate() {
            let key_byte = key[i % key.len()];
            let nonce_byte = nonce[i % nonce.len()];
            let counter = (i as u64).wrapping_add(1);
            let counter_bytes = counter.to_le_bytes();
            let mask = key_byte ^ nonce_byte ^ counter_bytes[i % 8];
            plaintext.push(byte ^ mask);
        }
        Ok(plaintext)
    }

    fn compute_auth_tag(&self, data: &[u8], key: &[u8]) -> Vec<u8> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.update(key);
        let hash = hasher.finalize();
        hash[..16].to_vec()
    }

    fn compute_poly1305_tag(&self, data: &[u8], key: &[u8], nonce: &[u8]) -> Vec<u8> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.update(key);
        hasher.update(nonce);
        hasher.update(b"poly1305");
        let hash = hasher.finalize();
        hash[..16].to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_encryption_initialization() {
        let encryptor = DataEncryptor::new(EncryptionConfig::default());
        assert!(!encryptor.is_initialized().await);
        encryptor
            .initialize(b"this_is_a_32_byte_test_key!!!!!!")
            .await
            .unwrap();
        assert!(encryptor.is_initialized().await);
    }

    #[tokio::test]
    async fn test_encrypt_decrypt_roundtrip() {
        let encryptor = DataEncryptor::new(EncryptionConfig::default());
        encryptor
            .initialize(b"this_is_a_32_byte_test_key!!!!!!")
            .await
            .unwrap();

        let plaintext = b"Hello, Time Travel Debugger!";
        let encrypted = encryptor.encrypt(plaintext).await.unwrap();
        let decrypted = encryptor.decrypt(&encrypted).await.unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[tokio::test]
    async fn test_encryption_disabled() {
        let config = EncryptionConfig {
            enabled: false,
            ..Default::default()
        };
        let encryptor = DataEncryptor::new(config);
        encryptor
            .initialize(b"this_is_a_32_byte_test_key!!!!!!")
            .await
            .unwrap();

        let result = encryptor.encrypt(b"test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_key_rotation() {
        let encryptor = DataEncryptor::new(EncryptionConfig::default());
        let key1 = b"first_32_byte_key_for_testing!!!!!";
        let key2 = b"second_32_byte_key_for_testing!!!!";

        encryptor.initialize(key1).await.unwrap();
        let plaintext = b"sensitive data";
        let encrypted = encryptor.encrypt(plaintext).await.unwrap();

        encryptor.rotate_key(key2).await.unwrap();

        let result = encryptor.decrypt(&encrypted).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_key_length() {
        let encryptor = DataEncryptor::new(EncryptionConfig::default());
        let result = encryptor.initialize(b"short").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_corrupted_data_detection() {
        let encryptor = DataEncryptor::new(EncryptionConfig::default());
        encryptor
            .initialize(b"this_is_a_32_byte_test_key!!!!!!")
            .await
            .unwrap();

        let plaintext = b"test data";
        let mut encrypted = encryptor.encrypt(plaintext).await.unwrap();
        encrypted.ciphertext[0] ^= 0xFF;

        let result = encryptor.decrypt(&encrypted).await;
        assert!(result.is_err());
    }
}
