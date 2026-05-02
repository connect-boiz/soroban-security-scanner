//! Core types and structures for rate limiting

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Rate limit tier types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RateLimitTier {
    /// Unauthenticated users - lowest limits
    Unauthenticated,
    /// Basic authenticated users
    Basic,
    /// Premium users
    Premium,
    /// Enterprise users - highest limits
    Enterprise,
    /// Admin users - bypass most limits
    Admin,
}

impl RateLimitTier {
    pub fn from_role(role: &str) -> Self {
        match role.to_lowercase().as_str() {
            "admin" => RateLimitTier::Admin,
            "enterprise" => RateLimitTier::Enterprise,
            "premium" => RateLimitTier::Premium,
            "basic" => RateLimitTier::Basic,
            _ => RateLimitTier::Unauthenticated,
        }
    }
}

/// Rate limit window types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RateLimitWindow {
    /// Per second limits
    Second,
    /// Per minute limits
    Minute,
    /// Per hour limits
    Hour,
    /// Per day limits
    Day,
    /// Per week limits
    Week,
    /// Per month limits
    Month,
}

impl RateLimitWindow {
    pub fn as_duration(&self) -> Duration {
        match self {
            RateLimitWindow::Second => Duration::from_secs(1),
            RateLimitWindow::Minute => Duration::from_secs(60),
            RateLimitWindow::Hour => Duration::from_secs(3600),
            RateLimitWindow::Day => Duration::from_secs(86400),
            RateLimitWindow::Week => Duration::from_secs(604800),
            RateLimitWindow::Month => Duration::from_secs(2592000), // 30 days
        }
    }

    pub fn as_seconds(&self) -> u64 {
        match self {
            RateLimitWindow::Second => 1,
            RateLimitWindow::Minute => 60,
            RateLimitWindow::Hour => 3600,
            RateLimitWindow::Day => 86400,
            RateLimitWindow::Week => 604800,
            RateLimitWindow::Month => 2592000,
        }
    }
}

/// Rate limit policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitPolicy {
    /// Maximum requests allowed in the window
    pub max_requests: u64,
    /// Time window for the limit
    pub window: RateLimitWindow,
    /// Burst capacity (allows short bursts above the average rate)
    pub burst_capacity: Option<u64>,
    /// Penalty duration when limit is exceeded
    pub penalty_duration: Option<Duration>,
}

impl RateLimitPolicy {
    pub fn new(max_requests: u64, window: RateLimitWindow) -> Self {
        Self {
            max_requests,
            window,
            burst_capacity: None,
            penalty_duration: None,
        }
    }

    pub fn with_burst_capacity(mut self, capacity: u64) -> Self {
        self.burst_capacity = Some(capacity);
        self
    }

    pub fn with_penalty(mut self, duration: Duration) -> Self {
        self.penalty_duration = Some(duration);
        self
    }
}

/// Rate limit request context
#[derive(Debug, Clone)]
pub struct RateLimitContext {
    /// User ID if authenticated
    pub user_id: Option<Uuid>,
    /// User tier
    pub tier: RateLimitTier,
    /// Client IP address
    pub ip_address: IpAddr,
    /// API endpoint or resource being accessed
    pub resource: String,
    /// HTTP method
    pub method: String,
    /// User agent string
    pub user_agent: Option<String>,
    /// Geographic location (if available)
    pub country: Option<String>,
    /// API key if used
    pub api_key: Option<String>,
    /// Request timestamp
    pub timestamp: DateTime<Utc>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl RateLimitContext {
    pub fn new(ip_address: IpAddr, resource: String, method: String) -> Self {
        Self {
            user_id: None,
            tier: RateLimitTier::Unauthenticated,
            ip_address,
            resource,
            method,
            user_agent: None,
            country: None,
            api_key: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_user_id(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_tier(mut self, tier: RateLimitTier) -> Self {
        self.tier = tier;
        self
    }

    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    pub fn with_country(mut self, country: String) -> Self {
        self.country = Some(country);
        self
    }

    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Rate limit result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitResult {
    /// Request is allowed
    Allowed {
        /// Remaining requests in current window
        remaining: u64,
        /// Time until window resets
        reset_time: DateTime<Utc>,
        /// Current usage count
        current_usage: u64,
    },
    /// Request is blocked due to rate limit
    Blocked {
        /// Reason for blocking
        reason: String,
        /// Time until limit resets
        retry_after: Duration,
        /// Current usage count
        current_usage: u64,
        /// Maximum allowed requests
        max_requests: u64,
    },
}

impl RateLimitResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, RateLimitResult::Allowed { .. })
    }

    pub fn remaining(&self) -> Option<u64> {
        match self {
            RateLimitResult::Allowed { remaining, .. } => Some(*remaining),
            _ => None,
        }
    }

    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            RateLimitResult::Blocked { retry_after, .. } => Some(*retry_after),
            _ => None,
        }
    }
}

