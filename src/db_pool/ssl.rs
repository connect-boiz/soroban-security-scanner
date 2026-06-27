//! SSL/TLS configuration for database connections.
//!
//! Models the libpq-style `sslmode` ladder plus certificate-validation
//! settings, and validates that a configuration is internally consistent
//! (e.g. a verifying mode must be given a CA certificate). Encryption with
//! certificate validation defends against credential interception and
//! man-in-the-middle attacks.

use serde::{Deserialize, Serialize};

/// TLS negotiation / verification mode, mirroring libpq `sslmode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SslMode {
    /// No TLS. Only permitted for explicitly-local development.
    Disable,
    /// Use TLS if available, but do not require or verify it.
    Prefer,
    /// Require TLS but do not validate the server certificate.
    Require,
    /// Require TLS and validate the certificate against a CA.
    VerifyCa,
    /// Require TLS, validate the CA chain, and check the hostname.
    VerifyFull,
}

impl SslMode {
    /// The libpq `sslmode` parameter value.
    pub fn as_param(&self) -> &'static str {
        match self {
            SslMode::Disable => "disable",
            SslMode::Prefer => "prefer",
            SslMode::Require => "require",
            SslMode::VerifyCa => "verify-ca",
            SslMode::VerifyFull => "verify-full",
        }
    }

    /// Whether this mode actually encrypts the connection.
    pub fn is_encrypted(&self) -> bool {
        !matches!(self, SslMode::Disable)
    }

    /// Whether this mode validates the server certificate.
    pub fn validates_certificate(&self) -> bool {
        matches!(self, SslMode::VerifyCa | SslMode::VerifyFull)
    }
}

/// TLS configuration for a database endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Negotiation/verification mode.
    pub mode: SslMode,
    /// Path to the CA certificate (required for verifying modes).
    pub root_cert_path: Option<String>,
    /// Optional client certificate path (mutual TLS).
    pub client_cert_path: Option<String>,
    /// Optional client key path (mutual TLS).
    pub client_key_path: Option<String>,
}

impl Default for TlsConfig {
    /// Secure-by-default: full verification.
    fn default() -> Self {
        Self {
            mode: SslMode::VerifyFull,
            root_cert_path: None,
            client_cert_path: None,
            client_key_path: None,
        }
    }
}

/// Why a TLS configuration is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TlsConfigError {
    /// A verifying mode was selected without a CA certificate.
    MissingRootCert,
    /// A client certificate was given without its key (or vice versa).
    IncompleteClientCert,
}

impl TlsConfig {
    /// Whether the connection will be encrypted *and* certificate-validated.
    pub fn is_secure(&self) -> bool {
        self.mode.validates_certificate()
    }

    /// Validates internal consistency of the configuration.
    pub fn validate(&self) -> Result<(), TlsConfigError> {
        if self.mode.validates_certificate() && self.root_cert_path.is_none() {
            return Err(TlsConfigError::MissingRootCert);
        }
        if self.client_cert_path.is_some() != self.client_key_path.is_some() {
            return Err(TlsConfigError::IncompleteClientCert);
        }
        Ok(())
    }

    /// Renders the `sslmode`/cert key-value pairs for a connection string.
    pub fn connection_params(&self) -> Vec<(&'static str, String)> {
        let mut params = vec![("sslmode", self.mode.as_param().to_string())];
        if let Some(ca) = &self.root_cert_path {
            params.push(("sslrootcert", ca.clone()));
        }
        if let Some(cert) = &self.client_cert_path {
            params.push(("sslcert", cert.clone()));
        }
        if let Some(key) = &self.client_key_path {
            params.push(("sslkey", key.clone()));
        }
        params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_secure() {
        let cfg = TlsConfig::default();
        assert_eq!(cfg.mode, SslMode::VerifyFull);
        assert!(cfg.mode.is_encrypted());
        assert!(cfg.is_secure());
    }

    #[test]
    fn verifying_mode_requires_ca() {
        let cfg = TlsConfig {
            mode: SslMode::VerifyFull,
            root_cert_path: None,
            ..Default::default()
        };
        assert_eq!(cfg.validate(), Err(TlsConfigError::MissingRootCert));

        let cfg = TlsConfig {
            mode: SslMode::VerifyFull,
            root_cert_path: Some("/etc/ssl/ca.pem".to_string()),
            ..Default::default()
        };
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn require_mode_needs_no_ca() {
        let cfg = TlsConfig {
            mode: SslMode::Require,
            root_cert_path: None,
            ..Default::default()
        };
        assert!(cfg.validate().is_ok());
        assert!(cfg.mode.is_encrypted());
        assert!(!cfg.is_secure()); // encrypted but not validated
    }

    #[test]
    fn incomplete_client_cert_is_rejected() {
        let cfg = TlsConfig {
            mode: SslMode::Require,
            client_cert_path: Some("/c.pem".to_string()),
            client_key_path: None,
            ..Default::default()
        };
        assert_eq!(cfg.validate(), Err(TlsConfigError::IncompleteClientCert));
    }

    #[test]
    fn connection_params_render() {
        let cfg = TlsConfig {
            mode: SslMode::VerifyFull,
            root_cert_path: Some("/ca.pem".to_string()),
            client_cert_path: Some("/c.pem".to_string()),
            client_key_path: Some("/k.pem".to_string()),
        };
        let params = cfg.connection_params();
        assert!(params.contains(&("sslmode", "verify-full".to_string())));
        assert!(params.contains(&("sslrootcert", "/ca.pem".to_string())));
        assert!(params.contains(&("sslkey", "/k.pem".to_string())));
    }
}
