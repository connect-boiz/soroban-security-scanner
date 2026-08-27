//! Token Revocation List for JWT force-invalidation
//!
//! When a user changes their password or an account compromise is detected,
//! all existing JWTs issued to that user must be invalidated immediately.
//! Since JWTs are stateless (validated by signature, not server-side lookup),
//! we maintain a revocation list of JWT IDs (jti claims) that should be
//! rejected even if their signature is valid and they haven't expired yet.
//!
//! # Backends
//!
//! [`TokenRevocationList::new`] uses an in-memory store which is only safe for
//! single-instance deployments: revocations are lost on restart and are not
//! shared between instances. For production (Issue #486), construct the list
//! with [`TokenRevocationList::new_redis`] (available with the `redis-cache`
//! feature) so revoked jtis are stored with a TTL equal to the token's
//! remaining lifetime and are visible to every instance sharing the Redis.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RevocationError {
    #[error("Token is revoked: {0}")]
    Revoked(String),
    #[error("Storage error: {0}")]
    Storage(String),
}

/// A revoked JWT token entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokedToken {
    /// The JWT ID (jti claim) of the revoked token
    pub jti: String,
    /// The user ID who owned this token
    pub user_id: String,
    /// When the token was revoked
    pub revoked_at: DateTime<Utc>,
    /// When the original token expires (for cleanup)
    pub original_expiry: DateTime<Utc>,
}

/// Index of the tokens we've seen for each user: `user_id -> (jti -> original expiry)`.
/// Kept separate from the revoked set so `revoke_all_user_tokens` can enumerate a
/// user's active tokens and the expiry is available for pruning.
type UserTokenIndex = HashMap<String, HashMap<String, DateTime<Utc>>>;

/// Storage backend for the token revocation list.
///
/// Implementations must be safe to share across threads and — for the Redis
/// backend — across processes, so a revocation performed on one instance is
/// visible to every other instance behind a load balancer.
trait RevocationBackend: Send + Sync {
    /// Record a token that has just been issued to a user (deny-list indexing).
    fn track_issued_token(&self, jti: &str, user_id: &str, original_expiry: DateTime<Utc>);
    /// Revoke a single token by its jti.
    fn revoke_token(&self, jti: &str, user_id: &str, original_expiry: DateTime<Utc>);
    /// Revoke all active tokens for a user, returning how many were revoked.
    fn revoke_all_user_tokens(&self, user_id: &str) -> Result<usize, RevocationError>;
    /// Check whether a token is revoked by its jti.
    fn is_revoked(&self, jti: &str) -> bool;
    /// Remove entries whose underlying token has expired; returns entries removed.
    fn prune_expired(&self) -> usize;
    /// Number of currently revoked tokens.
    fn count(&self) -> usize;
    /// All revoked tokens for a user (debugging/admin).
    fn get_user_revoked_tokens(&self, user_id: &str) -> Vec<RevokedToken>;
}

/// In-memory revocation backend. Only suitable for single-instance, ephemeral
/// deployments — see the module docs for the production recommendation.
#[derive(Default)]
struct MemoryRevocationBackend {
    /// Map of jti -> RevokedToken
    revoked_tokens: RwLock<HashMap<String, RevokedToken>>,
    /// Map of user_id -> (jti -> original expiry) for every token we've seen
    /// issued to the user. This is what lets `revoke_all_user_tokens` find the
    /// user's *active* tokens; without it there is nothing to revoke in bulk.
    user_tokens: RwLock<UserTokenIndex>,
}

impl RevocationBackend for MemoryRevocationBackend {
    fn track_issued_token(&self, jti: &str, user_id: &str, original_expiry: DateTime<Utc>) {
        self.user_tokens
            .write()
            .unwrap()
            .entry(user_id.to_string())
            .or_default()
            .insert(jti.to_string(), original_expiry);
    }

    fn revoke_token(&self, jti: &str, user_id: &str, original_expiry: DateTime<Utc>) {
        let revoked = RevokedToken {
            jti: jti.to_string(),
            user_id: user_id.to_string(),
            revoked_at: Utc::now(),
            original_expiry,
        };

        self.revoked_tokens
            .write()
            .unwrap()
            .insert(jti.to_string(), revoked);

        self.user_tokens
            .write()
            .unwrap()
            .entry(user_id.to_string())
            .or_default()
            .insert(jti.to_string(), original_expiry);
    }

