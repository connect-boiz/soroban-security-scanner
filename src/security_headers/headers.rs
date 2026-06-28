//! Individual security header types beyond CSP.
//!
//! HSTS, X-Frame-Options, X-Content-Type-Options, Referrer-Policy and
//! Permissions-Policy — each renders to its `(name, value)` header pair.

use serde::{Deserialize, Serialize};

/// HTTP Strict Transport Security.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hsts {
    /// `max-age` in seconds.
    pub max_age_secs: u64,
    /// Apply to subdomains.
    pub include_subdomains: bool,
    /// Eligible for the browser preload list.
    pub preload: bool,
}

impl Default for Hsts {
    /// Secure default: 1 year, subdomains, preload.
    fn default() -> Self {
        Self {
            max_age_secs: 31_536_000,
            include_subdomains: true,
            preload: true,
        }
    }
}

impl Hsts {
    /// Renders the header value.
    pub fn value(&self) -> String {
        let mut v = format!("max-age={}", self.max_age_secs);
        if self.include_subdomains {
            v.push_str("; includeSubDomains");
        }
        if self.preload {
            v.push_str("; preload");
        }
        v
    }

    /// The `(name, value)` pair.
    pub fn header(&self) -> (&'static str, String) {
        ("Strict-Transport-Security", self.value())
    }
}

/// X-Frame-Options (clickjacking protection).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameOptions {
    /// Disallow framing entirely.
    Deny,
    /// Allow framing only by same origin.
    SameOrigin,
}

impl FrameOptions {
    /// The `(name, value)` pair.
    pub fn header(&self) -> (&'static str, String) {
        let v = match self {
            FrameOptions::Deny => "DENY",
            FrameOptions::SameOrigin => "SAMEORIGIN",
        };
        ("X-Frame-Options", v.to_string())
    }
}

/// X-Content-Type-Options: always `nosniff`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ContentTypeOptions;

impl ContentTypeOptions {
    /// The `(name, value)` pair.
    pub fn header(&self) -> (&'static str, String) {
        ("X-Content-Type-Options", "nosniff".to_string())
    }
}

/// Referrer-Policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferrerPolicy {
    /// `no-referrer`.
    NoReferrer,
    /// `strict-origin`.
    StrictOrigin,
    /// `strict-origin-when-cross-origin` (a good default).
    StrictOriginWhenCrossOrigin,
    /// `same-origin`.
    SameOrigin,
}

impl ReferrerPolicy {
    /// The `(name, value)` pair.
    pub fn header(&self) -> (&'static str, String) {
        let v = match self {
            ReferrerPolicy::NoReferrer => "no-referrer",
            ReferrerPolicy::StrictOrigin => "strict-origin",
            ReferrerPolicy::StrictOriginWhenCrossOrigin => "strict-origin-when-cross-origin",
            ReferrerPolicy::SameOrigin => "same-origin",
        };
        ("Referrer-Policy", v.to_string())
    }
}

/// Permissions-Policy: maps a feature to its allowlist (empty = disabled).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PermissionsPolicy {
    features: Vec<(String, Vec<String>)>,
}

impl PermissionsPolicy {
    /// An empty policy.
    pub fn new() -> Self {
        Self::default()
    }

    /// A locked-down default: disables camera, microphone, geolocation and
    /// payment (`feature=()`).
    pub fn locked_down() -> Self {
        let mut p = Self::new();
        for feature in ["camera", "microphone", "geolocation", "payment"] {
            p.disable(feature);
        }
        p
    }

    /// Disables a feature for all origins (`feature=()`).
    pub fn disable(&mut self, feature: &str) {
        self.features.push((feature.to_string(), Vec::new()));
    }

    /// Allows a feature for the listed allowlist tokens (e.g. `"self"`).
    pub fn allow(&mut self, feature: &str, allowlist: Vec<String>) {
        self.features.push((feature.to_string(), allowlist));
    }

    /// Renders the header value.
    pub fn value(&self) -> String {
        self.features
            .iter()
            .map(|(f, list)| format!("{}=({})", f, list.join(" ")))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The `(name, value)` pair.
    pub fn header(&self) -> (&'static str, String) {
        ("Permissions-Policy", self.value())
    }

    /// Whether any feature is configured.
    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsts_default_is_strong() {
        let (name, value) = Hsts::default().header();
        assert_eq!(name, "Strict-Transport-Security");
        assert!(value.contains("max-age=31536000"));
        assert!(value.contains("includeSubDomains"));
        assert!(value.contains("preload"));
    }

    #[test]
    fn frame_and_content_type() {
        assert_eq!(
            FrameOptions::Deny.header(),
            ("X-Frame-Options", "DENY".to_string())
        );
        assert_eq!(
            ContentTypeOptions.header(),
            ("X-Content-Type-Options", "nosniff".to_string())
        );
    }

    #[test]
    fn referrer_policy_values() {
        assert_eq!(
            ReferrerPolicy::StrictOriginWhenCrossOrigin.header().1,
            "strict-origin-when-cross-origin"
        );
    }

    #[test]
    fn permissions_policy_renders() {
        let p = PermissionsPolicy::locked_down();
        let v = p.value();
        assert!(v.contains("camera=()"));
        assert!(v.contains("geolocation=()"));
        assert!(!p.is_empty());
    }

    #[test]
    fn permissions_allow_with_list() {
        let mut p = PermissionsPolicy::new();
        p.allow("fullscreen", vec!["self".to_string()]);
        assert_eq!(p.value(), "fullscreen=(self)");
    }
}
