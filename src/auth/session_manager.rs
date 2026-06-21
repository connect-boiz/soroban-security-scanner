use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use uuid::Uuid;

#[cfg(feature = "redis-cache")]
use redis::{AsyncCommands, Client as RedisClient};

#[cfg(feature = "database")]
use sqlx::{postgres::PgPool, Row};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub session_id: String,
    pub user_id: String,
    pub user_email: String,
    pub user_role: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub last_accessed: chrono::DateTime<Utc>,
    pub expires_at: chrono::DateTime<Utc>,
    pub is_active: bool,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Session expired: {0}")]
    Expired(String),
    #[error("Session revoked: {0}")]
    Revoked(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Invalid session data: {0}")]
    InvalidData(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Redis error: {0}")]
    Redis(String),
}

#[async_trait::async_trait]
pub trait SessionStore: Send + Sync {
    async fn create_session(&self, session: SessionData) -> Result<(), SessionError>;
    async fn get_session(&self, session_id: &str) -> Result<Option<SessionData>, SessionError>;
    async fn update_session(&self, session: SessionData) -> Result<(), SessionError>;
    async fn delete_session(&self, session_id: &str) -> Result<(), SessionError>;
    async fn revoke_user_sessions(&self, user_id: &str) -> Result<(), SessionError>;
    async fn cleanup_expired_sessions(&self) -> Result<usize, SessionError>;
    async fn get_active_sessions(&self, user_id: &str) -> Result<Vec<SessionData>, SessionError>;
}

pub struct InMemorySessionStore {
    sessions: Arc<RwLock<HashMap<String, SessionData>>>,
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl SessionStore for InMemorySessionStore {
    async fn create_session(&self, session: SessionData) -> Result<(), SessionError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| SessionError::Storage(e.to_string()))?;
        sessions.insert(session.session_id.clone(), session);
        Ok(())
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<SessionData>, SessionError> {
        let sessions = self
            .sessions
            .read()
            .map_err(|e| SessionError::Storage(e.to_string()))?;
        Ok(sessions.get(session_id).cloned())
    }

    async fn update_session(&self, session: SessionData) -> Result<(), SessionError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| SessionError::Storage(e.to_string()))?;
        sessions.insert(session.session_id.clone(), session);
        Ok(())
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), SessionError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| SessionError::Storage(e.to_string()))?;
        sessions.remove(session_id).ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;
        Ok(())
    }

    async fn revoke_user_sessions(&self, user_id: &str) -> Result<usize, SessionError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| SessionError::Storage(e.to_string()))?;
        
        let mut count = 0;
        sessions.retain(|_, session| {
            if session.user_id == user_id {
                count += 1;
                false
            } else {
                true
            }
        });
        
        Ok(count)
    }

    async fn cleanup_expired_sessions(&self) -> Result<usize, SessionError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| SessionError::Storage(e.to_string()))?;
        
        let now = Utc::now();
        let mut count = 0;
        sessions.retain(|_, session| {
            if session.expires_at <= now {
                count += 1;
                false
            } else {
                true
            }
        });
        
        Ok(count)
    }

    async fn get_active_sessions(&self, user_id: &str) -> Result<Vec<SessionData>, SessionError> {
        let sessions = self
            .sessions
            .read()
            .map_err(|e| SessionError::Storage(e.to_string()))?;
        
        let now = Utc::now();
        let user_sessions: Vec<SessionData> = sessions
            .values()
            .filter(|session| session.user_id == user_id && session.expires_at > now && session.is_active)
            .cloned()
            .collect();
        
        Ok(user_sessions)
    }
}

#[cfg(feature = "redis-cache")]
pub struct RedisSessionStore {
    client: RedisClient,
    key_prefix: String,
}

#[cfg(feature = "redis-cache")]
impl RedisSessionStore {
    pub fn new(client: RedisClient, key_prefix: String) -> Self {
        Self { client, key_prefix }
    }

    fn session_key(&self, session_id: &str) -> String {
        format!("{}:session:{}", self.key_prefix, session_id)
    }