    fn revoke_all_user_tokens(&self, user_id: &str) -> Result<usize, RevocationError> {
        let tokens = self
            .user_tokens
            .read()
            .unwrap()
            .get(user_id)
            .cloned()
            .unwrap_or_default();

        let now = Utc::now();
        let mut revoked = self.revoked_tokens.write().unwrap();
        for (jti, original_expiry) in &tokens {
            revoked.entry(jti.clone()).or_insert_with(|| RevokedToken {
                jti: jti.clone(),
                user_id: user_id.to_string(),
                revoked_at: now,
                original_expiry: *original_expiry,
            });
        }

        Ok(tokens.len())
    }

    fn is_revoked(&self, jti: &str) -> bool {
        self.revoked_tokens.read().unwrap().contains_key(jti)
    }

    fn prune_expired(&self) -> usize {
        let now = Utc::now();
        let mut revoked = self.revoked_tokens.write().unwrap();
        let mut user_tokens = self.user_tokens.write().unwrap();

        let to_remove: Vec<String> = revoked
            .iter()
            .filter(|(_, token)| token.original_expiry < now)
            .map(|(jti, _)| jti.clone())
            .collect();

        let count = to_remove.len();
        for jti in &to_remove {
            revoked.remove(jti);
        }

        // Drop expired jtis from the per-user index and remove users left empty.
        user_tokens.retain(|_, jtis| {
            jtis.retain(|_, expiry| *expiry >= now);
            !jtis.is_empty()
        });

        count
    }

    fn count(&self) -> usize {
        self.revoked_tokens.read().unwrap().len()
    }

    fn get_user_revoked_tokens(&self, user_id: &str) -> Vec<RevokedToken> {
        let user_tokens = self.user_tokens.read().unwrap();
        let jtis: Vec<String> = user_tokens
            .get(user_id)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        drop(user_tokens);

        let revoked = self.revoked_tokens.read().unwrap();
        jtis.iter()
            .filter_map(|jti| revoked.get(jti).cloned())
            .collect()
    }
}

/// Redis-backed revocation backend (feature `redis-cache`).
///
/// Uses the `redis` crate's blocking connection API because the revocation
/// list (and `JwtService`) is synchronous. In high-traffic async servers this
/// is best paired with a dedicated runtime / `spawn_blocking`, or replaced by
/// an async store when `JwtService` itself moves to an async API.
///
/// Key layout (all keys are namespaced with the configured `key_prefix`):
/// - `revoked:{jti}` — JSON [`RevokedToken`], TTL = remaining token lifetime.
///   This is what `is_revoked` checks; Redis expires the entry automatically
///   once the underlying token has passed its natural expiry.
/// - `user_tokens:{user_id}` — sorted set of `jti` members scored by their
///   original expiry, so `revoke_all_user_tokens` can enumerate the user's
///   *active* tokens across instances. The set's TTL tracks its longest-lived
///   member.
///
/// Because every key is TTL'd, revocation state survives restarts and is
/// shared by all instances connected to the same Redis, closing the gap where
/// a "revoked" token became valid again after a restart or on an instance that
/// never saw the revocation.
#[cfg(feature = "redis-cache")]
struct RedisRevocationBackend {
    client: redis::Client,
    key_prefix: String,
}

#[cfg(feature = "redis-cache")]
impl RedisRevocationBackend {
    fn new(client: redis::Client, key_prefix: String) -> Result<Self, RevocationError> {
        let backend = Self { client, key_prefix };
        // Fail fast: a revocation list that cannot reach its backing store is a
        // security hazard, so surface connectivity problems at startup.
        let mut conn = backend.connection()?;
        redis::cmd("PING")
            .query::<String>(&mut conn)
            .map_err(|e| {
                RevocationError::Storage(format!("Redis ping failed: {e}"))
            })?;
        Ok(backend)
    }

    fn connection(&self) -> Result<redis::Connection, RevocationError> {
        self.client.get_connection().map_err(|e| {
            RevocationError::Storage(format!("Redis connection failed: {e}"))
        })
    }

    fn revoked_key(&self, jti: &str) -> String {
        format!("{}revoked:{}", self.key_prefix, jti)
    }

    fn user_tokens_key(&self, user_id: &str) -> String {
        format!("{}user_tokens:{}", self.key_prefix, user_id)
    }

