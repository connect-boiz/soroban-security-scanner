//! Core rate limiting implementation

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;
use chrono::{DateTime, Utc};
use async_trait::async_trait;
use crate::rate_limiting::types::*;
use crate::rate_limiting::config::RateLimitConfig;
use crate::rate_limiting::storage::RateLimitStorage;
use tracing::{debug, warn, error};

/// Main rate limiter implementation
pub struct RateLimiter {
    config: RateLimitConfig,
    storage: Box<dyn RateLimitStorage<Error = Box<dyn std::error::Error + Send + Sync>>>,
    ip_reputation: Option<Box<dyn IpReputationProvider>>,
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration
    pub async fn new(
        config: RateLimitConfig,
        storage: Box<dyn RateLimitStorage<Error = Box<dyn std::error::Error + Send + Sync>>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let ip_reputation = if config.ip_restrictions.enable_reputation_check {
            Some(Box::new(DefaultIpReputationProvider::new(
                config.ip_restrictions.reputation_service.clone()
            )?))
        } else {
            None
        };

        Ok(Self {
            config,
            storage,
            ip_reputation,
        })
    }

    /// Check if a request should be allowed
    pub async fn check_rate_limit(&self, context: &RateLimitContext) -> RateLimitResult {
        if !self.config.enabled {
            return RateLimitResult::Allowed {
                remaining: u64::MAX,
                reset_time: Utc::now() + Duration::from_secs(3600),
                current_usage: 0,
            };
        }

        // Check IP restrictions first
        if let Some(block_result) = self.check_ip_restrictions(context).await {
            return block_result;
        }

        // Check geographic restrictions
        if let Some(block_result) = self.check_geo_restrictions(context).await {
            return block_result;
        }

        // Check IP reputation
        if let Some(block_result) = self.check_ip_reputation(context).await {
            return block_result;
        }

        // Get applicable policies
        let policies = self.get_applicable_policies(context);

        // Check each policy
        for policy in policies {
            let result = self.check_policy(context, policy).await;
            if !result.is_allowed() {
                return result;
            }
        }

        // All policies passed - request is allowed
        RateLimitResult::Allowed {
            remaining: 100, // This should be calculated from the most restrictive policy
            reset_time: Utc::now() + Duration::from_secs(60),
            current_usage: 1,
        }
    }

    /// Record a request (for tracking purposes)
    pub async fn record_request(&self, context: &RateLimitContext) -> Result<(), Box<dyn std::error::Error>> {
        // Record in storage for analytics
        let key = self.generate_storage_key(context, "requests");
        let window = Duration::from_secs(60); // 1-minute window for analytics
        
        self.storage.record_request(&key, window, context.timestamp).await?;
        Ok(())
    }

    /// Record a rate limit violation
    pub async fn record_violation(
        &self,
        context: &RateLimitContext,
        policy: &RateLimitPolicy,
        reason: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let violation = RateLimitViolation {
            id: uuid::Uuid::new_v4(),
            context: context.clone(),
            policy: policy.clone(),
            timestamp: Utc::now(),
            violation_type: ViolationType::RateLimitExceeded,
            severity: self.calculate_severity(context, policy),
        };

        self.storage.record_violation(&violation).await?;

        if self.config.monitoring.log_violations {
            warn!(
                "Rate limit violation: {} from {} ({}) - {}",
                context.resource,
                context.ip_address,
                context.user_id.map(|u| u.to_string()).unwrap_or_else(|| "anonymous".to_string()),
                reason
            );
        }

        Ok(())
    }

    /// Get rate limit statistics
    pub async fn get_stats(&self) -> Result<RateLimitStats, Box<dyn std::error::Error>> {
        self.storage.get_stats().await
    }

    /// Get violations for a user or IP
    pub async fn get_violations(
        &self,
        user_id: Option<uuid::Uuid>,
        ip_address: Option<IpAddr>,
        limit: Option<u64>,
    ) -> Result<Vec<RateLimitViolation>, Box<dyn std::error::Error>> {
        self.storage.get_violations(user_id, ip_address, limit).await
    }

    /// Check if the storage backend is healthy
    pub async fn health_check(&self) -> Result<bool, Box<dyn std::error::Error>> {
        self.storage.health_check().await
    }

