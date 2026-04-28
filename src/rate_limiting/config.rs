//! Rate limiting configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;
use crate::rate_limiting::types::*;

/// Main rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Global enabled flag
    pub enabled: bool,
    /// Default policies for each tier
    pub default_policies: HashMap<RateLimitTier, Vec<RateLimitPolicy>>,
    /// Endpoint-specific configurations
    pub endpoints: Vec<EndpointRateLimit>,
    /// IP-based restrictions
    pub ip_restrictions: IpRestrictions,
    /// Geographic restrictions
    pub geo_restrictions: GeoRestrictions,
    /// Distributed configuration
    pub distributed: DistributedConfig,
    /// Monitoring and alerting
    pub monitoring: MonitoringConfig,
    /// Cache configuration
    pub cache: CacheConfig,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        let mut default_policies = HashMap::new();
        
        // Unauthenticated users - very restrictive
        default_policies.insert(
            RateLimitTier::Unauthenticated,
            vec![
                RateLimitPolicy::new(10, RateLimitWindow::Minute),
                RateLimitPolicy::new(100, RateLimitWindow::Hour),
                RateLimitPolicy::new(1000, RateLimitWindow::Day),
            ],
        );

        // Basic users - moderate limits
        default_policies.insert(
            RateLimitTier::Basic,
            vec![
                RateLimitPolicy::new(60, RateLimitWindow::Minute),
                RateLimitPolicy::new(1000, RateLimitWindow::Hour),
                RateLimitPolicy::new(10000, RateLimitWindow::Day),
            ],
        );

        // Premium users - generous limits
        default_policies.insert(
            RateLimitTier::Premium,
            vec![
                RateLimitPolicy::new(300, RateLimitWindow::Minute),
                RateLimitPolicy::new(5000, RateLimitWindow::Hour),
                RateLimitPolicy::new(50000, RateLimitWindow::Day),
            ],
        );

        // Enterprise users - very generous limits
        default_policies.insert(
            RateLimitTier::Enterprise,
            vec![
                RateLimitPolicy::new(1000, RateLimitWindow::Minute),
                RateLimitPolicy::new(20000, RateLimitWindow::Hour),
                RateLimitPolicy::new(200000, RateLimitWindow::Day),
            ],
        );

        // Admin users - minimal restrictions
        default_policies.insert(
            RateLimitTier::Admin,
            vec![
                RateLimitPolicy::new(5000, RateLimitWindow::Minute),
                RateLimitPolicy::new(100000, RateLimitWindow::Hour),
                RateLimitPolicy::new(1000000, RateLimitWindow::Day),
            ],
        );

        Self {
            enabled: true,
            default_policies,
            endpoints: Vec::new(),
            ip_restrictions: IpRestrictions::default(),
            geo_restrictions: GeoRestrictions::default(),
            distributed: DistributedConfig::default(),
            monitoring: MonitoringConfig::default(),
            cache: CacheConfig::default(),
        }
    }
}

/// IP-based restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpRestrictions {
    /// Blocked IP addresses
    pub blocked_ips: Vec<IpAddr>,
    /// Blocked IP ranges (CIDR notation)
    pub blocked_ranges: Vec<String>,
    /// Whitelisted IP addresses (bypass all limits)
    pub whitelisted_ips: Vec<IpAddr>,
    /// Trusted proxy IPs
    pub trusted_proxies: Vec<IpAddr>,
    /// Maximum concurrent connections per IP
    pub max_concurrent_per_ip: u32,
    /// Rate limit for unknown/new IPs
    pub unknown_ip_policy: Option<RateLimitPolicy>,
    /// Enable IP reputation checking
    pub enable_reputation_check: bool,
    /// Reputation service configuration
    pub reputation_service: Option<ReputationServiceConfig>,
}

