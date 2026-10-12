use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthUserInfo {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub provider: String,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
    pub auth_url: String,
    pub token_url: String,
    pub scopes: Vec<String>,
    #[serde(default)]
    pub use_pkce: bool,
}

impl OAuthConfig {
    pub fn with_pkce(mut self) -> Self {
        self.use_pkce = true;
        self
    }
}

#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("OAuth configuration error: {0}")]
    ConfigError(String),
    #[error("Authorization URL generation failed: {0}")]
    AuthUrlError(String),
    #[error("Token exchange failed: {0}")]
    TokenError(String),
    #[error("User info retrieval failed: {0}")]
    UserInfoError(String),
    #[error("Invalid state parameter")]
    InvalidState,
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

#[derive(Debug, Clone)]
pub enum OAuthProvider {
    Google,
    GitHub,
    Microsoft,
    Discord,
    Facebook,
}

impl OAuthProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            OAuthProvider::Google => "google",
            OAuthProvider::GitHub => "github",
            OAuthProvider::Microsoft => "microsoft",
            OAuthProvider::Discord => "discord",
            OAuthProvider::Facebook => "facebook",
        }
    }

    pub fn default_config(&self) -> OAuthConfig {
        match self {
            OAuthProvider::Google => OAuthConfig {
                client_id: "".to_string(),
                client_secret: "".to_string(),
                redirect_url: "http://localhost:3000/auth/google/callback".to_string(),
                auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
                token_url: "https://oauth2.googleapis.com/token".to_string(),
                scopes: vec![
                    "https://www.googleapis.com/auth/userinfo.email".to_string(),
                    "https://www.googleapis.com/auth/userinfo.profile".to_string(),
                ],
                use_pkce: false,
            },
            OAuthProvider::GitHub => OAuthConfig {
                client_id: "".to_string(),
                client_secret: "".to_string(),
                redirect_url: "http://localhost:3000/auth/github/callback".to_string(),
                auth_url: "https://github.com/login/oauth/authorize".to_string(),
                token_url: "https://github.com/login/oauth/access_token".to_string(),
                scopes: vec!["user:email".to_string()],
                use_pkce: false,
            },
            OAuthProvider::Microsoft => OAuthConfig {
                client_id: "".to_string(),
                client_secret: "".to_string(),
                redirect_url: "http://localhost:3000/auth/microsoft/callback".to_string(),
                auth_url: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize"
                    .to_string(),
                token_url: "https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string(),
                scopes: vec![
                    "openid".to_string(),
                    "email".to_string(),
                    "profile".to_string(),
                ],
                use_pkce: false,
            },
            OAuthProvider::Discord => OAuthConfig {
                client_id: "".to_string(),
                client_secret: "".to_string(),
                redirect_url: "http://localhost:3000/auth/discord/callback".to_string(),
                auth_url: "https://discord.com/api/oauth2/authorize".to_string(),
                token_url: "https://discord.com/api/oauth2/token".to_string(),
                scopes: vec!["identify".to_string(), "email".to_string()],
                use_pkce: false,
            },
            OAuthProvider::Facebook => OAuthConfig {
                client_id: "".to_string(),
                client_secret: "".to_string(),
                redirect_url: "http://localhost:3000/auth/facebook/callback".to_string(),
                auth_url: "https://www.facebook.com/v18.0/dialog/oauth".to_string(),
                token_url: "https://graph.facebook.com/v18.0/oauth/access_token".to_string(),
                scopes: vec!["email".to_string(), "public_profile".to_string()],
                use_pkce: false,
            },
        }
    }
}

/// A stored OAuth state token entry with expiry timing and optional PKCE verifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateEntry {
    pub provider: String,
    pub expires_at: u64,
    #[serde(default)]
    pub pkce_verifier: Option<String>,
}

#[async_trait::async_trait]
pub trait OAuthStateStore: Send + Sync {
    async fn store_state(&self, state: &str, entry: StateEntry) -> Result<(), OAuthError>;
    async fn validate_and_remove_state(&self, state: &str) -> Result<StateEntry, OAuthError>;
    async fn cleanup_expired(&self) -> Result<usize, OAuthError>;
}

#[derive(Default)]
pub struct InMemoryOAuthStateStore {
    store: Mutex<HashMap<String, StateEntry>>,
}

impl InMemoryOAuthStateStore {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl OAuthStateStore for InMemoryOAuthStateStore {
    async fn store_state(&self, state: &str, entry: StateEntry) -> Result<(), OAuthError> {
        let mut store = self.store.lock().map_err(|e| {
            OAuthError::ConfigError(format!("Failed to acquire state store lock: {}", e))
        })?;
        store.insert(state.to_string(), entry);
        Ok(())
    }

    async fn validate_and_remove_state(&self, state: &str) -> Result<StateEntry, OAuthError> {
        let mut store = self.store.lock().map_err(|e| {
            OAuthError::ConfigError(format!("Failed to acquire state store lock: {}", e))
        })?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let entry = store.remove(state).ok_or(OAuthError::InvalidState)?;

        if now > entry.expires_at {
            return Err(OAuthError::InvalidState);
        }

        Ok(entry)
    }

