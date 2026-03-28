use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClaims {
    pub session_id: String,
    pub subject: String,
    pub issued_at: i64,
    pub expires_at: i64,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStoreRecord {
    pub session_id: String,
    pub subject: String,
    pub expires_at: i64,
    pub revoked: bool,
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("invalid token")]
    InvalidToken,
    #[error("token expired")]
    Expired,
    #[error("session revoked")]
    Revoked,
    #[error("session not found")]
    NotFound,
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("storage error: {0}")]
    Storage(String),
}

pub trait ExternalSessionStore: Send + Sync {
    fn put(&self, record: SessionStoreRecord) -> Result<(), SessionError>;
    fn get(&self, session_id: &str) -> Result<Option<SessionStoreRecord>, SessionError>;
    fn revoke(&self, session_id: &str) -> Result<(), SessionError>;
}

#[derive(Clone, Default)]
pub struct InMemorySessionStore {
    records: Arc<RwLock<HashMap<String, SessionStoreRecord>>>,
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ExternalSessionStore for InMemorySessionStore {
    fn put(&self, record: SessionStoreRecord) -> Result<(), SessionError> {
        let mut map = self
            .records
            .write()
            .map_err(|err| SessionError::Storage(err.to_string()))?;
        map.insert(record.session_id.clone(), record);
        Ok(())
    }

    fn get(&self, session_id: &str) -> Result<Option<SessionStoreRecord>, SessionError> {
        let map = self
            .records
            .read()
            .map_err(|err| SessionError::Storage(err.to_string()))?;
        Ok(map.get(session_id).cloned())
    }

    fn revoke(&self, session_id: &str) -> Result<(), SessionError> {
        let mut map = self
            .records
            .write()
            .map_err(|err| SessionError::Storage(err.to_string()))?;
        if let Some(record) = map.get_mut(session_id) {
            record.revoked = true;
            return Ok(());
        }
        Err(SessionError::NotFound)
    }
}

pub struct StatelessSessionManager<S: ExternalSessionStore> {
    signing_key: Vec<u8>,
    store: S,
}

impl<S: ExternalSessionStore> StatelessSessionManager<S> {
    pub fn new(signing_key: impl AsRef<[u8]>, store: S) -> Self {
        Self {
            signing_key: signing_key.as_ref().to_vec(),
            store,
        }
    }

    pub fn issue(
        &self,
        subject: impl Into<String>,
        scopes: Vec<String>,
        ttl: Duration,
    ) -> Result<String, SessionError> {
        let now = Utc::now().timestamp();
        let expires_at = now + ttl.as_secs() as i64;
        let claims = SessionClaims {
            session_id: Uuid::new_v4().to_string(),
            subject: subject.into(),
            issued_at: now,
            expires_at,
            scopes,
        };

        self.store.put(SessionStoreRecord {
            session_id: claims.session_id.clone(),
            subject: claims.subject.clone(),
            expires_at: claims.expires_at,
            revoked: false,
        })?;

        self.encode_token(&claims)
    }

    pub fn validate(&self, token: &str) -> Result<SessionClaims, SessionError> {
        let claims = self.decode_token(token)?;
        let now = Utc::now().timestamp();
        if claims.expires_at <= now {
            return Err(SessionError::Expired);
        }

        let record = self
            .store
            .get(&claims.session_id)?
            .ok_or(SessionError::NotFound)?;

        if record.revoked {
            return Err(SessionError::Revoked);
        }

        if record.expires_at <= now {
            return Err(SessionError::Expired);
        }

        Ok(claims)
    }

    pub fn revoke(&self, token: &str) -> Result<(), SessionError> {
        let claims = self.decode_token(token)?;
        self.store.revoke(&claims.session_id)
    }

    fn encode_token(&self, claims: &SessionClaims) -> Result<String, SessionError> {
        let payload = serde_json::to_vec(claims)
            .map_err(|err| SessionError::Serialization(err.to_string()))?;
        let payload_b64 = URL_SAFE_NO_PAD.encode(payload);
        let signature = self.sign(payload_b64.as_bytes());
        Ok(format!("{}.{}", payload_b64, signature))
    }

    fn decode_token(&self, token: &str) -> Result<SessionClaims, SessionError> {
        let mut segments = token.split('.');
        let payload_segment = segments.next().ok_or(SessionError::InvalidToken)?;
        let signature_segment = segments.next().ok_or(SessionError::InvalidToken)?;

        if segments.next().is_some() {
            return Err(SessionError::InvalidToken);
        }

        let expected_signature = self.sign(payload_segment.as_bytes());
        if expected_signature != signature_segment {
            return Err(SessionError::InvalidToken);
        }

        let payload = URL_SAFE_NO_PAD
            .decode(payload_segment)
            .map_err(|_| SessionError::InvalidToken)?;
        let claims = serde_json::from_slice::<SessionClaims>(&payload)
            .map_err(|err| SessionError::Serialization(err.to_string()))?;
        Ok(claims)
    }

    fn sign(&self, payload: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.signing_key);
        hasher.update(payload);
        let digest = hasher.finalize();
        hex::encode(digest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_stateless_session_with_external_store() {
        let manager = StatelessSessionManager::new("test-signing-key", InMemorySessionStore::new());
        let token = manager
            .issue("user-123", vec!["scan:read".to_string()], Duration::from_secs(60))
            .expect("token should be issued");

        let claims = manager.validate(&token).expect("token should validate");
        assert_eq!(claims.subject, "user-123");
    }

    #[test]
    fn revocation_is_enforced_without_session_affinity() {
        let manager = StatelessSessionManager::new("test-signing-key", InMemorySessionStore::new());
        let token = manager
            .issue("user-456", vec!["scan:write".to_string()], Duration::from_secs(60))
            .expect("token should be issued");

        manager.revoke(&token).expect("token should be revoked");
        let result = manager.validate(&token);
        assert!(matches!(result, Err(SessionError::Revoked)));
    }
}