    /// Seconds until `expiry`, clamped to a minimum of 1 so entries created
    /// for already-expired tokens still get cleaned up promptly.
    fn ttl_seconds(expiry: DateTime<Utc>) -> u64 {
        (expiry - Utc::now()).num_seconds().max(1) as u64
    }

    /// Persist a revoked token entry with a TTL matching its remaining life.
    fn set_revoked(
        &self,
        conn: &mut redis::Connection,
        revoked: &RevokedToken,
    ) -> Result<(), RevocationError> {
        let json = serde_json::to_string(revoked)
            .map_err(|e| RevocationError::Storage(format!("Serialization failed: {e}")))?;
        let ttl = Self::ttl_seconds(revoked.original_expiry);
        redis::cmd("SET")
            .arg(self.revoked_key(&revoked.jti))
            .arg(json)
            .arg("EX")
            .arg(ttl)
            .query::<()>(conn)
            .map_err(|e| RevocationError::Storage(e.to_string()))
    }

    /// Keep the user's token index alive for as long as its longest-lived
    /// member token (the sorted set's maximum score).
    fn refresh_user_tokens_ttl(
        &self,
        conn: &mut redis::Connection,
        user_id: &str,
    ) -> Result<(), RevocationError> {
        let key = self.user_tokens_key(user_id);
        let top: Vec<(String, f64)> = redis::cmd("ZREVRANGE")
            .arg(&key)
            .arg(0)
            .arg(0)
            .arg("WITHSCORES")
            .query(conn)
            .map_err(|e| RevocationError::Storage(e.to_string()))?;
        if let Some((_, score)) = top.first() {
            let remaining = (*score as i64 - Utc::now().timestamp()).max(1);
            redis::cmd("EXPIRE")
                .arg(&key)
                .arg(remaining)
                .query::<()>(conn)
                .map_err(|e| RevocationError::Storage(e.to_string()))?;
        }
        Ok(())
    }

    /// Enumerate the user's active jtis (those whose original expiry is still
    /// in the future).
    fn active_user_jtis(
        &self,
        conn: &mut redis::Connection,
        user_id: &str,
    ) -> Result<Vec<String>, RevocationError> {
        redis::cmd("ZRANGEBYSCORE")
            .arg(self.user_tokens_key(user_id))
            .arg(Utc::now().timestamp())
            .arg("+inf")
            .query(conn)
            .map_err(|e| RevocationError::Storage(e.to_string()))
    }
}

#[cfg(feature = "redis-cache")]
impl RevocationBackend for RedisRevocationBackend {
    fn track_issued_token(&self, jti: &str, user_id: &str, original_expiry: DateTime<Utc>) {
        let mut conn = match self.connection() {
            Ok(conn) => conn,
            Err(e) => {
                tracing::error!(jti, user_id, error = %e, "failed to track issued JWT in Redis");
                return;
            }
        };
        let key = self.user_tokens_key(user_id);
        let score = original_expiry.timestamp() as f64;
        if let Err(e) = redis::cmd("ZADD")
            .arg(&key)
            .arg(score)
            .arg(jti)
            .query::<()>(&mut conn)
        {
            tracing::error!(jti, user_id, error = %e, "failed to track issued JWT in Redis");
            return;
        }
        if let Err(e) = self.refresh_user_tokens_ttl(&mut conn, user_id) {
            tracing::error!(jti, user_id, error = %e, "failed to refresh user token index TTL in Redis");
        }
    }

    fn revoke_token(&self, jti: &str, user_id: &str, original_expiry: DateTime<Utc>) {
        let revoked = RevokedToken {
            jti: jti.to_string(),
            user_id: user_id.to_string(),
            revoked_at: Utc::now(),
            original_expiry,
        };

        let mut conn = match self.connection() {
            Ok(conn) => conn,
            Err(e) => {
                tracing::error!(jti, user_id, error = %e, "failed to revoke JWT in Redis");
                return;
            }
        };

        if let Err(e) = self.set_revoked(&mut conn, &revoked) {
            tracing::error!(jti, user_id, error = %e, "failed to persist JWT revocation in Redis");
            return;
        }

        // Keep the token reachable for future bulk revocations.
        let key = self.user_tokens_key(user_id);
        let score = original_expiry.timestamp() as f64;
        if let Err(e) = redis::cmd("ZADD")
            .arg(&key)
            .arg(score)
            .arg(jti)
            .query::<()>(&mut conn)
        {
            tracing::error!(jti, user_id, error = %e, "failed to index revoked JWT in Redis");
            return;
        }
        if let Err(e) = self.refresh_user_tokens_ttl(&mut conn, user_id) {
            tracing::error!(jti, user_id, error = %e, "failed to refresh user token index TTL in Redis");
        }
    }