    async fn cleanup_expired(&self) -> Result<usize, OAuthError> {
        let mut store = self.store.lock().map_err(|e| {
            OAuthError::ConfigError(format!("Failed to acquire state store lock: {}", e))
        })?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let before = store.len();
        store.retain(|_, entry| entry.expires_at > now);
        Ok(before - store.len())
    }
}

#[cfg(feature = "redis-cache")]
pub struct RedisOAuthStateStore {
    client: redis::Client,
    key_prefix: String,
}

#[cfg(feature = "redis-cache")]
impl RedisOAuthStateStore {
    pub fn new(client: redis::Client, key_prefix: String) -> Self {
        Self { client, key_prefix }
    }

    fn state_key(&self, state: &str) -> String {
        format!("{}:oauth:state:{}", self.key_prefix, state)
    }
}

#[cfg(feature = "redis-cache")]
#[async_trait::async_trait]
impl OAuthStateStore for RedisOAuthStateStore {
    async fn store_state(&self, state: &str, entry: StateEntry) -> Result<(), OAuthError> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| OAuthError::NetworkError(format!("Redis connection failed: {}", e)))?;

        let json = serde_json::to_string(&entry)
            .map_err(|e| OAuthError::SerializationError(e.to_string()))?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let ttl = entry.expires_at.saturating_sub(now).max(1);

        redis::cmd("SETEX")
            .arg(self.state_key(state))
            .arg(ttl)
            .arg(json)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| OAuthError::NetworkError(format!("Redis setex failed: {}", e)))?;

        Ok(())
    }

    async fn validate_and_remove_state(&self, state: &str) -> Result<StateEntry, OAuthError> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| OAuthError::NetworkError(format!("Redis connection failed: {}", e)))?;

        let key = self.state_key(state);

        let script = r#"
            local val = redis.call('GET', KEYS[1])
            if val then
                redis.call('DEL', KEYS[1])
            end
            return val
        "#;

        let json_opt: Option<String> = redis::Script::new(script)
            .key(&key)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| OAuthError::NetworkError(format!("Redis script failed: {}", e)))?;

        let json = json_opt.ok_or(OAuthError::InvalidState)?;
        let entry: StateEntry = serde_json::from_str(&json)
            .map_err(|e| OAuthError::SerializationError(e.to_string()))?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if now > entry.expires_at {
            return Err(OAuthError::InvalidState);
        }

        Ok(entry)
    }

    async fn cleanup_expired(&self) -> Result<usize, OAuthError> {
        Ok(0)
    }
}

pub struct SignedOAuthStateStore {
    secret: Vec<u8>,
}

impl SignedOAuthStateStore {
    pub fn new(secret: impl Into<Vec<u8>>) -> Self {
        Self {
            secret: secret.into(),
        }
    }

    pub fn generate_signed_state(&self, entry: &StateEntry) -> Result<String, OAuthError> {
        use base64::Engine;
        let json_bytes = serde_json::to_vec(entry)
            .map_err(|e| OAuthError::SerializationError(e.to_string()))?;
        let hmac_hex = self.compute_hmac(&json_bytes);
        let encoded_payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&json_bytes);
        Ok(format!("{}.{}", encoded_payload, hmac_hex))
    }

    fn compute_hmac(&self, data: &[u8]) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.secret).expect("HMAC can take any key length");
        mac.update(data);
        hex::encode(mac.finalize().into_bytes())
    }
}

#[async_trait::async_trait]
impl OAuthStateStore for SignedOAuthStateStore {
    async fn store_state(&self, _state: &str, _entry: StateEntry) -> Result<(), OAuthError> {
        Ok(())
    }

    async fn validate_and_remove_state(&self, state: &str) -> Result<StateEntry, OAuthError> {
        use base64::Engine;
        let parts: Vec<&str> = state.split('.').collect();
        if parts.len() != 2 {
            return Err(OAuthError::InvalidState);
        }

        let json_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[0])
            .map_err(|_| OAuthError::InvalidState)?;

        let expected_hmac = self.compute_hmac(&json_bytes);
        if parts[1] != expected_hmac {
            return Err(OAuthError::InvalidState);
        }

        let entry: StateEntry = serde_json::from_slice(&json_bytes)
            .map_err(|_| OAuthError::InvalidState)?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if now > entry.expires_at {
            return Err(OAuthError::InvalidState);
        }

        Ok(entry)
    }

    async fn cleanup_expired(&self) -> Result<usize, OAuthError> {
        Ok(0)
    }
}

/// OAuth service for managing social login integrations
pub struct OAuthService {
    clients: HashMap<String, BasicClient>,
    configs: HashMap<String, OAuthConfig>,
    pub(crate) state_store: Mutex<HashMap<String, StateEntry>>,
    custom_state_store: Option<Arc<dyn OAuthStateStore>>,
}

