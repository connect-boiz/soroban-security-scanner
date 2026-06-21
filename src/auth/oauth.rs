use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

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
            },
            OAuthProvider::GitHub => OAuthConfig {
                client_id: "".to_string(),
                client_secret: "".to_string(),
                redirect_url: "http://localhost:3000/auth/github/callback".to_string(),
                auth_url: "https://github.com/login/oauth/authorize".to_string(),
                token_url: "https://github.com/login/oauth/access_token".to_string(),
                scopes: vec!["user:email".to_string()],
            },
            OAuthProvider::Microsoft => OAuthConfig {
                client_id: "".to_string(),
                client_secret: "".to_string(),
                redirect_url: "http://localhost:3000/auth/microsoft/callback".to_string(),
                auth_url: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize".to_string(),
                token_url: "https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string(),
                scopes: vec![
                    "openid".to_string(),
                    "email".to_string(),
                    "profile".to_string(),
                ],
            },
            OAuthProvider::Discord => OAuthConfig {
                client_id: "".to_string(),
                client_secret: "".to_string(),
                redirect_url: "http://localhost:3000/auth/discord/callback".to_string(),
                auth_url: "https://discord.com/api/oauth2/authorize".to_string(),
                token_url: "https://discord.com/api/oauth2/token".to_string(),
                scopes: vec!["identify".to_string(), "email".to_string()],
            },
            OAuthProvider::Facebook => OAuthConfig {
                client_id: "".to_string(),
                client_secret: "".to_string(),
                redirect_url: "http://localhost:3000/auth/facebook/callback".to_string(),
                auth_url: "https://www.facebook.com/v18.0/dialog/oauth".to_string(),
                token_url: "https://graph.facebook.com/v18.0/oauth/access_token".to_string(),
                scopes: vec!["email".to_string(), "public_profile".to_string()],
            },
        }
    }
}

/// A stored OAuth state token entry with expiry timing
#[derive(Debug, Clone)]
struct StateEntry {
    provider: String,
    created_at: u64,
    expires_at: u64,
}

/// OAuth service for managing social login integrations
pub struct OAuthService {
    clients: HashMap<String, BasicClient>,
    configs: HashMap<String, OAuthConfig>,
    state_store: Mutex<HashMap<String, StateEntry>>,
}

