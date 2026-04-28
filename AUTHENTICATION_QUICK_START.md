# Authentication Service Quick Start Guide

## 🚀 Getting Started

This guide will help you quickly integrate and use the comprehensive authentication service in your Soroban Security Scanner project.

## 📋 Prerequisites

- Rust 1.70+ (for compilation)
- Redis (optional, for production use)
- OAuth provider credentials (optional, for social login)

## 🔧 Basic Setup

### 1. Add Dependencies

The authentication service is already included in the main project. Key dependencies:

```toml
[dependencies]
# Authentication & Security
jsonwebtoken = "9.1"
argon2 = "0.5"
base64 = "0.21"
rand = "0.8"

# HTTP & Web
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "compression-br"] }

# OAuth
oauth2 = "4.4"
url = "2.5"

# Optional: Redis for production
redis = { version = "0.23", features = ["tokio-comp"], optional = true }
```

### 2. Import the Authentication Module

```rust
use soroban_security_scanner::auth::{
    JwtService, PasswordService, SessionManager, RateLimitService,
    AccountLockoutService, AuthServices, InMemorySessionStore,
    InMemoryRateLimitStore, InMemoryLockoutStore,
    PasswordConfig, RateLimitConfig, LockoutConfig
};
use chrono::Duration;
```

## 🔐 Basic Authentication Setup

### 1. Initialize Services

```rust
async fn setup_auth_services() -> AuthServices<InMemorySessionStore> {
    // JWT Service
    let jwt_service = JwtService::new(
        "your-secret-key-here",
        "soroban-security-scanner".to_string(),
        "soroban-users".to_string(),
    );

    // Password Service
    let password_service = PasswordService::new(PasswordConfig::high_security());

    // Session Manager
    let session_store = InMemorySessionStore::new();
    let session_manager = SessionManager::new(session_store, Duration::hours(24));

    // Rate Limiting Service
    let rate_limit_store = InMemoryRateLimitStore::new();
    let mut rate_limit_service = RateLimitService::new(rate_limit_store);
    
    // Add rate limit configurations
    rate_limit_service.add_config("auth", RateLimitConfig::auth()).unwrap();
    rate_limit_service.add_config("api", RateLimitConfig::api()).unwrap();

    // Account Lockout Service
    let lockout_store = InMemoryLockoutStore::new();
    let mut lockout_service = AccountLockoutService::new(lockout_store);
    
    // Add lockout configurations
    lockout_service.add_config("login", LockoutConfig::moderate()).unwrap();

    // Create services bundle
    AuthServices::new(
        jwt_service,
        session_manager,
        rate_limit_service,
        lockout_service,
    )
}
```

### 2. Create Web Server with Authentication

```rust
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup authentication services
    let auth_services = setup_auth_services().await;

    // Create router
    let app = Router::new()
        // Public routes
        .route("/auth/login", post(login))
        .route("/auth/register", post(register))
        .route("/health", get(health_check))
        
        // Protected routes
        .route("/api/profile", get(get_profile))
        .route("/api/admin/users", get(get_users))
        
        .with_state(auth_services)
        .layer(axum::middleware::from_fn_with_state(
            auth_services.clone(),
            auth_middleware
        ));

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("🚀 Server running on http://0.0.0.0:3000");
    
    axum::serve(listener, app).await?;
    Ok(())
}
```

## 🔑 Login Endpoint Example

