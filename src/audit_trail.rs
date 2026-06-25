//! Comprehensive audit trail for security-critical operations.
//!
//! Records who performed what action, when, from which IP, and whether
//! it succeeded. Entries are append-only and timestamped.
//! Persisted to a write-ahead log (`audit.log`) and optionally
//! to the database via the `AuditStore` trait.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::AsyncWriteExt;

// ---------------------------------------------------------------------------
// Audit event types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    // Authentication
    Login,
    LoginFailed,
    Logout,
    // Vulnerability lifecycle
    VulnerabilityReported,
    VulnerabilityVerified,
    VulnerabilityRejected,
    // Bounty
    BountyFunded,
    BountyPaid,
    BountyRefunded,
    // Scan operations
    ScanStarted,
    ScanCompleted,
    ScanAccessDenied,
    // Administrative
    AdminAction,
    ContractPaused,
    ContractUnpaused,
    UpgradeProposed,
    UpgradeExecuted,
    // Access control
    RoleGranted,
    RoleRevoked,
    UnauthorizedAccess,
}

impl fmt::Display for AuditAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// A single audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// RFC3339 timestamp.
    pub timestamp:    String,
    /// The principal who performed the action.
    pub principal_id: String,
    /// Originating IP address.
    pub ip_address:   String,
    /// The action performed.
    pub action:       AuditAction,
    /// Target resource identifier (scan ID, report ID, etc.).
    pub resource_id:  Option<String>,
    /// Whether the operation succeeded.
    pub success:      bool,
    /// Additional context (error message, diff summary, etc.).
    pub details:      Option<String>,
}

impl AuditEntry {
    pub fn new(
        principal_id: impl Into<String>,
        ip_address:   impl Into<String>,
        action:       AuditAction,
        resource_id:  Option<String>,
        success:      bool,
        details:      Option<String>,
    ) -> Self {
        Self {
            timestamp:    chrono::Utc::now().to_rfc3339(),
            principal_id: principal_id.into(),
            ip_address:   ip_address.into(),
            action,
            resource_id,
            success,
            details,
        }
    }
}

// ---------------------------------------------------------------------------
// Audit logger
// ---------------------------------------------------------------------------

/// Async audit logger that writes JSON-lines to a file.
#[derive(Clone)]
pub struct AuditLogger {
    inner: Arc<Mutex<tokio::fs::File>>,
}

impl AuditLogger {
    /// Open (or create) the audit log file in append mode.
    pub async fn open(path: &str) -> anyhow::Result<Self> {
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;
        Ok(Self { inner: Arc::new(Mutex::new(file)) })
    }

    /// Append an entry to the audit log.
    pub async fn log(&self, entry: &AuditEntry) -> anyhow::Result<()> {
        let line = serde_json::to_string(entry)?;
        let mut file = self.inner.lock().await;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;

        // Mirror to tracing for real-time visibility.
        tracing::info!(
            principal = %entry.principal_id,
            ip        = %entry.ip_address,
            action    = %entry.action,
            success   = entry.success,
            resource  = ?entry.resource_id,
            "audit"
        );
        Ok(())
    }
}