    fn user_sessions_key(&self, user_id: &str) -> String {
        format!("{}:user_sessions:{}", self.key_prefix, user_id)
    }
}

#[cfg(feature = "redis-cache")]
#[async_trait::async_trait]
impl SessionStore for RedisSessionStore {
    async fn create_session(&self, session: SessionData) -> Result<(), SessionError> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| SessionError::Redis(e.to_string()))?;

        let session_json = serde_json::to_string(&session)
            .map_err(|e| SessionError::Serialization(e.to_string()))?;

        let session_key = self.session_key(&session.session_id);
        let user_sessions_key = self.user_sessions_key(&session.user_id);

        // Store session data with TTL
        let ttl = (session.expires_at - Utc::now()).num_seconds() as usize;
        conn.set_ex(&session_key, session_json, ttl)
            .await
            .map_err(|e| SessionError::Redis(e.to_string()))?;

        // Add session to user's session list
        conn.sadd(&user_sessions_key, &session.session_id)
            .await
            .map_err(|e| SessionError::Redis(e.to_string()))?;

        Ok(())
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<SessionData>, SessionError> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| SessionError::Redis(e.to_string()))?;

        let session_key = self.session_key(session_id);
        let session_json: Option<String> = conn
            .get(&session_key)
            .await
            .map_err(|e| SessionError::Redis(e.to_string()))?;

        match session_json {
            Some(json) => {
                let session = serde_json::from_str(&json)
                    .map_err(|e| SessionError::Serialization(e.to_string()))?;
                Ok(Some(session))
            }
            None => Ok(None),
        }
    }

    async fn update_session(&self, session: SessionData) -> Result<(), SessionError> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| SessionError::Redis(e.to_string()))?;

        let session_json = serde_json::to_string(&session)
            .map_err(|e| SessionError::Serialization(e.to_string()))?;

        let session_key = self.session_key(&session.session_id);
        let ttl = (session.expires_at - Utc::now()).num_seconds() as usize;

        conn.set_ex(&session_key, session_json, ttl)
            .await
            .map_err(|e| SessionError::Redis(e.to_string()))?;

        Ok(())
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), SessionError> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| SessionError::Redis(e.to_string()))?;

        let session_key = self.session_key(session_id);
        
        // Get session data to find user_id
        let session: Option<SessionData> = self.get_session(session_id).await?;
        
        if let Some(session_data) = session {
            let user_sessions_key = self.user_sessions_key(&session_data.user_id);
            
            // Remove from user's session list
            conn.srem(&user_sessions_key, session_id)
                .await
                .map_err(|e| SessionError::Redis(e.to_string()))?;
        }

        // Delete session
        conn.del(&session_key)
            .await
            .map_err(|e| SessionError::Redis(e.to_string()))?;

        Ok(())
    }

    async fn revoke_user_sessions(&self, user_id: &str) -> Result<usize, SessionError> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| SessionError::Redis(e.to_string()))?;

        let user_sessions_key = self.user_sessions_key(user_id);
        
        // Get all session IDs for the user
        let session_ids: Vec<String> = conn
            .smembers(&user_sessions_key)
            .await
            .map_err(|e| SessionError::Redis(e.to_string()))?;

        // Delete each session
        let mut count = 0;
        for session_id in &session_ids {
            let session_key = self.session_key(session_id);
            conn.del(&session_key)
                .await
                .map_err(|e| SessionError::Redis(e.to_string()))?;
            count += 1;
        }

        // Clear user's session list
        conn.del(&user_sessions_key)
            .await
            .map_err(|e| SessionError::Redis(e.to_string()))?;

        Ok(count)
    }

    async fn cleanup_expired_sessions(&self) -> Result<usize, SessionError> {
        // Redis automatically handles expiration, so we just need to clean up user session lists
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| SessionError::Redis(e.to_string()))?;

        // This is a simplified cleanup - in production you'd want a more sophisticated approach
        let pattern = format!("{}:user_sessions:*", self.key_prefix);
        let keys: Vec<String> = conn
            .keys(&pattern)
            .await
            .map_err(|e| SessionError::Redis(e.to_string()))?;

        let mut total_cleaned = 0;
        for user_sessions_key in keys {
            let session_ids: Vec<String> = conn
                .smembers(&user_sessions_key)
                .await
                .map_err(|e| SessionError::Redis(e.to_string()))?;

            let mut valid_sessions = Vec::new();
            for session_id in session_ids {
                let session_key = self.session_key(&session_id);
                let exists: bool = conn
                    .exists(&session_key)
                    .await
                    .map_err(|e| SessionError::Redis(e.to_string()))?;
                
                if exists {
                    valid_sessions.push(session_id);
                }
            }

            // Update user session list with only valid sessions
            conn.del(&user_sessions_key)
                .await
                .map_err(|e| SessionError::Redis(e.to_string()))?;

            if !valid_sessions.is_empty() {
                conn.sadd(&user_sessions_key, &valid_sessions)
                    .await
                    .map_err(|e| SessionError::Redis(e.to_string()))?;
            }

            total_cleaned += valid_sessions.len();
        }

        Ok(total_cleaned)
    }

    async fn get_active_sessions(&self, user_id: &str) -> Result<Vec<SessionData>, SessionError> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| SessionError::Redis(e.to_string()))?;

        let user_sessions_key = self.user_sessions_key(user_id);
        let session_ids: Vec<String> = conn
            .smembers(&user_sessions_key)
            .await
            .map_err(|e| SessionError::Redis(e.to_string()))?;

        let mut active_sessions = Vec::new();
        for session_id in session_ids {
            if let Some(session) = self.get_session(&session_id).await? {
                if session.is_active && session.expires_at > Utc::now() {
                    active_sessions.push(session);
                }
            }
        }

        Ok(active_sessions)
    }
}