```rust
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    token: String,
    session_id: String,
    user: UserInfo,
    expires_in: i64,
}

#[derive(Serialize)]
struct UserInfo {
    id: String,
    email: String,
    role: String,
}

async fn login(
    State(services): State<AuthServices<InMemorySessionStore>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let email = &payload.email;
    let password = &payload.password;

    // 1. Check rate limiting
    let rate_limit_key = format!("login:{}", email);
    match services.rate_limit_service.check_rate_limit(&rate_limit_key, "auth").await {
        Ok(result) => {
            if !result.allowed {
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }
        }
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    }

    // 2. Check account lockout
    if !services.account_lockout_service.can_attempt_login(email, "login").await.unwrap_or(false) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    // 3. Validate credentials (replace with your user validation logic)
    if validate_user_credentials(email, password).await {
        // 4. Generate JWT token
        let token = services.jwt_service.generate_token(
            "user123", // Replace with actual user ID
            email,
            "user",    // Replace with actual user role
            vec!["read".to_string(), "write".to_string()],
            24, // 24 hours
        ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // 5. Create session
        let session_id = services.session_manager.create_session(
            "user123", // Replace with actual user ID
            email,
            "user",    // Replace with actual user role
            Some("127.0.0.1".to_string()),
            Some("User-Agent".to_string()),
            None,
        ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // 6. Reset failed attempts
        let _ = services.account_lockout_service.reset_failed_attempts(email).await;

        Ok(Json(LoginResponse {
            token,
            session_id,
            user: UserInfo {
                id: "user123".to_string(),
                email: email.clone(),
                role: "user".to_string(),
            },
            expires_in: 86400, // 24 hours in seconds
        }))
    } else {
        // Record failed attempt
        let _ = services.account_lockout_service.record_failed_attempt(
            email,
            "login",
            Some("127.0.0.1".to_string()),
            Some("User-Agent".to_string()),
            "Invalid credentials".to_string(),
        ).await;

        Err(StatusCode::UNAUTHORIZED)
    }
}

// Replace this with your actual user validation logic
async fn validate_user_credentials(email: &str, password: &str) -> bool {
    // This is a mock implementation
    // In production, you would:
    // 1. Look up user in database
    // 2. Verify password hash using PasswordService
    // 3. Check if account is active
    
    // Demo credentials
    (email == "admin@example.com" && password == "admin123!") ||
    (email == "user@example.com" && password == "user123!")
}
```

## 🛡️ Protected Route Example

```rust
use soroban_security_scanner::auth::{get_auth_context, AuthContext};

async fn get_profile(
    State(_services): State<AuthServices<InMemorySessionStore>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Get authentication context from request extensions
    // This is set by the auth middleware
    let auth_context = get_auth_context(&axum::extract::Request::builder()
        .uri("/api/profile")
        .body(axum::body::Body::empty())
        .unwrap())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "message": "This is your protected profile",
        "user_id": auth_context.user_id,
        "email": auth_context.email,
        "role": auth_context.role,
        "permissions": auth_context.permissions,
        "session_id": auth_context.session_id,
        "issued_at": auth_context.issued_at.to_rfc3339(),
        "expires_at": auth_context.expires_at.to_rfc3339()
    })))
}

async fn get_users(
    State(services): State<AuthServices<InMemorySessionStore>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // This endpoint would require admin role
    // The middleware would handle role checking
    
    // Mock user list
    let users = vec![
        json!({
            "id": "admin",
            "email": "admin@example.com",
            "role": "admin"
        }),
        json!({
            "id": "user",
            "email": "user@example.com",
            "role": "user"
        })
    ];

    Ok(Json(json!({
        "users": users,
        "total": users.len()
    })))
}
```

## 🔧 Authentication Middleware

The authentication middleware automatically handles:

1. **JWT Token Validation**: Extracts and validates Bearer tokens
2. **Session Validation**: Checks if session is still valid
3. **Account Lockout**: Prevents locked-out users from accessing
4. **Rate Limiting**: Applies rate limits to requests
5. **Permission Checking**: Validates user permissions and roles

### Custom Middleware Configuration

```rust
use soroban_security_scanner::auth::{AuthMiddlewareConfig, AuthMiddleware};

let config = AuthMiddlewareConfig {
    require_auth: true,
    allowed_paths: vec![
        "/health".to_string(),
        "/auth/login".to_string(),
        "/auth/register".to_string(),
    ],
    required_permissions: vec!["read".to_string()],
    required_roles: vec!["user".to_string()],
    rate_limit_config: Some("api".to_string()),
    session_validation: true,
    ip_whitelist: vec![],
    cors_origins: vec![],
};

let middleware = AuthMiddleware::new(services, config);
```