    fn revoke_all_user_tokens(&self, user_id: &str) -> Result<usize, RevocationError> {
        let mut conn = self.connection()?;
        let jtis = self.active_user_jtis(&mut conn, user_id)?;

        let now = Utc::now();
        for jti in &jtis {
            // Reuse the expiry recorded in the index (the ZSET score is the
            // original expiry timestamp) so the TTL matches the token's real
            // remaining life.
            let expiry = redis::cmd("ZSCORE")
                .arg(self.user_tokens_key(user_id))
                .arg(jti)
                .query::<Option<f64>>(&mut conn)
                .map_err(|e| RevocationError::Storage(e.to_string()))?
                .unwrap_or(now.timestamp() as f64);

            let revoked = RevokedToken {
                jti: jti.clone(),
                user_id: user_id.to_string(),
                revoked_at: now,
                original_expiry: DateTime::from_timestamp(expiry as i64, 0)
                    .unwrap_or(now),
            };

            if let Err(e) = self.set_revoked(&mut conn, &revoked) {
                tracing::error!(jti, user_id, error = %e, "failed to persist JWT revocation in Redis");
            }
        }

        // Drop expired members from the index so it does not grow without bound.
        let _ = redis::cmd("ZREMRANGEBYSCORE")
            .arg(self.user_tokens_key(user_id))
            .arg("-inf")
            .arg(Utc::now().timestamp())
            .query::<i64>(&mut conn)
            .map_err(|e| RevocationError::Storage(e.to_string()))?;

        Ok(jtis.len())
    }

    fn is_revoked(&self, jti: &str) -> bool {
        let mut conn = match self.connection() {
            Ok(conn) => conn,
            Err(e) => {
                // Fail closed: if the shared store cannot be consulted we must
                // not treat a possibly-revoked token as valid.
                tracing::error!(jti, error = %e, "failed to check JWT revocation in Redis; rejecting token");
                return true;
            }
        };
        match redis::cmd("EXISTS")
            .arg(self.revoked_key(jti))
            .query::<i64>(&mut conn)
        {
            Ok(exists) => exists > 0,
            Err(e) => {
                tracing::error!(jti, error = %e, "failed to check JWT revocation in Redis; rejecting token");
                true
            }
        }
    }

    fn prune_expired(&self) -> usize {
        // Revoked entries self-expire via their TTL, so pruning only needs to
        // clean up expired members of the per-user indexes.
        let mut conn = match self.connection() {
            Ok(conn) => conn,
            Err(e) => {
                tracing::error!(error = %e, "failed to prune expired JWT revocations in Redis");
                return 0;
            }
        };

        let pattern = format!("{}user_tokens:*", self.key_prefix);
        let mut cursor: u64 = 0;
        let mut removed = 0usize;
        loop {
            let (next, keys): (u64, Vec<String>) = match redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100)
                .query(&mut conn)
            {
                Ok(result) => result,
                Err(e) => {
                    tracing::error!(error = %e, "failed to scan user token indexes in Redis");
                    break;
                }
            };
            cursor = next;
            for key in &keys {
                let count: i64 = redis::cmd("ZREMRANGEBYSCORE")
                    .arg(key)
                    .arg("-inf")
                    .arg(Utc::now().timestamp())
                    .query(&mut conn)
                    .unwrap_or(0);
                removed += count.max(0) as usize;
            }
            if cursor == 0 {
                break;
            }
        }
        removed
    }

    fn count(&self) -> usize {
        let mut conn = match self.connection() {
            Ok(conn) => conn,
            Err(e) => {
                tracing::error!(error = %e, "failed to count revoked JWTs in Redis");
                return 0;
            }
        };

        let pattern = format!("{}revoked:*", self.key_prefix);
        let mut cursor: u64 = 0;
        let mut total = 0usize;
        loop {
            let (next, keys): (u64, Vec<String>) = match redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100)
                .query(&mut conn)
            {
                Ok(result) => result,
                Err(e) => {
                    tracing::error!(error = %e, "failed to scan revoked JWTs in Redis");
                    break;
                }
            };
            cursor = next;
            total += keys.len();
            if cursor == 0 {
                break;
            }
        }
        total
    }

    fn get_user_revoked_tokens(&self, user_id: &str) -> Vec<RevokedToken> {
        let mut conn = match self.connection() {
            Ok(conn) => conn,
            Err(e) => {
                tracing::error!(user_id, error = %e, "failed to load revoked JWTs from Redis");
                return Vec::new();
            }
        };

        let jtis: Vec<String> = redis::cmd("ZRANGE")
            .arg(self.user_tokens_key(user_id))
            .arg(0)
            .arg(-1)
            .query(&mut conn)
            .unwrap_or_default();

        let mut revoked = Vec::new();
        for jti in jtis {
            let json: Option<String> = redis::cmd("GET")
                .arg(self.revoked_key(&jti))
                .query(&mut conn)
                .unwrap_or(None);
            if let Some(json) = json {
                if let Ok(token) = serde_json::from_str::<RevokedToken>(&json) {
                    revoked.push(token);
                }
            }
        }
        revoked
    }
}