impl Default for OAuthService {
    fn default() -> Self {
        Self::new()
    }
}

impl OAuthService {
    /// Default state TTL: 10 minutes
    pub const STATE_TTL_SECONDS: u64 = 600;

    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            configs: HashMap::new(),
            state_store: Mutex::new(HashMap::new()),
            custom_state_store: None,
        }
    }

    pub fn with_state_store(mut self, custom_state_store: Arc<dyn OAuthStateStore>) -> Self {
        self.custom_state_store = Some(custom_state_store);
        self
    }

    #[cfg(feature = "redis-cache")]
    pub fn with_redis_store(self, client: redis::Client, key_prefix: String) -> Self {
        self.with_state_store(Arc::new(RedisOAuthStateStore::new(client, key_prefix)))
    }

    pub fn with_signed_state(self, secret: impl Into<Vec<u8>>) -> Self {
        self.with_state_store(Arc::new(SignedOAuthStateStore::new(secret)))
    }

    pub fn add_provider(
        &mut self,
        provider: OAuthProvider,
        config: OAuthConfig,
    ) -> Result<(), OAuthError> {
        let client_id = ClientId::new(config.client_id.clone());
        let client_secret = ClientSecret::new(config.client_secret.clone());

        let auth_url = AuthUrl::new(config.auth_url.clone())
            .map_err(|e| OAuthError::ConfigError(format!("Invalid auth URL: {}", e)))?;

        let token_url = TokenUrl::new(config.token_url.clone())
            .map_err(|e| OAuthError::ConfigError(format!("Invalid token URL: {}", e)))?;

        let redirect_url = RedirectUrl::new(config.redirect_url.clone())
            .map_err(|e| OAuthError::ConfigError(format!("Invalid redirect URL: {}", e)))?;

        let client = BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
            .set_redirect_uri(redirect_url);

        let provider_name = provider.as_str().to_string();
        self.clients.insert(provider_name.clone(), client);
        self.configs.insert(provider_name, config);

        Ok(())
    }

    pub fn get_authorization_url(&self, provider: &str) -> Result<(String, CsrfToken), OAuthError> {
        let config = self.configs.get(provider).ok_or_else(|| {
            OAuthError::ConfigError(format!("Provider {} config not found", provider))
        })?;

        if config.use_pkce {
            let (url, csrf_token, _) = self.get_authorization_url_with_pkce(provider)?;
            Ok((url, csrf_token))
        } else {
            let (url, csrf_token, _) = self.generate_auth_url(provider, false)?;
            self.store_state(provider, csrf_token.secret())?;
            Ok((url, csrf_token))
        }
    }

    pub fn get_authorization_url_with_pkce(
        &self,
        provider: &str,
    ) -> Result<(String, CsrfToken, PkceCodeVerifier), OAuthError> {
        let (url, csrf_token, pkce_verifier) = self.generate_auth_url(provider, true)?;
        let verifier = pkce_verifier.ok_or_else(|| {
            OAuthError::ConfigError("Failed to generate PKCE verifier".to_string())
        })?;
        self.store_state_with_pkce(provider, csrf_token.secret(), Some(verifier.secret().clone()))?;
        Ok((url, csrf_token, verifier))
    }

    pub async fn get_authorization_url_async(
        &self,
        provider: &str,
    ) -> Result<(String, CsrfToken), OAuthError> {
        let config = self.configs.get(provider).ok_or_else(|| {
            OAuthError::ConfigError(format!("Provider {} config not found", provider))
        })?;

        if config.use_pkce {
            let (url, csrf_token, _) = self.get_authorization_url_with_pkce_async(provider).await?;
            Ok((url, csrf_token))
        } else {
            let (url, csrf_token, _) = self.generate_auth_url(provider, false)?;
            self.store_state_async(provider, csrf_token.secret(), None).await?;
            Ok((url, csrf_token))
        }
    }

    pub async fn get_authorization_url_with_pkce_async(
        &self,
        provider: &str,
    ) -> Result<(String, CsrfToken, PkceCodeVerifier), OAuthError> {
        let (url, csrf_token, pkce_verifier) = self.generate_auth_url(provider, true)?;
        let verifier = pkce_verifier.ok_or_else(|| {
            OAuthError::ConfigError("Failed to generate PKCE verifier".to_string())
        })?;
        self.store_state_async(provider, csrf_token.secret(), Some(verifier.secret().clone())).await?;
        Ok((url, csrf_token, verifier))
    }

    fn generate_auth_url(
        &self,
        provider: &str,
        use_pkce: bool,
    ) -> Result<(String, CsrfToken, Option<PkceCodeVerifier>), OAuthError> {
        let client = self.clients.get(provider).ok_or_else(|| {
            OAuthError::ConfigError(format!("Provider {} not configured", provider))
        })?;

        let config = self.configs.get(provider).ok_or_else(|| {
            OAuthError::ConfigError(format!("Provider {} config not found", provider))
        })?;

        let mut auth_request = client.authorize_url(CsrfToken::new_random);

        for scope in &config.scopes {
            auth_request = auth_request.add_scope(Scope::new(scope.clone()));
        }

        let pkce_verifier = if use_pkce || config.use_pkce {
            let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
            auth_request = auth_request.set_pkce_challenge(challenge);
            Some(verifier)
        } else {
            None
        };

        let (auth_url, csrf_token) = auth_request.url();

        Ok((auth_url.to_string(), csrf_token, pkce_verifier))
    }

    /// Store a state token so it can be validated on callback
    pub fn store_state(&self, provider: &str, state: &str) -> Result<(), OAuthError> {
        self.store_state_with_pkce(provider, state, None)
    }

    pub fn store_state_with_pkce(
        &self,
        provider: &str,
        state: &str,
        pkce_verifier: Option<String>,
    ) -> Result<(), OAuthError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let entry = StateEntry {
            provider: provider.to_string(),
            expires_at: now + Self::STATE_TTL_SECONDS,
            pkce_verifier,
        };

        let mut store = self.state_store.lock().map_err(|e| {
            OAuthError::ConfigError(format!("Failed to acquire state store lock: {}", e))
        })?;

        store.insert(state.to_string(), entry);
        Ok(())
    }

    async fn store_state_async(
        &self,
        provider: &str,
        state: &str,
        pkce_verifier: Option<String>,
    ) -> Result<(), OAuthError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let entry = StateEntry {
            provider: provider.to_string(),
            expires_at: now + Self::STATE_TTL_SECONDS,
            pkce_verifier: pkce_verifier.clone(),
        };

        if let Some(ref custom_store) = self.custom_state_store {
            custom_store.store_state(state, entry).await?;
        } else {
            self.store_state_with_pkce(provider, state, pkce_verifier)?;
        }

        Ok(())
    }

    /// Validate that a state token exists and has not expired
    pub fn validate_state(&self, state: &str) -> Result<String, OAuthError> {
        let entry = self.validate_state_entry(state)?;
        Ok(entry.provider)
    }

    pub fn validate_state_entry(&self, state: &str) -> Result<StateEntry, OAuthError> {
        let mut store = self.state_store.lock().map_err(|e| {
            OAuthError::ConfigError(format!("Failed to acquire state store lock: {}", e))
        })?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let entry = store.remove(state).ok_or(OAuthError::InvalidState)?;

        if now > entry.expires_at {
            return Err(OAuthError::InvalidState);
        }

        Ok(entry)
    }

    pub async fn validate_state_async(&self, state: &str) -> Result<StateEntry, OAuthError> {
        if let Some(ref custom) = self.custom_state_store {
            return custom.validate_and_remove_state(state).await;
        }
        self.validate_state_entry(state)
    }

    /// Clean up expired state tokens
    pub fn cleanup_expired_states(&self) -> Result<usize, OAuthError> {
        let mut store = self.state_store.lock().map_err(|e| {
            OAuthError::ConfigError(format!("Failed to acquire state store lock: {}", e))
        })?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let before = store.len();
        store.retain(|_, entry| entry.expires_at > now);
        let cleaned = before - store.len();

        Ok(cleaned)
    }

    pub async fn cleanup_expired_states_async(&self) -> Result<usize, OAuthError> {
        let mut cleaned = self.cleanup_expired_states()?;
        if let Some(ref custom) = self.custom_state_store {
            cleaned += custom.cleanup_expired().await?;
        }
        Ok(cleaned)
    }

    pub async fn exchange_code_for_token(
        &self,
        provider: &str,
        code: &str,
        state: &str,
    ) -> Result<OAuthUserInfo, OAuthError> {
        // Validate the state parameter to prevent CSRF attacks
        let entry = self.validate_state_async(state).await?;

        // Ensure the state belongs to the expected provider
        if entry.provider != provider {
            return Err(OAuthError::InvalidState);
        }

        let client = self.clients.get(provider).ok_or_else(|| {
            OAuthError::ConfigError(format!("Provider {} not configured", provider))
        })?;

        let mut req = client.exchange_code(oauth2::AuthorizationCode::new(code.to_string()));
        if let Some(ref verifier_secret) = entry.pkce_verifier {
            req = req.set_pkce_verifier(PkceCodeVerifier::new(verifier_secret.clone()));
        }

        let token_response = req
            .request_async(oauth2::reqwest::async_http_client)
            .await
            .map_err(|e| OAuthError::TokenError(format!("Failed to exchange code: {}", e)))?;

        let access_token = token_response.access_token().secret();
        let user_info = self.get_user_info(provider, access_token).await?;

        Ok(user_info)
    }

    pub async fn exchange_code_for_token_with_pkce(
        &self,
        provider: &str,
        code: &str,
        state: &str,
        pkce_verifier: PkceCodeVerifier,
    ) -> Result<OAuthUserInfo, OAuthError> {
        let entry = self.validate_state_async(state).await?;
        if entry.provider != provider {
            return Err(OAuthError::InvalidState);
        }

        let client = self.clients.get(provider).ok_or_else(|| {
            OAuthError::ConfigError(format!("Provider {} not configured", provider))
        })?;

        let token_response = client
            .exchange_code(oauth2::AuthorizationCode::new(code.to_string()))
            .set_pkce_verifier(pkce_verifier)
            .request_async(oauth2::reqwest::async_http_client)
            .await
            .map_err(|e| OAuthError::TokenError(format!("Failed to exchange code: {}", e)))?;

        let access_token = token_response.access_token().secret();
        let user_info = self.get_user_info(provider, access_token).await?;

        Ok(user_info)
    }

    async fn get_user_info(
        &self,
        provider: &str,
        access_token: &str,
    ) -> Result<OAuthUserInfo, OAuthError> {
        match provider {
            "google" => self.get_google_user_info(access_token).await,
            "github" => self.get_github_user_info(access_token).await,
            "microsoft" => self.get_microsoft_user_info(access_token).await,
            "discord" => self.get_discord_user_info(access_token).await,
            "facebook" => self.get_facebook_user_info(access_token).await,
            _ => Err(OAuthError::ConfigError(format!(
                "Unsupported provider: {}",
                provider
            ))),
        }
    }

    async fn get_google_user_info(&self, access_token: &str) -> Result<OAuthUserInfo, OAuthError> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| OAuthError::NetworkError(e.to_string()))?;

        let user_data: GoogleUserInfo = response
            .json()
            .await
            .map_err(|e| OAuthError::SerializationError(e.to_string()))?;

        Ok(OAuthUserInfo {
            id: user_data.id,
            email: user_data.email,
            name: user_data.name,
            username: None,
            avatar_url: user_data.picture,
            provider: "google".to_string(),
            verified: user_data.verified_email.unwrap_or(false),
        })
    }

    async fn get_github_user_info(&self, access_token: &str) -> Result<OAuthUserInfo, OAuthError> {
        let client = reqwest::Client::new();

        // Get user info
        let user_response = client
            .get("https://api.github.com/user")
            .bearer_auth(access_token)
            .header("User-Agent", "Soroban-Security-Scanner")
            .send()
            .await
            .map_err(|e| OAuthError::NetworkError(e.to_string()))?;

        let user_data: GitHubUserInfo = user_response
            .json()
            .await
            .map_err(|e| OAuthError::SerializationError(e.to_string()))?;

        // Get user emails (to find primary verified email)
        let emails_response = client
            .get("https://api.github.com/user/emails")
            .bearer_auth(access_token)
            .header("User-Agent", "Soroban-Security-Scanner")
            .send()
            .await
            .map_err(|e| OAuthError::NetworkError(e.to_string()))?;

        let emails: Vec<GitHubEmail> = emails_response
            .json()
            .await
            .map_err(|e| OAuthError::SerializationError(e.to_string()))?;

        let has_verified = emails.iter().any(|e| e.verified && !e.email.is_empty());
        let primary_email = emails
            .iter()
            .find(|email| email.primary && email.verified)
            .or_else(|| emails.iter().find(|email| email.verified))
            .map(|email| email.email.clone())
            .unwrap_or_else(|| format!("{}@users.noreply.github.com", user_data.login));

        Ok(OAuthUserInfo {
            id: user_data.id.to_string(),
            email: primary_email,
            name: user_data.name,
            username: Some(user_data.login),
            avatar_url: user_data.avatar_url,
            provider: "github".to_string(),
            verified: has_verified,
        })
    }

    async fn get_microsoft_user_info(
        &self,
        access_token: &str,
    ) -> Result<OAuthUserInfo, OAuthError> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://graph.microsoft.com/v1.0/me")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| OAuthError::NetworkError(e.to_string()))?;

        let user_data: MicrosoftUserInfo = response
            .json()
            .await
            .map_err(|e| OAuthError::SerializationError(e.to_string()))?;

        // In Microsoft Graph, `mail` is the provisioned/verified mailbox address.
        // If `mail` is missing, user_principal_name is an unconfirmed or tenant-generated UPN.
        let is_verified = user_data.mail.is_some();
        let email = user_data.mail.unwrap_or(user_data.user_principal_name);

        Ok(OAuthUserInfo {
            id: user_data.id,
            email,
            name: user_data.display_name,
            username: None,
            avatar_url: None,
            provider: "microsoft".to_string(),
            verified: is_verified,
        })
    }

    async fn get_discord_user_info(&self, access_token: &str) -> Result<OAuthUserInfo, OAuthError> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://discord.com/api/users/@me")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| OAuthError::NetworkError(e.to_string()))?;

        let user_data: DiscordUserInfo = response
            .json()
            .await
            .map_err(|e| OAuthError::SerializationError(e.to_string()))?;

        let has_email = user_data.email.as_ref().map(|e| !e.is_empty()).unwrap_or(false);
        let is_verified = user_data.verified.unwrap_or(false) && has_email;

        Ok(OAuthUserInfo {
            id: user_data.id.clone(),
            email: user_data.email.unwrap_or_default(),
            name: Some(format!(
                "{}#{}",
                user_data.username, user_data.discriminator
            )),
            username: Some(user_data.username),
            avatar_url: Some(format!(
                "https://cdn.discordapp.com/avatars/{}/{}.png",
                user_data.id,
                user_data.avatar.unwrap_or_default()
            )),
            provider: "discord".to_string(),
            verified: is_verified,
        })
    }

    async fn get_facebook_user_info(
        &self,
        access_token: &str,
    ) -> Result<OAuthUserInfo, OAuthError> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://graph.facebook.com/me")
            .query(&[
                ("fields", "id,name,email,picture,verified"),
                ("access_token", access_token),
            ])
            .send()
            .await
            .map_err(|e| OAuthError::NetworkError(e.to_string()))?;

        let user_data: FacebookUserInfo = response
            .json()
            .await
            .map_err(|e| OAuthError::SerializationError(e.to_string()))?;

        let has_email = user_data.email.as_ref().map(|e| !e.is_empty()).unwrap_or(false);
        let is_verified = user_data.verified.unwrap_or(false) && has_email;

        Ok(OAuthUserInfo {
            id: user_data.id,
            email: user_data.email.unwrap_or_default(),
            name: user_data.name,
            username: None,
            avatar_url: user_data.picture.map(|p| p.data.url),
            provider: "facebook".to_string(),
            verified: is_verified,
        })
    }

    pub fn get_configured_providers(&self) -> Vec<String> {
        self.clients.keys().cloned().collect()
    }

    pub fn is_provider_configured(&self, provider: &str) -> bool {
        self.clients.contains_key(provider)
    }
}

