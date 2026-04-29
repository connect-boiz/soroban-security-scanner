//! Examples demonstrating rate limiting usage

use std::net::IpAddr;
use std::time::Duration;
use chrono::Utc;
use uuid::Uuid;
use crate::rate_limiting::*;

/// Example 1: Basic rate limiting setup
pub async fn basic_rate_limiting_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Basic Rate Limiting Example ===");

    // Create configuration
    let config = RateLimitConfig::default();
    
    // Create storage (in-memory for this example)
    let storage = Box::new(storage::MemoryStorage::new());
    
    // Create rate limiter
    let limiter = RateLimiter::new(config, storage).await?;

    // Create a request context
    let context = RateLimitContext::new(
        "192.168.1.100".parse().unwrap(),
        "/api/scan".to_string(),
        "POST".to_string(),
    );

    // Check rate limit
    let result = limiter.check_rate_limit(&context).await;
    
    match result {
        RateLimitResult::Allowed { remaining, reset_time, .. } => {
            println!("✅ Request allowed!");
            println!("   Remaining requests: {}", remaining);
            println!("   Reset time: {}", reset_time);
        }
        RateLimitResult::Blocked { reason, retry_after, .. } => {
            println!("🚫 Request blocked: {}", reason);
            println!("   Retry after: {} seconds", retry_after.as_secs());
        }
    }

    Ok(())
}

/// Example 2: User tier-based rate limiting
pub async fn tiered_rate_limiting_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Tiered Rate Limiting Example ===");

    let config = RateLimitConfig::default();
    let storage = Box::new(storage::MemoryStorage::new());
    let limiter = RateLimiter::new(config, storage).await?;

    // Test different user tiers
    let tiers = vec![
        (RateLimitTier::Unauthenticated, "anonymous@example.com"),
        (RateLimitTier::Basic, "basic@example.com"),
        (RateLimitTier::Premium, "premium@example.com"),
        (RateLimitTier::Enterprise, "enterprise@example.com"),
    ];

    for (tier, email) in tiers {
        let user_id = Uuid::new_v4();
        let context = RateLimitContext::new(
            "192.168.1.100".parse().unwrap(),
            "/api/scan".to_string(),
            "POST".to_string(),
        )
        .with_user_id(user_id)
        .with_tier(tier)
        .with_metadata("email".to_string(), email.to_string());

        let result = limiter.check_rate_limit(&context).await;
        
        println!("👤 {:?} user ({}): {:?}", tier, email, 
            if result.is_allowed() { "✅ Allowed" } else { "🚫 Blocked" });
    }

    Ok(())
}

/// Example 3: IP-based restrictions
pub async fn ip_restrictions_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== IP Restrictions Example ===");

    let mut config = RateLimitConfig::default();
    
    // Add some IP restrictions
    config.ip_restrictions.blocked_ips.push("192.168.1.50".parse().unwrap());
    config.ip_restrictions.whitelisted_ips.push("192.168.1.200".parse().unwrap());
    config.ip_restrictions.blocked_ranges.push("10.0.0.0/8".to_string());

    let storage = Box::new(storage::MemoryStorage::new());
    let limiter = RateLimiter::new(config, storage).await?;

    let test_ips = vec![
        ("192.168.1.50", "Blocked IP"),
        ("192.168.1.200", "Whitelisted IP"),
        ("10.0.0.5", "Blocked range"),
        ("8.8.8.8", "Normal IP"),
    ];

    for (ip, description) in test_ips {
        let context = RateLimitContext::new(
            ip.parse().unwrap(),
            "/api/scan".to_string(),
            "POST".to_string(),
        );

        let result = limiter.check_rate_limit(&context).await;
        
        println!("🌐 {} ({}): {:?}", ip, description,
            if result.is_allowed() { "✅ Allowed" } else { "🚫 Blocked" });
    }

    Ok(())
}

/// Example 4: Endpoint-specific rate limiting
pub async fn endpoint_specific_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Endpoint-Specific Rate Limiting Example ===");

    let mut config = RateLimitConfig::default();
    
    // Add endpoint-specific policies
    let scan_endpoint = EndpointRateLimit::new("/api/scan".to_string())
        .with_methods(vec!["POST".to_string()])
        .with_policy(
            RateLimitTier::Basic,
            RateLimitPolicy::new(10, RateLimitWindow::Minute)
        )
        .requires_auth(true);

    let status_endpoint = EndpointRateLimit::new("/api/status".to_string())
        .with_policy(
            RateLimitTier::Unauthenticated,
            RateLimitPolicy::new(100, RateLimitWindow::Minute)
        );

    config = config
        .add_endpoint(scan_endpoint)
        .add_endpoint(status_endpoint);

    let storage = Box::new(storage::MemoryStorage::new());
    let limiter = RateLimiter::new(config, storage).await?;

    let endpoints = vec![
        ("/api/scan", "POST", "Scanning endpoint"),
        ("/api/status", "GET", "Status endpoint"),
    ];

    for (endpoint, method, description) in endpoints {
        let context = RateLimitContext::new(
            "192.168.1.100".parse().unwrap(),
            endpoint.to_string(),
            method.to_string(),
        );

        let result = limiter.check_rate_limit(&context).await;
        
        println!("🔗 {} ({}): {:?}", description, endpoint,
            if result.is_allowed() { "✅ Allowed" } else { "🚫 Blocked" });
    }

    Ok(())
}

