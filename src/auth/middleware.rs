use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

use crate::auth::{JwtService, SessionManager, RateLimitService, AccountLockoutService};

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
                "/api/v1/public".to_string(),
            ],
            required_permissions: Vec::new(),
            required_roles: Vec::new(),
            rate_limit_config: None,
            session_validation: true,
            ip_whitelist: Vec::new(),
            cors_origins: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct AuthServices<S> {
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

pub struct AuthMiddleware<S> {
    services: AuthServices<S>,
    config: AuthMiddlewareConfig,
}

impl<S> AuthMiddleware<S>
where
    S: crate::auth::SessionStore + Send + Sync + 'static,
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
        let method = request.method().to_string();
        let ip_address = self.extract_ip_address(&request);
        let user_agent = self.extract_user_agent(&request);

        // Check if path is allowed without authentication
        if self.is_path_allowed(path) {
            return Ok(next.run(request).await);
        }

        // Apply rate limiting if configured
        if let Some(rate_limit_config) = &self.config.rate_limit_config {
            let rate_limit_key = format!("{}:{}", ip_address.as_deref().unwrap_or("unknown"), path);
            
            match self.services.rate_limit_service.check_rate_limit(&rate_limit_key, rate_limit_config).await {
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
        let auth_context = match self.extract_and_validate_token(request.headers(), &ip_address, &user_agent).await {
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
                if !self.config.ip_whitelist.contains(ip) {
                    return Err(StatusCode::FORBIDDEN);
                }
            } else {
                return Err(StatusCode::FORBIDDEN);
            }
        }

        // Check required roles
        if !self.config.required_roles.is_empty() {
            if !self.config.required_roles.contains(&auth_context.role) {
                return Err(StatusCode::FORBIDDEN);
            }
        }

        // Check required permissions
        if !self.config.required_permissions.is_empty() {
            let has_all_permissions = self.config.required_permissions.iter()
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

        let token = self.services.jwt_service.extract_token_from_header(auth_header)
            .ok_or(AuthMiddlewareError::InvalidToken("Invalid Bearer token format".to_string()))?;

        // Validate JWT token
        let claims = self.services.jwt_service.validate_token(&token)
            .map_err(|e| match e {
                crate::auth::JwtError::Expired => AuthMiddlewareError::TokenExpired,
                crate::auth::JwtError::InvalidToken(_) => AuthMiddlewareError::InvalidToken(e.to_string()),
                _ => AuthMiddlewareError::ServiceError(e.to_string()),
            })?;

        // Check account lockout status
        let lockout_status = self.services.account_lockout_service.check_account_status(&claims.sub).await
            .map_err(|e| AuthMiddlewareError::ServiceError(e.to_string()))?;

        if lockout_status.is_locked || lockout_status.is_permanently_locked {
            return Err(AuthMiddlewareError::AccountLocked(
                lockout_status.permanent_lock_reason.unwrap_or_else(|| "Account is locked".to_string())
            ));
        }

        // Validate session if required
        if self.config.session_validation {
            let session = self.services.session_manager.validate_session(&claims.session_id).await
                .map_err(|e| match e {
                    crate::auth::SessionError::NotFound(_) => AuthMiddlewareError::SessionNotFound,
                    crate::auth::SessionError::Expired(_) => AuthMiddlewareError::SessionNotFound,
                    crate::auth::SessionError::Revoked(_) => AuthMiddlewareError::SessionNotFound,
                    _ => AuthMiddlewareError::ServiceError(e.to_string()),
                })?;

            // Update session with current request info
            if let Err(e) = self.services.session_manager.update_session_metadata(
                &claims.session_id,
                "last_ip",
                &ip_address.clone().unwrap_or_else(|| "unknown".to_string())
            ).await {
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
            issued_at: chrono::DateTime::from_timestamp(claims.iat, 0)
                .unwrap_or_else(|| Utc::now()),
            expires_at: chrono::DateTime::from_timestamp(claims.exp, 0)
                .unwrap_or_else(|| Utc::now()),
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

    fn extract_ip_address(&self, request: &Request) -> Option<String> {
        // Try various headers in order of preference
        let headers = [
            "x-forwarded-for",
            "x-real-ip",
            "cf-connecting-ip",
            "x-client-ip",
            "x-forwarded",
            "forwarded-for",
            "forwarded",
        ];

        for header in &headers {
            if let Some(value) = request.headers().get(*header) {
                if let Ok(ip_str) = value.to_str() {
                    // X-Forwarded-For can contain multiple IPs, take the first one
                    let ip = ip_str.split(',').next()?.trim();
                    if !ip.is_empty() {
                        return Some(ip.to_string());
                    }
                }
            }
        }

        // Fall back to connection info (not available in all contexts)
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
pub fn require_permissions(permissions: Vec<String>) -> impl Fn(AuthMiddlewareConfig) -> AuthMiddlewareConfig {
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
    use crate::auth::{JwtService, SessionManager, RateLimitService, AccountLockoutService, InMemorySessionStore, InMemoryRateLimitStore, InMemoryLockoutStore};
    use axum::{body::Body, http::Method, routing::get, Router};
    use chrono::Duration;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_auth_middleware_with_valid_token() {
        let jwt_service = JwtService::new("test-secret", "test-issuer".to_string(), "test-audience".to_string());
        let session_store = InMemorySessionStore::new();
        let session_manager = SessionManager::new(session_store, Duration::hours(24));
        let rate_limit_store = InMemoryRateLimitStore::new();
        let mut rate_limit_service = RateLimitService::new(rate_limit_store);
        rate_limit_service.add_config("api", crate::auth::RateLimitConfig::api()).unwrap();
        let lockout_store = InMemoryLockoutStore::new();
        let account_lockout_service = AccountLockoutService::new(lockout_store);
        account_lockout_service.add_config("auth", crate::auth::LockoutConfig::moderate()).unwrap();

        let services = AuthServices::new(
            jwt_service.clone(),
            session_manager.clone(),
            rate_limit_service,
            account_lockout_service,
        );

        // Generate a valid token
        let token = jwt_service.generate_token(
            "user123",
            "test@example.com",
            "user",
            vec!["read".to_string()],
            1,
        ).unwrap();

        let app = Router::new()
            .route("/protected", get(|| async { "Protected content" }))
            .route("/public", get(|| async { "Public content" }))
            .with_state(services)
            .layer(axum::middleware::from_fn_with_state(
                services.clone(),
                auth_middleware
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
            .uri("/public")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_middleware_with_invalid_token() {
        let jwt_service = JwtService::new("test-secret", "test-issuer".to_string(), "test-audience".to_string());
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
            .with_state(services)
            .layer(axum::middleware::from_fn_with_state(
                services.clone(),
                auth_middleware
            ));

        // Test with invalid token
        let request = axum::http::Request::builder()
            .method(Method::GET)
            .uri("/protected")
            .header("authorization", "Bearer invalid-token")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
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
                jwt_service: Arc::new(JwtService::new("test", "test".to_string(), "test".to_string())),
                session_manager: Arc::new(SessionManager::new(InMemorySessionStore::new(), Duration::hours(24))),
                rate_limit_service: Arc::new(RateLimitService::new(InMemoryRateLimitStore::new())),
                account_lockout_service: Arc::new(AccountLockoutService::new(InMemoryLockoutStore::new())),
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
            permissions: vec!["read".to_string(), "write".to_string(), "delete".to_string()],
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
}
