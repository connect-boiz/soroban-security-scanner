use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;
#[cfg(feature = "redis-cache")]
use uuid::Uuid;

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

impl RateLimitRecord {
    pub const DEFAULT_MAX_HISTORY: usize = 1000;

    pub fn new(key: String, now: chrono::DateTime<Utc>) -> Self {
        Self {
            key,
            requests: Vec::new(),
            blocked_until: None,
            total_requests: 0,
            last_request: now,
        }
    }

    /// Add a request timestamp and bound the request history to prevent unbounded memory growth
    pub fn add_request(&mut self, timestamp: chrono::DateTime<Utc>, max_history: usize) {
        self.requests.push(timestamp);
        self.total_requests += 1;
        self.last_request = timestamp;
        self.bound_history(max_history);
    }

    /// Retain only requests newer than cutoff, and bound the total entries to max_history
    pub fn trim_requests(&mut self, cutoff: chrono::DateTime<Utc>, max_history: usize) {
        self.requests.retain(|&req| req > cutoff);
        self.bound_history(max_history);
    }

    /// Bound requests history to the most recent `max_history` items
    pub fn bound_history(&mut self, max_history: usize) {
        let cap = max_history.max(1);
        if self.requests.len() > cap {
            let drain_count = self.requests.len() - cap;
            self.requests.drain(0..drain_count);
        }
    }
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

    /// Atomic rate limit check and record across instances (e.g. using Redis sliding-window ZSET or atomic in-memory lock).
    /// Returns Ok(Some(result)) if handled atomically, or Ok(None) to fallback to get_record/save_record.
    async fn check_and_record(
        &self,
        _key: &str,
        _config: &RateLimitConfig,
        _now: chrono::DateTime<Utc>,
    ) -> Result<Option<RateLimitResult>, RateLimitError> {
        Ok(None)
    }
}

pub struct InMemoryRateLimitStore {
    records: Arc<RwLock<HashMap<String, RateLimitRecord>>>,
    max_entries: usize,
}

impl InMemoryRateLimitStore {
    pub const DEFAULT_MAX_ENTRIES: usize = 100_000;

    pub fn new() -> Self {
        Self::with_max_entries(Self::DEFAULT_MAX_ENTRIES)
    }

    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
            max_entries: max_entries.max(1),
        }
    }

    pub async fn cleanup_expired_before(
        &self,
        cutoff: chrono::DateTime<Utc>,
    ) -> Result<usize, RateLimitError> {
        let mut records = self
            .records
            .write()
            .map_err(|e| RateLimitError::Storage(e.to_string()))?;

        let now = Utc::now();
        let initial_len = records.len();

        records.retain(|_, record| {
            record.trim_requests(cutoff, RateLimitRecord::DEFAULT_MAX_HISTORY);
            let should_keep = !record.requests.is_empty()
                || record.blocked_until.is_some_and(|blocked| blocked > now);
            should_keep
        });

        Ok(initial_len - records.len())
    }
}