pub struct SessionManager<S: SessionStore> {
    store: S,
    default_ttl: Duration,
}

impl<S: SessionStore> SessionManager<S> {
    pub fn new(store: S, default_ttl: Duration) -> Self {
        Self { store, default_ttl }
    }

    pub async fn create_session(
        &self,
        user_id: &str,
        user_email: &str,
        user_role: &str,
        ip_address: Option<String>,
        user_agent: Option<String>,
        custom_ttl: Option<Duration>,
    ) -> Result<String, SessionError> {
        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let ttl = custom_ttl.unwrap_or(self.default_ttl);
        let expires_at = now + ttl;

        let session = SessionData {
            session_id: session_id.clone(),
            user_id: user_id.to_string(),
            user_email: user_email.to_string(),
            user_role: user_role.to_string(),
            ip_address,
            user_agent,
            created_at: now,
            last_accessed: now,
            expires_at,
            is_active: true,
            metadata: HashMap::new(),
        };

        self.store.create_session(session).await?;
        Ok(session_id)
    }

    pub async fn validate_session(&self, session_id: &str) -> Result<SessionData, SessionError> {
        let mut session = self
            .store
            .get_session(session_id)
            .await?
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;

        if !session.is_active {
            return Err(SessionError::Revoked(session_id.to_string()));
        }

        if session.expires_at <= Utc::now() {
            return Err(SessionError::Expired(session_id.to_string()));
        }

        // Update last accessed time
        session.last_accessed = Utc::now();
        self.store.update_session(session.clone()).await?;

        Ok(session)
    }

    pub async fn refresh_session(
        &self,
        session_id: &str,
        new_ttl: Option<Duration>,
    ) -> Result<(), SessionError> {
        let mut session = self
            .store
            .get_session(session_id)
            .await?
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;

        if !session.is_active {
            return Err(SessionError::Revoked(session_id.to_string()));
        }

        let ttl = new_ttl.unwrap_or(self.default_ttl);
        session.expires_at = Utc::now() + ttl;
        session.last_accessed = Utc::now();

        self.store.update_session(session).await?;
        Ok(())
    }