/// Rate limit violation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitViolation {
    /// Unique violation ID
    pub id: Uuid,
    /// Context of the violation
    pub context: RateLimitContext,
    /// Policy that was violated
    pub policy: RateLimitPolicy,
    /// When the violation occurred
    pub timestamp: DateTime<Utc>,
    /// Violation type
    pub violation_type: ViolationType,
    /// Severity of the violation
    pub severity: ViolationSeverity,
}

/// Types of rate limit violations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationType {
    /// Standard rate limit exceeded
    RateLimitExceeded,
    /// Burst capacity exceeded
    BurstCapacityExceeded,
    /// Concurrent request limit exceeded
    ConcurrentLimitExceeded,
    /// Suspicious activity detected
    SuspiciousActivity,
    /// Geographic restriction violated
    GeographicRestriction,
    /// API key limit exceeded
    ApiKeyLimitExceeded,
}

/// Violation severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Rate limit statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStats {
    /// Total requests processed
    pub total_requests: u64,
    /// Requests allowed
    pub allowed_requests: u64,
    /// Requests blocked
    pub blocked_requests: u64,
    /// Current active users
    pub active_users: u64,
    /// Current active IPs
    pub active_ips: u64,
    /// Violations by type
    pub violations_by_type: HashMap<ViolationType, u64>,
    /// Top violators by IP
    pub top_violators_by_ip: HashMap<IpAddr, u64>,
    /// Top violators by user
    pub top_violators_by_user: HashMap<Uuid, u64>,
    /// Statistics timestamp
    pub timestamp: DateTime<Utc>,
}

impl Default for RateLimitStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            allowed_requests: 0,
            blocked_requests: 0,
            active_users: 0,
            active_ips: 0,
            violations_by_type: HashMap::new(),
            top_violators_by_ip: HashMap::new(),
            top_violators_by_user: HashMap::new(),
            timestamp: Utc::now(),
        }
    }
}

/// Rate limit configuration for different endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointRateLimit {
    /// Endpoint path pattern (supports wildcards)
    pub path_pattern: String,
    /// HTTP methods this applies to
    pub methods: Vec<String>,
    /// Rate limit policies by tier
    pub policies: HashMap<RateLimitTier, RateLimitPolicy>,
    /// Whether this endpoint requires authentication
    pub requires_auth: bool,
    /// Custom priority for this endpoint
    pub priority: Option<u8>,
}

impl EndpointRateLimit {
    pub fn new(path_pattern: String) -> Self {
        Self {
            path_pattern,
            methods: vec!["*".to_string()], // All methods by default
            policies: HashMap::new(),
            requires_auth: false,
            priority: None,
        }
    }

    pub fn with_methods(mut self, methods: Vec<String>) -> Self {
        self.methods = methods;
        self
    }

    pub fn with_policy(mut self, tier: RateLimitTier, policy: RateLimitPolicy) -> Self {
        self.policies.insert(tier, policy);
        self
    }

    pub fn requires_auth(mut self, requires: bool) -> Self {
        self.requires_auth = requires;
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = Some(priority);
        self
    }
}