impl Default for InMemoryRateLimitStore {
    fn default() -> Self {
        Self::new()
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

    async fn save_record(&self, mut record: RateLimitRecord) -> Result<(), RateLimitError> {
        let mut records = self
            .records
            .write()
            .map_err(|e| RateLimitError::Storage(e.to_string()))?;

        let now = Utc::now();
        // Trim requests older than 1 hour on the hot path to prevent unbounded memory growth
        record.trim_requests(now - Duration::seconds(3600), RateLimitRecord::DEFAULT_MAX_HISTORY);

        // If store reaches capacity and key is new, evict expired records
        if records.len() >= self.max_entries && !records.contains_key(&record.key) {
            let cutoff = now - Duration::seconds(3600);
            records.retain(|_, r| {
                !r.requests.iter().all(|&t| t <= cutoff)
                    || r.blocked_until.is_some_and(|b| b > now)
            });

            // If still at capacity, evict the oldest accessed entry
            if records.len() >= self.max_entries {
                if let Some(oldest_key) = records
                    .iter()
                    .min_by_key(|(_, r)| r.last_request)
                    .map(|(k, _)| k.clone())
                {
                    records.remove(&oldest_key);
                }
            }
        }

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
        self.cleanup_expired_before(Utc::now() - Duration::seconds(3600)).await
    }

    async fn check_and_record(
        &self,
        key: &str,
        config: &RateLimitConfig,
        now: chrono::DateTime<Utc>,
    ) -> Result<Option<RateLimitResult>, RateLimitError> {
        let mut records = self
            .records
            .write()
            .map_err(|e| RateLimitError::Storage(e.to_string()))?;

        let window_start = now - Duration::seconds(config.window_seconds as i64);
        let max_history = (config.max_requests as usize * 2).clamp(100, 10_000);

        // Evict if store is at max capacity and key is new
        if records.len() >= self.max_entries && !records.contains_key(key) {
            let cutoff = now - Duration::seconds(3600);
            records.retain(|_, r| {
                !r.requests.iter().all(|&t| t <= cutoff)
                    || r.blocked_until.is_some_and(|b| b > now)
            });
            if records.len() >= self.max_entries {
                if let Some(oldest_key) = records
                    .iter()
                    .min_by_key(|(_, r)| r.last_request)
                    .map(|(k, _)| k.clone())
                {
                    records.remove(&oldest_key);
                }
            }
        }

        let record = records.entry(key.to_string()).or_insert_with(|| {
            RateLimitRecord::new(key.to_string(), now)
        });

        // Check if currently blocked
        if let Some(blocked_until) = record.blocked_until {
            if blocked_until > now {
                return Ok(Some(RateLimitResult {
                    allowed: false,
                    remaining: 0,
                    reset_time: blocked_until,
                    total_requests: record.total_requests,
                    retry_after: (blocked_until - now).num_seconds().max(1) as u64,
                }));
            } else {
                record.blocked_until = None;
            }
        }

        // Clean old requests outside the window and bound history
        record.trim_requests(window_start, max_history);

        // Check if over limit
        if record.requests.len() >= config.max_requests as usize {
            if let Some(penalty_seconds) = config.penalty_seconds {
                let blocked_until = now + Duration::seconds(penalty_seconds as i64);
                record.blocked_until = Some(blocked_until);
                return Ok(Some(RateLimitResult {
                    allowed: false,
                    remaining: 0,
                    reset_time: blocked_until,
                    total_requests: record.total_requests,
                    retry_after: penalty_seconds,
                }));
            }

            let reset_time = record.requests[0] + Duration::seconds(config.window_seconds as i64);
            let retry_after = (reset_time - now).num_seconds().max(1) as u64;
            return Ok(Some(RateLimitResult {
                allowed: false,
                remaining: 0,
                reset_time,
                total_requests: record.total_requests,
                retry_after,
            }));
        }

        // Add current request bounded
        record.add_request(now, max_history);

        let remaining = config.max_requests.saturating_sub(record.requests.len() as u32);
        let reset_time = record.requests[0] + Duration::seconds(config.window_seconds as i64);

        Ok(Some(RateLimitResult {
            allowed: true,
            remaining,
            reset_time,
            total_requests: record.total_requests,
            retry_after: 0,
        }))
    }
}

#[cfg(feature = "redis-cache")]
const SLIDING_WINDOW_LUA_SCRIPT: &str = r#"
local zset_key = KEYS[1]
local blocked_key = KEYS[2]
local total_key = KEYS[3]

local now_ms = tonumber(ARGV[1])
local window_ms = tonumber(ARGV[2])
local max_requests = tonumber(ARGV[3])
local penalty_seconds = tonumber(ARGV[4])
local member_id = ARGV[5]

-- 1. Check if blocked
local blocked_until_ms = redis.call('GET', blocked_key)
if blocked_until_ms then
    local blocked_ms = tonumber(blocked_until_ms)
    if blocked_ms and blocked_ms > now_ms then
        local retry_after = math.max(1, math.ceil((blocked_ms - now_ms) / 1000))
        local total = tonumber(redis.call('GET', total_key) or '0')
        return {0, 0, blocked_ms, total, retry_after}
    end
end

-- 2. Purge old requests outside sliding window
local window_start_ms = now_ms - window_ms
redis.call('ZREMRANGEBYSCORE', zset_key, '-inf', window_start_ms)

-- 3. Get current request count within window
local current_count = redis.call('ZCARD', zset_key)

-- 4. Check if limit exceeded
if current_count >= max_requests then
    local total = tonumber(redis.call('GET', total_key) or '0')
    if penalty_seconds > 0 then
        local blocked_until = now_ms + (penalty_seconds * 1000)
        redis.call('SETEX', blocked_key, penalty_seconds, tostring(blocked_until))
        return {0, 0, blocked_until, total, penalty_seconds}
    else
        local oldest = redis.call('ZRANGE', zset_key, 0, 0, 'WITHSCORES')
        local oldest_ts = now_ms
        if oldest and #oldest >= 2 then
            oldest_ts = tonumber(oldest[2]) or now_ms
        end
        local reset_time_ms = oldest_ts + window_ms
        local retry_after = math.max(1, math.ceil((reset_time_ms - now_ms) / 1000))
        return {0, 0, reset_time_ms, total, retry_after}
    end
end

-- 5. Under limit: record new request atomically
redis.call('ZADD', zset_key, now_ms, member_id)
local expire_seconds = math.max(3600, math.ceil(window_ms / 1000) * 2)
redis.call('EXPIRE', zset_key, expire_seconds)
local total = redis.call('INCR', total_key)
redis.call('EXPIRE', total_key, expire_seconds)

-- Find earliest timestamp for reset_time
local oldest = redis.call('ZRANGE', zset_key, 0, 0, 'WITHSCORES')
local oldest_ts = now_ms
if oldest and #oldest >= 2 then
    oldest_ts = tonumber(oldest[2]) or now_ms
end
local reset_time_ms = oldest_ts + window_ms
local remaining = max_requests - current_count - 1

return {1, remaining, reset_time_ms, total, 0}
"#;

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

