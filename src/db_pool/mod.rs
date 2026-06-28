//! Secure database connection pooling and monitoring (issue #331).
//!
//! A self-contained, driver-agnostic layer providing the connection-pool
//! sizing, TLS posture, retry, monitoring, leak recovery and read-replica
//! routing required to operate the database securely and reliably under load.
//! The pool is generic over the connection type and a [`pool::ConnectionFactory`],
//! so a production deployment wires it to `sqlx`/`tokio-postgres` while the
//! logic is exercised deterministically in tests.
//!
//! # Acceptance-criteria mapping
//!
//! | Requirement | Where |
//! |-------------|-------|
//! | SSL/TLS with certificate validation | [`ssl::TlsConfig`] |
//! | Pool limits (min 5 / max 100 / idle 30s) | [`config::PoolConfig`], [`pool::ConnectionPool`] |
//! | Connection validation query for stale detection | [`pool::ConnectionFactory::validate`] |
//! | Statement timeout (30s) | [`hardening::HardeningConfig`] |
//! | Pool-exhaustion monitoring & alerts | [`monitoring::DbMonitor`] |
//! | Connection-leak detection & automatic recovery | [`pool::ConnectionPool::reclaim_leaks`] |
//! | Slow-query logging (>1s) | [`monitoring::DbMonitor::record_query`] |
//! | Read-replica support | [`replica::ReplicaRouter`] |
//! | Retry with exponential backoff | [`retry::retry_with`] |
//! | Security hardening (extensions, network, timeouts) | [`hardening::HardeningConfig`] |
//! | Comprehensive testing | per-module tests + [`tests`] |
//!
//! # Example
//!
//! ```
//! use soroban_security_scanner::db_pool::*;
//!
//! let mut cfg = DbConfig::default();
//! cfg.tls.root_cert_path = Some("/etc/ssl/db-ca.pem".to_string());
//! assert!(cfg.validate().is_ok());
//! assert!(cfg.tls.is_secure());
//! ```

pub mod config;
pub mod hardening;
pub mod monitoring;
pub mod pool;
pub mod replica;
pub mod retry;
pub mod ssl;

#[cfg(test)]
mod tests;

pub use config::{DbConfig, DbConfigError, PoolConfig};
pub use hardening::{AuditFinding, AuditSeverity, HardeningConfig};
pub use monitoring::{Alert, DbMonitor, MonitorStats, SlowQuery};
pub use pool::{Clock, ConnectionFactory, ConnectionPool, DbError, PoolStats, PooledConnection};
pub use replica::{Endpoint, QueryKind, ReplicaRouter};
pub use retry::{retry_with, BackoffPolicy};
pub use ssl::{SslMode, TlsConfig, TlsConfigError};