## 🔒 Password Management

### Hashing Passwords

```rust
use soroban_security_scanner::auth::{PasswordService, PasswordConfig};

let password_service = PasswordService::new(PasswordConfig::high_security());

// Hash password
let password = "user_password_123!";
let hash = password_service.hash_password(password)?;

// Verify password
let is_valid = password_service.verify_password(password, &hash)?;

// Check password strength
let strength = password_service.check_password_strength(password)?;
println!("Password strength: {:?}", strength);
```

### Password Strength Levels

- **Weak**: < 8 characters, no variety
- **Medium**: 8+ characters, some variety
- **Strong**: 12+ characters, good variety
- **Very Strong**: 16+ characters, excellent variety

## 🚦 Rate Limiting

### Built-in Configurations

```rust
// Authentication endpoints: 5 attempts per 5 minutes
rate_limit_service.add_config("auth", RateLimitConfig::auth())?;

// API endpoints: 1000 requests per hour
rate_limit_service.add_config("api", RateLimitConfig::api())?;

// Strict endpoints: 3 requests per minute
rate_limit_service.add_config("strict", RateLimitConfig::strict())?;

// Lenient endpoints: 100 requests per minute
rate_limit_service.add_config("lenient", RateLimitConfig::lenient())?;
```

### Custom Rate Limiting

```rust
let custom_config = RateLimitConfig::new(10, 60) // 10 requests per minute
    .with_penalty(300) // 5 minute penalty
    .with_burst(5);    // Allow burst of 5

rate_limit_service.add_config("custom", custom_config)?;
```

## 🔐 Account Lockout

### Built-in Configurations

```rust
// Moderate: 5 attempts in 30 minutes, 15 minute lockout
lockout_service.add_config("login", LockoutConfig::moderate())?;

// Strict: 3 attempts in 15 minutes, progressive lockout
lockout_service.add_config("admin", LockoutConfig::strict())?;

// API: 20 attempts per hour, progressive lockout
lockout_service.add_config("api", LockoutConfig::api())?;
```

### Progressive Lockout

```rust
let progressive_config = LockoutConfig::new(3, 60, 10) // 3 attempts per hour, 10 minute base
    .with_progressive_lockout(vec![1, 2, 4, 8]); // Multipliers: 1x, 2x, 4x, 8x

lockout_service.add_config("progressive", progressive_config)?;
```

## 🔗 OAuth Integration

### Setting up OAuth

```rust
use soroban_security_scanner::auth::{OAuthService, OAuthProvider};

let mut oauth_service = OAuthService::new();

// Add Google OAuth
let mut google_config = OAuthProvider::Google.default_config();
google_config.client_id = "your-google-client-id".to_string();
google_config.client_secret = "your-google-client-secret".to_string();
oauth_service.add_provider(OAuthProvider::Google, google_config)?;

// Add GitHub OAuth
let mut github_config = OAuthProvider::GitHub.default_config();
github_config.client_id = "your-github-client-id".to_string();
github_config.client_secret = "your-github-client-secret".to_string();
oauth_service.add_provider(OAuthProvider::GitHub, github_config)?;
```

### OAuth Flow

```rust
// 1. Get authorization URL
let (auth_url, csrf_token) = oauth_service.get_authorization_url("google")?;

// 2. Redirect user to auth_url
// 3. Handle callback with authorization code
let user_info = oauth_service.exchange_code_for_token(
    "google",
    "authorization_code_from_callback",
    "csrf_token_from_step_1",
).await?;

// 4. Create user account or login existing user
println!("User logged in: {}", user_info.email);
```

## 🛡️ Security Headers

### Default Security Headers

```rust
use soroban_security_scanner::auth::{SecurityHeadersMiddleware, SecurityHeadersConfig};

let middleware = SecurityHeadersMiddleware::new(SecurityHeadersConfig::default());
```

### Custom CSP Policy