// Structs for parsing OAuth provider responses
#[derive(Deserialize)]
struct GoogleUserInfo {
    id: String,
    email: String,
    name: Option<String>,
    picture: Option<String>,
    verified_email: Option<bool>,
}

#[derive(Deserialize)]
struct GitHubUserInfo {
    id: u64,
    login: String,
    name: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct GitHubEmail {
    email: String,
    primary: bool,
    verified: bool,
}

#[derive(Deserialize)]
struct MicrosoftUserInfo {
    id: String,
    #[serde(alias = "displayName")]
    display_name: Option<String>,
    mail: Option<String>,
    #[serde(alias = "userPrincipalName")]
    user_principal_name: String,
}

#[derive(Deserialize)]
struct DiscordUserInfo {
    id: String,
    username: String,
    discriminator: String,
    email: Option<String>,
    avatar: Option<String>,
    #[serde(default)]
    verified: Option<bool>,
}

#[derive(Deserialize)]
struct FacebookUserInfo {
    id: String,
    name: Option<String>,
    email: Option<String>,
    picture: Option<FacebookPicture>,
    #[serde(default)]
    verified: Option<bool>,
}

#[derive(Deserialize)]
struct FacebookPicture {
    data: FacebookPictureData,
}

#[derive(Deserialize)]
struct FacebookPictureData {
    url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_provider_defaults() {
        let google_config = OAuthProvider::Google.default_config();
        assert_eq!(
            google_config.auth_url,
            "https://accounts.google.com/o/oauth2/v2/auth"
        );
        assert_eq!(google_config.scopes.len(), 2);

        let github_config = OAuthProvider::GitHub.default_config();
        assert_eq!(
            github_config.auth_url,
            "https://github.com/login/oauth/authorize"
        );
        assert_eq!(github_config.scopes, vec!["user:email"]);
    }

