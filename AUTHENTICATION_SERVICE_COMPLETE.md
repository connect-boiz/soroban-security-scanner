# Comprehensive Authentication Service

This document describes the complete authentication service implementation for the Soroban Security Scanner project. The service provides enterprise-grade security features including JWT tokens, password hashing, session management, OAuth integration, rate limiting, account lockout, and security headers.

## Overview

The authentication service is built with security as the primary concern and includes:

- **JWT Token Service**: Secure token generation and validation with configurable algorithms
- **Password Service**: Argon2-based password hashing with strength validation
- **Session Management**: Redis/database-backed session storage with cleanup
- **OAuth Integration**: Support for Google, GitHub, Microsoft, Discord, and Facebook
- **Rate Limiting**: Configurable rate limiting with progressive penalties
- **Account Lockout**: Configurable lockout policies with progressive penalties
- **Security Headers**: Comprehensive security header middleware
- **Authentication Middleware**: Integrated middleware for request authentication

## Architecture

```
src/auth/
├── mod.rs                    # Module exports
├── jwt.rs                    # JWT token service
├── password.rs               # Password hashing service
├── rate_limit.rs             # Rate limiting service
├── oauth.rs                  # OAuth integration service
├── account_lockout.rs        # Account lockout service
├── security_headers.rs       # Security headers middleware
├── session_manager.rs        # Session management service
└── middleware.rs             # Authentication middleware
```

## Features

### 🔐 JWT Token Service

**Features:**
- HS256 and RS256 algorithm support
- Access and refresh tokens
- Configurable expiration times
- Token validation with claims checking
- Automatic token refresh

**Usage:**
```rust
use soroban_security_scanner::auth::{JwtService};

// Create JWT service
let jwt_service = JwtService::new(
    "your-secret-key",
    "your-issuer".to_string(),
    "your-audience".to_string()
);

// Generate access token
let token = jwt_service.generate_token(
    "user123",
    "user@example.com",
    "admin",
    vec!["read".to_string(), "write".to_string()],
    24, // expires in 24 hours
)?;

// Validate token
let claims = jwt_service.validate_token(&token)?;
```

### 🔒 Password Service

**Features:**
- Argon2id hashing algorithm
- Configurable security parameters
- Password strength validation
- Common pattern detection
- Secure password generation

**Usage:**
```rust
use soroban_security_scanner::auth::{PasswordService, PasswordConfig};

// Create password service
let password_service = PasswordService::new(PasswordConfig::high_security());

// Hash password
let hash = password_service.hash_password("user_password_123!")?;

// Verify password
let is_valid = password_service.verify_password("user_password_123!", &hash)?;

// Check password strength
let strength = password_service.check_password_strength("user_password_123!")?;
```

### 🗂️ Session Management

**Features:**
- In-memory and Redis storage backends
- Session expiration and cleanup
- User session tracking
- Session metadata storage
- Session revocation

**Usage:**
```rust
use soroban_security_scanner::auth::{SessionManager, InMemorySessionStore};
use chrono::Duration;

// Create session manager
let session_store = InMemorySessionStore::new();
let session_manager = SessionManager::new(session_store, Duration::hours(24));

// Create session
let session_id = session_manager.create_session(
    "user123",
    "user@example.com",
    "admin",
    Some("127.0.0.1".to_string()),
    Some("Mozilla/5.0".to_string()),
    None, // default TTL
).await?;

// Validate session
let session = session_manager.validate_session(&session_id).await?;
```

### 🔗 OAuth Integration

**Features:**
- Support for Google, GitHub, Microsoft, Discord, Facebook
- Automatic user info retrieval
- Configurable scopes and callbacks
- Error handling and validation

**Usage:**
```rust
use soroban_security_scanner::auth::{OAuthService, OAuthProvider};

// Create OAuth service
let mut oauth_service = OAuthService::new();

// Add Google OAuth
let google_config = OAuthProvider::Google.default_config();
oauth_service.add_provider(OAuthProvider::Google, google_config)?;

// Get authorization URL
let (auth_url, csrf_token) = oauth_service.get_authorization_url("google")?;

// Exchange code for user info
let user_info = oauth_service.exchange_code_for_token(
    "google",
    "authorization_code",
    "csrf_token",
).await?;
```

### 🚦 Rate Limiting