```rust
use soroban_security_scanner::auth::CspBuilder;

let csp = CspBuilder::new()
    .default_src(&["'self'"])
    .script_src(&["'self'", "https://cdn.example.com"])
    .style_src(&["'self'", "'unsafe-inline'"])
    .img_src(&["'self'", "data:", "https:"])
    .connect_src(&["'self'"])
    .frame_ancestors(&["'none'"])
    .upgrade_insecure_requests()
    .build();

let config = SecurityHeadersConfig::new().with_csp(&csp);
let middleware = SecurityHeadersMiddleware::new(config);
```

## 🧪 Testing

### Running Tests

```bash
# Run all authentication tests
cargo test auth

# Run specific module tests
cargo test auth::jwt
cargo test auth::password
cargo test auth::rate_limit

# Run integration tests
cargo test --test auth_integration_tests
```

### Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use soroban_security_scanner::auth::*;

    #[tokio::test]
    async fn test_jwt_flow() {
        let jwt_service = JwtService::new("test-secret", "test-issuer", "test-audience");
        
        // Generate token
        let token = jwt_service.generate_token(
            "user123", "test@example.com", "user", 
            vec!["read".to_string()], 24
        ).unwrap();
        
        // Validate token
        let claims = jwt_service.validate_token(&token).unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.email, "test@example.com");
    }
}
```

## 🚀 Production Deployment

### Environment Variables

```bash
# JWT Configuration
export JWT_SECRET="your-super-secret-jwt-key-min-32-chars"
export JWT_ISSUER="soroban-security-scanner"
export JWT_AUDIENCE="soroban-users"

# Redis Configuration (optional)
export REDIS_URL="redis://localhost:6379"

# OAuth Configuration
export GOOGLE_CLIENT_ID="your-google-client-id"
export GOOGLE_CLIENT_SECRET="your-google-client-secret"
export GITHUB_CLIENT_ID="your-github-client-id"
export GITHUB_CLIENT_SECRET="your-github-client-secret"
```

### Production Configuration

```rust
// Use RSA for JWT in production
let jwt_service = JwtService::with_rsa(
    include_str!("private_key.pem"),
    include_str!("public_key.pem"),
    "soroban-security-scanner".to_string(),
    "soroban-users".to_string(),
);

// Use Redis for session storage
#[cfg(feature = "redis-cache")]
let redis_client = redis::Client::open(&redis_url)?;
let session_store = RedisSessionStore::new(redis_client, "app".to_string());

// Use high security password hashing
let password_service = PasswordService::new(PasswordConfig::high_security());

// Use strict rate limiting
rate_limit_service.add_config("auth", RateLimitConfig::strict())?;

// Use progressive account lockout
lockout_service.add_config("login", LockoutConfig::strict())?;
```

## 📚 Next Steps

1. **Review Documentation**: Read `AUTHENTICATION_SERVICE_COMPLETE.md`
2. **Run Examples**: Check `examples/auth_server.rs`
3. **Test Integration**: Run `tests/auth_integration_tests.rs`
4. **Configure for Production**: Set up Redis and environment variables
5. **Customize**: Adjust configurations for your specific needs

## 🆘 Troubleshooting

### Common Issues

1. **Compilation Errors**: Ensure all dependencies are up to date
2. **JWT Validation Failures**: Check secret key and issuer/audience settings
3. **Rate Limiting Issues**: Verify Redis connection if using distributed storage
4. **OAuth Problems**: Check client credentials and redirect URLs

### Getting Help

1. Check the comprehensive documentation
2. Review test cases for usage examples
3. Examine the example server implementation
4. Check the integration test scenarios

## 🎉 You're Ready!

You now have a complete, enterprise-grade authentication service integrated into your Soroban Security Scanner project. The service provides:

✅ **Secure JWT tokens** with configurable algorithms
✅ **Strong password hashing** with Argon2id
✅ **Flexible session management** with Redis support
✅ **Comprehensive rate limiting** with progressive penalties
✅ **Smart account lockout** with configurable policies
✅ **OAuth integration** for social logins
✅ **Security headers** for HTTP protection
✅ **Production-ready middleware** for easy integration

The authentication service is now ready to secure your application and protect your users! 🚀
