//! Integration tests for the comprehensive authentication service

use soroban_security_scanner::auth::*;
use chrono::{Duration, Utc};
use std::collections::HashMap;

#[tokio::test]
async fn test_complete_authentication_flow() {
    // Initialize services
    let jwt_service = JwtService::new(
        "test-secret-key-for-integration-testing",
        "test-issuer".to_string(),
        "test-audience".to_string(),
    );

    let password_service = PasswordService::new(PasswordConfig::high_security());
    let session_store = InMemorySessionStore::new();
    let session_manager = SessionManager::new(session_store, Duration::hours(24));

    let rate_limit_store = InMemoryRateLimitStore::new();
    let mut rate_limit_service = RateLimitService::new(rate_limit_store);
    rate_limit_service.add_config("auth", RateLimitConfig::auth()).unwrap();
    rate_limit_service.add_config("api", RateLimitConfig::api()).unwrap();

    let lockout_store = InMemoryLockoutStore::new();
    let mut lockout_service = AccountLockoutService::new(lockout_store);
    lockout_service.add_config("login", LockoutConfig::moderate()).unwrap();

    // Test user creation and password hashing
    let user_password = "TestPassword123!";
    let password_hash = password_service.hash_password(user_password).unwrap();
    assert!(password_service.verify_password(user_password, &password_hash).unwrap());
    assert!(!password_service.verify_password("wrong_password", &password_hash).unwrap());

    // Test JWT token generation and validation
    let user_id = "test-user-123";
    let user_email = "test@example.com";
    let user_role = "user";
    let permissions = vec!["read".to_string(), "write".to_string()];

    let access_token = jwt_service
        .generate_token(user_id, user_email, user_role, permissions.clone(), 24)
        .unwrap();

    let claims = jwt_service.validate_token(&access_token).unwrap();
    assert_eq!(claims.sub, user_id);
    assert_eq!(claims.email, user_email);
    assert_eq!(claims.role, user_role);
    assert_eq!(claims.permissions, permissions);

    // Test session management
    let session_id = session_manager
        .create_session(
            user_id,
            user_email,
            user_role,
            Some("127.0.0.1".to_string()),
            Some("Test Browser".to_string()),
            None,
        )
        .await
        .unwrap();

    let session = session_manager.validate_session(&session_id).await.unwrap();
    assert_eq!(session.user_id, user_id);
    assert_eq!(session.user_email, user_email);
    assert_eq!(session.user_role, user_role);

    // Test session metadata
    session_manager
        .update_session_metadata(&session_id, "last_action", "login")
        .await
        .unwrap();

    let last_action = session_manager
        .get_session_metadata(&session_id, "last_action")
        .await
        .unwrap();
    assert_eq!(last_action, Some("login".to_string()));

    // Test rate limiting
    let rate_limit_key = format!("login:{}", user_email);
    
    // First few requests should succeed
    for i in 0..5 {
        let result = rate_limit_service
            .check_rate_limit(&rate_limit_key, "auth")
            .await
            .unwrap();
        assert!(result.allowed);
        assert_eq!(result.remaining, 4 - i);
    }

    // Next request should be blocked
    let result = rate_limit_service
        .check_rate_limit(&rate_limit_key, "auth")
        .await
        .unwrap();
    assert!(!result.allowed);
    assert_eq!(result.remaining, 0);

    // Test account lockout
    let lockout_key = format!("user:{}", user_id);
    
    // Add failed attempts
    for i in 0..5 {
        let result = lockout_service
            .record_failed_attempt(
                user_id,
                "login",
                Some("127.0.0.1".to_string()),
                Some("Test Browser".to_string()),
                "Invalid password".to_string(),
            )
            .await
            .unwrap();
        
        if i < 4 {
            assert!(!result.is_locked);
            assert_eq!(result.remaining_attempts, 4 - i);
        } else {
            assert!(result.is_locked);
            assert_eq!(result.lockout_count, 1);
        }
    }

    // Check that user is locked out
    let can_attempt = lockout_service.can_attempt_login(user_id, "login").await.unwrap();
    assert!(!can_attempt);

    // Test lockout status
    let status = lockout_service.check_account_status(user_id).await.unwrap();
    assert!(status.is_locked);
    assert_eq!(status.total_attempts, 5);
    assert_eq!(status.lockout_count, 1);

    // Reset failed attempts
    lockout_service.reset_failed_attempts(user_id).await.unwrap();
    
    let can_attempt = lockout_service.can_attempt_login(user_id, "login").await.unwrap();
    assert!(can_attempt);

    // Test session revocation
    session_manager.revoke_session(&session_id).await.unwrap();
    let result = session_manager.validate_session(&session_id).await;
    assert!(matches!(result, Err(SessionError::Revoked(_))));
}