/// Token Revocation List — stores revoked JWT IDs
///
/// This type is the public face of the revocation list. By default it is
/// backed by an in-memory store ([`TokenRevocationList::new`]); in production
/// use [`TokenRevocationList::new_redis`] (feature `redis-cache`) so
/// revocations survive restarts and are shared across instances.
pub struct TokenRevocationList {
    backend: Arc<dyn RevocationBackend>,
}

impl TokenRevocationList {
    /// Create a new empty TokenRevocationList backed by in-memory storage.
    ///
    /// Suitable for tests and single-instance deployments. Revocations are
    /// lost on restart and are not shared between instances — use
    /// [`TokenRevocationList::new_redis`] in production.
    pub fn new() -> Self {
        Self {
            backend: Arc::new(MemoryRevocationBackend::default()),
        }
    }

    /// Create a Redis-backed TokenRevocationList (feature `redis-cache`).
    ///
    /// Revoked jtis are stored as keys with a TTL equal to the token's
    /// remaining lifetime, and the per-user token index is stored in a sorted
    /// set, so revocations survive restarts and are shared by every instance
    /// connected to the same Redis (Issue #486).
    ///
    /// The connection is verified with a PING so misconfiguration fails fast
    /// at startup instead of silently disabling revocation.
    #[cfg(feature = "redis-cache")]
    pub fn new_redis(client: redis::Client, key_prefix: String) -> Result<Self, RevocationError> {
        Ok(Self {
            backend: Arc::new(RedisRevocationBackend::new(client, key_prefix)?),
        })
    }

    /// Record a token that has just been issued to a user.
    ///
    /// The revocation list is a deny-list, so it can only force-invalidate a
    /// token whose jti it knows about. Callers (e.g. `JwtService::generate_token`)
    /// register every issued token here so that a later `revoke_all_user_tokens`
    /// can actually reach the tokens that are still active. This does *not*
    /// revoke the token — it only remembers that it exists.
    pub fn track_issued_token(&self, jti: &str, user_id: &str, original_expiry: DateTime<Utc>) {
        self.backend
            .track_issued_token(jti, user_id, original_expiry);
    }

    /// Revoke a single token by its jti
    pub fn revoke_token(&self, jti: &str, user_id: &str, original_expiry: DateTime<Utc>) {
        self.backend.revoke_token(jti, user_id, original_expiry);
    }

    /// Revoke ALL tokens for a specific user
    ///
    /// This is called when a user changes their password or when an account
    /// compromise is detected. Every token we've tracked for the user is added
    /// to the revoked set so it is rejected by `validate_token` from now on,
    /// even though its signature is still cryptographically valid.
    ///
    /// The operation is idempotent: tokens that are already revoked are left in
    /// place, and the user's token index is preserved so tokens issued after the
    /// revocation are still tracked. Entries are cleaned up by `prune_expired`
    /// once the underlying tokens pass their natural expiry.
    pub fn revoke_all_user_tokens(&self, user_id: &str) -> Result<usize, RevocationError> {
        self.backend.revoke_all_user_tokens(user_id)
    }

    /// Check if a token is revoked by its jti
    ///
    /// For the Redis backend this fails closed: if the shared store cannot be
    /// consulted, the token is treated as revoked rather than risking a
    /// replayed "revoked" token being accepted.
    pub fn is_revoked(&self, jti: &str) -> bool {
        self.backend.is_revoked(jti)
    }