    pub async fn revoke_session(&self, session_id: &str) -> Result<(), SessionError> {
        let mut session = self
            .store
            .get_session(session_id)
            .await?
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;

        session.is_active = false;
        self.store.update_session(session).await?;
        Ok(())
    }

    pub async fn revoke_all_user_sessions(&self, user_id: &str) -> Result<usize, SessionError> {
        self.store.revoke_user_sessions(user_id).await
    }

    pub async fn get_user_sessions(&self, user_id: &str) -> Result<Vec<SessionData>, SessionError> {
        self.store.get_active_sessions(user_id).await
    }

    pub async fn cleanup_expired_sessions(&self) -> Result<usize, SessionError> {
        self.store.cleanup_expired_sessions().await
    }

    pub async fn update_session_metadata(
        &self,
        session_id: &str,
        key: &str,
        value: &str,
    ) -> Result<(), SessionError> {
        let mut session = self
            .store
            .get_session(session_id)
            .await?
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;

        session.metadata.insert(key.to_string(), value.to_string());
        session.last_accessed = Utc::now();

        self.store.update_session(session).await?;
        Ok(())
    }

    pub async fn get_session_metadata(&self, session_id: &str, key: &str) -> Result<Option<String>, SessionError> {
        let session = self
            .store
            .get_session(session_id)
            .await?
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;

        Ok(session.metadata.get(key).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_in_memory_session_management() {
        let store = InMemorySessionStore::new();
        let manager = SessionManager::new(store, Duration::hours(24));

        let session_id = manager
            .create_session(
                "user123",
                "test@example.com",
                "admin",
                Some("127.0.0.1".to_string()),
                Some("Mozilla/5.0".to_string()),
                None,
            )
            .await
            .unwrap();

        let session = manager.validate_session(&session_id).await.unwrap();
        assert_eq!(session.user_id, "user123");
        assert_eq!(session.user_email, "test@example.com");
        assert_eq!(session.user_role, "admin");

        manager.revoke_session(&session_id).await.unwrap();
        let result = manager.validate_session(&session_id).await;
        assert!(matches!(result, Err(SessionError::Revoked(_))));
    }

    #[tokio::test]
    async fn test_session_expiration() {
        let store = InMemorySessionStore::new();
        let manager = SessionManager::new(store, Duration::seconds(1));

        let session_id = manager
            .create_session("user123", "test@example.com", "user", None, None, None)
            .await
            .unwrap();

        // Wait for expiration
        tokio::time::sleep(Duration::seconds(2).to_std().unwrap()).await;

        let result = manager.validate_session(&session_id).await;
        assert!(matches!(result, Err(SessionError::Expired(_))));
    }

    #[tokio::test]
    async fn test_multiple_user_sessions() {
        let store = InMemorySessionStore::new();
        let manager = SessionManager::new(store, Duration::hours(24));

        let session1 = manager
            .create_session("user123", "test@example.com", "user", None, None, None)
            .await
            .unwrap();
        let session2 = manager
            .create_session("user123", "test@example.com", "user", None, None, None)
            .await
            .unwrap();

        let sessions = manager.get_user_sessions("user123").await.unwrap();
        assert_eq!(sessions.len(), 2);

        let revoked_count = manager.revoke_all_user_sessions("user123").await.unwrap();
        assert_eq!(revoked_count, 2);

        let sessions = manager.get_user_sessions("user123").await.unwrap();
        assert_eq!(sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_session_metadata() {
        let store = InMemorySessionStore::new();
        let manager = SessionManager::new(store, Duration::hours(24));

        let session_id = manager
            .create_session("user123", "test@example.com", "user", None, None, None)
            .await
            .unwrap();

        manager
            .update_session_metadata(&session_id, "last_action", "login")
            .await
            .unwrap();

        let value = manager
            .get_session_metadata(&session_id, "last_action")
            .await
            .unwrap();
        assert_eq!(value, Some("login".to_string()));
    }
}