    fn zset_key(&self, key: &str) -> String {
        format!("{}:rate_limit:{{{}}}:zset", self.key_prefix, key)
    }

    fn blocked_key(&self, key: &str) -> String {
        format!("{}:rate_limit:{{{}}}:blocked", self.key_prefix, key)
    }

    fn total_key(&self, key: &str) -> String {
        format!("{}:rate_limit:{{{}}}:total", self.key_prefix, key)
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

        if let Some(json) = record_json {
            let mut record: RateLimitRecord = serde_json::from_str(&json)
                .map_err(|e| RateLimitError::Serialization(e.to_string()))?;
            let now = Utc::now();
            record.trim_requests(now - Duration::seconds(3600), RateLimitRecord::DEFAULT_MAX_HISTORY);
            return Ok(Some(record));
        }

        // Check if distributed ZSET exists
        let zset_key = self.zset_key(key);
        let blocked_key = self.blocked_key(key);
        let total_key = self.total_key(key);

        let now = Utc::now();
        let now_ms = now.timestamp_millis();
        let cutoff_ms = now_ms - 3600_000;

        let scores: Vec<i64> = redis::cmd("ZRANGEBYSCORE")
            .arg(&zset_key)
            .arg(cutoff_ms)
            .arg("+inf")
            .query_async(&mut conn)
            .await
            .map_err(|e| RateLimitError::Redis(e.to_string()))?;

        let blocked_until_ms_str: Option<String> = conn
            .get(&blocked_key)
            .await
            .map_err(|e| RateLimitError::Redis(e.to_string()))?;

        let total_requests: u64 = redis::cmd("GET")
            .arg(&total_key)
            .query_async(&mut conn)
            .await
            .unwrap_or(0);

        if scores.is_empty() && blocked_until_ms_str.is_none() && total_requests == 0 {
            return Ok(None);
        }

        let requests: Vec<chrono::DateTime<Utc>> = scores
            .into_iter()
            .filter_map(|ms| chrono::DateTime::from_timestamp_millis(ms))
            .collect();

        let blocked_until = blocked_until_ms_str.and_then(|s| {
            s.parse::<i64>().ok().and_then(|ms| {
                let dt = chrono::DateTime::from_timestamp_millis(ms)?;
                if dt > now {
                    Some(dt)
                } else {
                    None
                }
            })
        });

        let last_request = requests.last().copied().unwrap_or(now);

        Ok(Some(RateLimitRecord {
            key: key.to_string(),
            requests,
            blocked_until,
            total_requests,
            last_request,
        }))
    }