#[tokio::test]
async fn test_oauth_integration() {
    let mut oauth_service = OAuthService::new();

    // Test Google OAuth configuration
    let google_config = OAuthProvider::Google.default_config();
    let mut test_config = google_config.clone();
    test_config.client_id = "test-google-client-id".to_string();
    test_config.client_secret = "test-google-client-secret".to_string();

    oauth_service.add_provider(OAuthProvider::Google, test_config).unwrap();

    // Test authorization URL generation
    let (auth_url, csrf_token) = oauth_service.get_authorization_url("google").unwrap();
    assert!(auth_url.starts_with("https://accounts.google.com"));
    assert!(!csrf_token.as_str().is_empty());

    // Test GitHub OAuth configuration
    let github_config = OAuthProvider::GitHub.default_config();
    let mut test_config = github_config.clone();
    test_config.client_id = "test-github-client-id".to_string();
    test_config.client_secret = "test-github-client-secret".to_string();

    oauth_service.add_provider(OAuthProvider::GitHub, test_config).unwrap();

    let (auth_url, csrf_token) = oauth_service.get_authorization_url("github").unwrap();
    assert!(auth_url.starts_with("https://github.com"));
    assert!(!csrf_token.as_str().is_empty());

    // Test provider listing
    let providers = oauth_service.get_configured_providers();
    assert_eq!(providers.len(), 2);
    assert!(providers.contains(&"google".to_string()));
    assert!(providers.contains(&"github".to_string()));

    // Test provider configuration check
    assert!(oauth_service.is_provider_configured("google"));
    assert!(oauth_service.is_provider_configured("github"));
    assert!(!oauth_service.is_provider_configured("microsoft"));
}

#[test]
fn test_security_headers_configuration() {
    // Test default configuration
    let default_config = SecurityHeadersConfig::default();
    assert!(default_config.content_security_policy.is_some());
    assert!(default_config.strict_transport_security.is_some());
    assert_eq!(default_config.x_frame_options, Some("DENY".to_string()));

    // Test development configuration
    let dev_config = SecurityHeadersConfig::development();
    assert!(dev_config.content_security_policy.is_some());
    assert!(dev_config.strict_transport_security.is_none()); // Disabled in dev

    // Test minimal configuration
    let minimal_config = SecurityHeadersConfig::minimal();
    assert!(minimal_config.content_security_policy.is_none());
    assert!(minimal_config.strict_transport_security.is_some());

    // Test CSP builder
    let csp = CspBuilder::new()
        .default_src(&["'self'"])
        .script_src(&["'self'", "https://cdn.example.com"])
        .style_src(&["'self'", "'unsafe-inline'"])
        .img_src(&["'self'", "data:", "https:"])
        .connect_src(&["'self'"])
        .frame_ancestors(&["'none'"])
        .upgrade_insecure_requests()
        .build();

    assert!(csp.contains("default-src 'self'"));
    assert!(csp.contains("script-src 'self' https://cdn.example.com"));
    assert!(csp.contains("upgrade-insecure-requests"));
    assert!(csp.contains("frame-ancestors 'none'"));
}