/// Example 5: Distributed rate limiting with Redis
#[cfg(feature = "redis-cache")]
pub async fn distributed_rate_limiting_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Distributed Rate Limiting Example ===");

    let mut config = RateLimitConfig::default();
    config.distributed.enabled = true;
    config.distributed.redis.url = "redis://localhost:6379".to_string();

    let storage = storage::StorageFactory::create_storage(&config).await?;
    let limiter = RateLimiter::new(config, storage).await?;

    // Simulate requests from multiple instances
    let user_id = Uuid::new_v4();
    for i in 0..15 {
        let context = RateLimitContext::new(
            "192.168.1.100".parse().unwrap(),
            "/api/scan".to_string(),
            "POST".to_string(),
        )
        .with_user_id(user_id)
        .with_tier(RateLimitTier::Basic)
        .with_metadata("instance".to_string(), format!("instance-{}", i % 3));

        let result = limiter.check_rate_limit(&context).await;
        
        println!("🔄 Request {}: {:?}", i + 1,
            if result.is_allowed() { "✅ Allowed" } else { "🚫 Blocked" });

        if let RateLimitResult::Allowed { remaining, .. } = result {
            if remaining == 0 {
                break;
            }
        }
    }

    Ok(())
}

/// Example 6: Rate limiting statistics and monitoring
pub async fn monitoring_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Rate Limiting Monitoring Example ===");

    let config = RateLimitConfig::default();
    let storage = Box::new(storage::MemoryStorage::new());
    let limiter = RateLimiter::new(config, storage).await?;

    // Generate some traffic
    let user_id = Uuid::new_v4();
    for i in 0..50 {
        let context = RateLimitContext::new(
            "192.168.1.100".parse().unwrap(),
            "/api/scan".to_string(),
            "POST".to_string(),
        )
        .with_user_id(user_id)
        .with_tier(RateLimitTier::Basic);

        let result = limiter.check_rate_limit(&context).await;
        
        // Record the request
        limiter.record_request(&context).await?;

        // Intentionally violate rate limits for some requests
        if i > 40 {
            let policy = RateLimitPolicy::new(10, RateLimitWindow::Minute);
            limiter.record_violation(&context, &policy, "Test violation").await?;
        }
    }

    // Get statistics
    let stats = limiter.get_stats().await?;
    
    println!("📊 Rate Limiting Statistics:");
    println!("   Total requests: {}", stats.total_requests);
    println!("   Allowed requests: {}", stats.allowed_requests);
    println!("   Blocked requests: {}", stats.blocked_requests);
    println!("   Active users: {}", stats.active_users);
    println!("   Active IPs: {}", stats.active_ips);

    // Get violations
    let violations = limiter.get_violations(Some(user_id), None, Some(10)).await?;
    println!("   Recent violations: {}", violations.len());

    Ok(())
}

/// Example 7: Custom middleware integration
pub async fn middleware_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Middleware Integration Example ===");

    let config = RateLimitConfig::default();
    let storage = Box::new(storage::MemoryStorage::new());
    let limiter = RateLimiter::new(config, storage).await?;

    // Simulate HTTP request processing
    let mut requests = vec![
        MockRequest {
            method: "GET".to_string(),
            path: "/api/status".to_string(),
            ip: "192.168.1.100".parse().unwrap(),
            user_id: None,
            user_tier: None,
        },
        MockRequest {
            method: "POST".to_string(),
            path: "/api/scan".to_string(),
            ip: "192.168.1.100".parse().unwrap(),
            user_id: Some(Uuid::new_v4()),
            user_tier: Some("premium".to_string()),
        },
    ];

    for (i, request) in requests.iter().enumerate() {
        println!("\n🌐 Processing request {}:", i + 1);
        println!("   Method: {}", request.method);
        println!("   Path: {}", request.path);
        println!("   IP: {}", request.ip);

        // Create context from request
        let mut context = RateLimitContext::new(
            request.ip,
            request.path.clone(),
            request.method.clone(),
        );

        if let Some(user_id) = request.user_id {
            context = context.with_user_id(user_id);
        }

        if let Some(ref tier) = request.user_tier {
            context = context.with_tier(RateLimitTier::from_role(tier));
        }

        // Check rate limit
        let result = limiter.check_rate_limit(&context).await;
        
        match result {
            RateLimitResult::Allowed { remaining, .. } => {
                println!("   ✅ Request allowed ({} remaining)", remaining);
            }
            RateLimitResult::Blocked { reason, retry_after, .. } => {
                println!("   🚫 Request blocked: {}", reason);
                println!("   ⏰ Retry after: {} seconds", retry_after.as_secs());
            }
        }
    }

    Ok(())
}

// Mock request structure for middleware example
#[derive(Debug)]
struct MockRequest {
    method: String,
    path: String,
    ip: IpAddr,
    user_id: Option<Uuid>,
    user_tier: Option<String>,
}

/// Run all examples
pub async fn run_all_examples() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Rate Limiting Examples\n");

    basic_rate_limiting_example().await?;
    tiered_rate_limiting_example().await?;
    ip_restrictions_example().await?;
    endpoint_specific_example().await?;
    monitoring_example().await?;
    middleware_example().await?;

    #[cfg(feature = "redis-cache")]
    {
        distributed_rate_limiting_example().await?;
    }

    println!("\n✅ All examples completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_example() {
        basic_rate_limiting_example().await.unwrap();
    }

    #[tokio::test]
    async fn test_tiered_example() {
        tiered_rate_limiting_example().await.unwrap();
    }

    #[tokio::test]
    async fn test_ip_restrictions_example() {
        ip_restrictions_example().await.unwrap();
    }

    #[tokio::test]
    async fn test_endpoint_specific_example() {
        endpoint_specific_example().await.unwrap();
    }

    #[tokio::test]
    async fn test_monitoring_example() {
        monitoring_example().await.unwrap();
    }

    #[tokio::test]
    async fn test_middleware_example() {
        middleware_example().await.unwrap();
    }
}
