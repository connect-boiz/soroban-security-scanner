//! Storage backends for rate limiting data

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::rate_limiting::types::*;
use crate::rate_limiting::config::DistributedConfig;

/// Storage backend trait for rate limiting data
#[async_trait]
pub trait RateLimitStorage: Send + Sync {
    /// Error type for storage operations
    type Error: std::error::Error + Send + Sync + 'static;

    /// Record a request for rate limiting
    async fn record_request(
        &self,
        key: &str,
        window: Duration,
        timestamp: DateTime<Utc>,
    ) -> Result<u64, Self::Error>;

    /// Get current request count for a key and window
    async fn get_request_count(
        &self,
        key: &str,
        window: Duration,
    ) -> Result<u64, Self::Error>;

    /// Reset request count for a key
    async fn reset_request_count(&self, key: &str) -> Result<(), Self::Error>;

    /// Record a rate limit violation
    async fn record_violation(
        &self,
        violation: &RateLimitViolation,
    ) -> Result<(), Self::Error>;

    /// Get violations for a user or IP
    async fn get_violations(
        &self,
        user_id: Option<Uuid>,
        ip_address: Option<std::net::IpAddr>,
        limit: Option<u64>,
    ) -> Result<Vec<RateLimitViolation>, Self::Error>;

    /// Get rate limit statistics
    async fn get_stats(&self) -> Result<RateLimitStats, Self::Error>;

    /// Clean up old data
    async fn cleanup(&self, retention: Duration) -> Result<u64, Self::Error>;

    /// Check if storage is healthy
    async fn health_check(&self) -> Result<bool, Self::Error>;
}

/// In-memory storage implementation (for development/testing)
pub struct MemoryStorage {
    data: std::sync::Arc<tokio::sync::RwLock<HashMap<String, StorageData>>>,
    violations: std::sync::Arc<tokio::sync::RwLock<Vec<RateLimitViolation>>>,
    stats: std::sync::Arc<tokio::sync::RwLock<RateLimitStats>>,
}

#[derive(Debug, Clone)]
struct StorageData {
    requests: Vec<DateTime<Utc>>,
    last_updated: DateTime<Utc>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            data: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            violations: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
            stats: std::sync::Arc::new(tokio::sync::RwLock::new(RateLimitStats::default())),
        }
    }

    fn cleanup_old_requests(&self, data: &mut StorageData, window: Duration) {
        let cutoff = Utc::now() - chrono::Duration::from_std(window).unwrap();
        data.requests.retain(|&timestamp| timestamp > cutoff);
    }
}

#[async_trait]
impl RateLimitStorage for MemoryStorage {
    type Error = MemoryStorageError;

    async fn record_request(
        &self,
        key: &str,
        window: Duration,
        timestamp: DateTime<Utc>,
    ) -> Result<u64, Self::Error> {
        let mut data = self.data.write().await;
        let storage_data = data.entry(key.to_string()).or_insert_with(|| StorageData {
            requests: Vec::new(),
            last_updated: timestamp,
        });

        self.cleanup_old_requests(storage_data, window);
        storage_data.requests.push(timestamp);
        storage_data.last_updated = timestamp;

        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        stats.allowed_requests += 1;

        Ok(storage_data.requests.len() as u64)
    }

    async fn get_request_count(
        &self,
        key: &str,
        window: Duration,
    ) -> Result<u64, Self::Error> {
        let mut data = self.data.write().await;
        if let Some(storage_data) = data.get_mut(key) {
            self.cleanup_old_requests(storage_data, window);
            Ok(storage_data.requests.len() as u64)
        } else {
            Ok(0)
        }
    }

    async fn reset_request_count(&self, key: &str) -> Result<(), Self::Error> {
        let mut data = self.data.write().await;
        data.remove(key);
        Ok(())
    }

    async fn record_violation(
        &self,
        violation: &RateLimitViolation,
    ) -> Result<(), Self::Error> {
        let mut violations = self.violations.write().await;
        violations.push(violation.clone());

        let mut stats = self.stats.write().await;
        stats.blocked_requests += 1;
        
        let entry = stats.violations_by_type
            .entry(violation.violation_type.clone())
            .or_insert(0);
        *entry += 1;

        if let Some(user_id) = violation.context.user_id {
            let entry = stats.top_violators_by_user
                .entry(user_id)
                .or_insert(0);
            *entry += 1;
        }

        let entry = stats.top_violators_by_ip
            .entry(violation.context.ip_address)
            .or_insert(0);
        *entry += 1;

        Ok(())
    }