    async fn save_record(&self, mut record: RateLimitRecord) -> Result<(), RateLimitError> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| RateLimitError::Redis(e.to_string()))?;

        let now = Utc::now();
        record.trim_requests(now - Duration::seconds(3600), RateLimitRecord::DEFAULT_MAX_HISTORY);

        let record_json = serde_json::to_string(&record)
            .map_err(|e| RateLimitError::Serialization(e.to_string()))?;

        let record_key = self.record_key(&record.key);

        let ttl_seconds = if let Some(blocked_until) = record.blocked_until {
            if blocked_until > now {
                ((blocked_until - now).num_seconds() + 60).max(3600) as u64
            } else {
                3600
            }
        } else {
            3600
        };

        conn.set_ex::<_, _, ()>(&record_key, record_json, ttl_seconds as usize)
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
        let zset_key = self.zset_key(key);
        let blocked_key = self.blocked_key(key);
        let total_key = self.total_key(key);

        let _: () = redis::cmd("DEL")
            .arg(&[&record_key, &zset_key, &blocked_key, &total_key])
            .query_async(&mut conn)
            .await
            .map_err(|e| RateLimitError::Redis(e.to_string()))?;

        Ok(())
    }

    async fn cleanup_expired(&self) -> Result<usize, RateLimitError> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| RateLimitError::Redis(e.to_string()))?;

        let pattern = format!("{}:rate_limit:*", self.key_prefix);
        let mut cursor = 0u64;
        let mut cleaned = 0usize;
        let now = Utc::now();
        let now_ms = now.timestamp_millis();
        let cutoff_ms = now_ms - 3600_000;

        loop {
            let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await
                .map_err(|e| RateLimitError::Redis(e.to_string()))?;

            for k in keys {
                if k.ends_with(":zset") {
                    let _: () = redis::cmd("ZREMRANGEBYSCORE")
                        .arg(&k)
                        .arg("-inf")
                        .arg(cutoff_ms)
                        .query_async(&mut conn)
                        .await
                        .unwrap_or(());

                    let count: u64 = redis::cmd("ZCARD")
                        .arg(&k)
                        .query_async(&mut conn)
                        .await
                        .unwrap_or(0);

                    if count == 0 {
                        let deleted: u64 = redis::cmd("DEL")
                            .arg(&k)
                            .query_async(&mut conn)
                            .await
                            .unwrap_or(0);
                        if deleted > 0 {
                            cleaned += 1;
                        }
                    }
                } else if k.ends_with(":blocked") {
                    let val: Option<String> = conn.get(&k).await.unwrap_or(None);
                    if let Some(blocked_until_ms_str) = val {
                        if let Ok(blocked_ms) = blocked_until_ms_str.parse::<i64>() {
                            if blocked_ms <= now_ms {
                                let deleted: u64 = redis::cmd("DEL")
                                    .arg(&k)
                                    .query_async(&mut conn)
                                    .await
                                    .unwrap_or(0);
                                if deleted > 0 {
                                    cleaned += 1;
                                }
                            }
                        }
                    }
                } else if !k.ends_with(":total") {
                    let ttl: i64 = redis::cmd("TTL")
                        .arg(&k)
                        .query_async(&mut conn)
                        .await
                        .unwrap_or(0);
                    if ttl <= 0 {
                        let deleted: u64 = redis::cmd("DEL")
                            .arg(&k)
                            .query_async(&mut conn)
                            .await
                            .unwrap_or(0);
                        if deleted > 0 {
                            cleaned += 1;
                        }
                    }
                }
            }

            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }

        Ok(cleaned)
    }

    async fn check_and_record(
        &self,
        key: &str,
        config: &RateLimitConfig,
        now: chrono::DateTime<Utc>,
    ) -> Result<Option<RateLimitResult>, RateLimitError> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| RateLimitError::Redis(e.to_string()))?;

        let zset_key = self.zset_key(key);
        let blocked_key = self.blocked_key(key);
        let total_key = self.total_key(key);

        let now_ms = now.timestamp_millis();
        let window_ms = (config.window_seconds * 1000) as i64;
        let penalty_seconds = config.penalty_seconds.unwrap_or(0) as i64;
        let member_id = format!("{}:{}", now_ms, Uuid::new_v4());

        let script = redis::Script::new(SLIDING_WINDOW_LUA_SCRIPT);
        let (allowed, remaining, reset_time_ms, total_requests, retry_after): (
            i64,
            u32,
            i64,
            u64,
            u64,
        ) = script
            .key(&zset_key)
            .key(&blocked_key)
            .key(&total_key)
            .arg(now_ms)
            .arg(window_ms)
            .arg(config.max_requests as i64)
            .arg(penalty_seconds)
            .arg(&member_id)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| RateLimitError::Redis(e.to_string()))?;

        let reset_time = chrono::DateTime::from_timestamp_millis(reset_time_ms).unwrap_or(now);

        Ok(Some(RateLimitResult {
            allowed: allowed == 1,
            remaining,
            reset_time,
            total_requests,
            retry_after,
        }))
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

    pub fn add_config(
        &mut self,
        name: &str,
        config: RateLimitConfig,
    ) -> Result<(), RateLimitError> {
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
        let config = self.configs.get(config_name).ok_or_else(|| {
            RateLimitError::InvalidConfig(format!("Config '{}' not found", config_name))
        })?;

        let now = Utc::now();

        // 1. Check if the store provides atomic check-and-record (e.g. Redis distributed sliding-window or atomic InMemory)
        if let Some(result) = self.store.check_and_record(key, config, now).await? {
            return Ok(result);
        }

        let window_start = now - Duration::seconds(config.window_seconds as i64);
        let max_history = (config.max_requests as usize * 2).clamp(100, 10_000);

        let mut record = match self.store.get_record(key).await? {
            Some(mut r) => {
                r.trim_requests(window_start, max_history);
                r
            }
            None => RateLimitRecord::new(key.to_string(), now),
        };

        // Check if currently blocked
        if let Some(blocked_until) = record.blocked_until {
            if blocked_until > now {
                return Ok(RateLimitResult {
                    allowed: false,
                    remaining: 0,
                    reset_time: blocked_until,
                    total_requests: record.total_requests,
                    retry_after: (blocked_until - now).num_seconds().max(1) as u64,
                });
            } else {
                // Block period expired
                record.blocked_until = None;
            }
        }

        // Clean old requests outside the window
        record.trim_requests(window_start, max_history);

        // Check if over limit
        if record.requests.len() >= config.max_requests as usize {
            // Apply penalty if configured
            if let Some(penalty_seconds) = config.penalty_seconds {
                let blocked_until = now + Duration::seconds(penalty_seconds as i64);
                record.blocked_until = Some(blocked_until);
                self.store.save_record(record.clone()).await?;

                return Ok(RateLimitResult {
                    allowed: false,
                    remaining: 0,
                    reset_time: blocked_until,
                    total_requests: record.total_requests,
                    retry_after: penalty_seconds,
                });
            }

            let reset_time = record.requests[0] + Duration::seconds(config.window_seconds as i64);
            return Ok(RateLimitResult {
                allowed: false,
                remaining: 0,
                reset_time,
                total_requests: record.total_requests,
                retry_after: (reset_time - now).num_seconds().max(1) as u64,
            });
        }

        // Add current request bounded
        record.add_request(now, max_history);

        self.store.save_record(record.clone()).await?;

        let remaining = config.max_requests.saturating_sub(record.requests.len() as u32);
        let reset_time = record.requests[0] + Duration::seconds(config.window_seconds as i64);

        Ok(RateLimitResult {
            allowed: true,
            remaining,
            reset_time,
            total_requests: record.total_requests,
            retry_after: 0,
        })
    }

    pub async fn get_status(
        &self,
        key: &str,
        config_name: &str,
    ) -> Result<RateLimitStatus, RateLimitError> {
        let config = self.configs.get(config_name).ok_or_else(|| {
            RateLimitError::InvalidConfig(format!("Config '{}' not found", config_name))
        })?;

        let now = Utc::now();
        let window_start = now - Duration::seconds(config.window_seconds as i64);
        let max_history = (config.max_requests as usize * 2).clamp(100, 10_000);

        let record = match self.store.get_record(key).await? {
            Some(mut r) => {
                r.trim_requests(window_start, max_history);
                r
            }
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
        let cleaned_requests: Vec<_> = record
            .requests
            .iter()
            .filter(|&&req| req > window_start)
            .collect();
        let current_requests = cleaned_requests.len() as u32;

        let is_blocked = record.blocked_until.is_some_and(|blocked| blocked > now);
        let remaining = if is_blocked {
            0
        } else {
            config.max_requests.saturating_sub(current_requests)
        };
        let reset_time = if cleaned_requests.is_empty() {
            now + Duration::seconds(config.window_seconds as i64)
        } else {
            *cleaned_requests[0] + Duration::seconds(config.window_seconds as i64)
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

        service
            .add_config("test", RateLimitConfig::new(3, 60))
            .unwrap();

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

        service
            .add_config("test", RateLimitConfig::new(2, 1))
            .unwrap(); // 2 requests per 1 second

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

        service
            .add_config("test", RateLimitConfig::new(2, 60).with_penalty(5))
            .unwrap();

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

        service
            .add_config("test", RateLimitConfig::new(2, 60))
            .unwrap();

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

        service
            .add_config("test", RateLimitConfig::new(5, 60))
            .unwrap();

        // Make a request
        service.check_rate_limit("user1", "test").await.unwrap();

        let status = service.get_status("user1", "test").await.unwrap();
        assert_eq!(status.current_requests, 1);
        assert_eq!(status.max_requests, 5);
        assert_eq!(status.remaining, 4);
        assert!(!status.is_blocked);
    }

    #[tokio::test]
    async fn test_bounded_history_prevents_unbounded_growth() {
        let store = InMemoryRateLimitStore::new();
        let mut service = RateLimitService::new(store);

        service
            .add_config("burst", RateLimitConfig::new(100, 60))
            .unwrap();

        for _ in 0..150 {
            let _ = service.check_rate_limit("bounded_user", "burst").await.unwrap();
        }

        let record = service.store.get_record("bounded_user").await.unwrap().unwrap();
        assert!(record.requests.len() <= 200, "Request history must remain bounded");
        assert_eq!(record.total_requests, 100);
    }

    #[tokio::test]
    async fn test_in_memory_store_capacity_eviction() {
        let store = InMemoryRateLimitStore::with_max_entries(3);
        let mut service = RateLimitService::new(store);

        service
            .add_config("test", RateLimitConfig::new(5, 60))
            .unwrap();

        for i in 0..5 {
            let key = format!("user_{}", i);
            let _ = service.check_rate_limit(&key, "test").await.unwrap();
        }

        let records = service.store.records.read().unwrap();
        assert!(records.len() <= 3, "Store records count must not exceed max_entries");
    }

    #[tokio::test]
    async fn test_concurrent_atomic_rate_limiting() {
        let store = Arc::new(InMemoryRateLimitStore::new());
        let config = RateLimitConfig::new(10, 60);

        let mut handles = Vec::new();
        for _ in 0..25 {
            let store_clone = store.clone();
            let config_clone = config.clone();
            handles.push(tokio::spawn(async move {
                store_clone
                    .check_and_record("shared_user", &config_clone, Utc::now())
                    .await
                    .unwrap()
                    .unwrap()
            }));
        }

        let mut allowed_count = 0;
        let mut blocked_count = 0;
        for handle in handles {
            let result = handle.await.unwrap();
            if result.allowed {
                allowed_count += 1;
            } else {
                blocked_count += 1;
            }
        }

        assert_eq!(allowed_count, 10, "Exactly max_requests must be allowed under concurrency");
        assert_eq!(blocked_count, 15, "Excess requests must be blocked");
    }

    #[tokio::test]
    async fn test_cleanup_expired_records() {
        let store = InMemoryRateLimitStore::new();
        let mut service = RateLimitService::new(store);

        service
            .add_config("short", RateLimitConfig::new(5, 1))
            .unwrap();

        for i in 0..3 {
            let key = format!("user_{}", i);
            let _ = service.check_rate_limit(&key, "short").await.unwrap();
        }

        // Cleanup before cutoff in the future
        let future_cutoff = Utc::now() + Duration::seconds(10);
        let cleaned = service.store.cleanup_expired_before(future_cutoff).await.unwrap();
        assert_eq!(cleaned, 3, "Expired records should be cleaned up");
    }
}
