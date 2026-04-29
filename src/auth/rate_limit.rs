use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;

#[cfg(feature = "redis-cache")]
use redis::{AsyncCommands, Client as RedisClient};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub max_requests: u32,
    pub window_seconds: u64,
    pub burst_size: Option<u32>,
    pub penalty_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitRecord {
    pub key: String,
    pub requests: Vec<chrono::DateTime<Utc>>,
    pub blocked_until: Option<chrono::DateTime<Utc>>,
    pub total_requests: u64,
    pub last_request: chrono::DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum RateLimitError {
    #[error("Rate limit exceeded for key: {0}")]
    RateLimitExceeded(String),
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
pub trait RateLimitStore: Send + Sync {
    async fn get_record(&self, key: &str) -> Result<Option<RateLimitRecord>, RateLimitError>;
    async fn save_record(&self, record: RateLimitRecord) -> Result<(), RateLimitError>;
    async fn delete_record(&self, key: &str) -> Result<(), RateLimitError>;
    async fn cleanup_expired(&self) -> Result<usize, RateLimitError>;
}

pub struct InMemoryRateLimitStore {
    records: Arc<RwLock<HashMap<String, RateLimitRecord>>>,
}

impl InMemoryRateLimitStore {
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl RateLimitStore for InMemoryRateLimitStore {
    async fn get_record(&self, key: &str) -> Result<Option<RateLimitRecord>, RateLimitError> {
        let records = self
            .records
            .read()
            .map_err(|e| RateLimitError::Storage(e.to_string()))?;
        Ok(records.get(key).cloned())
    }

    async fn save_record(&self, record: RateLimitRecord) -> Result<(), RateLimitError> {
        let mut records = self
            .records
            .write()
            .map_err(|e| RateLimitError::Storage(e.to_string()))?;
        records.insert(record.key.clone(), record);
        Ok(())
    }

    async fn delete_record(&self, key: &str) -> Result<(), RateLimitError> {
        let mut records = self
            .records
            .write()
            .map_err(|e| RateLimitError::Storage(e.to_string()))?;
        records.remove(key);
        Ok(())
    }

    async fn cleanup_expired(&self) -> Result<usize, RateLimitError> {
        let mut records = self
            .records
            .write()
            .map_err(|e| RateLimitError::Storage(e.to_string()))?;
        
        let now = Utc::now();
        let mut count = 0;
        
        records.retain(|_, record| {
            let should_keep = record.requests.iter().any(|&req| req > now - Duration::seconds(3600)) ||
                             record.blocked_until.map_or(false, |blocked| blocked > now);
            if !should_keep {
                count += 1;
            }
            should_keep
        });
        
        Ok(count)
    }
}

#[cfg(feature = "redis-cache")]
pub struct RedisRateLimitStore {
    client: RedisClient,
    key_prefix: String,
}

#[cfg(feature = "redis-cache")]
impl RedisRateLimitStore {
    pub fn new(client: RedisClient, key_prefix: String) -> Self {
        Self { client, key_prefix }
    }

    fn record_key(&self, key: &str) -> String {
        format!("{}:rate_limit:{}", self.key_prefix, key)
    }
}

#[cfg(feature = "redis-cache")]
#[async_trait::async_trait]
impl RateLimitStore for RedisRateLimitStore {
    async fn get_record(&self, key: &str) -> Result<Option<RateLimitRecord>, RateLimitError> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| RateLimitError::Redis(e.to_string()))?;

        let record_key = self.record_key(key);
        let record_json: Option<String> = conn
            .get(&record_key)
            .await
            .map_err(|e| RateLimitError::Redis(e.to_string()))?;

        match record_json {
            Some(json) => {
                let record = serde_json::from_str(&json)
                    .map_err(|e| RateLimitError::Serialization(e.to_string()))?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    async fn save_record(&self, record: RateLimitRecord) -> Result<(), RateLimitError> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| RateLimitError::Redis(e.to_string()))?;

        let record_json = serde_json::to_string(&record)
            .map_err(|e| RateLimitError::Serialization(e.to_string()))?;

        let record_key = self.record_key(&record.key);
        
        // Set with TTL of 1 hour
        conn.set_ex(&record_key, record_json, 3600)
            .await
            .map_err(|e| RateLimitError::Redis(e.to_string()))?;

        Ok(())
    }

    async fn delete_record(&self, key: &str) -> Result<(), RateLimitError> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| RateLimitError::Redis(e.to_string()))?;

        let record_key = self.record_key(key);
        conn.del(&record_key)
            .await
            .map_err(|e| RateLimitError::Redis(e.to_string()))?;

        Ok(())
    }

    async fn cleanup_expired(&self) -> Result<usize, RateLimitError> {
        // Redis handles TTL automatically, so this is a no-op
        Ok(0)
    }
}

pub struct RateLimitService<S: RateLimitStore> {
    store: S,
    configs: HashMap<String, RateLimitConfig>,
}

