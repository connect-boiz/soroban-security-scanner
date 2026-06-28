//! The full security-header set for an HTTP response.
//!
//! Aggregates CSP, HSTS, X-Frame-Options, X-Content-Type-Options,
//! Referrer-Policy and Permissions-Policy into the list of `(name, value)`
//! header pairs to attach to every response, with a secure-by-default preset.

use crate::security_headers::csp::ContentSecurityPolicy;
use crate::security_headers::headers::{
    ContentTypeOptions, FrameOptions, Hsts, PermissionsPolicy, ReferrerPolicy,
};

/// A complete, configured set of response security headers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityHeaders {
    /// Content Security Policy.
    pub csp: ContentSecurityPolicy,
    /// HSTS.
    pub hsts: Hsts,
    /// X-Frame-Options.
    pub frame_options: FrameOptions,
    /// X-Content-Type-Options.
    pub content_type_options: ContentTypeOptions,
    /// Referrer-Policy.
    pub referrer_policy: ReferrerPolicy,
    /// Permissions-Policy.
    pub permissions_policy: PermissionsPolicy,
}

impl SecurityHeaders {
    /// A secure-by-default header set; `script_nonce` binds the CSP to a
    /// per-response nonce.
    pub fn secure_default(script_nonce: &str) -> Self {
        Self {
            csp: ContentSecurityPolicy::strict(script_nonce),
            hsts: Hsts::default(),
            frame_options: FrameOptions::Deny,
            content_type_options: ContentTypeOptions,
            referrer_policy: ReferrerPolicy::StrictOriginWhenCrossOrigin,
            permissions_policy: PermissionsPolicy::locked_down(),
        }
    }

    /// Renders all headers as `(name, value)` pairs ready to attach to a response.
    pub fn to_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = vec![(
            "Content-Security-Policy".to_string(),
            self.csp.to_header_value(),
        )];
        let (n, v) = self.hsts.header();
        pairs.push((n.to_string(), v));
        let (n, v) = self.frame_options.header();
        pairs.push((n.to_string(), v));
        let (n, v) = self.content_type_options.header();
        pairs.push((n.to_string(), v));
        let (n, v) = self.referrer_policy.header();
        pairs.push((n.to_string(), v));
        if !self.permissions_policy.is_empty() {
            let (n, v) = self.permissions_policy.header();
            pairs.push((n.to_string(), v));
        }
        pairs
    }

    /// Looks up a rendered header value by name.
    pub fn get(&self, name: &str) -> Option<String> {
        self.to_pairs()
            .into_iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_default_emits_all_headers() {
        let headers = SecurityHeaders::secure_default("nonce123");
        let names: Vec<String> = headers.to_pairs().into_iter().map(|(n, _)| n).collect();
        for expected in [
            "Content-Security-Policy",
            "Strict-Transport-Security",
            "X-Frame-Options",
            "X-Content-Type-Options",
            "Referrer-Policy",
            "Permissions-Policy",
        ] {
            assert!(names.iter().any(|n| n == expected), "missing {expected}");
        }
    }

    #[test]
    fn get_is_case_insensitive() {
        let headers = SecurityHeaders::secure_default("n");
        assert_eq!(headers.get("x-frame-options").as_deref(), Some("DENY"));
        assert!(headers
            .get("content-security-policy")
            .unwrap()
            .contains("default-src 'none'"));
    }
}
