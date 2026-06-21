use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;

#[cfg(feature = "redis-cache")]
use redis::{AsyncCommands, Client as RedisClient};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockoutConfig {
    pub max_attempts: u32,
    pub window_minutes: u64,
    pub lockout_duration_minutes: u64,
    pub progressive_lockout: bool,
    pub lockout_multipliers: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedAttempt {
    pub timestamp: chrono::DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockoutRecord {
    pub user_id: String,
    pub failed_attempts: Vec<FailedAttempt>,
    pub current_lockout: Option<chrono::DateTime<Utc>>,
    pub total_attempts: u32,
    pub lockout_count: u32,
    pub last_attempt: chrono::DateTime<Utc>,
    pub is_permanently_locked: bool,
    pub permanent_lock_reason: Option<String>,
}

#[derive(Debug, Error)]
pub enum LockoutError {
    #[error("Account is locked: {0}")]
    AccountLocked(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Redis error: {0}")]
    Redis(String),
}

#[async_trait::async_trait]
pub trait LockoutStore: Send + Sync {
    async fn get_record(&self, user_id: &str) -> Result<Option<LockoutRecord>, LockoutError>;
    async fn save_record(&self, record: LockoutRecord) -> Result<(), LockoutError>;
    async fn delete_record(&self, user_id: &str) -> Result<(), LockoutError>;
    async fn cleanup_expired(&self) -> Result<usize, LockoutError>;
}

pub struct InMemoryLockoutStore {
    records: Arc<RwLock<HashMap<String, LockoutRecord>>>,
}

impl InMemoryLockoutStore {
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl LockoutStore for InMemoryLockoutStore {
    async fn get_record(&self, user_id: &str) -> Result<Option<LockoutRecord>, LockoutError> {
        let records = self
            .records
            .read()
            .map_err(|e| LockoutError::Storage(e.to_string()))?;
        Ok(records.get(user_id).cloned())
    }

    async fn save_record(&self, record: LockoutRecord) -> Result<(), LockoutError> {
        let mut records = self
            .records
            .write()
            .map_err(|e| LockoutError::Storage(e.to_string()))?;
        records.insert(record.user_id.clone(), record);
        Ok(())
    }

    async fn delete_record(&self, user_id: &str) -> Result<(), LockoutError> {
        let mut records = self
            .records
            .write()
            .map_err(|e| LockoutError::Storage(e.to_string()))?;
        records.remove(user_id);
        Ok(())
    }

    async fn cleanup_expired(&self) -> Result<usize, LockoutError> {
        let mut records = self
            .records
            .write()
            .map_err(|e| LockoutError::Storage(e.to_string()))?;
        
        let now = Utc::now();
        let mut count = 0;
        
        records.retain(|_, record| {
            let should_keep = !record.failed_attempts.is_empty() &&
                             record.failed_attempts.iter().any(|attempt| 
                                 attempt.timestamp > now - Duration::hours(24)) ||
                             record.current_lockout.map_or(false, |lockout| lockout > now);
            if !should_keep {
                count += 1;
            }
            should_keep
        });
        
        Ok(count)
    }
}

#[cfg(feature = "redis-cache")]
pub struct RedisLockoutStore {
    client: RedisClient,
    key_prefix: String,
}

#[cfg(feature = "redis-cache")]
impl RedisLockoutStore {
    pub fn new(client: RedisClient, key_prefix: String) -> Self {
        Self { client, key_prefix }
    }

    fn record_key(&self, user_id: &str) -> String {
        format!("{}:lockout:{}", self.key_prefix, user_id)
    }
}

#[cfg(feature = "redis-cache")]
#[async_trait::async_trait]
impl LockoutStore for RedisLockoutStore {
    async fn get_record(&self, user_id: &str) -> Result<Option<LockoutRecord>, LockoutError> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| LockoutError::Redis(e.to_string()))?;

        let record_key = self.record_key(user_id);
        let record_json: Option<String> = conn
            .get(&record_key)
            .await
            .map_err(|e| LockoutError::Redis(e.to_string()))?;

        match record_json {
            Some(json) => {
                let record = serde_json::from_str(&json)
                    .map_err(|e| LockoutError::Serialization(e.to_string()))?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    async fn save_record(&self, record: LockoutRecord) -> Result<(), LockoutError> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| LockoutError::Redis(e.to_string()))?;

        let record_json = serde_json::to_string(&record)
            .map_err(|e| LockoutError::Serialization(e.to_string()))?;

        let record_key = self.record_key(&record.user_id);
        
        // Set with TTL of 24 hours
        conn.set_ex(&record_key, record_json, 86400)
            .await
            .map_err(|e| LockoutError::Redis(e.to_string()))?;

        Ok(())
    }

    async fn delete_record(&self, user_id: &str) -> Result<(), LockoutError> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| LockoutError::Redis(e.to_string()))?;

        let record_key = self.record_key(user_id);
        conn.del(&record_key)
            .await
            .map_err(|e| LockoutError::Redis(e.to_string()))?;

        Ok(())
    }

    async fn cleanup_expired(&self) -> Result<usize, LockoutError> {
        // Redis handles TTL automatically, so this is a no-op
        Ok(0)
    }
}

pub struct AccountLockoutService<S: LockoutStore> {
    store: S,
    configs: HashMap<String, LockoutConfig>,
}

impl<S: LockoutStore> AccountLockoutService<S> {
    pub fn new(store: S) -> Self {
        Self {
            store,
            configs: HashMap::new(),
        }
    }

    pub fn add_config(&mut self, name: &str, config: LockoutConfig) -> Result<(), LockoutError> {
        if config.max_attempts == 0 || config.window_minutes == 0 || config.lockout_duration_minutes == 0 {
            return Err(LockoutError::InvalidConfig(
                "max_attempts, window_minutes, and lockout_duration_minutes must be greater than 0".to_string(),
            ));
        }
        self.configs.insert(name.to_string(), config);
        Ok(())
    }

    pub async fn record_failed_attempt(
        &self,
        user_id: &str,
        config_name: &str,
        ip_address: Option<String>,
        user_agent: Option<String>,
        reason: String,
    ) -> Result<LockoutResult, LockoutError> {
        let config = self
            .configs
            .get(config_name)
            .ok_or_else(|| LockoutError::InvalidConfig(format!("Config '{}' not found", config_name)))?;

        let now = Utc::now();
        let window_start = now - Duration::minutes(config.window_minutes as i64);

        let mut record = match self.store.get_record(user_id).await? {
            Some(r) => r,
            None => LockoutRecord {
                user_id: user_id.to_string(),
                failed_attempts: Vec::new(),
                current_lockout: None,
                total_attempts: 0,
                lockout_count: 0,
                last_attempt: now,
                is_permanently_locked: false,
                permanent_lock_reason: None,
            },
        };

        // Check if permanently locked
        if record.is_permanently_locked {
            return Ok(LockoutResult {
                is_locked: true,
                lockout_duration: None,
                lockout_reason: record.permanent_lock_reason.clone(),
                remaining_attempts: 0,
                total_attempts: record.total_attempts,
                lockout_count: record.lockout_count,
                lockout_expires_at: None,
            });
        }

        // Clean old attempts outside the window
        record.failed_attempts.retain(|attempt| attempt.timestamp > window_start);

        // Add new failed attempt
        record.failed_attempts.push(FailedAttempt {
            timestamp: now,
            ip_address,
            user_agent,
            reason: reason.clone(),
        });
        record.total_attempts += 1;
        record.last_attempt = now;

        // Check if should be locked
        let recent_attempts = record.failed_attempts.len() as u32;
        let remaining_attempts = config.max_attempts.saturating_sub(recent_attempts);

        if recent_attempts >= config.max_attempts {
            record.lockout_count += 1;

            // Calculate lockout duration
            let lockout_duration = if config.progressive_lockout && !config.lockout_multipliers.is_empty() {
                let multiplier_index = (record.lockout_count as usize - 1).min(config.lockout_multipliers.len() - 1);
                let multiplier = config.lockout_multipliers[multiplier_index];
                config.lockout_duration_minutes * multiplier
            } else {
                config.lockout_duration_minutes
            };

            let lockout_expires = now + Duration::minutes(lockout_duration as i64);
            record.current_lockout = Some(lockout_expires);

            self.store.save_record(record.clone()).await?;

            Ok(LockoutResult {
                is_locked: true,
                lockout_duration: Some(lockout_duration),
                lockout_reason: Some(format!("Too many failed attempts: {}", reason)),
                remaining_attempts: 0,
                total_attempts: record.total_attempts,
                lockout_count: record.lockout_count,
                lockout_expires_at: Some(lockout_expires),
            })
        } else {
            self.store.save_record(record.clone()).await?;

            Ok(LockoutResult {
                is_locked: false,
                lockout_duration: None,
                lockout_reason: None,
                remaining_attempts,
                total_attempts: record.total_attempts,
                lockout_count: record.lockout_count,
                lockout_expires_at: None,
            })
        }
    }

    pub async fn check_account_status(&self, user_id: &str) -> Result<LockoutStatus, LockoutError> {
        let record = match self.store.get_record(user_id).await? {
            Some(r) => r,
            None => {
                return Ok(LockoutStatus {
                    is_locked: false,
                    is_permanently_locked: false,
                    lockout_expires_at: None,
                    remaining_attempts: None,
                    total_attempts: 0,
                    lockout_count: 0,
                    last_attempt: None,
                    recent_attempts: Vec::new(),
                });
            }
        };

        let now = Utc::now();
        let is_locked = record.current_lockout.map_or(false, |lockout| lockout > now);

        Ok(LockoutStatus {
            is_locked,
            is_permanently_locked: record.is_permanently_locked,
            lockout_expires_at: record.current_lockout,
            remaining_attempts: None, // Would need config to calculate
            total_attempts: record.total_attempts,
            lockout_count: record.lockout_count,
            last_attempt: Some(record.last_attempt),
            recent_attempts: record.failed_attempts,
        })
    }

    pub async fn can_attempt_login(&self, user_id: &str, config_name: &str) -> Result<bool, LockoutError> {
        let config = self
            .configs
            .get(config_name)
            .ok_or_else(|| LockoutError::InvalidConfig(format!("Config '{}' not found", config_name)))?;

        let record = match self.store.get_record(user_id).await? {
            Some(r) => r,
            None => return Ok(true),
        };

        // Check permanent lockout
        if record.is_permanently_locked {
            return Ok(false);
        }

        // Check temporary lockout
        if let Some(lockout_expires) = record.current_lockout {
            if lockout_expires > Utc::now() {
                return Ok(false);
            }
        }

        // Check recent attempts
        let window_start = Utc::now() - Duration::minutes(config.window_minutes as i64);
        let recent_attempts = record.failed_attempts.iter()
            .filter(|attempt| attempt.timestamp > window_start)
            .count() as u32;

        Ok(recent_attempts < config.max_attempts)
    }

    pub async fn reset_failed_attempts(&self, user_id: &str) -> Result<(), LockoutError> {
        let mut record = match self.store.get_record(user_id).await? {
            Some(r) => r,
            None => return Ok(()), // Nothing to reset
        };

        record.failed_attempts.clear();
        record.current_lockout = None;
        record.last_attempt = Utc::now();

        self.store.save_record(record).await?;
        Ok(())
    }

    pub async fn permanently_lock_account(&self, user_id: &str, reason: String) -> Result<(), LockoutError> {
        let mut record = match self.store.get_record(user_id).await? {
            Some(r) => r,
            None => LockoutRecord {
                user_id: user_id.to_string(),
                failed_attempts: Vec::new(),
                current_lockout: None,
                total_attempts: 0,
                lockout_count: 0,
                last_attempt: Utc::now(),
                is_permanently_locked: false,
                permanent_lock_reason: None,
            },
        };

        record.is_permanently_locked = true;
        record.permanent_lock_reason = Some(reason);

        self.store.save_record(record).await?;
        Ok(())
    }

    pub async fn unlock_account(&self, user_id: &str) -> Result<(), LockoutError> {
        let mut record = match self.store.get_record(user_id).await? {
            Some(r) => r,
            None => return Ok(()), // Nothing to unlock
        };

        record.failed_attempts.clear();
        record.current_lockout = None;
        record.is_permanently_locked = false;
        record.permanent_lock_reason = None;
        record.last_attempt = Utc::now();

        self.store.save_record(record).await?;
        Ok(())
    }

    pub async fn cleanup_expired(&self) -> Result<usize, LockoutError> {
        self.store.cleanup_expired().await
    }

    pub fn get_configs(&self) -> &HashMap<String, LockoutConfig> {
        &self.configs
    }
}

#[derive(Debug, Clone)]
pub struct LockoutResult {
    pub is_locked: bool,
    pub lockout_duration: Option<u64>,
    pub lockout_reason: Option<String>,
    pub remaining_attempts: u32,
    pub total_attempts: u32,
    pub lockout_count: u32,
    pub lockout_expires_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct LockoutStatus {
    pub is_locked: bool,
    pub is_permanently_locked: bool,
    pub lockout_expires_at: Option<chrono::DateTime<Utc>>,
    pub remaining_attempts: Option<u32>,
    pub total_attempts: u32,
    pub lockout_count: u32,
    pub last_attempt: Option<chrono::DateTime<Utc>>,
    pub recent_attempts: Vec<FailedAttempt>,
}

impl LockoutConfig {
    pub fn new(max_attempts: u32, window_minutes: u64, lockout_duration_minutes: u64) -> Self {
        Self {
            max_attempts,
            window_minutes,
            lockout_duration_minutes,
            progressive_lockout: false,
            lockout_multipliers: Vec::new(),
        }
    }

    pub fn with_progressive_lockout(mut self, multipliers: Vec<u32>) -> Self {
        self.progressive_lockout = true;
        self.lockout_multipliers = multipliers;
        self
    }

    // Common configurations
    pub fn strict() -> Self {
        Self::new(3, 15, 30).with_progressive_lockout(vec![1, 2, 4, 8]) // 3 attempts in 15 min, progressive lockout
    }

    pub fn moderate() -> Self {
        Self::new(5, 30, 15) // 5 attempts in 30 min, 15 min lockout
    }

    pub fn lenient() -> Self {
        Self::new(10, 60, 5) // 10 attempts in 1 hour, 5 min lockout
    }

    pub fn api() -> Self {
        Self::new(20, 60, 60).with_progressive_lockout(vec![1, 2, 4]) // API endpoints
    }

    pub fn admin() -> Self {
        Self::new(2, 5, 60).with_progressive_lockout(vec![1, 3, 6]) // Admin accounts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_basic_account_lockout() {
        let store = InMemoryLockoutStore::new();
        let mut service = AccountLockoutService::new(store);
        
        service.add_config("test", LockoutConfig::new(3, 60, 30)).unwrap();

        // First 2 attempts should not lock the account
        for i in 0..2 {
            let result = service.record_failed_attempt(
                "user1", 
                "test", 
                Some("127.0.0.1".to_string()), 
                None, 
                "Invalid password".to_string()
            ).await.unwrap();
            assert!(!result.is_locked);
            assert_eq!(result.remaining_attempts, 2 - i);
        }

        // 3rd attempt should lock the account
        let result = service.record_failed_attempt(
            "user1", 
            "test", 
            Some("127.0.0.1".to_string()), 
            None, 
            "Invalid password".to_string()
        ).await.unwrap();
        assert!(result.is_locked);
        assert_eq!(result.lockout_duration, Some(30));
        assert_eq!(result.lockout_count, 1);
    }

    #[tokio::test]
    async fn test_progressive_lockout() {
        let store = InMemoryLockoutStore::new();
        let mut service = AccountLockoutService::new(store);
        
        service.add_config("test", LockoutConfig::new(2, 60, 10).with_progressive_lockout(vec![1, 2, 3])).unwrap();

        // First lockout
        service.record_failed_attempt("user1", "test", None, None, "Invalid".to_string()).await.unwrap();
        let result1 = service.record_failed_attempt("user1", "test", None, None, "Invalid".to_string()).await.unwrap();
        assert!(result1.is_locked);
        assert_eq!(result1.lockout_duration, Some(10));

        // Reset and trigger second lockout
        service.unlock_account("user1").await.unwrap();
        service.record_failed_attempt("user1", "test", None, None, "Invalid".to_string()).await.unwrap();
        let result2 = service.record_failed_attempt("user1", "test", None, None, "Invalid".to_string()).await.unwrap();
        assert!(result2.is_locked);
        assert_eq!(result2.lockout_duration, Some(20)); // 10 * 2
        assert_eq!(result2.lockout_count, 2);
    }

    #[tokio::test]
    async fn test_permanent_lockout() {
        let store = InMemoryLockoutStore::new();
        let mut service = AccountLockoutService::new(store);
        
        service.add_config("test", LockoutConfig::new(5, 60, 30)).unwrap();

        // Permanently lock account
        service.permanently_lock_account("user1", "Suspicious activity detected".to_string()).await.unwrap();

        // Should not be able to attempt login
        let can_attempt = service.can_attempt_login("user1", "test").await.unwrap();
        assert!(!can_attempt);

        let status = service.check_account_status("user1").await.unwrap();
        assert!(status.is_permanently_locked);
        assert_eq!(status.permanent_lock_reason, Some("Suspicious activity detected".to_string()));
    }

    #[tokio::test]
    async fn test_reset_failed_attempts() {
        let store = InMemoryLockoutStore::new();
        let mut service = AccountLockoutService::new(store);
        
        service.add_config("test", LockoutConfig::new(3, 60, 30)).unwrap();

        // Add failed attempts
        service.record_failed_attempt("user1", "test", None, None, "Invalid".to_string()).await.unwrap();
        service.record_failed_attempt("user1", "test", None, None, "Invalid".to_string()).await.unwrap();

        // Reset
        service.reset_failed_attempts("user1").await.unwrap();

        // Should be able to attempt login again
        let can_attempt = service.can_attempt_login("user1", "test").await.unwrap();
        assert!(can_attempt);

        let status = service.check_account_status("user1").await.unwrap();
        assert_eq!(status.total_attempts, 2); // Total attempts preserved
        assert_eq!(status.recent_attempts.len(), 0); // Recent attempts cleared
    }

    #[tokio::test]
    async fn test_window_expiration() {
        let store = InMemoryLockoutStore::new();
        let mut service = AccountLockoutService::new(store);
        
        service.add_config("test", LockoutConfig::new(3, 1, 30)).unwrap(); // 1 minute window

        // Add attempts
        service.record_failed_attempt("user1", "test", None, None, "Invalid".to_string()).await.unwrap();
        service.record_failed_attempt("user1", "test", None, None, "Invalid".to_string()).await.unwrap();

        // Wait for window to expire (in real tests, you'd use a mock clock)
        // For now, just test the logic structure
        
        let can_attempt = service.can_attempt_login("user1", "test").await.unwrap();
        assert!(can_attempt); // Should still be able to attempt (2 < 3)
    }
}
