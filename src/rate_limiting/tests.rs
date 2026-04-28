//! Comprehensive tests for the rate limiting system

use std::net::IpAddr;
use std::time::Duration;
use chrono::{Utc, DateTime};
use uuid::Uuid;
use crate::rate_limiting::*;

/// Test basic rate limiting functionality
#[tokio::test]
async fn test_basic_rate_limiting() {
    let config = RateLimitConfig::default();
    let storage = Box::new(storage::MemoryStorage::new());
    let limiter = RateLimiter::new(config, storage).await.unwrap();

    let context = RateLimitContext::new(
        "127.0.0.1".parse().unwrap(),
        "/api/test".to_string(),
        "GET".to_string(),
    );

    // First request should be allowed
    let result = limiter.check_rate_limit(&context).await;
    assert!(result.is_allowed());

    // Record the request
    limiter.record_request(&context).await.unwrap();
}

#[tokio::test]
async fn test_rate_limit_exceeded() {
    let mut config = RateLimitConfig::default();
    
    // Set very restrictive limits for testing
    config.default_policies.insert(
        RateLimitTier::Unauthenticated,
        vec![RateLimitPolicy::new(2, RateLimitWindow::Minute)],
    );

    let storage = Box::new(storage::MemoryStorage::new());
    let limiter = RateLimiter::new(config, storage).await.unwrap();

    let context = RateLimitContext::new(
        "127.0.0.1".parse().unwrap(),
        "/api/test".to_string(),
        "GET".to_string(),
    );

    // First two requests should be allowed
    for i in 0..2 {
        let result = limiter.check_rate_limit(&context).await;
        assert!(result.is_allowed(), "Request {} should be allowed", i + 1);
        limiter.record_request(&context).await.unwrap();
    }

    // Third request should be blocked
    let result = limiter.check_rate_limit(&context).await;
    assert!(!result.is_allowed(), "Third request should be blocked");

    if let RateLimitResult::Blocked { reason, .. } = result {
        assert!(reason.contains("Rate limit exceeded"));
    }
}

#[tokio::test]
async fn test_user_tier_rate_limits() {
    let config = RateLimitConfig::default();
    let storage = Box::new(storage::MemoryStorage::new());
    let limiter = RateLimiter::new(config, storage).await.unwrap();

    let tiers = vec![
        (RateLimitTier::Unauthenticated, 10),
        (RateLimitTier::Basic, 60),
        (RateLimitTier::Premium, 300),
        (RateLimitTier::Enterprise, 1000),
    ];

    for (tier, expected_min_requests) in tiers {
        let user_id = Uuid::new_v4();
        let context = RateLimitContext::new(
            "127.0.0.1".parse().unwrap(),
            "/api/test".to_string(),
            "GET".to_string(),
        )
        .with_user_id(user_id)
        .with_tier(tier);

        // Should be able to make at least the expected minimum requests
        for i in 0..expected_min_requests {
            let result = limiter.check_rate_limit(&context).await;
            assert!(result.is_allowed(), 
                "Tier {:?} should allow at least {} requests, failed at {}", 
                tier, expected_min_requests, i + 1);
        }
    }
}

#[tokio::test]
async fn test_ip_whitelist() {
    let mut config = RateLimitConfig::default();
    config.ip_restrictions.whitelisted_ips.push("127.0.0.1".parse().unwrap());
    
    let storage = Box::new(storage::MemoryStorage::new());
    let limiter = RateLimiter::new(config, storage).await.unwrap();

    let context = RateLimitContext::new(
        "127.0.0.1".parse().unwrap(),
        "/api/test".to_string(),
        "GET".to_string(),
    );

    // Whitelisted IP should bypass all limits
    for i in 0..1000 {
        let result = limiter.check_rate_limit(&context).await;
        assert!(result.is_allowed(), "Whitelisted IP request {} should be allowed", i + 1);
    }
}

#[tokio::test]
async fn test_ip_blocklist() {
    let mut config = RateLimitConfig::default();
    config.ip_restrictions.blocked_ips.push("192.168.1.100".parse().unwrap());
    
    let storage = Box::new(storage::MemoryStorage::new());
    let limiter = RateLimiter::new(config, storage).await.unwrap();

    let context = RateLimitContext::new(
        "192.168.1.100".parse().unwrap(),
        "/api/test".to_string(),
        "GET".to_string(),
    );

    // Blocked IP should not be allowed any requests
    let result = limiter.check_rate_limit(&context).await;
    assert!(!result.is_allowed());

    if let RateLimitResult::Blocked { reason, .. } = result {
        assert!(reason.contains("blocked"));
    }
}