**Features:**
- Configurable rate limits per endpoint
- Progressive penalty system
- Multiple storage backends
- Automatic cleanup
- Per-IP and per-user limiting

**Usage:**
```rust
use soroban_security_scanner::auth::{RateLimitService, InMemoryRateLimitStore, RateLimitConfig};

// Create rate limit service
let rate_limit_store = InMemoryRateLimitStore::new();
let mut rate_limit_service = RateLimitService::new(rate_limit_store);

// Add rate limit configuration
rate_limit_service.add_config("api", RateLimitConfig::api())?;
rate_limit_service.add_config("auth", RateLimitConfig::auth())?;

// Check rate limit
let result = rate_limit_service.check_rate_limit("user123", "auth").await?;
if !result.allowed {
    println!("Rate limit exceeded. Retry after {} seconds", result.retry_after);
}
```

### 🔐 Account Lockout

**Features:**
- Configurable lockout policies
- Progressive lockout durations
- Permanent lockout support
- Automatic unlock
- Failed attempt tracking

**Usage:**
```rust
use soroban_security_scanner::auth::{AccountLockoutService, InMemoryLockoutStore, LockoutConfig};

// Create lockout service
let lockout_store = InMemoryLockoutStore::new();
let mut lockout_service = AccountLockoutService::new(lockout_store);

// Add lockout configuration
lockout_service.add_config("login", LockoutConfig::moderate())?;

// Record failed attempt
let result = lockout_service.record_failed_attempt(
    "user123",
    "login",
    Some("127.0.0.1".to_string()),
    Some("Mozilla/5.0".to_string()),
    "Invalid password".to_string(),
).await?;

if result.is_locked {
    println!("Account locked for {} minutes", result.lockout_duration.unwrap());
}
```

### 🛡️ Security Headers

**Features:**
- Comprehensive security header support
- CSP builder for content security policies
- Environment-specific configurations
- Custom header support

**Usage:**
```rust
use soroban_security_scanner::auth::{SecurityHeadersMiddleware, SecurityHeadersConfig, CspBuilder};

// Create security headers middleware
let config = SecurityHeadersConfig::new()
    .with_csp(&CspBuilder::new()
        .default_src(&["'self'"])
        .script_src(&["'self'", "https://cdn.example.com"])
        .build());

let middleware = SecurityHeadersMiddleware::new(config);
```

### 🔧 Authentication Middleware

**Features:**
- JWT token validation
- Session validation
- Permission and role checking
- IP whitelisting
- Rate limiting integration

**Usage:**
```rust
use soroban_security_scanner::auth::{AuthServices, AuthMiddlewareConfig, auth_middleware};
use axum::{Router, routing::get};

// Create auth services
let services = AuthServices::new(
    jwt_service,
    session_manager,
    rate_limit_service,
    account_lockout_service,
);

// Create router with auth middleware
let app = Router::new()
    .route("/protected", get(protected_handler))
    .with_state(services)
    .layer(axum::middleware::from_fn_with_state(
        services.clone(),
        auth_middleware
    ));
```

## Configuration

### Environment Variables

```bash
# JWT Configuration
JWT_SECRET=your-super-secret-jwt-key
JWT_ISSUER=soroban-security-scanner
JWT_AUDIENCE=soroban-users

# Redis Configuration (optional)
REDIS_URL=redis://localhost:6379

# OAuth Configuration
GOOGLE_CLIENT_ID=your-google-client-id
GOOGLE_CLIENT_SECRET=your-google-client-secret

GITHUB_CLIENT_ID=your-github-client-id
GITHUB_CLIENT_SECRET=your-github-client-secret

# Rate Limiting
RATE_LIMIT_REDIS_URL=redis://localhost:6379
```

### Security Configurations

#### JWT Service
```rust
// Production configuration
let jwt_service = JwtService::new(
    std::env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
    std::env::var("JWT_ISSUER").unwrap_or_else(|_| "soroban-security-scanner".to_string()),
    std::env::var("JWT_AUDIENCE").unwrap_or_else(|_| "soroban-users".to_string()),
);

// RSA-based configuration (more secure)
let jwt_service = JwtService::with_rsa(
    private_key_pem,
    public_key_pem,
    "soroban-security-scanner".to_string(),
    "soroban-users".to_string(),
);
```