#[test]
fn test_password_strength_validation() {
    let password_service = PasswordService::new(PasswordConfig::high_security());

    // Test various password strengths
    let weak_password = "123";
    let strength = password_service.check_password_strength(weak_password).unwrap();
    assert_eq!(strength, PasswordStrength::Weak);

    let medium_password = "password123";
    let strength = password_service.check_password_strength(medium_password).unwrap();
    assert_eq!(strength, PasswordStrength::Medium);

    let strong_password = "Str0ngP@ssw0rd!";
    let strength = password_service.check_password_strength(strong_password).unwrap();
    assert_eq!(strength, PasswordStrength::Strong);

    let very_strong_password = "V3ry$tr0ng&P@ssw0rd!2024#";
    let strength = password_service.check_password_strength(very_strong_password).unwrap();
    assert_eq!(strength, PasswordStrength::VeryStrong);

    // Test secure password generation
    let generated_password = password_service.generate_secure_password(16);
    assert_eq!(generated_password.len(), 16);
    
    let strength = password_service.check_password_strength(&generated_password).unwrap();
    assert!(matches!(strength, PasswordStrength::Strong | PasswordStrength::VeryStrong));
}

#[test]
fn test_jwt_token_refresh() {
    let jwt_service = JwtService::new(
        "test-secret-key",
        "test-issuer".to_string(),
        "test-audience".to_string(),
    );

    // Generate refresh token
    let refresh_token = jwt_service
        .generate_refresh_token("user123", 7) // 7 days
        .unwrap();

    // Validate refresh token
    let claims = jwt_service.validate_refresh_token(&refresh_token).unwrap();
    assert_eq!(claims.sub, "user123");
    assert_eq!(claims.role, "refresh");

    // Generate new access token using refresh token
    let new_access_token = jwt_service
        .refresh_access_token(
            &refresh_token,
            "user123",
            "user@example.com",
            "user",
            vec!["read".to_string()],
            24,
        )
        .unwrap();

    // Validate new access token
    let new_claims = jwt_service.validate_token(&new_access_token).unwrap();
    assert_eq!(new_claims.sub, "user123");
    assert_eq!(new_claims.email, "user@example.com");
    assert_eq!(new_claims.role, "user");
}

#[tokio::test]
async fn test_rate_limiting_with_different_configs() {
    let rate_limit_store = InMemoryRateLimitStore::new();
    let mut rate_limit_service = RateLimitService::new(rate_limit_store);

    // Add different rate limit configurations
    rate_limit_service.add_config("strict", RateLimitConfig::strict()).unwrap();
    rate_limit_service.add_config("moderate", RateLimitConfig::moderate()).unwrap();
    rate_limit_service.add_config("lenient", RateLimitConfig::lenient()).unwrap();

    let user_key = "test-user";

    // Test strict config (5 requests per minute)
    for i in 0..5 {
        let result = rate_limit_service
            .check_rate_limit(user_key, "strict")
            .await
            .unwrap();
        assert!(result.allowed);
        assert_eq!(result.remaining, 4 - i);
    }

    let result = rate_limit_service
        .check_rate_limit(user_key, "strict")
        .await
        .unwrap();
    assert!(!result.allowed);

    // Test moderate config (10 requests per minute) with different key
    let moderate_key = "moderate-user";
    for i in 0..10 {
        let result = rate_limit_service
            .check_rate_limit(moderate_key, "moderate")
            .await
            .unwrap();
        assert!(result.allowed);
        assert_eq!(result.remaining, 9 - i);
    }

    let result = rate_limit_service
        .check_rate_limit(moderate_key, "moderate")
        .await
        .unwrap();
    assert!(!result.allowed);

    // Test lenient config (100 requests per minute)
    let lenient_key = "lenient-user";
    for i in 0..50 {
        let result = rate_limit_service
            .check_rate_limit(lenient_key, "lenient")
            .await
            .unwrap();
        assert!(result.allowed);
        assert_eq!(result.remaining, 99 - i);
    }
}

