use axum::{
    extract::{ConnectInfo, Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use thiserror::Error;

use crate::auth::{
    AccountLockoutService, JwtService, RateLimitService, SessionManager, SessionStore,
};

#[derive(Debug, Error)]
pub enum AuthMiddlewareError {
    #[error("Missing authentication token")]
    MissingToken,
    #[error("Invalid authentication token: {0}")]
    InvalidToken(String),
    #[error("Token expired")]
    TokenExpired,
    #[error("Session not found or expired")]
    SessionNotFound,
    #[error("Account locked: {0}")]
    AccountLocked(String),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Insufficient permissions")]
    InsufficientPermissions,
    #[error("Authentication service error: {0}")]
    ServiceError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    pub user_id: String,
    pub email: String,
    pub role: String,
    pub permissions: Vec<String>,
    pub session_id: String,
    pub issued_at: chrono::DateTime<Utc>,
    pub expires_at: chrono::DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthMiddlewareConfig {
    pub require_auth: bool,
    pub allowed_paths: Vec<String>,
    pub required_permissions: Vec<String>,
    pub required_roles: Vec<String>,
    pub rate_limit_config: Option<String>,
    pub session_validation: bool,
    pub ip_whitelist: Vec<String>,
    pub cors_origins: Vec<String>,
    pub trusted_proxies: Vec<String>,
    pub trust_all_proxies: bool,
    pub client_ip_header: Option<String>,
}

impl Default for AuthMiddlewareConfig {
    fn default() -> Self {
        Self {
            require_auth: true,
            allowed_paths: vec![
                "/health".to_string(),
                "/metrics".to_string(),
                "/auth/login".to_string(),
                "/auth/register".to_string(),
                "/auth/forgot-password".to_string(),
                "/auth/reset-password".to_string(),
                "/auth/oauth".to_string(),
                "/docs".to_string(),
                "/swagger-ui".to_string(),
                "/api/v1/public*".to_string(),
            ],
            required_permissions: Vec::new(),
            required_roles: Vec::new(),
            rate_limit_config: None,
            session_validation: true,
            ip_whitelist: Vec::new(),
            cors_origins: Vec::new(),
            trusted_proxies: Vec::new(),
            trust_all_proxies: false,
            client_ip_header: None,
        }
    }
}

#[derive(Clone)]
pub struct AuthServices<S: SessionStore> {
    pub jwt_service: Arc<JwtService>,
    pub session_manager: Arc<SessionManager<S>>,
    pub rate_limit_service: Arc<RateLimitService<crate::auth::InMemoryRateLimitStore>>,
    pub account_lockout_service: Arc<AccountLockoutService<crate::auth::InMemoryLockoutStore>>,
}

impl<S> AuthServices<S>
where
    S: crate::auth::SessionStore + Send + Sync + 'static,
{
    pub fn new(
        jwt_service: JwtService,
        session_manager: SessionManager<S>,
        rate_limit_service: RateLimitService<crate::auth::InMemoryRateLimitStore>,
        account_lockout_service: AccountLockoutService<crate::auth::InMemoryLockoutStore>,
    ) -> Self {
        Self {
            jwt_service: Arc::new(jwt_service),
            session_manager: Arc::new(session_manager),
            rate_limit_service: Arc::new(rate_limit_service),
            account_lockout_service: Arc::new(account_lockout_service),
        }
    }
}

pub struct AuthMiddleware<S: SessionStore> {
    services: AuthServices<S>,
    config: AuthMiddlewareConfig,
}

impl<S> AuthMiddleware<S>
where
    S: SessionStore + Send + Sync + 'static,
{
    pub fn new(services: AuthServices<S>, config: AuthMiddlewareConfig) -> Self {
        Self { services, config }
    }

    pub fn from_services(services: AuthServices<S>) -> Self {
        Self::new(services, AuthMiddlewareConfig::default())
    }

    pub async fn authenticate_request(
        &self,
        request: Request,
        next: Next,
    ) -> Result<Response, StatusCode> {
        let path = request.uri().path();
        let ip_address = self.extract_ip_address(&request);
        let user_agent = self.extract_user_agent(&request);

        // Check if path is allowed without authentication
        if self.is_path_allowed(path) {
            return Ok(next.run(request).await);
        }

        // Apply rate limiting if configured
        if let Some(rate_limit_config) = &self.config.rate_limit_config {
            let rate_limit_key = format!("{}:{}", ip_address.as_deref().unwrap_or("unknown"), path);

            match self
                .services
                .rate_limit_service
                .check_rate_limit(&rate_limit_key, rate_limit_config)
                .await
            {
                Ok(result) => {
                    if !result.allowed {
                        return Err(StatusCode::TOO_MANY_REQUESTS);
                    }
                }
                Err(_) => {
                    // Log error but continue processing
                }
            }
        }

        // Extract and validate JWT token
        let auth_context = match self
            .extract_and_validate_token(request.headers(), &ip_address, &user_agent)
            .await
        {
            Ok(context) => context,
            Err(e) => {
                return match e {
                    AuthMiddlewareError::MissingToken => Err(StatusCode::UNAUTHORIZED),
                    AuthMiddlewareError::InvalidToken(_) => Err(StatusCode::UNAUTHORIZED),
                    AuthMiddlewareError::TokenExpired => Err(StatusCode::UNAUTHORIZED),
                    AuthMiddlewareError::SessionNotFound => Err(StatusCode::UNAUTHORIZED),
                    AuthMiddlewareError::AccountLocked(_) => Err(StatusCode::FORBIDDEN),
                    AuthMiddlewareError::RateLimitExceeded => Err(StatusCode::TOO_MANY_REQUESTS),
                    AuthMiddlewareError::InsufficientPermissions => Err(StatusCode::FORBIDDEN),
                    AuthMiddlewareError::ServiceError(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                };
            }
        };

        // Check IP whitelist if configured
        if !self.config.ip_whitelist.is_empty() {
            if let Some(ip) = &ip_address {
                if !is_ip_in_whitelist(ip, &self.config.ip_whitelist) {
                    return Err(StatusCode::FORBIDDEN);
                }
            } else {
                return Err(StatusCode::FORBIDDEN);
            }
        }

        // Check required roles
        if !self.config.required_roles.is_empty()
            && !self.config.required_roles.contains(&auth_context.role)
        {
            return Err(StatusCode::FORBIDDEN);
        }

        // Check required permissions
        if !self.config.required_permissions.is_empty() {
            let has_all_permissions = self
                .config
                .required_permissions
                .iter()
                .all(|required| auth_context.permissions.contains(required));

            if !has_all_permissions {
                return Err(StatusCode::FORBIDDEN);
            }
        }

        // Add auth context to request extensions
        let mut request = request;
        request.extensions_mut().insert(auth_context);

        Ok(next.run(request).await)
    }

    async fn extract_and_validate_token(
        &self,
        headers: &HeaderMap,
        ip_address: &Option<String>,
        user_agent: &Option<String>,
    ) -> Result<AuthContext, AuthMiddlewareError> {
        // Extract token from Authorization header
        let auth_header = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or(AuthMiddlewareError::MissingToken)?;

        let token = self
            .services
            .jwt_service
            .extract_token_from_header(auth_header)
            .ok_or(AuthMiddlewareError::InvalidToken(
                "Invalid Bearer token format".to_string(),
            ))?;

        // Validate JWT token
        let claims = self
            .services
            .jwt_service
            .validate_token(&token)
            .map_err(|e| match e {
                crate::auth::JwtError::Expired => AuthMiddlewareError::TokenExpired,
                crate::auth::JwtError::InvalidToken(_) => {
                    AuthMiddlewareError::InvalidToken(e.to_string())
                }
                _ => AuthMiddlewareError::ServiceError(e.to_string()),
            })?;

        // Check account lockout status
        let lockout_status = self
            .services
            .account_lockout_service
            .check_account_status(&claims.sub)
            .await
            .map_err(|e| AuthMiddlewareError::ServiceError(e.to_string()))?;

        if lockout_status.is_locked || lockout_status.is_permanently_locked {
            let reason = if lockout_status.is_permanently_locked {
                "Account is permanently locked".to_string()
            } else {
                "Account is temporarily locked".to_string()
            };
            return Err(AuthMiddlewareError::AccountLocked(reason));
        }

        // Validate session if required
        if self.config.session_validation {
            let _session = self
                .services
                .session_manager
                .validate_session(&claims.session_id)
                .await
                .map_err(|e| match e {
                    crate::auth::SessionError::NotFound(_) => AuthMiddlewareError::SessionNotFound,
                    crate::auth::SessionError::Expired(_) => AuthMiddlewareError::SessionNotFound,
                    crate::auth::SessionError::Revoked(_) => AuthMiddlewareError::SessionNotFound,
                    _ => AuthMiddlewareError::ServiceError(e.to_string()),
                })?;

            // Update session with current request info
            if let Err(e) = self
                .services
                .session_manager
                .update_session_metadata(
                    &claims.session_id,
                    "last_ip",
                    &ip_address.clone().unwrap_or_else(|| "unknown".to_string()),
                )
                .await
            {
                // Log error but continue processing
                eprintln!("Failed to update session metadata: {}", e);
            }
        }

        Ok(AuthContext {
            user_id: claims.sub,
            email: claims.email,
            role: claims.role,
            permissions: claims.permissions,
            session_id: claims.session_id,
            issued_at: chrono::DateTime::from_timestamp(claims.iat, 0).unwrap_or_else(Utc::now),
            expires_at: chrono::DateTime::from_timestamp(claims.exp, 0).unwrap_or_else(Utc::now),
            ip_address: ip_address.clone(),
            user_agent: user_agent.clone(),
        })
    }

    fn is_path_allowed(&self, path: &str) -> bool {
        // Check exact matches
        if self.config.allowed_paths.contains(&path.to_string()) {
            return true;
        }

        // Check prefix matches
        for allowed_path in &self.config.allowed_paths {
            if allowed_path.ends_with('*') {
                let prefix = &allowed_path[..allowed_path.len() - 1];
                if path.starts_with(prefix) {
                    return true;
                }
            }
        }

        false
    }

    pub fn extract_ip_address(&self, request: &Request) -> Option<String> {
        let peer_ip = request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ci| ci.0.ip())
            .or_else(|| request.extensions().get::<SocketAddr>().map(|sa| sa.ip()));

        // Case 1: Peer socket is known
        if let Some(peer) = peer_ip {
            let is_peer_trusted = self.config.trust_all_proxies
                || is_ip_trusted(&peer, &self.config.trusted_proxies);

            if !is_peer_trusted {
                // The connection came directly from an untrusted peer.
                // Client-supplied forwarding headers MUST NOT be trusted.
                return Some(peer.to_string());
            }

            // The immediate peer is a trusted proxy. Inspect forwarding headers.
            if let Some(forwarded_ip) = self.extract_from_forwarding_headers(request) {
                return Some(forwarded_ip);
            }

            // Fallback to the trusted proxy's IP if headers contained no valid untrusted hop
            return Some(peer.to_string());
        }

        // Case 2: Peer socket not in extensions (e.g. test environments or specific proxy layers)
        if self.config.trust_all_proxies || !self.config.trusted_proxies.is_empty() {
            if let Some(forwarded_ip) = self.extract_from_forwarding_headers(request) {
                return Some(forwarded_ip);
            }
        }

        None
    }

    fn extract_from_forwarding_headers(&self, request: &Request) -> Option<String> {
        // 1. Explicit configured client IP header (e.g. "cf-connecting-ip" or "x-real-ip")
        if let Some(header_name) = &self.config.client_ip_header {
            if let Some(value) = request.headers().get(header_name) {
                if let Ok(s) = value.to_str() {
                    let first = s.split(',').next().unwrap_or("").trim();
                    if let Ok(ip) = first.parse::<IpAddr>() {
                        return Some(ip.to_string());
                    }
                }
            }
        }

        // 2. Traverse X-Forwarded-For from right to left (most-recently added hop by trusted proxies)
        if let Some(value) = request.headers().get("x-forwarded-for") {
            if let Ok(s) = value.to_str() {
                let hops: Vec<&str> = s
                    .split(',')
                    .map(|h| h.trim())
                    .filter(|h| !h.is_empty())
                    .collect();

                for hop in hops.into_iter().rev() {
                    if let Ok(ip) = hop.parse::<IpAddr>() {
                        if !is_ip_trusted(&ip, &self.config.trusted_proxies) {
                            return Some(ip.to_string());
                        }
                    }
                }
            }
        }

        // 3. Fallback proxy headers if no explicit client_ip_header was set
        let fallback_headers = [
            "cf-connecting-ip",
            "x-real-ip",
            "x-client-ip",
            "forwarded-for",
        ];

        for header in &fallback_headers {
            if let Some(value) = request.headers().get(*header) {
                if let Ok(s) = value.to_str() {
                    let first = s.split(',').next().unwrap_or("").trim();
                    if let Ok(ip) = first.parse::<IpAddr>() {
                        if !is_ip_trusted(&ip, &self.config.trusted_proxies) {
                            return Some(ip.to_string());
                        }
                    }
                }
            }
        }

        None
    }

    fn extract_user_agent(&self, request: &Request) -> Option<String> {
        request
            .headers()
            .get("user-agent")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
    }
}

fn is_ip_trusted(ip: &IpAddr, trusted_proxies: &[String]) -> bool {
    for proxy_str in trusted_proxies {
        if let Ok(network) = proxy_str.parse::<ipnetwork::IpNetwork>() {
            if network.contains(*ip) {
                return true;
            }
        } else if let Ok(proxy_ip) = proxy_str.parse::<IpAddr>() {
            if &proxy_ip == ip {
                return true;
            }
        }
    }
    false
}

fn is_ip_in_whitelist(ip_str: &str, whitelist: &[String]) -> bool {
    if whitelist.contains(&ip_str.to_string()) {
        return true;
    }
    if let Ok(ip) = ip_str.parse::<IpAddr>() {
        for entry in whitelist {
            if let Ok(network) = entry.parse::<ipnetwork::IpNetwork>() {
                if network.contains(ip) {
                    return true;
                }
            } else if let Ok(target_ip) = entry.parse::<IpAddr>() {
                if target_ip == ip {
                    return true;
                }
            }
        }
    }
    false
}

// Axum middleware function
pub async fn auth_middleware<S>(
    State(services): State<AuthServices<S>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode>
where
    S: crate::auth::SessionStore + Send + Sync + 'static,
{
    let middleware = AuthMiddleware::from_services(services);
    middleware.authenticate_request(request, next).await
}

// Permission-based middleware
pub fn require_permissions(
    permissions: Vec<String>,
) -> impl Fn(AuthMiddlewareConfig) -> AuthMiddlewareConfig {
    move |mut config| {
        config.required_permissions = permissions.clone();
        config
    }
}

// Role-based middleware
pub fn require_roles(roles: Vec<String>) -> impl Fn(AuthMiddlewareConfig) -> AuthMiddlewareConfig {
    move |mut config| {
        config.required_roles = roles.clone();
        config
    }
}

// Utility functions for extracting auth context in handlers
pub fn get_auth_context(request: &Request) -> Result<&AuthContext, AuthMiddlewareError> {
    request
        .extensions()
        .get::<AuthContext>()
        .ok_or(AuthMiddlewareError::MissingToken)
}

pub fn get_user_id(request: &Request) -> Result<String, AuthMiddlewareError> {
    get_auth_context(request).map(|ctx| ctx.user_id.clone())
}

pub fn get_user_permissions(request: &Request) -> Result<Vec<String>, AuthMiddlewareError> {
    get_auth_context(request).map(|ctx| ctx.permissions.clone())
}

pub fn has_permission(request: &Request, permission: &str) -> bool {
    get_auth_context(request)
        .map(|ctx| ctx.permissions.contains(&permission.to_string()))
        .unwrap_or(false)
}

pub fn has_role(request: &Request, role: &str) -> bool {
    get_auth_context(request)
        .map(|ctx| ctx.role == role)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{
        AccountLockoutService, InMemoryLockoutStore, InMemoryRateLimitStore, InMemorySessionStore,
        JwtService, RateLimitService, SessionData, SessionManager,
    };
    use axum::{body::Body, http::Method, routing::get, Router};
    use chrono::Duration;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_auth_middleware_with_valid_token() {
        let jwt_service = JwtService::new(
            "test-secret",
            "test-issuer".to_string(),
            "test-audience".to_string(),
        );
        let session_store = InMemorySessionStore::new();
        let session_manager = SessionManager::new(session_store.clone(), Duration::hours(24));
        let rate_limit_store = InMemoryRateLimitStore::new();
        let mut rate_limit_service = RateLimitService::new(rate_limit_store);
        rate_limit_service
            .add_config("api", crate::auth::RateLimitConfig::api())
            .unwrap();
        let lockout_store = InMemoryLockoutStore::new();
        let mut account_lockout_service = AccountLockoutService::new(lockout_store);
        account_lockout_service
            .add_config("auth", crate::auth::LockoutConfig::moderate())
            .unwrap();

        let services = AuthServices::new(
            jwt_service.clone(),
            session_manager.clone(),
            rate_limit_service,
            account_lockout_service,
        );

        // Generate a valid token
        let token = jwt_service
            .generate_token(
                "user123",
                "test@example.com",
                "user",
                vec!["read".to_string()],
                1,
            )
            .unwrap();

        // Create a matching session so session validation passes
        let claims = jwt_service.get_token_claims(&token).unwrap();
        let now = Utc::now();
        let session_data = SessionData {
            session_id: claims.session_id.clone(),
            user_id: "user123".to_string(),
            user_email: "test@example.com".to_string(),
            user_role: "user".to_string(),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("Test-Agent".to_string()),
            created_at: now,
            last_accessed: now,
            expires_at: now + Duration::hours(1),
            is_active: true,
            metadata: std::collections::HashMap::new(),
        };
        session_store.create_session(session_data).await.unwrap();

        let app = Router::new()
            .route("/protected", get(|| async { "Protected content" }))
            .route("/health", get(|| async { "Healthy" }))
            .with_state(services.clone())
            .layer(axum::middleware::from_fn_with_state(
                services.clone(),
                auth_middleware,
            ));

        // Test protected route with valid token
        let request = axum::http::Request::builder()
            .method(Method::GET)
            .uri("/protected")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test public route without token
        let request = axum::http::Request::builder()
            .method(Method::GET)
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_middleware_with_invalid_token() {
        let jwt_service = JwtService::new(
            "test-secret",
            "test-issuer".to_string(),
            "test-audience".to_string(),
        );
        let session_store = InMemorySessionStore::new();
        let session_manager = SessionManager::new(session_store, Duration::hours(24));
        let rate_limit_store = InMemoryRateLimitStore::new();
        let rate_limit_service = RateLimitService::new(rate_limit_store);
        let lockout_store = InMemoryLockoutStore::new();
        let account_lockout_service = AccountLockoutService::new(lockout_store);

        let services = AuthServices::new(
            jwt_service,
            session_manager,
            rate_limit_service,
            account_lockout_service,
        );

        let app = Router::new()
            .route("/protected", get(|| async { "Protected content" }))
            .with_state(services.clone())
            .layer(axum::middleware::from_fn_with_state(
                services.clone(),
                auth_middleware,
            ));

        // Test with invalid token
        let request = axum::http::Request::builder()
            .method(Method::GET)
            .uri("/protected")
            .header("authorization", "Bearer invalid-token")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // Test without token
        let request = axum::http::Request::builder()
            .method(Method::GET)
            .uri("/protected")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_path_matching() {
        let config = AuthMiddlewareConfig::default();
        let middleware = AuthMiddleware::new(
            AuthServices {
                jwt_service: Arc::new(JwtService::new(
                    "test",
                    "test".to_string(),
                    "test".to_string(),
                )),
                session_manager: Arc::new(SessionManager::new(
                    InMemorySessionStore::new(),
                    Duration::hours(24),
                )),
                rate_limit_service: Arc::new(RateLimitService::new(InMemoryRateLimitStore::new())),
                account_lockout_service: Arc::new(AccountLockoutService::new(
                    InMemoryLockoutStore::new(),
                )),
            },
            config,
        );

        assert!(middleware.is_path_allowed("/health"));
        assert!(middleware.is_path_allowed("/auth/login"));
        assert!(middleware.is_path_allowed("/api/v1/public/users"));
        assert!(!middleware.is_path_allowed("/api/v1/private/users"));
    }

    #[test]
    fn test_permission_utilities() {
        let auth_context = AuthContext {
            user_id: "user123".to_string(),
            email: "test@example.com".to_string(),
            role: "admin".to_string(),
            permissions: vec![
                "read".to_string(),
                "write".to_string(),
                "delete".to_string(),
            ],
            session_id: "session123".to_string(),
            issued_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(1),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("Test-Agent".to_string()),
        };

        let mut request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        request.extensions_mut().insert(auth_context);

        assert!(has_permission(&request, "read"));
        assert!(has_permission(&request, "write"));
        assert!(!has_permission(&request, "admin"));
        assert!(has_role(&request, "admin"));
        assert!(!has_role(&request, "user"));
    }

    #[test]
    fn test_extract_ip_untrusted_peer_ignores_spoofed_headers() {
        let config = AuthMiddlewareConfig {
            trusted_proxies: vec!["10.0.0.0/8".to_string()],
            ..Default::default()
        };
        let middleware = AuthMiddleware::new(
            AuthServices {
                jwt_service: Arc::new(JwtService::new("test", "test".to_string(), "test".to_string())),
                session_manager: Arc::new(SessionManager::new(InMemorySessionStore::new(), Duration::hours(24))),
                rate_limit_service: Arc::new(RateLimitService::new(InMemoryRateLimitStore::new())),
                account_lockout_service: Arc::new(AccountLockoutService::new(InMemoryLockoutStore::new())),
            },
            config,
        );

        let mut request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("x-forwarded-for", "127.0.0.1, 10.0.0.1")
            .header("cf-connecting-ip", "127.0.0.1")
            .body(Body::empty())
            .unwrap();

        // Direct connection from public IP (untrusted peer)
        let peer_addr: SocketAddr = "198.51.100.77:4321".parse().unwrap();
        request.extensions_mut().insert(ConnectInfo(peer_addr));

        let extracted = middleware.extract_ip_address(&request);
        assert_eq!(extracted, Some("198.51.100.77".to_string()), "Must ignore spoofed headers when peer is untrusted");
    }

    #[test]
    fn test_extract_ip_trusted_proxy_extracts_client_hop() {
        let config = AuthMiddlewareConfig {
            trusted_proxies: vec!["10.0.0.0/8".to_string()],
            ..Default::default()
        };
        let middleware = AuthMiddleware::new(
            AuthServices {
                jwt_service: Arc::new(JwtService::new("test", "test".to_string(), "test".to_string())),
                session_manager: Arc::new(SessionManager::new(InMemorySessionStore::new(), Duration::hours(24))),
                rate_limit_service: Arc::new(RateLimitService::new(InMemoryRateLimitStore::new())),
                account_lockout_service: Arc::new(AccountLockoutService::new(InMemoryLockoutStore::new())),
            },
            config,
        );

        let mut request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("x-forwarded-for", "1.1.1.1, 203.0.113.195, 10.0.0.1")
            .body(Body::empty())
            .unwrap();

        // Connection from trusted proxy
        let peer_addr: SocketAddr = "10.0.0.2:8080".parse().unwrap();
        request.extensions_mut().insert(ConnectInfo(peer_addr));

        let extracted = middleware.extract_ip_address(&request);
        assert_eq!(extracted, Some("203.0.113.195".to_string()), "Must extract right-most untrusted hop");
    }

    #[test]
    fn test_extract_ip_custom_header_with_trusted_proxy() {
        let config = AuthMiddlewareConfig {
            trusted_proxies: vec!["172.16.0.0/12".to_string()],
            client_ip_header: Some("cf-connecting-ip".to_string()),
            ..Default::default()
        };
        let middleware = AuthMiddleware::new(
            AuthServices {
                jwt_service: Arc::new(JwtService::new("test", "test".to_string(), "test".to_string())),
                session_manager: Arc::new(SessionManager::new(InMemorySessionStore::new(), Duration::hours(24))),
                rate_limit_service: Arc::new(RateLimitService::new(InMemoryRateLimitStore::new())),
                account_lockout_service: Arc::new(AccountLockoutService::new(InMemoryLockoutStore::new())),
            },
            config,
        );

        let mut request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("cf-connecting-ip", "198.51.100.99")
            .body(Body::empty())
            .unwrap();

        // Peer is in trusted proxy range
        let peer_addr: SocketAddr = "172.16.5.1:443".parse().unwrap();
        request.extensions_mut().insert(ConnectInfo(peer_addr));

        let extracted = middleware.extract_ip_address(&request);
        assert_eq!(extracted, Some("198.51.100.99".to_string()));
    }
}