#[tokio::test]
async fn test_ip_range_blocking() {
    let mut config = RateLimitConfig::default();
    config.ip_restrictions.blocked_ranges.push("192.168.1.0/24".to_string());
    
    let storage = Box::new(storage::MemoryStorage::new());
    let limiter = RateLimiter::new(config, storage).await.unwrap();

    let test_ips = vec![
        "192.168.1.50",
        "192.168.1.100",
        "192.168.1.200",
    ];

    for ip in test_ips {
        let context = RateLimitContext::new(
            ip.parse().unwrap(),
            "/api/test".to_string(),
            "GET".to_string(),
        );

        let result = limiter.check_rate_limit(&context).await;
        assert!(!result.is_allowed(), "IP {} should be blocked", ip);
    }

    // IP outside the range should be allowed
    let context = RateLimitContext::new(
        "10.0.0.1".parse().unwrap(),
        "/api/test".to_string(),
        "GET".to_string(),
    );

    let result = limiter.check_rate_limit(&context).await;
    assert!(result.is_allowed());
}

#[tokio::test]
async fn test_endpoint_specific_limits() {
    let mut config = RateLimitConfig::default();
    
    // Add restrictive endpoint-specific policy
    let scan_endpoint = EndpointRateLimit::new("/api/scan".to_string())
        .with_policy(
            RateLimitTier::Unauthenticated,
            RateLimitPolicy::new(1, RateLimitWindow::Minute)
        );

    config = config.add_endpoint(scan_endpoint);

    let storage = Box::new(storage::MemoryStorage::new());
    let limiter = RateLimiter::new(config, storage).await.unwrap();

    // Test scan endpoint (should be limited to 1 request per minute)
    let scan_context = RateLimitContext::new(
        "127.0.0.1".parse().unwrap(),
        "/api/scan".to_string(),
        "POST".to_string(),
    );

    let result1 = limiter.check_rate_limit(&scan_context).await;
    assert!(result1.is_allowed());

    let result2 = limiter.check_rate_limit(&scan_context).await;
    assert!(!result2.is_allowed());

    // Test other endpoint (should use default limits)
    let status_context = RateLimitContext::new(
        "127.0.0.1".parse().unwrap(),
        "/api/status".to_string(),
        "GET".to_string(),
    );

    for i in 0..5 {
        let result = limiter.check_rate_limit(&status_context).await;
        assert!(result.is_allowed(), "Status request {} should be allowed", i + 1);
    }
}

#[tokio::test]
async fn test_sliding_window() {
    let mut config = RateLimitConfig::default();
    config.default_policies.insert(
        RateLimitTier::Unauthenticated,
        vec![RateLimitPolicy::new(3, RateLimitWindow::Second)],
    );

    let storage = Box::new(storage::MemoryStorage::new());
    let limiter = RateLimiter::new(config, storage).await.unwrap();

    let context = RateLimitContext::new(
        "127.0.0.1".parse().unwrap(),
        "/api/test".to_string(),
        "GET".to_string(),
    );

    let start_time = Utc::now();

    // Make 3 requests quickly (should all be allowed)
    for i in 0..3 {
        let result = limiter.check_rate_limit(&context).await;
        assert!(result.is_allowed(), "Request {} should be allowed", i + 1);
    }

    // Next request should be blocked
    let result = limiter.check_rate_limit(&context).await;
    assert!(!result.is_allowed());

    // Wait for window to slide
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Should be able to make requests again
    let result = limiter.check_rate_limit(&context).await;
    assert!(result.is_allowed());
}

#[tokio::test]
async fn test_violation_recording() {
    let config = RateLimitConfig::default();
    let storage = Box::new(storage::MemoryStorage::new());
    let limiter = RateLimiter::new(config, storage).await.unwrap();

    let user_id = Uuid::new_v4();
    let context = RateLimitContext::new(
        "127.0.0.1".parse().unwrap(),
        "/api/test".to_string(),
        "GET".to_string(),
    )
    .with_user_id(user_id);

    let policy = RateLimitPolicy::new(1, RateLimitWindow::Minute);

    // Record a violation
    limiter.record_violation(&context, &policy, "Test violation").await.unwrap();

    // Check that violation was recorded
    let violations = limiter.get_violations(Some(user_id), None, Some(10)).await.unwrap();
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].context.user_id, Some(user_id));
    assert_eq!(violations[0].violation_type, ViolationType::RateLimitExceeded);
}