impl Default for IpRestrictions {
    fn default() -> Self {
        Self {
            blocked_ips: Vec::new(),
            blocked_ranges: Vec::new(),
            whitelisted_ips: Vec::new(),
            trusted_proxies: Vec::new(),
            max_concurrent_per_ip: 10,
            unknown_ip_policy: Some(RateLimitPolicy::new(5, RateLimitWindow::Minute)),
            enable_reputation_check: false,
            reputation_service: None,
        }
    }
}

/// Reputation service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationServiceConfig {
    /// Service provider (e.g., "ipqualityscore", "maxmind")
    pub provider: String,
    /// API key
    pub api_key: String,
    /// Score threshold for blocking
    pub block_threshold: f64,
    /// Request timeout
    pub timeout: Duration,
    /// Cache duration for reputation results
    pub cache_duration: Duration,
}

/// Geographic restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoRestrictions {
    /// Enable geographic filtering
    pub enabled: bool,
    /// Blocked countries (ISO country codes)
    pub blocked_countries: Vec<String>,
    /// Allowed countries (if set, only these are allowed)
    pub allowed_countries: Option<Vec<String>>,
    /// Rate limit adjustments by country
    pub country_multipliers: HashMap<String, f64>,
    /// VPN/proxy detection
    pub block_vpn_proxy: bool,
    /// Geographic database path
    pub geoip_database_path: Option<String>,
}

impl Default for GeoRestrictions {
    fn default() -> Self {
        Self {
            enabled: false,
            blocked_countries: Vec::new(),
            allowed_countries: None,
            country_multipliers: HashMap::new(),
            block_vpn_proxy: false,
            geoip_database_path: None,
        }
    }
}

/// Distributed rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedConfig {
    /// Enable distributed rate limiting
    pub enabled: bool,
    /// Redis connection configuration
    pub redis: RedisConfig,
    /// Synchronization settings
    pub sync: SyncConfig,
    /// Failover behavior
    pub failover: FailoverConfig,
}

impl Default for DistributedConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            redis: RedisConfig::default(),
            sync: SyncConfig::default(),
            failover: FailoverConfig::default(),
        }
    }
}

/// Redis configuration for distributed rate limiting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URL
    pub url: String,
    /// Connection pool size
    pub pool_size: u32,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Command timeout
    pub command_timeout: Duration,
    /// Key prefix for rate limiting data
    pub key_prefix: String,
    /// TLS configuration
    pub tls: Option<TlsConfig>,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            pool_size: 10,
            connection_timeout: Duration::from_secs(5),
            command_timeout: Duration::from_secs(1),
            key_prefix: "rate_limit:".to_string(),
            tls: None,
        }
    }
}

/// TLS configuration for Redis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Enable TLS
    pub enabled: bool,
    /// CA certificate path
    pub ca_cert_path: Option<String>,
    /// Client certificate path
    pub client_cert_path: Option<String>,
    /// Client private key path
    pub client_key_path: Option<String>,
    /// Verify server certificate
    pub verify_server: bool,
}

/// Synchronization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Sync interval for local caches
    pub sync_interval: Duration,
    /// Maximum sync delay tolerance
    pub max_sync_delay: Duration,
    /// Enable clock synchronization
    pub enable_clock_sync: bool,
    /// Clock drift tolerance
    pub clock_drift_tolerance: Duration,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            sync_interval: Duration::from_secs(5),
            max_sync_delay: Duration::from_millis(100),
            enable_clock_sync: true,
            clock_drift_tolerance: Duration::from_millis(50),
        }
    }
}

/// Failover configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverConfig {
    /// Enable failover to local cache
    pub enable_local_fallback: bool,
    /// Local cache duration during failover
    pub fallback_cache_duration: Duration,
    /// Maximum failover duration
    pub max_failover_duration: Duration,
    /// Alert on failover
    pub alert_on_failover: bool,
}