    /// Clean up old data
    pub async fn cleanup(&self) -> Result<u64, Box<dyn std::error::Error>> {
        self.storage.cleanup(self.config.monitoring.violation_retention).await
    }

    // Private helper methods

    async fn check_ip_restrictions(&self, context: &RateLimitContext) -> Option<RateLimitResult> {
        // Check if IP is whitelisted
        if self.config.ip_restrictions.whitelisted_ips.contains(&context.ip_address) {
            debug!("IP {} is whitelisted, bypassing rate limits", context.ip_address);
            return None;
        }

        // Check if IP is blocked
        if self.config.ip_restrictions.blocked_ips.contains(&context.ip_address) {
            return Some(RateLimitResult::Blocked {
                reason: "IP address is blocked".to_string(),
                retry_after: Duration::from_secs(3600),
                current_usage: 0,
                max_requests: 0,
            });
        }

        // Check IP ranges (CIDR blocks)
        for range in &self.config.ip_restrictions.blocked_ranges {
            if let Ok(cidr) = range.parse::<ipnetwork::IpNetwork>() {
                if cidr.contains(context.ip_address) {
                    return Some(RateLimitResult::Blocked {
                        reason: format!("IP address is in blocked range: {}", range),
                        retry_after: Duration::from_secs(3600),
                        current_usage: 0,
                        max_requests: 0,
                    });
                }
            }
        }

        None
    }

    async fn check_geo_restrictions(&self, context: &RateLimitContext) -> Option<RateLimitResult> {
        if !self.config.geo_restrictions.enabled {
            return None;
        }

        let country = match &context.country {
            Some(c) => c.clone(),
            None => {
                // In a real implementation, we would use GeoIP lookup here
                return None;
            }
        };

        // Check blocked countries
        if self.config.geo_restrictions.blocked_countries.contains(&country) {
            return Some(RateLimitResult::Blocked {
                reason: format!("Access from country {} is not allowed", country),
                retry_after: Duration::from_secs(3600),
                current_usage: 0,
                max_requests: 0,
            });
        }

        // Check allowed countries (if specified)
        if let Some(allowed) = &self.config.geo_restrictions.allowed_countries {
            if !allowed.contains(&country) {
                return Some(RateLimitResult::Blocked {
                    reason: format!("Access from country {} is not allowed", country),
                    retry_after: Duration::from_secs(3600),
                    current_usage: 0,
                    max_requests: 0,
                });
            }
        }

        None
    }

    async fn check_ip_reputation(&self, context: &RateLimitContext) -> Option<RateLimitResult> {
        if let Some(reputation) = &self.ip_reputation {
            match reputation.check_reputation(context.ip_address).await {
                Ok(score) => {
                    let threshold = self.config.ip_restrictions.reputation_service
                        .as_ref()
                        .map(|s| s.block_threshold)
                        .unwrap_or(0.5);

                    if score < threshold {
                        return Some(RateLimitResult::Blocked {
                            reason: format!("Low IP reputation score: {}", score),
                            retry_after: Duration::from_secs(300), // 5 minutes
                            current_usage: 0,
                            max_requests: 0,
                        });
                    }
                }
                Err(e) => {
                    error!("Failed to check IP reputation: {}", e);
                    // Continue processing if reputation check fails
                }
            }
        }
        None
    }

    fn get_applicable_policies(&self, context: &RateLimitContext) -> Vec<&RateLimitPolicy> {
        // Check for endpoint-specific policies first
        if let Some(endpoint_config) = self.config.find_endpoint_config(&context.resource, &context.method) {
            if let Some(policies) = endpoint_config.policies.get(&context.tier) {
                return policies.iter().collect();
            }
        }

        // Fall back to default policies for the tier
        self.config.get_policies_for_tier(context.tier)
            .map(|policies| policies.iter().collect())
            .unwrap_or_default()
    }

