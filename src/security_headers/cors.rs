//! CORS configuration with strict origin validation.
//!
//! An allowlist-based CORS policy: an `Origin` is reflected only if it exactly
//! matches a configured origin (no wildcard with credentials, the classic
//! footgun). Produces the response headers for both preflight and actual
//! requests.

use serde::{Deserialize, Serialize};

/// CORS policy configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Exact origins permitted (scheme + host + optional port).
    pub allowed_origins: Vec<String>,
    /// Methods permitted on cross-origin requests.
    pub allowed_methods: Vec<String>,
    /// Request headers permitted.
    pub allowed_headers: Vec<String>,
    /// Whether credentials (cookies/Authorization) are allowed.
    pub allow_credentials: bool,
    /// Preflight cache lifetime in seconds.
    pub max_age_secs: u64,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: Vec::new(),
            allowed_methods: vec!["GET".into(), "POST".into(), "OPTIONS".into()],
            allowed_headers: vec!["Content-Type".into(), "Authorization".into()],
            allow_credentials: false,
            max_age_secs: 600,
        }
    }
}

/// The decision for a cross-origin request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorsDecision {
    /// Origin is allowed; attach these response headers.
    Allowed(Vec<(String, String)>),
    /// Origin is not on the allowlist; do not add CORS headers.
    Rejected,
}

impl CorsConfig {
    /// Adds an allowed origin.
    pub fn allow_origin(mut self, origin: impl Into<String>) -> Self {
        self.allowed_origins.push(origin.into());
        self
    }

    /// Whether an origin is explicitly allowlisted.
    pub fn is_origin_allowed(&self, origin: &str) -> bool {
        self.allowed_origins.iter().any(|o| o == origin)
    }

    /// Evaluates a (non-preflight) cross-origin request from `origin`.
    pub fn evaluate(&self, origin: &str) -> CorsDecision {
        if !self.is_origin_allowed(origin) {
            return CorsDecision::Rejected;
        }
        let mut headers = vec![
            (
                "Access-Control-Allow-Origin".to_string(),
                origin.to_string(),
            ),
            // Reflecting a specific origin requires Vary: Origin for caches.
            ("Vary".to_string(), "Origin".to_string()),
        ];
        if self.allow_credentials {
            headers.push((
                "Access-Control-Allow-Credentials".to_string(),
                "true".to_string(),
            ));
        }
        CorsDecision::Allowed(headers)
    }

    /// Evaluates a CORS preflight (OPTIONS) request from `origin`.
    pub fn evaluate_preflight(&self, origin: &str) -> CorsDecision {
        if !self.is_origin_allowed(origin) {
            return CorsDecision::Rejected;
        }
        let mut headers = vec![
            (
                "Access-Control-Allow-Origin".to_string(),
                origin.to_string(),
            ),
            (
                "Access-Control-Allow-Methods".to_string(),
                self.allowed_methods.join(", "),
            ),
            (
                "Access-Control-Allow-Headers".to_string(),
                self.allowed_headers.join(", "),
            ),
            (
                "Access-Control-Max-Age".to_string(),
                self.max_age_secs.to_string(),
            ),
            ("Vary".to_string(), "Origin".to_string()),
        ];
        if self.allow_credentials {
            headers.push((
                "Access-Control-Allow-Credentials".to_string(),
                "true".to_string(),
            ));
        }
        CorsDecision::Allowed(headers)
    }

    /// Validates the config is not dangerously permissive: credentialed CORS
    /// must not be combined with a `*` wildcard origin.
    pub fn is_safe(&self) -> bool {
        !(self.allow_credentials && self.allowed_origins.iter().any(|o| o == "*"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> CorsConfig {
        CorsConfig::default().allow_origin("https://app.example.com")
    }

    #[test]
    fn allowed_origin_is_reflected() {
        match config().evaluate("https://app.example.com") {
            CorsDecision::Allowed(h) => {
                assert!(h
                    .iter()
                    .any(|(k, v)| k == "Access-Control-Allow-Origin"
                        && v == "https://app.example.com"));
                assert!(h.iter().any(|(k, v)| k == "Vary" && v == "Origin"));
            }
            CorsDecision::Rejected => panic!("should be allowed"),
        }
    }

    #[test]
    fn unknown_origin_is_rejected() {
        assert_eq!(
            config().evaluate("https://evil.example.com"),
            CorsDecision::Rejected
        );
    }

    #[test]
    fn preflight_includes_methods_and_headers() {
        match config().evaluate_preflight("https://app.example.com") {
            CorsDecision::Allowed(h) => {
                assert!(h.iter().any(|(k, _)| k == "Access-Control-Allow-Methods"));
                assert!(h.iter().any(|(k, _)| k == "Access-Control-Max-Age"));
            }
            CorsDecision::Rejected => panic!("should be allowed"),
        }
    }

    #[test]
    fn credentials_with_wildcard_is_unsafe() {
        let cfg = CorsConfig {
            allow_credentials: true,
            ..CorsConfig::default()
        }
        .allow_origin("*");
        assert!(!cfg.is_safe());
    }

    #[test]
    fn exact_match_only_no_subdomain_bypass() {
        let cfg = config();
        assert!(!cfg.is_origin_allowed("https://app.example.com.evil.com"));
        assert!(!cfg.is_origin_allowed("https://sub.app.example.com"));
    }
}