    #[test]
    fn test_oauth_service_creation() {
        let service = OAuthService::new();
        assert_eq!(service.get_configured_providers().len(), 0);
    }

    #[test]
    fn test_provider_as_str() {
        assert_eq!(OAuthProvider::Google.as_str(), "google");
        assert_eq!(OAuthProvider::GitHub.as_str(), "github");
        assert_eq!(OAuthProvider::Microsoft.as_str(), "microsoft");
        assert_eq!(OAuthProvider::Discord.as_str(), "discord");
        assert_eq!(OAuthProvider::Facebook.as_str(), "facebook");
    }

    #[test]
    fn test_state_store_and_validate() {
        let service = OAuthService::new();

        // Store a state token
        service
            .store_state("google", "valid-state-token-123")
            .unwrap();

        // Validate it should succeed and return the provider
        let provider = service.validate_state("valid-state-token-123").unwrap();
        assert_eq!(provider, "google");

        // Second validation of the same token should fail (single-use)
        let result = service.validate_state("valid-state-token-123");
        assert!(matches!(result, Err(OAuthError::InvalidState)));
    }

    #[test]
    fn test_state_store_multiple_providers() {
        let service = OAuthService::new();

        service.store_state("google", "google-state-1").unwrap();
        service.store_state("github", "github-state-1").unwrap();
        service.store_state("google", "google-state-2").unwrap();

        assert_eq!(service.validate_state("google-state-1").unwrap(), "google");
        assert_eq!(service.validate_state("github-state-1").unwrap(), "github");
        assert_eq!(service.validate_state("google-state-2").unwrap(), "google");
    }

