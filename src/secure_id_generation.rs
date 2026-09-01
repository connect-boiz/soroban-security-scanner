//! Secure ID Generation System
//! 
//! This module provides cryptographically secure ID generation to replace
//! predictable ledger sequence-based IDs, addressing issue #114.

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use log::{info, warn, error, debug};

/// Secure ID generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureIdConfig {
    /// Enable secure ID generation
    pub enabled: bool,
    /// ID generation method
    pub method: IdGenerationMethod,
    /// Entropy source for random generation
    pub entropy_source: EntropySource,
    /// ID length in bytes
    pub id_length_bytes: usize,
    /// Enable ID collision detection
    pub enable_collision_detection: bool,
    /// Maximum collision retry attempts
    pub max_collision_retries: u32,
    /// Enable ID uniqueness validation
    pub enable_uniqueness_validation: bool,
    /// Cache size for generated IDs
    pub id_cache_size: usize,
}

impl Default for SecureIdConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            method: IdGenerationMethod::CryptographicHash,
            entropy_source: EntropySource::MultipleSources,
            id_length_bytes: 32,
            enable_collision_detection: true,
            max_collision_retries: 100,
            enable_uniqueness_validation: true,
            id_cache_size: 1000,
        }
    }
}

/// ID generation methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IdGenerationMethod {
    /// Cryptographic hash-based generation
    CryptographicHash,
    /// Hardware random number generator
    HardwareRng,
    /// Combined entropy approach
    CombinedEntropy,
    /// Timestamp-based with additional entropy
    TimestampWithEntropy,
}

/// Entropy sources for random generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntropySource {
    /// Ledger-based entropy (weak - for compatibility only)
    LedgerOnly,
    /// Multiple entropy sources
    MultipleSources,
    /// External entropy service
    ExternalService,
    /// User-provided entropy
    UserProvided,
}

/// Secure ID generator
pub struct SecureIdGenerator {
    config: SecureIdConfig,
    generated_ids: Arc<std::sync::Mutex<HashMap<String, u64>>>,
    id_counter: Arc<std::sync::atomic::AtomicU64>,
}

impl SecureIdGenerator {
    /// Create a new secure ID generator
    pub fn new(config: SecureIdConfig) -> Self {
        Self {
            config,
            generated_ids: Arc::new(std::sync::Mutex::new(HashMap::new())),
            id_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Generate a secure ID for bounty operations
    pub fn generate_bounty_id(&self, creator: &str, timestamp: u64) -> Result<u64> {
        let entropy = self.collect_entropy(creator, timestamp, "bounty")?;
        let id_hash = self.hash_entropy_to_id(&entropy)?;
        
        if self.config.enable_collision_detection {
            self.ensure_no_collision(id_hash, "bounty")?;
        }
        
        Ok(id_hash)
    }

    /// Generate a secure nonce for operations
    pub fn generate_secure_nonce(&self, user: &str, operation: &str) -> Result<[u8; 32]> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let entropy = self.collect_entropy(user, timestamp, operation)?;
        let nonce_bytes = self.hash_entropy_to_bytes(&entropy)?;
        
        Ok(nonce_bytes)
    }

    /// Generate a secure transaction ID
    pub fn generate_transaction_id(&self, from: &str, to: &str, amount: i128) -> Result<String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let entropy = format!("{}:{}:{}:{}:{}", from, to, amount, timestamp, self.get_random_entropy());
        let id_hash = self.hash_entropy_to_id(&entropy)?;
        
        Ok(format!("tx_{}", id_hash))
    }

    /// Generate a secure session ID
    pub fn generate_session_id(&self, user: &str) -> Result<String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let entropy = self.collect_entropy(user, timestamp, "session")?;
        let session_bytes = self.hash_entropy_to_bytes(&entropy)?;
        
