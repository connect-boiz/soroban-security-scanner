//! Database security hardening configuration and auditing.
//!
//! Captures server-side hardening intent — extensions to disable, networks
//! permitted to connect, and session-level timeouts — and audits a
//! configuration for weaknesses, emitting actionable findings.

use crate::db_pool::ssl::TlsConfig;
use serde::{Deserialize, Serialize};

/// Hardening settings applied to the database/session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardeningConfig {
    /// Extensions that must be disabled if present (attack surface reduction).
    pub disabled_extensions: Vec<String>,
    /// CIDR ranges permitted to connect (empty = rely on external firewall).
    pub allowed_networks: Vec<String>,
    /// `statement_timeout` in milliseconds (caps long-running queries).
    pub statement_timeout_ms: u64,
    /// `idle_in_transaction_session_timeout` in milliseconds.
    pub idle_in_transaction_timeout_ms: u64,
    /// Whether superuser login over the network is forbidden.
    pub forbid_superuser_remote: bool,
}

impl Default for HardeningConfig {
    fn default() -> Self {
        Self {
            // Commonly-abused or rarely-needed extensions.
            disabled_extensions: vec![
                "dblink".to_string(),
                "pg_execute_server_program".to_string(),
                "adminpack".to_string(),
            ],
            allowed_networks: Vec::new(),
            statement_timeout_ms: 30_000,
            idle_in_transaction_timeout_ms: 60_000,
            forbid_superuser_remote: true,
        }
    }
}

/// Severity of an audit finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AuditSeverity {
    /// Advisory.
    Low,
    /// Should be addressed.
    Medium,
    /// Must be addressed before production.
    High,
}

/// A single hardening audit finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditFinding {
    /// Stable id.
    pub code: String,
    /// Description of the weakness.
    pub message: String,
    /// Severity.
    pub severity: AuditSeverity,
}

impl HardeningConfig {
    /// Generates the session-level `SET` statements implied by this config.
    pub fn session_statements(&self) -> Vec<String> {
        vec![
            format!("SET statement_timeout = {}", self.statement_timeout_ms),
            format!(
                "SET idle_in_transaction_session_timeout = {}",
                self.idle_in_transaction_timeout_ms
            ),
        ]
    }

    /// Audits the combined hardening + TLS posture and returns findings,
    /// most-severe first.
    pub fn audit(&self, tls: &TlsConfig) -> Vec<AuditFinding> {
        let mut findings = Vec::new();

        if !tls.is_secure() {
            findings.push(AuditFinding {
                code: "tls-not-verified".to_string(),
                message: "database TLS does not validate the server certificate".to_string(),
                severity: AuditSeverity::High,
            });
        }
        if self.statement_timeout_ms == 0 {
            findings.push(AuditFinding {
                code: "no-statement-timeout".to_string(),
                message: "statement_timeout is disabled; long-running queries are unbounded"
                    .to_string(),
                severity: AuditSeverity::Medium,
            });
        }
        if self.idle_in_transaction_timeout_ms == 0 {
            findings.push(AuditFinding {
                code: "no-idle-tx-timeout".to_string(),
                message: "idle-in-transaction sessions can hold locks indefinitely".to_string(),
                severity: AuditSeverity::Medium,
            });
        }
        if self.allowed_networks.is_empty() {
            findings.push(AuditFinding {
                code: "no-network-restriction".to_string(),
                message: "no allowed-network restriction configured at the database layer"
                    .to_string(),
                severity: AuditSeverity::Low,
            });
        }
        if !self.forbid_superuser_remote {
            findings.push(AuditFinding {
                code: "superuser-remote".to_string(),
                message: "remote superuser login is permitted".to_string(),
                severity: AuditSeverity::High,
            });
        }

        findings.sort_by_key(|f| std::cmp::Reverse(f.severity));
        findings
    }

    /// Whether the posture is free of High-severity findings.
    pub fn is_production_ready(&self, tls: &TlsConfig) -> bool {
        !self
            .audit(tls)
            .iter()
            .any(|f| f.severity == AuditSeverity::High)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_pool::ssl::SslMode;

    fn secure_tls() -> TlsConfig {
        TlsConfig {
            mode: SslMode::VerifyFull,
            root_cert_path: Some("/ca.pem".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn default_with_secure_tls_is_production_ready() {
        let cfg = HardeningConfig::default();
        assert!(cfg.is_production_ready(&secure_tls()));
    }

    #[test]
    fn insecure_tls_is_high_finding() {
        let cfg = HardeningConfig::default();
        let tls = TlsConfig {
            mode: SslMode::Disable,
            ..Default::default()
        };
        let findings = cfg.audit(&tls);
        assert!(findings.iter().any(|f| f.code == "tls-not-verified"));
        assert!(!cfg.is_production_ready(&tls));
        // High-severity findings sort first.
        assert_eq!(findings[0].severity, AuditSeverity::High);
    }

    #[test]
    fn missing_timeouts_flagged() {
        let cfg = HardeningConfig {
            statement_timeout_ms: 0,
            idle_in_transaction_timeout_ms: 0,
            ..Default::default()
        };
        let findings = cfg.audit(&secure_tls());
        assert!(findings.iter().any(|f| f.code == "no-statement-timeout"));
        assert!(findings.iter().any(|f| f.code == "no-idle-tx-timeout"));
    }

    #[test]
    fn superuser_remote_is_high() {
        let cfg = HardeningConfig {
            forbid_superuser_remote: false,
            ..Default::default()
        };
        assert!(!cfg.is_production_ready(&secure_tls()));
    }

    #[test]
    fn session_statements_include_timeouts() {
        let cfg = HardeningConfig::default();
        let stmts = cfg.session_statements();
        assert!(stmts.iter().any(|s| s.contains("statement_timeout = 30000")));
    }
}