    async fn check_policy(&self, context: &RateLimitContext, policy: &RateLimitPolicy) -> RateLimitResult {
        let key = self.generate_storage_key(context, &format!("policy_{:?}", policy.window));
        let window = policy.window.as_duration();

        match self.storage.get_request_count(&key, window).await {
            Ok(current_usage) => {
                if current_usage >= policy.max_requests {
                    let retry_after = self.calculate_retry_after(policy, context.timestamp);
                    
                    // Record violation
                    if let Err(e) = self.record_violation(context, policy, "Rate limit exceeded").await {
                        error!("Failed to record violation: {}", e);
                    }

                    RateLimitResult::Blocked {
                        reason: format!("Rate limit exceeded: {} requests per {:?}", 
                                       policy.max_requests, policy.window),
                        retry_after,
                        current_usage,
                        max_requests: policy.max_requests,
                    }
                } else {
                    // Record the request
                    if let Err(e) = self.storage.record_request(&key, window, context.timestamp).await {
                        error!("Failed to record request: {}", e);
                    }

                    let remaining = policy.max_requests.saturating_sub(current_usage + 1);
                    let reset_time = context.timestamp + window;

                    RateLimitResult::Allowed {
                        remaining,
                        reset_time,
                        current_usage: current_usage + 1,
                    }
                }
            }
            Err(e) => {
                error!("Failed to get request count: {}", e);
                // Fail open - allow the request if storage fails
                RateLimitResult::Allowed {
                    remaining: policy.max_requests,
                    reset_time: context.timestamp + window,
                    current_usage: 0,
                }
            }
        }
    }

    fn generate_storage_key(&self, context: &RateLimitContext, prefix: &str) -> String {
        match context.user_id {
            Some(user_id) => format!("user:{}:{}", user_id, prefix),
            None => format!("ip:{}:{}", context.ip_address, prefix),
        }
    }

    fn calculate_retry_after(&self, policy: &RateLimitPolicy, timestamp: DateTime<Utc>) -> Duration {
        if let Some(penalty) = policy.penalty_duration {
            penalty
        } else {
            // Default to the remaining time in the current window
            let window_start = timestamp.timestamp() % policy.window.as_seconds() as i64;
            let window_end = policy.window.as_seconds() as i64 - window_start;
            Duration::from_secs(window_end.max(0) as u64)
        }
    }

    fn calculate_severity(&self, context: &RateLimitContext, policy: &RateLimitPolicy) -> ViolationSeverity {
        // Calculate severity based on context and policy
        match context.tier {
            RateLimitTier::Admin => ViolationSeverity::Low,
            RateLimitTier::Enterprise => ViolationSeverity::Low,
            RateLimitTier::Premium => ViolationSeverity::Medium,
            RateLimitTier::Basic => ViolationSeverity::Medium,
            RateLimitTier::Unauthenticated => ViolationSeverity::High,
        }
    }
}

/// IP reputation provider trait
#[async_trait]
pub trait IpReputationProvider: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn check_reputation(&self, ip: IpAddr) -> Result<f64, Self::Error>;
}

/// Default IP reputation provider implementation
pub struct DefaultIpReputationProvider {
    config: Option<crate::rate_limiting::config::ReputationServiceConfig>,
    cache: std::sync::Arc<tokio::sync::RwLock<HashMap<IpAddr, (f64, DateTime<Utc>)>>>,
}

impl DefaultIpReputationProvider {
    pub fn new(
        config: Option<crate::rate_limiting::config::ReputationServiceConfig>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            config,
            cache: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        })
    }
}

#[async_trait]
impl IpReputationProvider for DefaultIpReputationProvider {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn check_reputation(&self, ip: IpAddr) -> Result<f64, Self::Error> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some((score, timestamp)) = cache.get(&ip) {
                if *timestamp > Utc::now() - chrono::Duration::minutes(5) {
                    return Ok(*score);
                }
            }
        }

        // In a real implementation, this would call an external reputation service
        // For now, return a neutral score
        let score = 0.8;

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(ip, (score, Utc::now()));
        }

        Ok(score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rate_limiting::storage::MemoryStorage;

    #[tokio::test]
    async fn test_basic_rate_limiting() {
        let config = RateLimitConfig::default();
        let storage = Box::new(MemoryStorage::new());
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
    async fn test_ip_whitelist() {
        let mut config = RateLimitConfig::default();
        config.ip_restrictions.whitelisted_ips.push("127.0.0.1".parse().unwrap());
        
        let storage = Box::new(MemoryStorage::new());
        let limiter = RateLimiter::new(config, storage).await.unwrap();

        let context = RateLimitContext::new(
            "127.0.0.1".parse().unwrap(),
            "/api/test".to_string(),
            "GET".to_string(),
        );

        // Whitelisted IP should bypass all limits
        let result = limiter.check_rate_limit(&context).await;
        assert!(result.is_allowed());
    }
}