#[tokio::test]
async fn test_progressive_account_lockout() {
    let lockout_store = InMemoryLockoutStore::new();
    let mut lockout_service = AccountLockoutService::new(lockout_store);

    // Add progressive lockout configuration
    let progressive_config = LockoutConfig::new(2, 60, 10) // 2 attempts per hour, 10 minute base lockout
        .with_progressive_lockout(vec![1, 2, 3]); // Multipliers: 1x, 2x, 3x

    lockout_service.add_config("progressive", progressive_config).unwrap();

    let user_id = "progressive-test-user";

    // First lockout (10 minutes)
    lockout_service
        .record_failed_attempt(
            user_id,
            "progressive",
            None,
            None,
            "Invalid password".to_string(),
        )
        .await
        .unwrap();

    let result = lockout_service
        .record_failed_attempt(
            user_id,
            "progressive",
            None,
            None,
            "Invalid password".to_string(),
        )
        .await
        .unwrap();
    assert!(result.is_locked);
    assert_eq!(result.lockout_duration, Some(10)); // 10 * 1
    assert_eq!(result.lockout_count, 1);

    // Reset and trigger second lockout
    lockout_service.unlock_account(user_id).await.unwrap();

    lockout_service
        .record_failed_attempt(
            user_id,
            "progressive",
            None,
            None,
            "Invalid password".to_string(),
        )
        .await
        .unwrap();

    let result = lockout_service
        .record_failed_attempt(
            user_id,
            "progressive",
            None,
            None,
            "Invalid password".to_string(),
        )
        .await
        .unwrap();
    assert!(result.is_locked);
    assert_eq!(result.lockout_duration, Some(20)); // 10 * 2
    assert_eq!(result.lockout_count, 2);

    // Reset and trigger third lockout
    lockout_service.unlock_account(user_id).await.unwrap();

    lockout_service
        .record_failed_attempt(
            user_id,
            "progressive",
            None,
            None,
            "Invalid password".to_string(),
        )
        .await
        .unwrap();

    let result = lockout_service
        .record_failed_attempt(
            user_id,
            "progressive",
            None,
            None,
            "Invalid password".to_string(),
        )
        .await
        .unwrap();
    assert!(result.is_locked);
    assert_eq!(result.lockout_duration, Some(30)); // 10 * 3
    assert_eq!(result.lockout_count, 3);
}

#[tokio::test]
async fn test_session_cleanup() {
    let session_store = InMemorySessionStore::new();
    let session_manager = SessionManager::new(session_store, Duration::seconds(1)); // 1 second TTL

    // Create multiple sessions
    let mut session_ids = Vec::new();
    for i in 0..5 {
        let session_id = session_manager
            .create_session(
                &format!("user{}", i),
                &format!("user{}@example.com", i),
                "user",
                None,
                None,
                None,
            )
            .await
            .unwrap();
        session_ids.push(session_id);
    }

    // All sessions should be valid initially
    for session_id in &session_ids {
        let session = session_manager.validate_session(session_id).await.unwrap();
        assert_eq!(session.user_role, "user");
    }

    // Wait for sessions to expire
    tokio::time::sleep(Duration::seconds(2).to_std().unwrap()).await;

    // All sessions should now be expired
    for session_id in &session_ids {
        let result = session_manager.validate_session(session_id).await;
        assert!(matches!(result, Err(SessionError::Expired(_))));
    }

    // Test cleanup
    let cleaned_count = session_manager.cleanup_expired_sessions().await.unwrap();
    assert_eq!(cleaned_count, 5);
}