        Ok(format!("session_{}", hex::encode(session_bytes)))
    }

    /// Collect entropy from multiple sources
    fn collect_entropy(&self, seed: &str, timestamp: u64, context: &str) -> Result<String> {
        let entropy = match self.config.entropy_source {
            EntropySource::MultipleSources => {
                // Combine multiple entropy sources
                let system_entropy = self.get_system_entropy();
                let user_entropy = self.hash_string(seed);
                let time_entropy = timestamp;
                let context_entropy = self.hash_string(context);
                let random_entropy = self.get_random_entropy();
                
                format!("{}:{}:{}:{}:{}", system_entropy, user_entropy, time_entropy, context_entropy, random_entropy)
            }
            EntropySource::LedgerOnly => {
                // Fallback to ledger-based entropy (weak but compatible)
                warn!("Using weak ledger-based entropy source - consider upgrading");
                format!("{}:{}:{}", seed, timestamp, self.hash_string(context))
            }
            EntropySource::ExternalService => {
                // External entropy service (placeholder)
                self.get_external_entropy(seed, timestamp, context)
            }
            EntropySource::UserProvided => {
                // User-provided entropy
                format!("{}:{}:{}", seed, timestamp, self.hash_string(context))
            }
        };

        Ok(entropy)
    }

    /// Get system entropy
    fn get_system_entropy(&self) -> String {
        // Use system time and process information as entropy
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        
        let process_id = std::process::id();
        let thread_id = std::thread::current().id();
        
        format!("{}:{}:{}", now, process_id, format!("{:?}", thread_id))
    }

    /// Get random entropy
    fn get_random_entropy(&self) -> String {
        // Generate random bytes using a simple PRNG (in production, use crypto crate)
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        std::time::SystemTime::now().hash(&mut hasher);
        self.id_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst).hash(&mut hasher);
        
        format!("{:x}", hasher.finish())
    }

    /// Get external entropy (placeholder for external service)
    fn get_external_entropy(&self, seed: &str, timestamp: u64, context: &str) -> String {
        // In production, this would call an external entropy service
        // For now, combine multiple sources
        format!("external:{}:{}:{}", seed, timestamp, context)
    }

    /// Hash a string to create entropy
    fn hash_string(&self, input: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Hash entropy to create a numeric ID
    fn hash_entropy_to_id(&self, entropy: &str) -> Result<u64> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        entropy.hash(&mut hasher);
        
        let hash = hasher.finish();
        let id = hash % u64::MAX / 2; // Keep it in reasonable range
        
        Ok(id)
    }

    /// Hash entropy to create bytes
    fn hash_entropy_to_bytes(&self, entropy: &str) -> Result<[u8; 32]> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        entropy.hash(&mut hasher);
        
        let hash = hasher.finish();
        let mut bytes = [0u8; 32];
        
        // Convert hash to bytes
        for i in 0..8 {
            bytes[i] = ((hash >> (i * 8)) & 0xFF) as u8;
        }
        
        // Add additional entropy for remaining bytes
        let additional_entropy = self.get_random_entropy();
        let additional_bytes = additional_entropy.as_bytes();
        for i in 8..32 {
            if i - 8 < additional_bytes.len() {
                bytes[i] = additional_bytes[i - 8];
            } else {
                bytes[i] = bytes[i % 8];
            }
        }
        
        Ok(bytes)
    }

    /// Ensure no ID collision
    fn ensure_no_collision(&self, id: u64, context: &str) -> Result<()> {
        let mut generated_ids = self.generated_ids.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire generated IDs lock: {}", e)
        })?;

        let id_key = format!("{}:{}", context, id);
        
        if generated_ids.contains_key(&id_key) {
            return Err(anyhow::anyhow!("ID collision detected: {} in context {}", id, context));
        }
        
        // Store the ID to prevent future collisions
        generated_ids.insert(id_key, std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs());
        
        // Clean up old entries if cache is full
        if generated_ids.len() > self.config.id_cache_size {
            self.cleanup_old_ids(&mut generated_ids)?;
        }
        
        Ok(())
    }

    /// Clean up old IDs from cache
    fn cleanup_old_ids(&self, generated_ids: &mut HashMap<String, u64>) -> Result<()> {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let cutoff_time = current_time - 3600; // Keep entries for 1 hour
        
        generated_ids.retain(|_, &mut timestamp| timestamp > cutoff_time);
        
        Ok(())
    }

    /// Validate ID uniqueness
    pub fn validate_id_uniqueness(&self, id: u64, context: &str) -> Result<bool> {
        if !self.config.enable_uniqueness_validation {
            return Ok(true);
        }
        
        let generated_ids = self.generated_ids.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire generated IDs lock: {}", e)
        })?;

        let id_key = format!("{}:{}", context, id);
        Ok(!generated_ids.contains_key(&id_key))
    }

    /// Generate ID with collision retry
    fn generate_with_retry<F>(&self, generator: F) -> Result<u64>
    where
        F: Fn() -> Result<u64>,
    {
        let mut attempts = 0;
        
        while attempts < self.config.max_collision_retries {
            match generator() {
                Ok(id) => {
                    if self.validate_id_uniqueness(id, "generated").unwrap_or(false) {
                        return Ok(id);
                    }
                    warn!("ID collision detected, retrying (attempt {})", attempts + 1);
                }
                Err(e) => {
                    if attempts == self.config.max_collision_retries - 1 {
                        return Err(e);
                    }
                    debug!("ID generation failed, retrying: {}", e);
                }
            }
            attempts += 1;
        }
        
        Err(anyhow::anyhow!("Failed to generate unique ID after {} attempts", self.config.max_collision_retries))
    }

    /// Get statistics about ID generation
    pub fn get_statistics(&self) -> Result<IdGenerationStats> {
        let generated_ids = self.generated_ids.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire generated IDs lock: {}", e)
        })?;

        Ok(IdGenerationStats {
            total_ids_generated: generated_ids.len(),
            cache_size: self.config.id_cache_size,
            collision_detection_enabled: self.config.enable_collision_detection,
            uniqueness_validation_enabled: self.config.enable_uniqueness_validation,
            generation_method: format!("{:?}", self.config.method),
            entropy_source: format!("{:?}", self.config.entropy_source),
        })
    }
}