#### Password Service
```rust
// High security configuration
let password_service = PasswordService::new(PasswordConfig::high_security());

// Low memory configuration
let password_service = PasswordService::new(PasswordConfig::low_memory());
```

#### Rate Limiting
```rust
// API endpoints: 1000 requests per hour
rate_limit_service.add_config("api", RateLimitConfig::api())?;

// Authentication: 5 attempts per 5 minutes with 15 minute penalty
rate_limit_service.add_config("auth", RateLimitConfig::auth())?;

// Strict: 3 requests per minute with 5 minute penalty
rate_limit_service.add_config("strict", RateLimitConfig::strict())?;
```

#### Account Lockout
```rust
// Moderate: 5 attempts in 30 minutes, 15 minute lockout
lockout_service.add_config("login", LockoutConfig::moderate())?;

// Strict: 3 attempts in 15 minutes, progressive lockout
lockout_service.add_config("admin", LockoutConfig::strict())?;

// API: 20 attempts per hour, progressive lockout
lockout_service.add_config("api", LockoutConfig::api())?;
```

## Security Best Practices

### 1. JWT Security
- Use strong, randomly generated secrets
- Prefer RSA over HMAC for production
- Set appropriate expiration times
- Use refresh tokens for long-lived sessions
- Validate all claims

### 2. Password Security
- Use Argon2id with appropriate parameters
- Enforce strong password policies
- Implement password strength validation
- Store only hashed passwords
- Use unique salts for each password

### 3. Session Security
- Use secure session storage (Redis in production)
- Set appropriate session timeouts
- Implement session cleanup
- Track session metadata
- Support session revocation

### 4. Rate Limiting
- Implement multiple rate limit tiers
- Use progressive penalties
- Rate limit by IP and user
- Configure appropriate windows
- Monitor and adjust limits

### 5. Account Lockout
- Implement progressive lockout
- Support permanent lockout for abuse
- Provide unlock mechanisms
- Log lockout events
- Monitor lockout patterns

### 6. Security Headers
- Use comprehensive CSP policies
- Enable HSTS in production
- Implement proper CORS policies
- Use security-focused configurations
- Test header effectiveness

## Integration Examples

### Complete Authentication Flow

```rust
use soroban_security_scanner::auth::*;
use axum::{extract::State, response::Json, routing::{get, post}, Router};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize services
    let jwt_service = JwtService::new(
        std::env::var("JWT_SECRET")?,
        "soroban-security-scanner".to_string(),
        "soroban-users".to_string(),
    );

    let password_service = PasswordService::new(PasswordConfig::high_security());
    let session_store = InMemorySessionStore::new();
    let session_manager = SessionManager::new(session_store, chrono::Duration::hours(24));

    let rate_limit_store = InMemoryRateLimitStore::new();
    let mut rate_limit_service = RateLimitService::new(rate_limit_store);
    rate_limit_service.add_config("auth", RateLimitConfig::auth())?;

    let lockout_store = InMemoryLockoutStore::new();
    let mut lockout_service = AccountLockoutService::new(lockout_store);
    lockout_service.add_config("login", LockoutConfig::moderate())?;

    let services = AuthServices::new(
        jwt_service,
        session_manager,
        rate_limit_service,
        lockout_service,
    );

    // Create router
    let app = Router::new()
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh_token))
        .route("/protected", get(protected_route))
        .with_state(services)
        .layer(axum::middleware::from_fn_with_state(
            services.clone(),
            auth_middleware
        ));

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn login(
    State(services): State<AuthServices<InMemorySessionStore>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let email = payload.get("email").and_then(|v| v.as_str()).unwrap_or("");
    let password = payload.get("password").and_then(|v| v.as_str()).unwrap_or("");

    // Check rate limiting
    let rate_limit_key = format!("login:{}", email);
    match services.rate_limit_service.check_rate_limit(&rate_limit_key, "auth").await {
        Ok(result) => {
            if !result.allowed {
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }
        }
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    }

    // Check account lockout
    if !services.account_lockout_service.can_attempt_login(email, "login").await.unwrap_or(false) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    // Validate credentials (mock implementation)
    if email == "admin@example.com" && password == "correct_password" {
        // Generate JWT token
        let token = services.jwt_service.generate_token(
            "user123",
            email,
            "admin",
            vec!["read".to_string(), "write".to_string()],
            24,
        ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Create session
        let session_id = services.session_manager.create_session(
            "user123",
            email,
            "admin",
            Some("127.0.0.1".to_string()),
            None,
            None,
        ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Reset failed attempts
        let _ = services.account_lockout_service.reset_failed_attempts(email).await;

        Ok(Json(json!({
            "token": token,
            "session_id": session_id,
            "expires_in": 86400
        })))
    } else {
        // Record failed attempt
        let _ = services.account_lockout_service.record_failed_attempt(
            email,
            "login",
            Some("127.0.0.1".to_string()),
            None,
            "Invalid credentials".to_string(),
        ).await;

        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn protected_route(
    State(_services): State<AuthServices<InMemorySessionStore>>,
) -> Json<serde_json::Value> {
    Json(json!({
        "message": "This is a protected route",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn refresh_token(
    State(services): State<AuthServices<InMemorySessionStore>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let refresh_token = payload.get("refresh_token")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;

    // Validate refresh token and generate new access token
    let new_token = services.jwt_service.refresh_access_token(
        refresh_token,
        "user123",
        "admin@example.com",
        "admin",
        vec!["read".to_string(), "write".to_string()],
        24,
    ).map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(Json(json!({
        "token": new_token,
        "expires_in": 86400
    })))
}
```