impl OAuthService {
    /// Default state TTL: 10 minutes
    pub const STATE_TTL_SECONDS: u64 = 600;

    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            configs: HashMap::new(),
            state_store: Mutex::new(HashMap::new()),
        }
    }

    pub fn add_provider(&mut self, provider: OAuthProvider, config: OAuthConfig) -> Result<(), OAuthError> {
        let client_id = ClientId::new(config.client_id.clone());
        let client_secret = ClientSecret::new(config.client_secret.clone());

        let auth_url = AuthUrl::new(config.auth_url.clone())
            .map_err(|e| OAuthError::ConfigError(format!("Invalid auth URL: {}", e)))?;

        let token_url = TokenUrl::new(config.token_url.clone())
            .map_err(|e| OAuthError::ConfigError(format!("Invalid token URL: {}", e)))?;

        let redirect_url = RedirectUrl::new(config.redirect_url.clone())
            .map_err(|e| OAuthError::ConfigError(format!("Invalid redirect URL: {}", e)))?;

        let client = BasicClient::new(
            client_id,
            Some(client_secret),
            auth_url,
            Some(token_url),
        )
        .set_redirect_uri(redirect_url);

        let provider_name = provider.as_str().to_string();
        self.clients.insert(provider_name.clone(), client);
        self.configs.insert(provider_name, config);

        Ok(())
    }

    pub fn get_authorization_url(&self, provider: &str) -> Result<(String, CsrfToken), OAuthError> {
        let client = self.clients.get(provider)
            .ok_or_else(|| OAuthError::ConfigError(format!("Provider {} not configured", provider)))?;

        let config = self.configs.get(provider)
            .ok_or_else(|| OAuthError::ConfigError(format!("Provider {} config not found", provider)))?;

        let mut auth_request = client.authorize_url(CsrfToken::new_random);

        for scope in &config.scopes {
            auth_request = auth_request.add_scope(Scope::new(scope.clone()));
        }

        let (auth_url, csrf_token) = auth_request.url();

        // Store the CSRF token in the state store for later validation
        self.store_state(provider, csrf_token.secret())?;

        Ok((auth_url.to_string(), csrf_token))
    }

    /// Store a state token so it can be validated on callback
    fn store_state(&self, provider: &str, state: &str) -> Result<(), OAuthError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let entry = StateEntry {
            provider: provider.to_string(),
            created_at: now,
            expires_at: now + Self::STATE_TTL_SECONDS,
        };

        let mut store = self.state_store.lock().map_err(|e| {
            OAuthError::ConfigError(format!("Failed to acquire state store lock: {}", e))
        })?;

        store.insert(state.to_string(), entry);
        Ok(())
    }

    /// Validate that a state token exists and has not expired
    fn validate_state(&self, state: &str) -> Result<String, OAuthError> {
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

        Ok(entry.provider)
    }

    /// Remove a state token from the store (used for cleanup)
    fn remove_state(&self, state: &str) {
        if let Ok(mut store) = self.state_store.lock() {
            store.remove(state);
        }
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

    pub async fn exchange_code_for_token(
        &self,
        provider: &str,
        code: &str,
        state: &str,
    ) -> Result<OAuthUserInfo, OAuthError> {
        // Validate the state parameter to prevent CSRF attacks
        let stored_provider = self.validate_state(state)?;

        // Ensure the state belongs to the expected provider
        if stored_provider != provider {
            return Err(OAuthError::InvalidState);
        }

        let client = self.clients.get(provider)
            .ok_or_else(|| OAuthError::ConfigError(format!("Provider {} not configured", provider)))?;

        let token_response = client
            .exchange_code(oauth2::AuthorizationCode::new(code.to_string()))
            .request_async(oauth2::reqwest::async_http_client)
            .await
            .map_err(|e| OAuthError::TokenError(format!("Failed to exchange code: {}", e)))?;

        let access_token = token_response.access_token().secret();
        let user_info = self.get_user_info(provider, access_token).await?;

        Ok(user_info)
    }

    async fn get_user_info(&self, provider: &str, access_token: &str) -> Result<OAuthUserInfo, OAuthError> {
        match provider {
            "google" => self.get_google_user_info(access_token).await,
            "github" => self.get_github_user_info(access_token).await,
            "microsoft" => self.get_microsoft_user_info(access_token).await,
            "discord" => self.get_discord_user_info(access_token).await,
            "facebook" => self.get_facebook_user_info(access_token).await,
            _ => Err(OAuthError::ConfigError(format!("Unsupported provider: {}", provider))),
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
            verified: true, // GitHub OAuth requires verified email
        })
    }

    async fn get_microsoft_user_info(&self, access_token: &str) -> Result<OAuthUserInfo, OAuthError> {
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

        Ok(OAuthUserInfo {
            id: user_data.id,
            email: user_data.mail.or(user_data.user_principal_name),
            name: user_data.display_name,
            username: None,
            avatar_url: None,
            provider: "microsoft".to_string(),
            verified: true,
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

        Ok(OAuthUserInfo {
            id: user_data.id,
            email: user_data.email,
            name: Some(format!("{}#{}", user_data.username, user_data.discriminator)),
            username: Some(user_data.username),
            avatar_url: Some(format!(
                "https://cdn.discordapp.com/avatars/{}/{}.png",
                user_data.id,
                user_data.avatar.unwrap_or_default()
            )),
            provider: "discord".to_string(),
            verified: user_data.verified,
        })
    }

    async fn get_facebook_user_info(&self, access_token: &str) -> Result<OAuthUserInfo, OAuthError> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://graph.facebook.com/me")
            .query(&[
                ("fields", "id,name,email,picture"),
                ("access_token", access_token),
            ])
            .send()
            .await
            .map_err(|e| OAuthError::NetworkError(e.to_string()))?;

        let user_data: FacebookUserInfo = response
            .json()
            .await
            .map_err(|e| OAuthError::SerializationError(e.to_string()))?;

        Ok(OAuthUserInfo {
            id: user_data.id,
            email: user_data.email,
            name: user_data.name,
            username: None,
            avatar_url: user_data.picture.map(|p| p.data.url),
            provider: "facebook".to_string(),
            verified: true,
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
    display_name: Option<String>,
    mail: Option<String>,
    user_principal_name: String,
}

#[derive(Deserialize)]
struct DiscordUserInfo {
    id: String,
    username: String,
    discriminator: String,
    email: Option<String>,
    avatar: Option<String>,
    verified: bool,
}

#[derive(Deserialize)]
struct FacebookUserInfo {
    id: String,
    name: Option<String>,
    email: Option<String>,
    picture: Option<FacebookPicture>,
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
        assert_eq!(google_config.auth_url, "https://accounts.google.com/o/oauth2/v2/auth");
        assert_eq!(google_config.scopes.len(), 2);

        let github_config = OAuthProvider::GitHub.default_config();
        assert_eq!(github_config.auth_url, "https://github.com/login/oauth/authorize");
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
        service.store_state("google", "valid-state-token-123").unwrap();

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

            store.insert("expired-state".to_string(), StateEntry {
                provider: "google".to_string(),
                created_at: past,
                expires_at: past + 1, // expired 3599 seconds ago
            });
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
            store.insert("old-1".to_string(), StateEntry {
                provider: "google".to_string(),
                created_at: now - 3600,
                expires_at: now - 1800,
            });
            store.insert("old-2".to_string(), StateEntry {
                provider: "github".to_string(),
                created_at: now - 7200,
                expires_at: now - 3600,
            });

            // Active state (not expired)
            store.insert("active".to_string(), StateEntry {
                provider: "google".to_string(),
                created_at: now,
                expires_at: now + 600,
            });
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

            store.insert("expired-state-rejected".to_string(), StateEntry {
                provider: "google".to_string(),
                created_at: past,
                expires_at: past + 1,
            });
        }

        let result = service.validate_state("expired-state-rejected");
        assert!(matches!(result, Err(OAuthError::InvalidState)));
    }

    #[test]
    fn test_get_authorization_url_stores_state() {
        let mut service = OAuthService::new();

        let mut config = OAuthConfig {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            redirect_url: "http://localhost:3000/callback".to_string(),
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_url: "https://oauth2.googleapis.com/token".to_string(),
            scopes: vec!["email".to_string()],
        };

        service.add_provider(OAuthProvider::Google, config).unwrap();

        let (url, csrf_token) = service.get_authorization_url("google").unwrap();
        assert!(url.starts_with("https://accounts.google.com"));

        // The CSRF token should now be stored and validatable
        let provider = service.validate_state(csrf_token.secret()).unwrap();
        assert_eq!(provider, "google");
    }
}