impl<S: RateLimitStore> RateLimitService<S> {
    pub fn new(store: S) -> Self {
        Self {
            store,
            configs: HashMap::new(),
        }
    }

    pub fn add_config(&mut self, name: &str, config: RateLimitConfig) -> Result<(), RateLimitError> {
        if config.max_requests == 0 || config.window_seconds == 0 {
            return Err(RateLimitError::InvalidConfig(
                "max_requests and window_seconds must be greater than 0".to_string(),
            ));
        }
        self.configs.insert(name.to_string(), config);
        Ok(())
    }

    pub async fn check_rate_limit(
        &self,
        key: &str,
        config_name: &str,
    ) -> Result<RateLimitResult, RateLimitError> {
        let config = self
            .configs
            .get(config_name)
            .ok_or_else(|| RateLimitError::InvalidConfig(format!("Config '{}' not found", config_name)))?;

        let now = Utc::now();
        let window_start = now - Duration::seconds(config.window_seconds as i64);

        let mut record = match self.store.get_record(key).await? {
            Some(r) => r,
            None => RateLimitRecord {
                key: key.to_string(),
                requests: Vec::new(),
                blocked_until: None,
                total_requests: 0,
                last_request: now,
            },
        };

        // Check if currently blocked
        if let Some(blocked_until) = record.blocked_until {
            if blocked_until > now {
                return Ok(RateLimitResult {
                    allowed: false,
                    remaining: 0,
                    reset_time: blocked_until,
                    total_requests: record.total_requests,
                    retry_after: (blocked_until - now).num_seconds() as u64,
                });
            } else {
                // Block period expired
                record.blocked_until = None;
            }
        }

        // Clean old requests outside the window
        record.requests.retain(|&req| req > window_start);

        // Check if over limit
        if record.requests.len() >= config.max_requests as usize {
            // Apply penalty if configured
            if let Some(penalty_seconds) = config.penalty_seconds {
                record.blocked_until = Some(now + Duration::seconds(penalty_seconds as i64));
                self.store.save_record(record.clone()).await?;
                
                return Ok(RateLimitResult {
                    allowed: false,
                    remaining: 0,
                    reset_time: record.blocked_until.unwrap(),
                    total_requests: record.total_requests,
                    retry_after: penalty_seconds,
                });
            }

            return Ok(RateLimitResult {
                allowed: false,
                remaining: 0,
                reset_time: record.requests[0] + Duration::seconds(config.window_seconds as i64),
                total_requests: record.total_requests,
                retry_after: (record.requests[0] + Duration::seconds(config.window_seconds as i64) - now).num_seconds() as u64,
            });
        }

        // Add current request
        record.requests.push(now);
        record.total_requests += 1;
        record.last_request = now;

        self.store.save_record(record.clone()).await?;

        let remaining = config.max_requests - record.requests.len() as u32;
        let reset_time = record.requests[0] + Duration::seconds(config.window_seconds as i64);

        Ok(RateLimitResult {
            allowed: true,
            remaining,
            reset_time,
            total_requests: record.total_requests,
            retry_after: 0,
        })
    }

    pub async fn get_status(&self, key: &str, config_name: &str) -> Result<RateLimitStatus, RateLimitError> {
        let config = self
            .configs
            .get(config_name)
            .ok_or_else(|| RateLimitError::InvalidConfig(format!("Config '{}' not found", config_name)))?;

        let now = Utc::now();
        let window_start = now - Duration::seconds(config.window_seconds as i64);

        let record = match self.store.get_record(key).await? {
            Some(r) => r,
            None => {
                return Ok(RateLimitStatus {
                    current_requests: 0,
                    max_requests: config.max_requests,
                    window_seconds: config.window_seconds,
                    remaining: config.max_requests,
                    reset_time: now + Duration::seconds(config.window_seconds as i64),
                    is_blocked: false,
                    blocked_until: None,
                    total_requests: 0,
                    last_request: None,
                });
            }
        };

        // Clean old requests outside the window
        let mut cleaned_requests: Vec<_> = record.requests.iter().filter(|&&req| req > window_start).collect();
        let current_requests = cleaned_requests.len() as u32;

        let is_blocked = record.blocked_until.map_or(false, |blocked| blocked > now);
        let remaining = if is_blocked { 0 } else { config.max_requests.saturating_sub(current_requests) };
        let reset_time = if cleaned_requests.is_empty() {
            now + Duration::seconds(config.window_seconds as i64)
        } else {
            cleaned_requests[0] + Duration::seconds(config.window_seconds as i64)
        };

        Ok(RateLimitStatus {
            current_requests,
            max_requests: config.max_requests,
            window_seconds: config.window_seconds,
            remaining,
            reset_time,
            is_blocked,
            blocked_until: record.blocked_until,
            total_requests: record.total_requests,
            last_request: Some(record.last_request),
        })
    }

    pub async fn reset(&self, key: &str) -> Result<(), RateLimitError> {
        self.store.delete_record(key).await
    }

    pub async fn cleanup_expired(&self) -> Result<usize, RateLimitError> {
        self.store.cleanup_expired().await
    }