    #[test]
    fn test_invalid_state_rejected() {
        let service = OAuthService::new();

        // Never stored → should fail
        let result = service.validate_state("nonexistent-state");
        assert!(matches!(result, Err(OAuthError::InvalidState)));
    }

    #[test]
    fn test_state_replay_attack_is_blocked() {
        let service = OAuthService::new();

        // Simulate normal OAuth flow: store state, consume once
        service.store_state("google", "single-use-state").unwrap();
        let provider = service.validate_state("single-use-state").unwrap();
        assert_eq!(provider, "google");

        // Replay attack: attacker tries to use the same state again
        let replay = service.validate_state("single-use-state");
        assert!(
            matches!(replay, Err(OAuthError::InvalidState)),
            "Replay attack should be blocked: state token consumed after first use"
        );
    }

    #[test]
    fn test_state_expiry() {
        let service = OAuthService::new();

        // Store a state with a very short TTL
        // We can't easily manipulate time, but we can test that the expiry check works
        // by manually inserting an expired state into the store
        {
            let mut store = service.state_store.lock().unwrap();
            let past = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .saturating_sub(3600); // 1 hour ago

            store.insert(
                "expired-state".to_string(),
                StateEntry {
                    provider: "google".to_string(),
                    expires_at: past + 1, // expired 3599 seconds ago
                    pkce_verifier: None,
                },
            );
        }

        // Expired state should be rejected
        let result = service.validate_state("expired-state");
        assert!(
            matches!(result, Err(OAuthError::InvalidState)),
            "Expired state token should be rejected"
        );
    }