## Testing

### Unit Tests

Each module includes comprehensive unit tests. Run with:

```bash
cargo test auth::jwt
cargo test auth::password
cargo test auth::rate_limit
cargo test auth::account_lockout
cargo test auth::oauth
cargo test auth::security_headers
cargo test auth::middleware
```

### Integration Tests

```bash
cargo test auth_integration
```

### Security Tests

```bash
cargo test auth_security
```

## Performance Considerations

### JWT Performance
- Token validation is fast (microseconds)
- Use appropriate key sizes
- Cache verification results when possible

### Password Hashing Performance
- Argon2id is intentionally slow
- Adjust parameters based on hardware
- Consider parallel processing

### Session Storage Performance
- Redis provides millisecond latency
- Use connection pooling
- Implement proper cleanup

### Rate Limiting Performance
- In-memory storage is fastest
- Redis provides distributed limiting
- Use efficient data structures

## Monitoring and Logging

### Key Metrics
- Authentication success/failure rates
- Token validation latency
- Session creation/revocation counts
- Rate limit violations
- Account lockout events

### Logging Levels
- `ERROR`: Security violations, system failures
- `WARN`: Rate limit exceeded, account locked
- `INFO`: Successful authentication, session created
- `DEBUG`: Token validation, permission checks

### Security Events
- Failed login attempts
- Account lockouts
- Token validation failures
- Rate limit violations
- Suspicious activity patterns

## Deployment Considerations

### Production Configuration
- Use environment-specific configurations
- Enable all security headers
- Use Redis for session and rate limit storage
- Implement proper secret management
- Enable comprehensive logging

### Scaling Considerations
- Use Redis cluster for distributed storage
- Implement load balancing
- Consider session affinity requirements
- Monitor performance metrics
- Plan for capacity

### Security Hardening
- Regular security audits
- Dependency updates
- Penetration testing
- Security monitoring
- Incident response planning

## Troubleshooting

### Common Issues

1. **JWT Validation Failures**
   - Check secret key configuration
   - Verify token expiration
   - Validate issuer/audience claims

2. **Rate Limiting Errors**
   - Check Redis connectivity
   - Verify configuration parameters
   - Monitor key expiration

3. **Session Issues**
   - Check storage backend health
   - Verify cleanup processes
   - Monitor session expiration

4. **OAuth Problems**
   - Verify client credentials
   - Check redirect URLs
   - Validate configuration

### Debug Mode

Enable debug logging by setting:
```rust
env_logger::init();
```

## License

This authentication service is part of the Soroban Security Scanner project and follows the same license terms.

## Contributing

When contributing to the authentication service:

1. Follow security best practices
2. Add comprehensive tests
3. Update documentation
4. Consider performance implications
5. Follow the established code patterns

## Support

For authentication service issues:
1. Check the documentation
2. Review test cases
3. Monitor logs
4. Create detailed bug reports
5. Include security considerations
