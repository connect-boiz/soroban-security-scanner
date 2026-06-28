//! Comprehensive web security headers and CSP (issue #341).
//!
//! A self-contained library for constructing and evaluating the HTTP security
//! headers that harden the web application: a strict Content Security Policy,
//! HSTS, X-Frame-Options, X-Content-Type-Options, Referrer-Policy and
//! Permissions-Policy, plus Subresource Integrity, an allowlist CORS policy,
//! an A+-style grader, and misconfiguration monitoring.
//!
//! # Acceptance-criteria mapping
//!
//! | Requirement | Where |
//! |-------------|-------|
//! | Strict Content Security Policy | [`csp::ContentSecurityPolicy::strict`] |
//! | HSTS (max-age + includeSubDomains) | [`headers::Hsts`] |
//! | X-Frame-Options (clickjacking) | [`headers::FrameOptions`] |
//! | X-Content-Type-Options (nosniff) | [`headers::ContentTypeOptions`] |
//! | Referrer-Policy | [`headers::ReferrerPolicy`] |
//! | Permissions-Policy | [`headers::PermissionsPolicy`] |
//! | Subresource Integrity (SRI) | [`sri::integrity`] |
//! | CORS with origin validation | [`cors::CorsConfig`] |
//! | Header monitoring & misconfiguration alerting | [`monitoring::HeaderMonitor`] |
//! | A+ rating on evaluation tools | [`grade::evaluate`] |
//! | Comprehensive testing | per-module tests + [`tests`] |
//!
//! # Example
//!
//! ```
//! use soroban_security_scanner::security_headers::*;
//!
//! let headers = SecurityHeaders::secure_default("per-response-nonce");
//! // The secure default earns an A+.
//! assert_eq!(evaluate(&headers).grade, Grade::APlus);
//! assert!(headers.get("Content-Security-Policy").unwrap().contains("default-src 'none'"));
//! ```

pub mod builder;
pub mod cors;
pub mod csp;
pub mod grade;
pub mod headers;
pub mod monitoring;
pub mod sri;

#[cfg(test)]
mod tests;

pub use builder::SecurityHeaders;
pub use cors::{CorsConfig, CorsDecision};
pub use csp::{ContentSecurityPolicy, CspSource};
pub use grade::{evaluate, Grade, GradeReport, MIN_HSTS_MAX_AGE};
pub use headers::{ContentTypeOptions, FrameOptions, Hsts, PermissionsPolicy, ReferrerPolicy};
pub use monitoring::{HeaderAlert, HeaderMonitor, HeaderStats};
pub use sri::{integrity, script_tag, verify, SriAlgorithm};