    #[test]
    fn test_cleanup_expired_states() {
        let service = OAuthService::new();

        // Insert states with different expiry times
        {
            let mut store = service.state_store.lock().unwrap();
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            // Expired state
            store.insert(
                "old-1".to_string(),
                StateEntry {
                    provider: "google".to_string(),
                    expires_at: now - 1800,
                    pkce_verifier: None,
                },
            );
            store.insert(
                "old-2".to_string(),
                StateEntry {
                    provider: "github".to_string(),
                    expires_at: now - 3600,
                    pkce_verifier: None,
                },
            );

            // Active state (not expired)
            store.insert(
                "active".to_string(),
                StateEntry {
                    provider: "google".to_string(),
                    expires_at: now + 600,
                    pkce_verifier: None,
                },
            );
        }

        let cleaned = service.cleanup_expired_states().unwrap();
        assert_eq!(cleaned, 2, "Should have cleaned 2 expired states");

        // Only the active state should remain
        let store = service.state_store.lock().unwrap();
        assert_eq!(store.len(), 1);
        assert!(store.contains_key("active"));
    }

    #[test]
    fn test_state_validate_expired_token_rejected() {
        // Verify that expired state tokens are rejected before any provider contact
        let service = OAuthService::new();

        // Manually insert an expired state into the store
        {
            let mut store = service.state_store.lock().unwrap();
            let past = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .saturating_sub(3600);

            store.insert(
                "expired-state-rejected".to_string(),
                StateEntry {
                    provider: "google".to_string(),
                    expires_at: past + 1,
                    pkce_verifier: None,
                },
            );
        }

        let result = service.validate_state("expired-state-rejected");
        assert!(matches!(result, Err(OAuthError::InvalidState)));
    }

    #[test]
    fn test_get_authorization_url_stores_state() {
        let mut service = OAuthService::new();

        let config = OAuthConfig {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            redirect_url: "http://localhost:3000/callback".to_string(),
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_url: "https://oauth2.googleapis.com/token".to_string(),
            scopes: vec!["email".to_string()],
            use_pkce: false,
        };

        service.add_provider(OAuthProvider::Google, config).unwrap();

        let (url, csrf_token) = service.get_authorization_url("google").unwrap();
        assert!(url.starts_with("https://accounts.google.com"));

        // The CSRF token should now be stored and validatable
        let provider = service.validate_state(csrf_token.secret()).unwrap();
        assert_eq!(provider, "google");
    }

    #[tokio::test]
    async fn test_pkce_authorization_url_generation() {
        let mut service = OAuthService::new();
        let config = OAuthProvider::Google
            .default_config()
            .with_pkce();

        service.add_provider(OAuthProvider::Google, config).unwrap();

        // Calling get_authorization_url on a PKCE-enabled config generates challenge
        let (url, csrf) = service.get_authorization_url("google").unwrap();
        assert!(url.contains("code_challenge="));
        assert!(url.contains("code_challenge_method=S256"));

        // Validate state returns Google and cleans up state
        let provider = service.validate_state(csrf.secret()).unwrap();
        assert_eq!(provider, "google");

        // Also test get_authorization_url_with_pkce
        let (pkce_url, pkce_csrf, verifier) = service.get_authorization_url_with_pkce("google").unwrap();
        assert!(pkce_url.contains("code_challenge="));
        assert!(!verifier.secret().is_empty());
        assert_eq!(service.validate_state(pkce_csrf.secret()).unwrap(), "google");
    }