#[tokio::test]
async fn test_statistics() {
    let config = RateLimitConfig::default();
    let storage = Box::new(storage::MemoryStorage::new());
    let limiter = RateLimiter::new(config, storage).await.unwrap();

    let context = RateLimitContext::new(
        "127.0.0.1".parse().unwrap(),
        "/api/test".to_string(),
        "GET".to_string(),
    );

    // Generate some traffic
    for i in 0..10 {
        let result = limiter.check_rate_limit(&context).await;
        if result.is_allowed() {
            limiter.record_request(&context).await.unwrap();
        }

        // Record some violations
        if i >= 7 {
            let policy = RateLimitPolicy::new(5, RateLimitWindow::Minute);
            limiter.record_violation(&context, &policy, "Test violation").await.unwrap();
        }
    }

    // Check statistics
    let stats = limiter.get_stats().await.unwrap();
    assert!(stats.total_requests > 0);
    assert!(stats.allowed_requests > 0);
    assert!(stats.blocked_requests > 0);
}

#[tokio::test]
async fn test_concurrent_requests() {
    let config = RateLimitConfig::default();
    let storage = Box::new(storage::MemoryStorage::new());
    let limiter = RateLimiter::new(config, storage).await.unwrap();

    let mut handles = vec![];

    // Spawn multiple concurrent requests
    for i in 0..20 {
        let limiter_clone = limiter.clone();
        let handle = tokio::spawn(async move {
            let context = RateLimitContext::new(
                "127.0.0.1".parse().unwrap(),
                "/api/test".to_string(),
                "GET".to_string(),
            );

            limiter_clone.check_rate_limit(&context).await
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    let mut allowed_count = 0;
    let mut blocked_count = 0;

    for handle in handles {
        let result = handle.await.unwrap();
        if result.is_allowed() {
            allowed_count += 1;
        } else {
            blocked_count += 1;
        }
    }

    // Should have some allowed and some blocked requests
    assert!(allowed_count > 0, "Should have some allowed requests");
    assert!(blocked_count > 0, "Should have some blocked requests");
}

#[tokio::test]
async fn test_health_check() {
    let config = RateLimitConfig::default();
    let storage = Box::new(storage::MemoryStorage::new());
    let limiter = RateLimiter::new(config, storage).await.unwrap();

    // Health check should pass
    let is_healthy = limiter.health_check().await.unwrap();
    assert!(is_healthy);
}

#[tokio::test]
async fn test_cleanup() {
    let config = RateLimitConfig::default();
    let storage = Box::new(storage::MemoryStorage::new());
    let limiter = RateLimiter::new(config, storage).await.unwrap();

    let context = RateLimitContext::new(
        "127.0.0.1".parse().unwrap(),
        "/api/test".to_string(),
        "GET".to_string(),
    );

    // Record some violations
    let policy = RateLimitPolicy::new(1, RateLimitWindow::Minute);
    for _ in 0..5 {
        limiter.record_violation(&context, &policy, "Test violation").await.unwrap();
    }

    // Run cleanup
    let cleaned_count = limiter.cleanup().await.unwrap();
    
    // Cleanup should run without errors
    assert!(cleaned_count >= 0);
}

#[tokio::test]
async fn test_rate_limit_policies() {
    let policy = RateLimitPolicy::new(100, RateLimitWindow::Minute)
        .with_burst_capacity(150)
        .with_penalty(Duration::from_secs(300));

    assert_eq!(policy.max_requests, 100);
    assert_eq!(policy.window, RateLimitWindow::Minute);
    assert_eq!(policy.burst_capacity, Some(150));
    assert_eq!(policy.penalty_duration, Some(Duration::from_secs(300)));
}

#[tokio::test]
async fn test_rate_limit_context() {
    let user_id = Uuid::new_v4();
    let context = RateLimitContext::new(
        "192.168.1.100".parse().unwrap(),
        "/api/scan".to_string(),
        "POST".to_string(),
    )
    .with_user_id(user_id)
    .with_tier(RateLimitTier::Premium)
    .with_user_agent("test-agent/1.0".to_string())
    .with_country("US".to_string())
    .with_api_key("test-key-123".to_string())
    .with_metadata("custom-field".to_string(), "custom-value".to_string());

    assert_eq!(context.user_id, Some(user_id));
    assert_eq!(context.tier, RateLimitTier::Premium);
    assert_eq!(context.ip_address, "192.168.1.100".parse::<IpAddr>().unwrap());
    assert_eq!(context.resource, "/api/scan");
    assert_eq!(context.method, "POST");
    assert_eq!(context.user_agent, Some("test-agent/1.0".to_string()));
    assert_eq!(context.country, Some("US".to_string()));
    assert_eq!(context.api_key, Some("test-key-123".to_string()));
    assert_eq!(context.metadata.get("custom-field"), Some(&"custom-value".to_string()));
}

#[tokio::test]
async fn test_rate_limit_tier_conversion() {
    assert_eq!(RateLimitTier::from_role("admin"), RateLimitTier::Admin);
    assert_eq!(RateLimitTier::from_role("enterprise"), RateLimitTier::Enterprise);
    assert_eq!(RateLimitTier::from_role("premium"), RateLimitTier::Premium);
    assert_eq!(RateLimitTier::from_role("basic"), RateLimitTier::Basic);
    assert_eq!(RateLimitTier::from_role("unknown"), RateLimitTier::Unauthenticated);
    assert_eq!(RateLimitTier::from_role("ADMIN"), RateLimitTier::Admin); // Case insensitive
}

#[tokio::test]
async fn test_rate_limit_window() {
    assert_eq!(RateLimitWindow::Second.as_duration(), Duration::from_secs(1));
    assert_eq!(RateLimitWindow::Minute.as_duration(), Duration::from_secs(60));
    assert_eq!(RateLimitWindow::Hour.as_duration(), Duration::from_secs(3600));
    assert_eq!(RateLimitWindow::Day.as_duration(), Duration::from_secs(86400));
    assert_eq!(RateLimitWindow::Week.as_duration(), Duration::from_secs(604800));
    assert_eq!(RateLimitWindow::Month.as_duration(), Duration::from_secs(2592000));
}

#[cfg(feature = "redis-cache")]
#[tokio::test]
async fn test_redis_storage() {
    use crate::rate_limiting::storage::RedisStorage;
    use crate::rate_limiting::config::DistributedConfig;

    // Skip test if Redis is not available
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    
    let mut distributed_config = DistributedConfig::default();
    distributed_config.redis.url = redis_url;
    distributed_config.enabled = true;

    // Try to create Redis storage
    match RedisStorage::new(distributed_config) {
        Ok(redis_storage) => {
            let storage = Box::new(redis_storage);
            let config = RateLimitConfig::default();
            let limiter = RateLimiter::new(config, storage).await.unwrap();

            // Test basic functionality
            let context = RateLimitContext::new(
                "127.0.0.1".parse().unwrap(),
                "/api/test".to_string(),
                "GET".to_string(),
            );

            let result = limiter.check_rate_limit(&context).await;
            assert!(result.is_allowed());

            // Test health check
            let is_healthy = limiter.health_check().await.unwrap();
            assert!(is_healthy);
        }
        Err(_) => {
            // Redis not available, skip test
            println!("Skipping Redis test - Redis not available");
        }
    }
}

// Benchmark test for performance
#[tokio::test]
async fn benchmark_rate_limiting() {
    let config = RateLimitConfig::default();
    let storage = Box::new(storage::MemoryStorage::new());
    let limiter = RateLimiter::new(config, storage).await.unwrap();

    let start_time = std::time::Instant::now();
    let num_requests = 1000;

    for i in 0..num_requests {
        let context = RateLimitContext::new(
            format!("127.0.0.{}", (i % 255) + 1).parse().unwrap(),
            "/api/test".to_string(),
            "GET".to_string(),
        );

        let _result = limiter.check_rate_limit(&context).await;
    }

    let duration = start_time.elapsed();
    let requests_per_second = num_requests as f64 / duration.as_secs_f64();

    println!("Benchmark: {} requests in {:?} ({:.2} req/s)", 
             num_requests, duration, requests_per_second);

    // Should be able to handle at least 1000 requests per second
    assert!(requests_per_second > 1000.0, 
            "Rate limiting should handle at least 1000 req/s, got {:.2}", 
            requests_per_second);
}