    /// Prune expired entries from the revocation list
    ///
    /// Entries whose original token has expired (past its `exp` claim) are
    /// safe to remove — the token would be rejected by normal expiry validation
    /// anyway. This should be called periodically by a background job. With the
    /// Redis backend, revoked entries self-expire via TTL and this only cleans
    /// up the per-user indexes.
    ///
    /// Returns the number of entries removed.
    pub fn prune_expired(&self) -> usize {
        self.backend.prune_expired()
    }

    /// Get the count of revoked tokens
    pub fn count(&self) -> usize {
        self.backend.count()
    }

    /// Get all revoked tokens for a user (for debugging/admin)
    pub fn get_user_revoked_tokens(&self, user_id: &str) -> Vec<RevokedToken> {
        self.backend.get_user_revoked_tokens(user_id)
    }
}

impl Default for TokenRevocationList {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use uuid::Uuid;

    #[test]
    fn test_revoke_single_token() {
        let list = TokenRevocationList::new();
        let jti = Uuid::new_v4().to_string();
        let expiry = Utc::now() + Duration::hours(1);

        list.revoke_token(&jti, "user123", expiry);
        assert!(list.is_revoked(&jti));
        assert_eq!(list.count(), 1);
    }

    #[test]
    fn test_revoke_all_user_tokens() {
        let list = TokenRevocationList::new();
        let expiry = Utc::now() + Duration::hours(1);

        // Revoke 3 tokens for user1
        for _ in 0..3 {
            let jti = Uuid::new_v4().to_string();
            list.revoke_token(&jti, "user1", expiry);
        }
        // Revoke 1 token for user2
        list.revoke_token(&Uuid::new_v4().to_string(), "user2", expiry);

        assert_eq!(list.count(), 4);

        // Bulk-revoking user1 reports the 3 tokens it acted on. It is
        // non-destructive: already-revoked tokens stay revoked and user2 is
        // untouched, so the total revoked count is unchanged.
        let revoked = list.revoke_all_user_tokens("user1").unwrap();
        assert_eq!(revoked, 3);
        assert_eq!(list.count(), 4);
    }

    #[test]
    fn test_revoke_all_revokes_active_issued_tokens() {
        let list = TokenRevocationList::new();
        let expiry = Utc::now() + Duration::hours(1);

        // Three active tokens issued to the user, none revoked yet.
        let jtis: Vec<String> = (0..3).map(|_| Uuid::new_v4().to_string()).collect();
        for jti in &jtis {
            list.track_issued_token(jti, "user1", expiry);
            assert!(!list.is_revoked(jti));
        }
        assert_eq!(list.count(), 0);

        // Password change / compromise -> every active token is now revoked.
        let revoked = list.revoke_all_user_tokens("user1").unwrap();
        assert_eq!(revoked, 3);
        for jti in &jtis {
            assert!(list.is_revoked(jti));
        }
        assert_eq!(list.count(), 3);
    }

    #[test]
    fn test_prune_expired() {
        let list = TokenRevocationList::new();
        let past_expiry = Utc::now() - Duration::hours(1);
        let future_expiry = Utc::now() + Duration::hours(1);

        list.revoke_token(&Uuid::new_v4().to_string(), "user1", past_expiry);
        list.revoke_token(&Uuid::new_v4().to_string(), "user1", future_expiry);

        assert_eq!(list.count(), 2);
        let pruned = list.prune_expired();
        assert_eq!(pruned, 1);
        assert_eq!(list.count(), 1);
    }

    #[test]
    fn test_is_not_revoked() {
        let list = TokenRevocationList::new();
        assert!(!list.is_revoked("nonexistent-jti"));
    }

    #[test]
    fn test_password_change_scenario() {
        let list = TokenRevocationList::new();
        let expiry = Utc::now() + Duration::hours(24);

        // User logs in on 3 devices — each issued token is tracked.
        let jti1 = Uuid::new_v4().to_string();
        let jti2 = Uuid::new_v4().to_string();
        let jti3 = Uuid::new_v4().to_string();
        list.track_issued_token(&jti1, "user1", expiry);
        list.track_issued_token(&jti2, "user1", expiry);
        list.track_issued_token(&jti3, "user1", expiry);

        // Nothing is revoked while the tokens are in normal use.
        assert!(!list.is_revoked(&jti1));
        assert!(!list.is_revoked(&jti2));
        assert!(!list.is_revoked(&jti3));

        // User changes password — all tokens revoked
        let count = list.revoke_all_user_tokens("user1").unwrap();
        assert_eq!(count, 3);

        // All 3 tokens are now rejected
        assert!(list.is_revoked(&jti1));
        assert!(list.is_revoked(&jti2));
        assert!(list.is_revoked(&jti3));
    }