    #[tokio::test]
    async fn test_signed_oauth_state_store_flow() {
        let store = SignedOAuthStateStore::new("super-secure-secret-key-12345");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let entry = StateEntry {
            provider: "github".to_string(),
            expires_at: now + 300,
            pkce_verifier: Some("test-verifier-xyz".to_string()),
        };

        let signed_token = store.generate_signed_state(&entry).unwrap();
        assert!(signed_token.contains('.'));

        // Validate token
        let validated = store.validate_and_remove_state(&signed_token).await.unwrap();
        assert_eq!(validated.provider, "github");
        assert_eq!(validated.pkce_verifier, Some("test-verifier-xyz".to_string()));

        // Tamper with payload
        let parts: Vec<&str> = signed_token.split('.').collect();
        let tampered_token = format!("{}tampered.{}", parts[0], parts[1]);
        assert!(matches!(
            store.validate_and_remove_state(&tampered_token).await,
            Err(OAuthError::InvalidState)
        ));

        // Tamper with signature
        let tampered_sig = format!("{}.{}", parts[0], "00".repeat(32));
        assert!(matches!(
            store.validate_and_remove_state(&tampered_sig).await,
            Err(OAuthError::InvalidState)
        ));

        // Expired token
        let expired_entry = StateEntry {
            provider: "github".to_string(),
            expires_at: now - 10,
            pkce_verifier: None,
        };
        let expired_token = store.generate_signed_state(&expired_entry).unwrap();
        assert!(matches!(
            store.validate_and_remove_state(&expired_token).await,
            Err(OAuthError::InvalidState)
        ));
    }

    #[tokio::test]
    async fn test_in_memory_state_store_expiration_and_removal() {
        let store = InMemoryOAuthStateStore::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        store
            .store_state(
                "token1",
                StateEntry {
                    provider: "discord".to_string(),
                    expires_at: now + 300,
                    pkce_verifier: None,
                },
            )
            .await
            .unwrap();

        store
            .store_state(
                "token2",
                StateEntry {
                    provider: "microsoft".to_string(),
                    expires_at: now - 50,
                    pkce_verifier: None,
                },
            )
            .await
            .unwrap();

        // token1 valid
        let res1 = store.validate_and_remove_state("token1").await.unwrap();
        assert_eq!(res1.provider, "discord");

        // replay token1 fails
        assert!(matches!(
            store.validate_and_remove_state("token1").await,
            Err(OAuthError::InvalidState)
        ));

        // token2 expired
        assert!(matches!(
            store.validate_and_remove_state("token2").await,
            Err(OAuthError::InvalidState)
        ));
    }

    #[test]
    fn test_provider_verification_parsing() {
        // Facebook user info deserialization
        let fb_verified_json = r#"{"id":"123","name":"Alice","email":"alice@fb.com","verified":true}"#;
        let fb_verified: FacebookUserInfo = serde_json::from_str(fb_verified_json).unwrap();
        assert_eq!(fb_verified.verified, Some(true));

        let fb_unverified_json = r#"{"id":"124","name":"Bob","email":"bob@fb.com","verified":false}"#;
        let fb_unverified: FacebookUserInfo = serde_json::from_str(fb_unverified_json).unwrap();
        assert_eq!(fb_unverified.verified, Some(false));

        // Discord user info deserialization
        let discord_verified_json = r#"{"id":"456","username":"alice","discriminator":"0001","email":"alice@discord.com","verified":true}"#;
        let discord_verified: DiscordUserInfo = serde_json::from_str(discord_verified_json).unwrap();
        assert_eq!(discord_verified.verified, Some(true));

        let discord_unverified_json = r#"{"id":"457","username":"bob","discriminator":"0002","email":"bob@discord.com","verified":false}"#;
        let discord_unverified: DiscordUserInfo = serde_json::from_str(discord_unverified_json).unwrap();
        assert_eq!(discord_unverified.verified, Some(false));

        // Microsoft user info deserialization
        let ms_with_mail_json = r#"{"id":"789","displayName":"Charlie","mail":"charlie@ms.com","userPrincipalName":"charlie@ms.com"}"#;
        let ms_with_mail: MicrosoftUserInfo = serde_json::from_str(ms_with_mail_json).unwrap();
        assert!(ms_with_mail.mail.is_some());

        let ms_no_mail_json = r#"{"id":"790","displayName":"Dave","mail":null,"userPrincipalName":"dave@ms.com"}"#;
        let ms_no_mail: MicrosoftUserInfo = serde_json::from_str(ms_no_mail_json).unwrap();
        assert!(ms_no_mail.mail.is_none());
    }
}