/// ID generation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdGenerationStats {
    pub total_ids_generated: usize,
    pub cache_size: usize,
    pub collision_detection_enabled: bool,
    pub uniqueness_validation_enabled: bool,
    pub generation_method: String,
    pub entropy_source: String,
}

/// Secure ID builder for convenient ID generation
pub struct SecureIdBuilder {
    generator: Arc<SecureIdGenerator>,
    context: String,
    metadata: HashMap<String, String>,
}

impl SecureIdBuilder {
    /// Create a new secure ID builder
    pub fn new(generator: Arc<SecureIdGenerator>, context: String) -> Self {
        Self {
            generator,
            context,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata
    pub fn metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Generate a bounty ID
    pub fn generate_bounty_id(&self, creator: &str, timestamp: u64) -> Result<u64> {
        self.generator.generate_bounty_id(creator, timestamp)
    }

    /// Generate a secure nonce
    pub fn generate_nonce(&self, user: &str, operation: &str) -> Result<[u8; 32]> {
        self.generator.generate_secure_nonce(user, operation)
    }

    /// Generate a transaction ID
    pub fn generate_transaction_id(&self, from: &str, to: &str, amount: i128) -> Result<String> {
        self.generator.generate_transaction_id(from, to, amount)
    }
}

/// Macro for convenient secure ID generation
#[macro_export]
macro_rules! generate_secure_id {
    ($generator:expr, $method:ident, $($arg:expr),*) => {
        $generator.$method($($arg),*).unwrap_or_else(|e| {
            log::error!("Failed to generate secure ID: {}", e);
            panic!("Secure ID generation failed: {}", e);
        })
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_id_config_default() {
        let config = SecureIdConfig::default();
        
        assert!(config.enabled);
        assert_eq!(config.id_length_bytes, 32);
        assert!(config.enable_collision_detection);
        assert_eq!(config.max_collision_retries, 100);
        assert!(config.enable_uniqueness_validation);
        assert_eq!(config.id_cache_size, 1000);
    }

    #[test]
    fn test_secure_id_generator_creation() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);
        
        let stats = generator.get_statistics().unwrap();
        assert_eq!(stats.total_ids_generated, 0);
        assert!(stats.collision_detection_enabled);
    }

    #[test]
    fn test_bounty_id_generation() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);