    async fn get_violations(
        &self,
        user_id: Option<Uuid>,
        ip_address: Option<std::net::IpAddr>,
        limit: Option<u64>,
    ) -> Result<Vec<RateLimitViolation>, Self::Error> {
        let violations = self.violations.read().await;
        let filtered: Vec<RateLimitViolation> = violations
            .iter()
            .filter(|v| {
                let user_match = user_id.map_or(true, |uid| v.context.user_id == Some(uid));
                let ip_match = ip_address.map_or(true, |ip| v.context.ip_address == ip);
                user_match && ip_match
            })
            .cloned()
            .take(limit.unwrap_or(100) as usize)
            .collect();

        Ok(filtered)
    }

    async fn get_stats(&self) -> Result<RateLimitStats, Self::Error> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn cleanup(&self, retention: Duration) -> Result<u64, Self::Error> {
        let cutoff = Utc::now() - chrono::Duration::from_std(retention).unwrap();
        
        let mut violations = self.violations.write().await;
        let initial_count = violations.len();
        violations.retain(|v| v.timestamp > cutoff);
        let removed_count = initial_count - violations.len();

        // Also clean up old request data
        let mut data = self.data.write().await;
        let cutoff_duration = chrono::Duration::from_std(retention).unwrap();
        data.retain(|_, storage_data| storage_data.last_updated > cutoff);

        Ok(removed_count as u64)
    }

    async fn health_check(&self) -> Result<bool, Self::Error> {
        // Memory storage is always healthy
        Ok(true)
    }
}

/// Error type for memory storage
#[derive(Debug, thiserror::Error)]
pub enum MemoryStorageError {
    #[error("Storage error: {0}")]
    Storage(String),
}

impl std::fmt::Display for MemoryStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

/// Redis-based distributed storage implementation
#[cfg(feature = "redis-cache")]
pub struct RedisStorage {
    client: redis::Client,
    config: DistributedConfig,
}

#[cfg(feature = "redis-cache")]
impl RedisStorage {
    pub fn new(config: DistributedConfig) -> Result<Self, RedisStorageError> {
        let client = redis::Client::open(config.redis.url.as_str())
            .map_err(|e| RedisStorageError::Connection(e.to_string()))?;

        Ok(Self { client, config })
    }

    fn build_key(&self, prefix: &str, key: &str) -> String {
        format!("{}{}:{}", self.config.redis.key_prefix, prefix, key)
    }

    async fn get_connection(&self) -> Result<redis::aio::MultiplexedConnection, RedisStorageError> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| RedisStorageError::Connection(e.to_string()))
    }
}

#[cfg(feature = "redis-cache")]
#[async_trait]
impl RateLimitStorage for RedisStorage {
    type Error = RedisStorageError;

    async fn record_request(
        &self,
        key: &str,
        window: Duration,
        timestamp: DateTime<Utc>,
    ) -> Result<u64, Self::Error> {
        let mut conn = self.get_connection().await?;
        let redis_key = self.build_key("requests", key);
        let window_secs = window.as_secs();

        // Use Redis sliding window algorithm with sorted set
        let count: u64 = redis::cmd("ZCARD")
            .arg(&redis_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| RedisStorageError::Operation(e.to_string()))?;

        // Remove old entries
        let min_score = timestamp.timestamp() as f64 - window_secs as f64;
        redis::cmd("ZREMRANGEBYSCORE")
            .arg(&redis_key)
            .arg("-inf")
            .arg(min_score)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| RedisStorageError::Operation(e.to_string()))?;

