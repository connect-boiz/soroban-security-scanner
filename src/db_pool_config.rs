//! Database connection pool configuration and security hardening.
//!
//! Provides a typed `PoolConfig` builder that enforces:
//! - Connection pool size limits (min/max connections)
//! - SSL/TLS mode requirements
//! - Statement, idle, and connection lifetime timeouts
//! - Retry policy with exponential backoff
//!
//! Designed to be passed to `sqlx::postgres::PgPoolOptions`.

use serde::{Deserialize, Serialize};
use std::time::Duration;

// ---------------------------------------------------------------------------
// SSL mode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SslMode {
    /// Refuse non-SSL connections. Required for production.
    Require,
    /// Verify server certificate (full mTLS).
    VerifyFull,
    /// Allow non-SSL (development only).
    Disable,
}

// ---------------------------------------------------------------------------
// Pool configuration
// ---------------------------------------------------------------------------

/// Validated database connection pool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Minimum idle connections to maintain.
    pub min_connections:      u32,
    /// Maximum total connections in the pool.
    pub max_connections:      u32,
    /// Maximum time to wait for an available connection.
    pub acquire_timeout:      Duration,
    /// Maximum idle time before a connection is closed.
    pub idle_timeout:         Duration,
    /// Maximum total lifetime of any connection.
    pub max_lifetime:         Duration,
    /// Maximum time a single statement may run (PostgreSQL).
    pub statement_timeout:    Duration,
    /// SSL/TLS mode.
    pub ssl_mode:             SslMode,
    /// Number of reconnect attempts on failure.
    pub reconnect_attempts:   u32,
    /// Initial backoff for reconnect attempts.
    pub reconnect_backoff:    Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self::production()
    }
}

impl PoolConfig {
    /// Hardened production configuration.
    pub fn production() -> Self {
        Self {
            min_connections:   5,
            max_connections:   20,
            acquire_timeout:   Duration::from_secs(5),
            idle_timeout:      Duration::from_secs(300),
            max_lifetime:      Duration::from_secs(1800),
            statement_timeout: Duration::from_secs(30),
            ssl_mode:          SslMode::Require,
            reconnect_attempts: 3,
            reconnect_backoff:  Duration::from_millis(500),
        }
    }

    /// Relaxed development configuration (no SSL).
    pub fn development() -> Self {
        Self {
            min_connections:   1,
            max_connections:   5,
            acquire_timeout:   Duration::from_secs(10),
            idle_timeout:      Duration::from_secs(600),
            max_lifetime:      Duration::from_secs(3600),
            statement_timeout: Duration::from_secs(60),
            ssl_mode:          SslMode::Disable,
            reconnect_attempts: 5,
            reconnect_backoff:  Duration::from_millis(200),
        }
    }

    /// Validate configuration invariants.
    pub fn validate(&self) -> Result<(), PoolConfigError> {
        if self.min_connections > self.max_connections {
            return Err(PoolConfigError::MinExceedsMax {
                min: self.min_connections,
                max: self.max_connections,
            });
        }
        if self.max_connections == 0 {
            return Err(PoolConfigError::ZeroMaxConnections);
        }
        if self.acquire_timeout.is_zero() {
            return Err(PoolConfigError::ZeroTimeout("acquire_timeout"));
        }
        Ok(())
    }

    /// Build the sqlx connection pool options string fragment for logging.
    /// Does not include the connection URL (which may contain credentials).
    pub fn describe(&self) -> String {
        format!(
            "pool={{min={} max={} ssl={:?} stmt_timeout={}s}}",
            self.min_connections,
            self.max_connections,
            self.ssl_mode,
            self.statement_timeout.as_secs()
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PoolConfigError {
    #[error("min_connections ({min}) must not exceed max_connections ({max})")]
    MinExceedsMax { min: u32, max: u32 },
    #[error("max_connections must be greater than zero")]
    ZeroMaxConnections,
    #[error("{0} must not be zero")]
    ZeroTimeout(&'static str),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_config_validates() {
        assert!(PoolConfig::production().validate().is_ok());
    }

    #[test]
    fn development_config_validates() {
        assert!(PoolConfig::development().validate().is_ok());
    }

    #[test]
    fn min_exceeds_max_fails() {
        let mut cfg = PoolConfig::production();
        cfg.min_connections = 50;
        cfg.max_connections = 10;
        assert!(matches!(cfg.validate(), Err(PoolConfigError::MinExceedsMax { .. })));
    }

    #[test]
    fn zero_max_connections_fails() {
        let mut cfg = PoolConfig::production();
        cfg.max_connections = 0;
        assert!(matches!(cfg.validate(), Err(PoolConfigError::ZeroMaxConnections)));
    }

    #[test]
    fn production_requires_ssl() {
        assert_eq!(PoolConfig::production().ssl_mode, SslMode::Require);
    }

    #[test]
    fn describe_does_not_contain_password() {
        let d = PoolConfig::production().describe();
        assert!(!d.contains("password"));
        assert!(!d.contains("secret"));
    }
}
