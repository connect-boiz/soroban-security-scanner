use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use soroban_security_scanner::auth::{
    AccountLockoutService, AuthContext, AuthMiddlewareConfig, AuthServices, InMemoryLockoutStore,
    InMemoryRateLimitStore, InMemorySessionStore, JwtService, LockoutConfig, PasswordConfig,
    PasswordService, RateLimitConfig, RateLimitService, SessionManager, SecurityHeadersConfig,
    SecurityHeadersMiddleware,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: String,
    email: String,
    password_hash: String,
    role: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
struct AppState {
    users: Arc<tokio::sync::RwLock<Vec<User>>>,
    auth_services: AuthServices<InMemorySessionStore>,
}

impl AppState {
    async fn new() -> Self {
        // Initialize JWT service
        let jwt_service = JwtService::new(
            "your-super-secret-jwt-key-change-in-production",
            "soroban-security-scanner".to_string(),
            "soroban-users".to_string(),
        );

        // Initialize password service
        let password_service = PasswordService::new(PasswordConfig::high_security());

        // Initialize session manager
        let session_store = InMemorySessionStore::new();
        let session_manager = SessionManager::new(session_store, Duration::hours(24));

        // Initialize rate limiting service
        let rate_limit_store = InMemoryRateLimitStore::new();
        let mut rate_limit_service = RateLimitService::new(rate_limit_store);
        rate_limit_service.add_config("auth", RateLimitConfig::auth()).unwrap();
        rate_limit_service.add_config("api", RateLimitConfig::api()).unwrap();
        rate_limit_service.add_config("strict", RateLimitConfig::strict()).unwrap();

        // Initialize account lockout service
        let lockout_store = InMemoryLockoutStore::new();
        let mut lockout_service = AccountLockoutService::new(lockout_store);
        lockout_service.add_config("login", LockoutConfig::moderate()).unwrap();
        lockout_service.add_config("admin", LockoutConfig::strict()).unwrap();

        // Create auth services
        let auth_services = AuthServices::new(
            jwt_service,
            session_manager,
            rate_limit_service,
            lockout_service,
        );

        // Create demo users
        let mut users = Vec::new();
        
        // Admin user
        let admin_hash = password_service.hash_password("admin123!").unwrap();
        users.push(User {
            id: "admin".to_string(),
            email: "admin@example.com".to_string(),
            password_hash: admin_hash,
            role: "admin".to_string(),
            created_at: chrono::Utc::now(),
        });

        // Regular user
        let user_hash = password_service.hash_password("user123!").unwrap();
        users.push(User {
            id: "user".to_string(),
            email: "user@example.com".to_string(),
            password_hash: user_hash,
            role: "user".to_string(),
            created_at: chrono::Utc::now(),
        });

        Self {
            users: Arc::new(tokio::sync::RwLock::new(users)),
            auth_services,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create application state
    let state = AppState::new().await;

    // Create router
    let app = Router::new()
        // Public routes
        .route("/", get(index))
        .route("/health", get(health_check))
        .route("/auth/login", post(login))
        .route("/auth/register", post(register))
        .route("/auth/forgot-password", post(forgot_password))
        .route("/auth/reset-password", post(reset_password))
        .route("/auth/status/:email", get(auth_status))
        
        // Protected routes
        .route("/api/profile", get(get_profile))
        .route("/api/admin/users", get(get_users))
        .route("/api/admin/sessions", get(get_sessions))
        .route("/api/admin/stats", get(get_stats))
        
        // Rate limited routes
        .route("/api/limited", get(rate_limited_endpoint))
        
        .with_state(state)
        .layer(
            // CORS middleware
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(
            // Security headers middleware
            middleware::from_fn_with_state(
                SecurityHeadersConfig::development(),
                security_headers_middleware,
            ),
        )
        .layer(
            // Authentication middleware for protected routes
            middleware::from_fn_with_state(
                AuthServices::new(
                    JwtService::new("test", "test".to_string(), "test".to_string()),
                    SessionManager::new(InMemorySessionStore::new(), Duration::hours(24)),
                    RateLimitService::new(InMemoryRateLimitStore::new()),
                    AccountLockoutService::new(InMemoryLockoutStore::new()),
                ),
                auth_middleware,
            ),
        );

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("🚀 Authentication server running on http://0.0.0.0:3000");
    println!("📚 API Documentation: http://0.0.0.0:3000");
    println!("🔐 Login: POST /auth/login");
    println!("👤 Profile: GET /api/profile");
    println!("👥 Admin Users: GET /api/admin/users");
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

// Middleware functions
async fn security_headers_middleware(
    State(config): State<SecurityHeadersConfig>,
    request: axum::extract::Request,
    next: Next,
) -> Result<axum::response::Response, StatusCode> {
    let middleware = SecurityHeadersMiddleware::new(config);
    middleware.apply_security_headers(request, next).await
}

async fn auth_middleware(
    State(services): State<AuthServices<InMemorySessionStore>>,
    request: axum::extract::Request,
    next: Next,
) -> Result<axum::response::Response, StatusCode> {
    let config = AuthMiddlewareConfig {
        require_auth: true,
        allowed_paths: vec![
            "/".to_string(),
            "/health".to_string(),
            "/auth/login".to_string(),
            "/auth/register".to_string(),
            "/auth/forgot-password".to_string(),
            "/auth/reset-password".to_string(),
            "/auth/status".to_string(),
        ],
        required_permissions: Vec::new(),
        required_roles: Vec::new(),
        rate_limit_config: Some("api".to_string()),
        session_validation: true,
        ip_whitelist: Vec::new(),
        cors_origins: Vec::new(),
    };

    let middleware = soroban_security_scanner::auth::AuthMiddleware::new(services, config);
    middleware.authenticate_request(request, next).await
}

// Route handlers
async fn index() -> Html<&'static str> {
    Html(include_str!("../static/auth_demo.html"))
}

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "soroban-auth-service"
    }))
}