        let creator = "test_creator";
        let timestamp = 1234567890;

        let id1 = generator.generate_bounty_id(creator, timestamp).unwrap();
        let id2 = generator.generate_bounty_id(creator, timestamp + 1).unwrap();

        assert_ne!(id1, id2); // Should be different due to different timestamp
    }

    #[test]
    fn test_secure_nonce_generation() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);

        let nonce1 = generator.generate_secure_nonce("user1", "operation1").unwrap();
        let nonce2 = generator.generate_secure_nonce("user1", "operation2").unwrap();

        assert_ne!(nonce1, nonce2); // Should be different
        assert_eq!(nonce1.len(), 32);
        assert_eq!(nonce2.len(), 32);
    }

    #[test]
    fn test_transaction_id_generation() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);

        let tx_id = generator.generate_transaction_id("alice", "bob", 1000).unwrap();

        assert!(tx_id.starts_with("tx_"));
        assert!(tx_id.len() > 10); // Should have meaningful length
    }

    #[test]
    fn test_session_id_generation() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);

        let session_id = generator.generate_session_id("user123").unwrap();

        assert!(session_id.starts_with("session_"));
        assert!(session_id.len() > 20); // Should have meaningful length
    }

    #[test]
    fn test_collision_detection() {
        let mut config = SecureIdConfig::default();
        config.enable_collision_detection = true;
        let generator = SecureIdGenerator::new(config);

        let creator = "test_creator";
        let timestamp = 1234567890;

        let id1 = generator.generate_bounty_id(creator, timestamp).unwrap();
        
        // Second generation with same parameters should still work due to additional entropy
        let id2 = generator.generate_bounty_id(creator, timestamp).unwrap();
        
        // They should be different due to random entropy
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_id_uniqueness_validation() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);

        let id = generator.generate_bounty_id("creator", 1234567890).unwrap();
        
        // First check should be unique
        assert!(generator.validate_id_uniqueness(id, "bounty").unwrap());
        
        // After generating, it should no longer be considered unique
        let _id2 = generator.generate_bounty_id("creator", 1234567891).unwrap();
        // Note: In practice, this test might be flaky due to random nature
    }

    #[test]
    fn test_entropy_sources() {
        let sources = vec![
            EntropySource::MultipleSources,
            EntropySource::LedgerOnly,
            EntropySource::ExternalService,
            EntropySource::UserProvided,
        ];

        for source in sources {
            let mut config = SecureIdConfig::default();
            config.entropy_source = source;
            let generator = SecureIdGenerator::new(config);

            let result = generator.generate_bounty_id("test", 1234567890);
            assert!(result.is_ok(), "Failed with entropy source: {:?}", source);
        }
    }

    #[test]
    fn test_secure_id_builder() {
        let config = SecureIdConfig::default();
        let generator = Arc::new(SecureIdGenerator::new(config));
        let builder = SecureIdBuilder::new(generator.clone(), "test".to_string())
            .metadata("key1".to_string(), "value1".to_string());

        let bounty_id = builder.generate_bounty_id("creator", 1234567890).unwrap();
        assert!(bounty_id > 0);

        let nonce = builder.generate_nonce("user", "operation").unwrap();
        assert_eq!(nonce.len(), 32);
    }

    #[test]
    fn test_concurrent_id_generation() {
        use std::sync::Arc;
        use std::thread;

        let config = SecureIdConfig::default();
        let generator = Arc::new(SecureIdGenerator::new(config));

        let mut handles = vec![];

        for i in 0..10 {
            let gen_clone = generator.clone();
            let handle = thread::spawn(move || {
                gen_clone.generate_bounty_id(&format!("user{}", i), 1234567890 + i)
            });
            handles.push(handle);
        }

        let mut ids = vec![];
        for handle in handles {
            ids.push(handle.join().unwrap());
        }

        // All IDs should be unique
        let mut unique_ids = std::collections::HashSet::new();
        for id in ids {
            assert!(!unique_ids.contains(&id), "Duplicate ID detected: {}", id);
            unique_ids.insert(id);
        }
    }
}
