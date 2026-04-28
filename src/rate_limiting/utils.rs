//! Utility functions for rate limiting

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;
use chrono::{DateTime, Utc};
use crate::rate_limiting::types::*;

/// Utility functions for IP address handling
pub mod ip_utils {
    use super::*;

    /// Extract IP address from various headers
    pub fn extract_ip_from_headers(headers: &HashMap<String, String>) -> Option<IpAddr> {
        // Try X-Forwarded-For header
        if let Some(xff) = headers.get("x-forwarded-for") {
            // X-Forwarded-For can contain multiple IPs, take the first one
            let first_ip = xff.split(',').next()?.trim();
            if let Ok(ip) = first_ip.parse() {
                return Some(ip);
            }
        }

        // Try X-Real-IP header
        if let Some(x_real_ip) = headers.get("x-real-ip") {
            if let Ok(ip) = x_real_ip.parse() {
                return Some(ip);
            }
        }

        // Try CF-Connecting-IP header (Cloudflare)
        if let Some(cf_ip) = headers.get("cf-connecting-ip") {
            if let Ok(ip) = cf_ip.parse() {
                return Some(ip);
            }
        }

        // Try X-Client-IP header
        if let Some(client_ip) = headers.get("x-client-ip") {
            if let Ok(ip) = client_ip.parse() {
                return Some(ip);
            }
        }

        None
    }