#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct RegisterRequest {
    email: String,
    password: String,
    name: Option<String>,
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let email = &payload.email;
    let password = &payload.password;

    // Check rate limiting
    let rate_limit_key = format!("login:{}", email);
    match state.auth_services.rate_limit_service.check_rate_limit(&rate_limit_key, "auth").await {
        Ok(result) => {
            if !result.allowed {
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }
        }
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    }

    // Check account lockout
    if !state.auth_services.account_lockout_service.can_attempt_login(email, "login").await.unwrap_or(false) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    // Find user
    let users = state.users.read().await;
    let user = users.iter().find(|u| u.email == *email);

    if let Some(user) = user {
        // Verify password
        let password_service = PasswordService::new(PasswordConfig::high_security());
        match password_service.verify_password(password, &user.password_hash) {
            Ok(true) => {
                // Generate JWT token
                let token = state.auth_services.jwt_service.generate_token(
                    &user.id,
                    &user.email,
                    &user.role,
                    vec!["read".to_string(), "write".to_string()],
                    24,
                ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                // Create session
                let session_id = state.auth_services.session_manager.create_session(
                    &user.id,
                    &user.email,
                    &user.role,
                    Some("127.0.0.1".to_string()),
                    Some("Demo Browser".to_string()),
                    None,
                ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                // Reset failed attempts
                let _ = state.auth_services.account_lockout_service.reset_failed_attempts(email).await;

                Ok(Json(json!({
                    "success": true,
                    "token": token,
                    "session_id": session_id,
                    "user": {
                        "id": user.id,
                        "email": user.email,
                        "role": user.role
                    },
                    "expires_in": 86400
                })))
            }
            Ok(false) => {
                // Record failed attempt
                let _ = state.auth_services.account_lockout_service.record_failed_attempt(
                    email,
                    "login",
                    Some("127.0.0.1".to_string()),
                    Some("Demo Browser".to_string()),
                    "Invalid password".to_string(),
                ).await;
                Err(StatusCode::UNAUTHORIZED)
            }
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    } else {
        // Record failed attempt (user not found)
        let _ = state.auth_services.account_lockout_service.record_failed_attempt(
            email,
            "login",
            Some("127.0.0.1".to_string()),
            Some("Demo Browser".to_string()),
            "User not found".to_string(),
        ).await;
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let email = &payload.email;
    let password = &payload.password;

    // Check if user already exists
    let users = state.users.read().await;
    if users.iter().any(|u| u.email == *email) {
        return Ok(Json(json!({
            "success": false,
            "error": "User already exists"
        })));
    }
    drop(users);

    // Hash password
    let password_service = PasswordService::new(PasswordConfig::high_security());
    let password_hash = password_service.hash_password(password)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create new user
    let new_user = User {
        id: uuid::Uuid::new_v4().to_string(),
        email: email.clone(),
        password_hash,
        role: "user".to_string(),
        created_at: chrono::Utc::now(),
    };

    // Save user
    let mut users = state.users.write().await;
    users.push(new_user.clone());

    Ok(Json(json!({
        "success": true,
        "user": {
            "id": new_user.id,
            "email": new_user.email,
            "role": new_user.role
        }
    })))
}

async fn forgot_password(
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let email = payload.get("email").and_then(|v| v.as_str()).unwrap_or("");
    
    // In a real implementation, you would:
    // 1. Generate a reset token
    // 2. Send email with reset link
    // 3. Store token with expiration
    
    Ok(Json(json!({
        "success": true,
        "message": "If the email exists, a reset link has been sent"
    })))
}

async fn reset_password(
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let token = payload.get("token").and_then(|v| v.as_str()).unwrap_or("");
    let new_password = payload.get("new_password").and_then(|v| v.as_str()).unwrap_or("");
    
    // In a real implementation, you would:
    // 1. Validate reset token
    // 2. Update user password
    // 3. Invalidate all sessions
    
    Ok(Json(json!({
        "success": true,
        "message": "Password has been reset successfully"
    })))
}

async fn auth_status(
    Path(email): Path<String>,
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let lockout_status = state.auth_services.account_lockout_service.check_account_status(&email).await.unwrap_or_else(|_| {
        soroban_security_scanner::auth::LockoutStatus {
            is_locked: false,
            is_permanently_locked: false,
            lockout_expires_at: None,
            remaining_attempts: None,
            total_attempts: 0,
            lockout_count: 0,
            last_attempt: None,
            recent_attempts: Vec::new(),
        }
    });

    let rate_limit_status = state.auth_services.rate_limit_service.get_status(&format!("login:{}", &email), "auth").await.unwrap_or_else(|_| {
        soroban_security_scanner::auth::RateLimitStatus {
            current_requests: 0,
            max_requests: 5,
            window_seconds: 300,
            remaining: 5,
            reset_time: chrono::Utc::now() + chrono::Duration::minutes(5),
            is_blocked: false,
            blocked_until: None,
            total_requests: 0,
            last_request: None,
        }
    });

    Json(json!({
        "email": email,
        "lockout": {
            "is_locked": lockout_status.is_locked,
            "is_permanently_locked": lockout_status.is_permanently_locked,
            "lockout_expires_at": lockout_status.lockout_expires_at,
            "total_attempts": lockout_status.total_attempts,
            "lockout_count": lockout_status.lockout_count,
            "recent_attempts": lockout_status.recent_attempts.len()
        },
        "rate_limit": {
            "current_requests": rate_limit_status.current_requests,
            "max_requests": rate_limit_status.max_requests,
            "remaining": rate_limit_status.remaining,
            "is_blocked": rate_limit_status.is_blocked,
            "blocked_until": rate_limit_status.blocked_until,
            "total_requests": rate_limit_status.total_requests
        }
    }))
}

// Protected routes
async fn get_profile(
    State(_state): State<AppState>,
) -> Json<serde_json::Value> {
    // This would normally extract user info from the auth context
    Json(json!({
        "message": "This is your protected profile",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn get_users(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let users = state.users.read().await;
    let user_list: Vec<_> = users.iter().map(|u| {
        json!({
            "id": u.id,
            "email": u.email,
            "role": u.role,
            "created_at": u.created_at.to_rfc3339()
        })
    }).collect();

    Json(json!({
        "users": user_list,
        "total": users.len()
    }))
}

async fn get_sessions(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    // Get all active sessions (simplified for demo)
    Json(json!({
        "sessions": [],
        "total": 0,
        "message": "Session management would be implemented here"
    }))
}

async fn get_stats(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let users = state.users.read().await;
    
    Json(json!({
        "stats": {
            "total_users": users.len(),
            "admin_users": users.iter().filter(|u| u.role == "admin").count(),
            "regular_users": users.iter().filter(|u| u.role == "user").count(),
            "active_sessions": 0, // Would be calculated from session store
            "rate_limit_violations": 0, // Would be calculated from rate limit store
            "locked_accounts": 0, // Would be calculated from lockout store
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn rate_limited_endpoint(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let rate_limit_key = "demo:endpoint";
    
    match state.auth_services.rate_limit_service.check_rate_limit(rate_limit_key, "strict").await {
        Ok(result) => {
            if !result.allowed {
                return Ok(Json(json!({
                    "error": "Rate limit exceeded",
                    "retry_after": result.retry_after,
                    "reset_time": result.reset_time.to_rfc3339()
                })));
            }
        }
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    }

    Ok(Json(json!({
        "message": "This endpoint has strict rate limiting",
        "remaining_requests": "Check rate limit headers",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}