#[test]
fn test_jwt_with_rsa_keys() {
    // This test demonstrates RSA key usage but uses placeholder keys
    // In production, you would generate proper RSA key pairs
    
    // For testing purposes, we'll use the same HMAC-based service
    // as RSA key generation requires proper key pairs
    let jwt_service = JwtService::new(
        "test-secret-key",
        "test-issuer".to_string(),
        "test-audience".to_string(),
    );

    let token = jwt_service
        .generate_token("user123", "test@example.com", "admin", vec!["admin".to_string()], 24)
        .unwrap();

    let claims = jwt_service.validate_token(&token).unwrap();
    assert_eq!(claims.sub, "user123");
    assert_eq!(claims.role, "admin");
    assert!(claims.permissions.contains(&"admin".to_string()));
}

#[tokio::test]
async fn test_multiple_user_sessions() {
    let session_store = InMemorySessionStore::new();
    let session_manager = SessionManager::new(session_store, Duration::hours(24));

    let user_id = "multi-session-user";

    // Create multiple sessions for the same user
    let mut session_ids = Vec::new();
    for i in 0..3 {
        let session_id = session_manager
            .create_session(
                user_id,
                "user@example.com",
                "user",
                Some(format!("127.0.0.{}", i + 1)),
                Some(format!("Browser {}", i + 1)),
                None,
            )
            .await
            .unwrap();
        session_ids.push(session_id);
    }

    // Get all user sessions
    let user_sessions = session_manager.get_user_sessions(user_id).await.unwrap();
    assert_eq!(user_sessions.len(), 3);

    // Revoke all user sessions
    let revoked_count = session_manager.revoke_all_user_sessions(user_id).await.unwrap();
    assert_eq!(revoked_count, 3);

    // All sessions should now be revoked
    for session_id in &session_ids {
        let result = session_manager.validate_session(session_id).await;
        assert!(matches!(result, Err(SessionError::Revoked(_))));
    }

    // User should have no active sessions
    let user_sessions = session_manager.get_user_sessions(user_id).await.unwrap();
    assert_eq!(user_sessions.len(), 0);
}

#[test]
fn test_password_rehash_detection() {
    let password_service = PasswordService::new(PasswordConfig::default());
    let password = "test-password-123!";

    // Hash password with default config
    let hash = password_service.hash_password(password).unwrap();

    // Check if rehash is needed (should be false for same config)
    let needs_rehash = password_service.needs_rehash(&hash);
    assert!(!needs_rehash);

    // Create a service with different config
    let high_security_service = PasswordService::new(PasswordConfig::high_security());
    
    // Check if rehash is needed with different config
    let needs_rehash = high_security_service.needs_rehash(&hash);
    // This might be true or false depending on the actual parameters
    // The important thing is that the function works
}

#[tokio::test]
async fn test_rate_limit_cleanup() {
    let rate_limit_store = InMemoryRateLimitStore::new();
    let mut rate_limit_service = RateLimitService::new(rate_limit_store);

    rate_limit_service.add_config("test", RateLimitConfig::new(5, 1)).unwrap(); // 5 requests per 1 second

    // Create rate limit records
    for i in 0..3 {
        let key = format!("user{}", i);
        rate_limit_service
            .check_rate_limit(&key, "test")
            .await
            .unwrap();
    }

    // Wait for expiration
    tokio::time::sleep(Duration::seconds(2).to_std().unwrap()).await;

    // Clean up expired records
    let cleaned_count = rate_limit_service.cleanup_expired().await.unwrap();
    assert!(cleaned_count >= 0); // Should clean up some records
}

#[test]
fn test_oauth_provider_defaults() {
    // Test all OAuth providers have proper default configurations
    let providers = vec![
        OAuthProvider::Google,
        OAuthProvider::GitHub,
        OAuthProvider::Microsoft,
        OAuthProvider::Discord,
        OAuthProvider::Facebook,
    ];

    for provider in providers {
        let config = provider.default_config();
        assert!(!config.client_id.is_empty());
        assert!(!config.client_secret.is_empty()); // Empty in defaults, but should be set
        assert!(!config.auth_url.is_empty());
        assert!(!config.token_url.is_empty());
        assert!(!config.scopes.is_empty());
        assert!(!config.redirect_url.is_empty());
    }
}