        // Add current request
        redis::cmd("ZADD")
            .arg(&redis_key)
            .arg(timestamp.timestamp())
            .arg(timestamp.timestamp_nanos())
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| RedisStorageError::Operation(e.to_string()))?;

        // Set expiration
        redis::cmd("EXPIRE")
            .arg(&redis_key)
            .arg(window_secs + 60) // Extra buffer
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| RedisStorageError::Operation(e.to_string()))?;

        // Get new count
        let new_count: u64 = redis::cmd("ZCARD")
            .arg(&redis_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| RedisStorageError::Operation(e.to_string()))?;

        Ok(new_count)
    }

    async fn get_request_count(
        &self,
        key: &str,
        window: Duration,
    ) -> Result<u64, Self::Error> {
        let mut conn = self.get_connection().await?;
        let redis_key = self.build_key("requests", key);
        let window_secs = window.as_secs();

        // Remove old entries first
        let min_score = (Utc::now().timestamp() as f64) - window_secs as f64;
        redis::cmd("ZREMRANGEBYSCORE")
            .arg(&redis_key)
            .arg("-inf")
            .arg(min_score)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| RedisStorageError::Operation(e.to_string()))?;

        // Get count
        let count: u64 = redis::cmd("ZCARD")
            .arg(&redis_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| RedisStorageError::Operation(e.to_string()))?;

        Ok(count)
    }

    async fn reset_request_count(&self, key: &str) -> Result<(), Self::Error> {
        let mut conn = self.get_connection().await?;
        let redis_key = self.build_key("requests", key);

        redis::cmd("DEL")
            .arg(&redis_key)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| RedisStorageError::Operation(e.to_string()))?;

        Ok(())
    }

    async fn record_violation(
        &self,
        violation: &RateLimitViolation,
    ) -> Result<(), Self::Error> {
        let mut conn = self.get_connection().await?;
        let violation_json = serde_json::to_string(violation)
            .map_err(|e| RedisStorageError::Serialization(e.to_string()))?;

        // Store violation in a list
        let violations_key = self.build_key("violations", "global");
        redis::cmd("LPUSH")
            .arg(&violations_key)
            .arg(&violation_json)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| RedisStorageError::Operation(e.to_string()))?;

        // Trim list to prevent unlimited growth
        redis::cmd("LTRIM")
            .arg(&violations_key)
            .arg(0)
            .arg(10000) // Keep last 10k violations
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| RedisStorageError::Operation(e.to_string()))?;

        // Store user-specific violations
        if let Some(user_id) = violation.context.user_id {
            let user_key = self.build_key("violations", &user_id.to_string());
            redis::cmd("LPUSH")
                .arg(&user_key)
                .arg(&violation_json)
                .query_async::<_, ()>(&mut conn)
                .await
                .map_err(|e| RedisStorageError::Operation(e.to_string()))?;

            redis::cmd("EXPIRE")
                .arg(&user_key)
                .arg(86400) // 24 hours
                .query_async::<_, ()>(&mut conn)
                .await
                .map_err(|e| RedisStorageError::Operation(e.to_string()))?;
        }

        Ok(())
    }

    async fn get_violations(
        &self,
        user_id: Option<Uuid>,
        ip_address: Option<std::net::IpAddr>,
        limit: Option<u64>,
    ) -> Result<Vec<RateLimitViolation>, Self::Error> {
        let mut conn = self.get_connection().await?;
        let key = if let Some(uid) = user_id {
            self.build_key("violations", &uid.to_string())
        } else {
            self.build_key("violations", "global")
        };

        let limit = limit.unwrap_or(100);
        let violation_jsons: Vec<String> = redis::cmd("LRANGE")
            .arg(&key)
            .arg(0)
            .arg(limit - 1)
            .query_async(&mut conn)
            .await
            .map_err(|e| RedisStorageError::Operation(e.to_string()))?;

        let mut violations = Vec::new();
        for json in violation_jsons {
            if let Ok(violation) = serde_json::from_str::<RateLimitViolation>(&json) {
                if let Some(ip) = ip_address {
                    if violation.context.ip_address == ip {
                        violations.push(violation);
                    }
                } else {
                    violations.push(violation);
                }
            }
        }

        Ok(violations)
    }

    async fn get_stats(&self) -> Result<RateLimitStats, Self::Error> {
        // For Redis implementation, we'd typically store stats in a separate hash
        // For now, return default stats
        Ok(RateLimitStats::default())
    }

    async fn cleanup(&self, retention: Duration) -> Result<u64, Self::Error> {
        // Redis automatically handles expiration with TTL
        // This would be more complex in a real implementation
        Ok(0)
    }

    async fn health_check(&self) -> Result<bool, Self::Error> {
        let mut conn = self.get_connection().await?;
        let _: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| RedisStorageError::Connection(e.to_string()))?;
        Ok(true)
    }
}

/// Error type for Redis storage
#[cfg(feature = "redis-cache")]
#[derive(Debug, thiserror::Error)]
pub enum RedisStorageError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Operation error: {0}")]
    Operation(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
}

#[cfg(feature = "redis-cache")]
impl std::fmt::Display for RedisStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

/// Storage factory for creating appropriate storage backend
pub struct StorageFactory;

impl StorageFactory {
    pub async fn create_storage(
        config: &crate::rate_limiting::config::RateLimitConfig,
    ) -> Result<Box<dyn RateLimitStorage<Error = Box<dyn std::error::Error + Send + Sync>>>, Box<dyn std::error::Error>> {
        if config.distributed.enabled {
            #[cfg(feature = "redis-cache")]
            {
                let redis_storage = RedisStorage::new(config.distributed.clone())?;
                Ok(Box::new(redis_storage))
            }
            #[cfg(not(feature = "redis-cache"))]
            {
                tracing::warn!("Redis storage requested but redis-cache feature not enabled, falling back to memory storage");
                Ok(Box::new(MemoryStorage::new()))
            }
        } else {
            Ok(Box::new(MemoryStorage::new()))
        }
    }
}