    pub fn get_configs(&self) -> &HashMap<String, RateLimitConfig> {
        &self.configs
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub remaining: u32,
    pub reset_time: chrono::DateTime<Utc>,
    pub total_requests: u64,
    pub retry_after: u64,
}

#[derive(Debug, Clone)]
pub struct RateLimitStatus {
    pub current_requests: u32,
    pub max_requests: u32,
    pub window_seconds: u64,
    pub remaining: u32,
    pub reset_time: chrono::DateTime<Utc>,
    pub is_blocked: bool,
    pub blocked_until: Option<chrono::DateTime<Utc>>,
    pub total_requests: u64,
    pub last_request: Option<chrono::DateTime<Utc>>,
}

impl RateLimitConfig {
    pub fn new(max_requests: u32, window_seconds: u64) -> Self {
        Self {
            max_requests,
            window_seconds,
            burst_size: None,
            penalty_seconds: None,
        }
    }

    pub fn with_penalty(mut self, penalty_seconds: u64) -> Self {
        self.penalty_seconds = Some(penalty_seconds);
        self
    }

    pub fn with_burst(mut self, burst_size: u32) -> Self {
        self.burst_size = Some(burst_size);
        self
    }

    // Common configurations
    pub fn strict() -> Self {
        Self::new(5, 60).with_penalty(300) // 5 requests per minute, 5 minute penalty
    }

    pub fn moderate() -> Self {
        Self::new(10, 60) // 10 requests per minute
    }

    pub fn lenient() -> Self {
        Self::new(100, 60) // 100 requests per minute
    }

    pub fn api() -> Self {
        Self::new(1000, 3600) // 1000 requests per hour
    }

    pub fn auth() -> Self {
        Self::new(5, 300).with_penalty(900) // 5 login attempts per 5 minutes, 15 minute penalty
    }

    pub fn password_reset() -> Self {
        Self::new(3, 3600).with_penalty(3600) // 3 password resets per hour, 1 hour penalty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_basic_rate_limiting() {
        let store = InMemoryRateLimitStore::new();
        let mut service = RateLimitService::new(store);
        
        service.add_config("test", RateLimitConfig::new(3, 60)).unwrap();

        // First 3 requests should be allowed
        for i in 0..3 {
            let result = service.check_rate_limit("user1", "test").await.unwrap();
            assert!(result.allowed);
            assert_eq!(result.remaining, 2 - i);
        }

        // 4th request should be blocked
        let result = service.check_rate_limit("user1", "test").await.unwrap();
        assert!(!result.allowed);
        assert_eq!(result.remaining, 0);
    }

    #[tokio::test]
    async fn test_rate_limit_reset() {
        let store = InMemoryRateLimitStore::new();
        let mut service = RateLimitService::new(store);
        
        service.add_config("test", RateLimitConfig::new(2, 1)).unwrap(); // 2 requests per 1 second

        // Use up the limit
        service.check_rate_limit("user1", "test").await.unwrap();
        service.check_rate_limit("user1", "test").await.unwrap();

        // Should be blocked
        let result = service.check_rate_limit("user1", "test").await.unwrap();
        assert!(!result.allowed);

        // Reset and should be allowed again
        service.reset("user1").await.unwrap();
        let result = service.check_rate_limit("user1", "test").await.unwrap();
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_penalty_system() {
        let store = InMemoryRateLimitStore::new();
        let mut service = RateLimitService::new(store);
        
        service.add_config("test", RateLimitConfig::new(2, 60).with_penalty(5)).unwrap();

        // Use up the limit
        service.check_rate_limit("user1", "test").await.unwrap();
        service.check_rate_limit("user1", "test").await.unwrap();

        // Should be blocked with penalty
        let result = service.check_rate_limit("user1", "test").await.unwrap();
        assert!(!result.allowed);
        assert!(result.retry_after > 0);
    }

    #[tokio::test]
    async fn test_different_keys() {
        let store = InMemoryRateLimitStore::new();
        let mut service = RateLimitService::new(store);
        
        service.add_config("test", RateLimitConfig::new(2, 60)).unwrap();

        // Different keys should have independent limits
        service.check_rate_limit("user1", "test").await.unwrap();
        service.check_rate_limit("user1", "test").await.unwrap();

        let result = service.check_rate_limit("user2", "test").await.unwrap();
        assert!(result.allowed); // Should still be allowed for user2
    }

    #[tokio::test]
    async fn test_rate_limit_status() {
        let store = InMemoryRateLimitStore::new();
        let mut service = RateLimitService::new(store);
        
        service.add_config("test", RateLimitConfig::new(5, 60)).unwrap();

        // Make a request
        service.check_rate_limit("user1", "test").await.unwrap();

        let status = service.get_status("user1", "test").await.unwrap();
        assert_eq!(status.current_requests, 1);
        assert_eq!(status.max_requests, 5);
        assert_eq!(status.remaining, 4);
        assert!(!status.is_blocked);
    }
}
