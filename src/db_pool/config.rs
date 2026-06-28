//! Database configuration: pool sizing plus the composed security posture.

use crate::db_pool::hardening::HardeningConfig;
use crate::db_pool::retry::BackoffPolicy;
use crate::db_pool::ssl::{TlsConfig, TlsConfigError};
use serde::{Deserialize, Serialize};

/// Connection-pool sizing and validation settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Minimum (pre-warmed) connections.
    pub min_connections: usize,
    /// Maximum concurrent connections.
    pub max_connections: usize,
    /// Idle connections older than this (seconds) are closed.
    pub idle_timeout_secs: i64,
    /// Connections checked out longer than this (seconds) are treated as leaks.
    pub leak_timeout_secs: i64,
    /// Validation query used to detect stale connections.
    pub validation_query: String,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 5,
            max_connections: 100,
            idle_timeout_secs: 30,
            leak_timeout_secs: 300,
            validation_query: "SELECT 1".to_string(),
        }
    }
}

/// Why a database configuration is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbConfigError {
    /// `min_connections` exceeds `max_connections`.
    MinExceedsMax,
    /// `max_connections` is zero.
    ZeroMax,
    /// The TLS configuration is invalid.
    Tls(TlsConfigError),
}

/// Top-level database configuration: pool, TLS, hardening and retry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbConfig {
    /// Connection-pool settings.
    pub pool: PoolConfig,
    /// TLS/SSL settings.
    pub tls: TlsConfig,
    /// Server/session hardening settings.
    pub hardening: HardeningConfig,
    /// Connection retry backoff.
    pub backoff: BackoffPolicy,
    /// Slow-query threshold in milliseconds.
    pub slow_query_ms: u64,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            pool: PoolConfig::default(),
            tls: TlsConfig::default(),
            hardening: HardeningConfig::default(),
            backoff: BackoffPolicy::default(),
            slow_query_ms: 1000,
        }
    }
}

impl DbConfig {
    /// Validates the whole configuration for internal consistency.
    pub fn validate(&self) -> Result<(), DbConfigError> {
        if self.pool.max_connections == 0 {
            return Err(DbConfigError::ZeroMax);
        }
        if self.pool.min_connections > self.pool.max_connections {
            return Err(DbConfigError::MinExceedsMax);
        }
        self.tls.validate().map_err(DbConfigError::Tls)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_pool::ssl::SslMode;

    #[test]
    fn defaults_match_acceptance_criteria() {
        let cfg = DbConfig::default();
        assert_eq!(cfg.pool.min_connections, 5);
        assert_eq!(cfg.pool.max_connections, 100);
        assert_eq!(cfg.pool.idle_timeout_secs, 30);
        assert_eq!(cfg.slow_query_ms, 1000);
        assert_eq!(cfg.hardening.statement_timeout_ms, 30_000);
    }

    #[test]
    fn valid_config_passes() {
        let mut cfg = DbConfig::default();
        cfg.tls.root_cert_path = Some("/ca.pem".to_string());
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn rejects_min_exceeds_max() {
        let mut cfg = DbConfig::default();
        cfg.tls.root_cert_path = Some("/ca.pem".to_string());
        cfg.pool.min_connections = 200;
        assert_eq!(cfg.validate(), Err(DbConfigError::MinExceedsMax));
    }

    #[test]
    fn rejects_zero_max() {
        let mut cfg = DbConfig::default();
        cfg.tls.root_cert_path = Some("/ca.pem".to_string());
        cfg.pool.max_connections = 0;
        assert_eq!(cfg.validate(), Err(DbConfigError::ZeroMax));
    }

    #[test]
    fn surfaces_tls_errors() {
        let cfg = DbConfig {
            tls: TlsConfig {
                mode: SslMode::VerifyFull,
                root_cert_path: None,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            cfg.validate(),
            Err(DbConfigError::Tls(TlsConfigError::MissingRootCert))
        );
    }
}