impl Default for FailoverConfig {
    fn default() -> Self {
        Self {
            enable_local_fallback: true,
            fallback_cache_duration: Duration::from_secs(300), // 5 minutes
            max_failover_duration: Duration::from_secs(3600), // 1 hour
            alert_on_failover: true,
        }
    }
}

/// Monitoring and alerting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable monitoring
    pub enabled: bool,
    /// Metrics collection interval
    pub metrics_interval: Duration,
    /// Alert thresholds
    pub alert_thresholds: AlertThresholds,
    /// Export metrics to Prometheus
    pub prometheus: Option<PrometheusConfig>,
    /// Log violations
    pub log_violations: bool,
    /// Violation retention period
    pub violation_retention: Duration,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            metrics_interval: Duration::from_secs(60),
            alert_thresholds: AlertThresholds::default(),
            prometheus: None,
            log_violations: true,
            violation_retention: Duration::from_secs(86400 * 7), // 7 days
        }
    }
}

/// Alert thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// Alert when violation rate exceeds this percentage
    pub violation_rate_threshold: f64,
    /// Alert when blocked requests exceed this count per minute
    pub blocked_requests_per_minute: u64,
    /// Alert when active users exceed this count
    pub active_users_threshold: u64,
    /// Alert when suspicious activity is detected
    pub suspicious_activity_threshold: u64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            violation_rate_threshold: 0.1, // 10%
            blocked_requests_per_minute: 100,
            active_users_threshold: 10000,
            suspicious_activity_threshold: 50,
        }
    }
}

/// Prometheus metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusConfig {
    /// Metrics endpoint path
    pub endpoint_path: String,
    /// Metrics port
    pub port: u16,
    /// Include labels in metrics
    pub include_labels: bool,
    /// Custom metric names
    pub custom_metric_names: HashMap<String, String>,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Local cache size (number of entries)
    pub local_cache_size: usize,
    /// Local cache TTL
    pub local_cache_ttl: Duration,
    /// Enable cache compression
    pub enable_compression: bool,
    /// Cache cleanup interval
    pub cleanup_interval: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            local_cache_size: 10000,
            local_cache_ttl: Duration::from_secs(300), // 5 minutes
            enable_compression: false,
            cleanup_interval: Duration::from_secs(60),
        }
    }
}

impl RateLimitConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Add endpoint-specific rate limit
    pub fn add_endpoint(mut self, endpoint: EndpointRateLimit) -> Self {
        self.endpoints.push(endpoint);
        self
    }

    /// Get rate limit policies for a specific tier
    pub fn get_policies_for_tier(&self, tier: RateLimitTier) -> Option<&Vec<RateLimitPolicy>> {
        self.default_policies.get(&tier)
    }

    /// Find endpoint configuration for a given path and method
    pub fn find_endpoint_config(&self, path: &str, method: &str) -> Option<&EndpointRateLimit> {
        self.endpoints.iter().find(|endpoint| {
            // Check if method matches
            let method_matches = endpoint.methods.contains(&"*".to_string()) 
                || endpoint.methods.contains(&method.to_string());
            
            if !method_matches {
                return false;
            }

            // Simple pattern matching - can be enhanced with regex
            if endpoint.path_pattern.contains('*') {
                let pattern = endpoint.path_pattern.replace('*', "");
                path.contains(&pattern)
            } else {
                path == endpoint.path_pattern
            }
        })
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        // Check if default policies exist for all tiers
        for tier in [
            RateLimitTier::Unauthenticated,
            RateLimitTier::Basic,
            RateLimitTier::Premium,
            RateLimitTier::Enterprise,
            RateLimitTier::Admin,
        ] {
            if !self.default_policies.contains_key(&tier) {
                return Err(format!("Missing default policy for tier: {:?}", tier));
            }
        }

        // Validate distributed config if enabled
        if self.distributed.enabled {
            if self.distributed.redis.url.is_empty() {
                return Err("Redis URL is required when distributed rate limiting is enabled".to_string());
            }
        }

        Ok(())
    }
}