    /// Redis-backed revocation tests. These require a reachable Redis (the URL
    /// is taken from `REDIS_URL`, defaulting to `redis://localhost:6379`) and
    /// are skipped when it is unavailable, matching the convention used by the
    /// rate limiting Redis tests.
    #[cfg(feature = "redis-cache")]
    mod redis_tests {
        use super::*;

        /// Build a Redis-backed list with a unique key prefix so tests are
        /// isolated from each other and from real data. Returns `None` when
        /// Redis is not reachable (test is skipped).
        fn redis_list() -> Option<TokenRevocationList> {
            let url =
                std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
            let client = redis::Client::open(url.as_str()).ok()?;
            let prefix = format!("soroban:test:revocation:{}:", Uuid::new_v4());
            TokenRevocationList::new_redis(client, prefix).ok()
        }

        #[test]
        fn test_redis_revoke_and_check() {
            let Some(list) = redis_list() else {
                println!("Skipping Redis test - Redis not available");
                return;
            };

            let jti = Uuid::new_v4().to_string();
            let expiry = Utc::now() + Duration::hours(1);

            assert!(!list.is_revoked(&jti));
            list.revoke_token(&jti, "user123", expiry);
            assert!(list.is_revoked(&jti));
            assert_eq!(list.count(), 1);
        }

        #[test]
        fn test_redis_revoke_all_user_tokens() {
            let Some(list) = redis_list() else {
                println!("Skipping Redis test - Redis not available");
                return;
            };

            let expiry = Utc::now() + Duration::hours(1);
            let jtis: Vec<String> = (0..3).map(|_| Uuid::new_v4().to_string()).collect();
            for jti in &jtis {
                list.track_issued_token(jti, "user1", expiry);
            }

            // Nothing revoked until the bulk operation.
            assert_eq!(list.count(), 0);
            let revoked = list.revoke_all_user_tokens("user1").unwrap();
            assert_eq!(revoked, 3);
            for jti in &jtis {
                assert!(list.is_revoked(jti));
            }

            // A different user's tokens are unaffected.
            let other = Uuid::new_v4().to_string();
            list.track_issued_token(&other, "user2", expiry);
            assert!(!list.is_revoked(&other));
        }

        /// The core fix for Issue #486: a revocation recorded through one
        /// instance (list) is immediately visible to a second instance
        /// sharing the same Redis — no in-memory state is involved.
        #[test]
        fn test_redis_revocation_shared_across_instances() {
            let url = std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string());
            let client = redis::Client::open(url.as_str()).ok();
            let Some(client) = client else {
                println!("Skipping Redis test - Redis not available");
                return;
            };

            let prefix = format!("soroban:test:revocation:{}:", Uuid::new_v4());
            let Ok(instance_a) = TokenRevocationList::new_redis(client.clone(), prefix.clone())
            else {
                println!("Skipping Redis test - Redis not available");
                return;
            };
            let Ok(instance_b) = TokenRevocationList::new_redis(client, prefix) else {
                println!("Skipping Redis test - Redis not available");
                return;
            };

            let jti = Uuid::new_v4().to_string();
            let expiry = Utc::now() + Duration::hours(1);
            instance_a.track_issued_token(&jti, "user1", expiry);

            // Instance B revokes; instance A sees it.
            instance_b.revoke_token(&jti, "user1", expiry);
            assert!(instance_a.is_revoked(&jti));
            assert!(instance_b.is_revoked(&jti));

            // Bulk revocation on B is visible to A as well.
            let jti2 = Uuid::new_v4().to_string();
            instance_a.track_issued_token(&jti2, "user1", expiry);
            assert!(!instance_a.is_revoked(&jti2));
            let revoked = instance_b.revoke_all_user_tokens("user1").unwrap();
            assert!(revoked >= 2);
            assert!(instance_a.is_revoked(&jti2));
        }
    }
}