    /// Check if an IP address is private/internal
    pub fn is_private_ip(ip: IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => is_private_ipv4(ipv4),
            IpAddr::V6(ipv6) => is_private_ipv6(ipv6),
        }
    }

    /// Check if an IPv4 address is private
    fn is_private_ipv4(ip: Ipv4Addr) -> bool {
        ip.is_private() || ip.is_loopback() || ip.is_link_local()
    }

    /// Check if an IPv6 address is private
    fn is_private_ipv6(ip: Ipv6Addr) -> bool {
        ip.is_loopback() || ip.is_unspecified()
    }

    /// Normalize IP address for consistent storage
    pub fn normalize_ip(ip: IpAddr) -> IpAddr {
        match ip {
            IpAddr::V4(ipv4) => IpAddr::V4(ipv4),
            IpAddr::V6(ipv6) => {
                // For IPv6, we might want to normalize certain addresses
                if let Some(ipv4) = ipv6.to_ipv4() {
                    // Convert IPv4-mapped IPv6 addresses to IPv4
                    IpAddr::V4(ipv4)
                } else {
                    IpAddr::V6(ipv6)
                }
            }
        }
    }

    /// Generate a hash of an IP address for privacy-conscious storage
    pub fn hash_ip(ip: IpAddr, salt: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        ip.hash(&mut hasher);
        salt.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

/// Utility functions for key generation
pub mod key_utils {
    use super::*;

    /// Generate a consistent storage key for rate limiting
    pub fn generate_storage_key(
        identifier: &str,
        resource: &str,
        window: RateLimitWindow,
        tier: RateLimitTier,
    ) -> String {
        format!("{}:{}:{}:{:?}", identifier, resource, window.as_seconds(), tier)
    }

    /// Generate a user-specific key
    pub fn generate_user_key(user_id: &str, resource: &str, window: RateLimitWindow) -> String {
        format!("user:{}:{}:{}", user_id, resource, window.as_seconds())
    }

    /// Generate an IP-specific key
    pub fn generate_ip_key(ip: &str, resource: &str, window: RateLimitWindow) -> String {
        format!("ip:{}:{}:{}", ip, resource, window.as_seconds())
    }

    /// Generate a global key for aggregate statistics
    pub fn generate_global_key(resource: &str, window: RateLimitWindow) -> String {
        format!("global:{}:{}", resource, window.as_seconds())
    }
}

/// Utility functions for time calculations
pub mod time_utils {
    use super::*;

    /// Calculate the start of the current time window
    pub fn window_start(timestamp: DateTime<Utc>, window: RateLimitWindow) -> DateTime<Utc> {
        let seconds = timestamp.timestamp();
        let window_seconds = window.as_seconds() as i64;
        let window_start = (seconds / window_seconds) * window_seconds;
        DateTime::from_timestamp(window_start, 0).unwrap_or(timestamp)
    }

    /// Calculate the end of the current time window
    pub fn window_end(timestamp: DateTime<Utc>, window: RateLimitWindow) -> DateTime<Utc> {
        window_start(timestamp, window) + chrono::Duration::from_std(window.as_duration()).unwrap()
    }

    /// Calculate remaining time in the current window
    pub fn remaining_time(timestamp: DateTime<Utc>, window: RateLimitWindow) -> Duration {
        let end = window_end(timestamp, window);
        let remaining = end.signed_duration_since(timestamp);
        Duration::from_secs(remaining.num_seconds().max(0) as u64)
    }

    /// Calculate how many windows have passed between two timestamps
    pub fn windows_passed(
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        window: RateLimitWindow,
    ) -> u64 {
        let duration = end.signed_duration_since(start);
        let window_duration = window.as_duration();
        (duration.num_milliseconds() / window_duration.as_millis() as i64).max(0) as u64
    }
}

/// Utility functions for statistics and monitoring
pub mod stats_utils {
    use super::*;

    /// Calculate rate limit utilization percentage
    pub fn calculate_utilization(current: u64, maximum: u64) -> f64 {
        if maximum == 0 {
            0.0
        } else {
            (current as f64 / maximum as f64) * 100.0
        }
    }

    /// Calculate requests per second
    pub fn calculate_rps(count: u64, duration: Duration) -> f64 {
        let seconds = duration.as_secs_f64();
        if seconds > 0.0 {
            count as f64 / seconds
        } else {
            0.0
        }
    }

    /// Calculate exponential moving average
    pub fn calculate_ema(current: f64, previous_ema: f64, alpha: f64) -> f64 {
        alpha * current + (1.0 - alpha) * previous_ema
    }

    /// Detect anomalies in request patterns
    pub fn detect_anomaly(requests: &[u64], threshold: f64) -> bool {
        if requests.len() < 2 {
            return false;
        }

        let mean = requests.iter().sum::<u64>() as f64 / requests.len() as f64;
        let variance = requests.iter()
            .map(|&x| (x as f64 - mean).powi(2))
            .sum::<f64>() / (requests.len() - 1) as f64;
        let std_dev = variance.sqrt();

        // Check if the latest request count is significantly higher than average
        let latest = requests[requests.len() - 1] as f64;
        (latest - mean) / std_dev > threshold
    }

    /// Calculate percentiles
    pub fn calculate_percentiles(values: &[u64]) -> HashMap<String, u64> {
        let mut sorted_values = values.to_vec();
        sorted_values.sort_unstable();

        let mut percentiles = HashMap::new();
        
        percentiles.insert("p50".to_string(), percentile(&sorted_values, 50.0));
        percentiles.insert("p90".to_string(), percentile(&sorted_values, 90.0));
        percentiles.insert("p95".to_string(), percentile(&sorted_values, 95.0));
        percentiles.insert("p99".to_string(), percentile(&sorted_values, 99.0));
        
        percentiles
    }

    /// Calculate a specific percentile
    fn percentile(sorted_values: &[u64], percentile: f64) -> u64 {
        if sorted_values.is_empty() {
            return 0;
        }

        let index = ((percentile / 100.0) * (sorted_values.len() - 1) as f64).round() as usize;
        sorted_values[index.min(sorted_values.len() - 1)]
    }
}

/// Utility functions for configuration validation
pub mod config_utils {
    use super::*;
    use crate::rate_limiting::config::RateLimitConfig;

    /// Validate rate limit configuration
    pub fn validate_config(config: &RateLimitConfig) -> Result<(), String> {
        // Check if any policies have zero max_requests
        for (tier, policies) in &config.default_policies {
            for policy in policies {
                if policy.max_requests == 0 {
                    return Err(format!("Policy for tier {:?} has zero max_requests", tier));
                }
            }
        }

        // Check if Redis configuration is valid when distributed is enabled
        if config.distributed.enabled {
            if config.distributed.redis.url.is_empty() {
                return Err("Redis URL is required when distributed rate limiting is enabled".to_string());
            }
            
            if config.distributed.redis.pool_size == 0 {
                return Err("Redis pool size must be greater than 0".to_string());
            }
        }

        // Validate IP restrictions
        for range in &config.ip_restrictions.blocked_ranges {
            if range.parse::<ipnetwork::IpNetwork>().is_err() {
                return Err(format!("Invalid IP range: {}", range));
            }
        }

        Ok(())
    }

    /// Get default configuration for different environments
    pub fn get_default_config_for_env(env: &str) -> RateLimitConfig {
        match env.to_lowercase().as_str() {
            "development" => RateLimitConfig {
                enabled: true,
                default_policies: {
                    let mut policies = HashMap::new();
                    policies.insert(
                        RateLimitTier::Unauthenticated,
                        vec![RateLimitPolicy::new(1000, RateLimitWindow::Minute)],
                    );
                    policies.insert(
                        RateLimitTier::Basic,
                        vec![RateLimitPolicy::new(5000, RateLimitWindow::Minute)],
                    );
                    policies
                },
                ..Default::default()
            },
            "staging" => RateLimitConfig {
                enabled: true,
                default_policies: {
                    let mut policies = HashMap::new();
                    policies.insert(
                        RateLimitTier::Unauthenticated,
                        vec![RateLimitPolicy::new(100, RateLimitWindow::Minute)],
                    );
                    policies.insert(
                        RateLimitTier::Basic,
                        vec![RateLimitPolicy::new(500, RateLimitWindow::Minute)],
                    );
                    policies
                },
                ..Default::default()
            },
            "production" => RateLimitConfig::default(),
            _ => RateLimitConfig::default(),
        }
    }
}

/// Utility functions for caching
pub mod cache_utils {
    use super::*;

    /// Generate cache key for rate limit data
    pub fn generate_cache_key(prefix: &str, identifier: &str, window: RateLimitWindow) -> String {
        format!("{}:{}:{}", prefix, identifier, window.as_seconds())
    }

    /// Calculate cache TTL based on window size
    pub fn calculate_cache_ttl(window: RateLimitWindow) -> Duration {
        match window {
            RateLimitWindow::Second => Duration::from_secs(5),
            RateLimitWindow::Minute => Duration::from_secs(60),
            RateLimitWindow::Hour => Duration::from_secs(300),
            RateLimitWindow::Day => Duration::from_secs(3600),
            RateLimitWindow::Week => Duration::from_secs(21600), // 6 hours
            RateLimitWindow::Month => Duration::from_secs(86400), // 24 hours
        }
    }

    /// Check if cache entry is still valid
    pub fn is_cache_valid(timestamp: DateTime<Utc>, ttl: Duration) -> bool {
        let now = Utc::now();
        let age = now.signed_duration_since(timestamp);
        age.to_std().unwrap_or(Duration::MAX) < ttl
    }
}

/// Utility functions for testing
#[cfg(test)]
pub mod test_utils {
    use super::*;

    /// Create a test rate limit context
    pub fn create_test_context(
        ip: &str,
        resource: &str,
        method: &str,
        tier: RateLimitTier,
    ) -> RateLimitContext {
        RateLimitContext::new(
            ip.parse().unwrap(),
            resource.to_string(),
            method.to_string(),
        )
        .with_tier(tier)
    }

    /// Create a test rate limit policy
    pub fn create_test_policy(
        max_requests: u64,
        window: RateLimitWindow,
    ) -> RateLimitPolicy {
        RateLimitPolicy::new(max_requests, window)
    }

    /// Generate test request timestamps
    pub fn generate_test_timestamps(
        start: DateTime<Utc>,
        count: usize,
        interval: Duration,
    ) -> Vec<DateTime<Utc>> {
        (0..count)
            .map(|i| start + chrono::Duration::from_std(interval * i as u32).unwrap())
            .collect()
    }

    /// Create a mock storage for testing
    pub fn create_mock_storage() -> crate::rate_limiting::storage::MemoryStorage {
        crate::rate_limiting::storage::MemoryStorage::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_extraction() {
        let mut headers = HashMap::new();
        headers.insert("x-forwarded-for".to_string(), "192.168.1.1, 10.0.0.1".to_string());
        
        let ip = ip_utils::extract_ip_from_headers(&headers);
        assert_eq!(ip, Some("192.168.1.1".parse().unwrap()));
    }

    #[test]
    fn test_private_ip_detection() {
        assert!(ip_utils::is_private_ip("192.168.1.1".parse().unwrap()));
        assert!(ip_utils::is_private_ip("127.0.0.1".parse().unwrap()));
        assert!(!ip_utils::is_private_ip("8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn test_window_calculations() {
        let timestamp = Utc::now();
        let window = RateLimitWindow::Minute;
        
        let start = time_utils::window_start(timestamp, window);
        let end = time_utils::window_end(timestamp, window);
        let remaining = time_utils::remaining_time(timestamp, window);
        
        assert!(end > start);
        assert!(remaining <= Duration::from_secs(60));
    }

    #[test]
    fn test_utilization_calculation() {
        assert_eq!(stats_utils::calculate_utilization(50, 100), 50.0);
        assert_eq!(stats_utils::calculate_utilization(100, 100), 100.0);
        assert_eq!(stats_utils::calculate_utilization(0, 100), 0.0);
    }

    #[test]
    fn test_percentiles() {
        let values = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let percentiles = stats_utils::calculate_percentiles(&values);
        
        assert_eq!(percentiles.get("p50"), Some(&5));
        assert_eq!(percentiles.get("p90"), Some(&9));
    }

    #[test]
    fn test_config_validation() {
        let config = RateLimitConfig::default();
        assert!(config_utils::validate_config(&config).is_ok());
    }
}
