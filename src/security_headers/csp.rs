//! Content Security Policy construction.
//!
//! A builder for strict CSPs: directives map to allowed [`CspSource`]s and the
//! whole policy renders to the `Content-Security-Policy` header value. A
//! `strict()` preset produces a tight, XSS-resistant default (no inline script,
//! `object-src 'none'`, `frame-ancestors 'none'`, `base-uri 'self'`).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A source expression in a CSP directive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CspSource {
    /// `'self'`.
    SelfOrigin,
    /// `'none'`.
    None,
    /// `https:` scheme.
    Https,
    /// `'strict-dynamic'`.
    StrictDynamic,
    /// A concrete host or origin (e.g. `cdn.example.com`).
    Host(String),
    /// A per-response nonce: rendered as `'nonce-<value>'`.
    Nonce(String),
    /// A hash source: rendered as `'sha256-<value>'`.
    Sha256(String),
}

impl CspSource {
    /// Renders the source token.
    pub fn render(&self) -> String {
        match self {
            CspSource::SelfOrigin => "'self'".to_string(),
            CspSource::None => "'none'".to_string(),
            CspSource::Https => "https:".to_string(),
            CspSource::StrictDynamic => "'strict-dynamic'".to_string(),
            CspSource::Host(h) => h.clone(),
            CspSource::Nonce(n) => format!("'nonce-{n}'"),
            CspSource::Sha256(h) => format!("'sha256-{h}'"),
        }
    }

    /// Whether this source is considered unsafe/weakening (for grading).
    pub fn is_weak(&self) -> bool {
        // Wildcards and unsafe-* would be weak; this type does not model them,
        // but a bare `https:` on script-src is broad.
        matches!(self, CspSource::Https)
    }
}

/// A Content Security Policy: ordered directives → sources.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentSecurityPolicy {
    directives: BTreeMap<String, Vec<CspSource>>,
    /// `upgrade-insecure-requests` flag (valueless directive).
    pub upgrade_insecure_requests: bool,
}

impl ContentSecurityPolicy {
    /// An empty policy.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the sources for a directive (replacing any existing).
    pub fn directive(mut self, name: &str, sources: Vec<CspSource>) -> Self {
        self.directives.insert(name.to_string(), sources);
        self
    }

    /// Enables `upgrade-insecure-requests`.
    pub fn upgrade_insecure(mut self) -> Self {
        self.upgrade_insecure_requests = true;
        self
    }

    /// A strict, XSS-resistant baseline policy. `script_nonce` binds inline
    /// scripts to a per-response nonce.
    pub fn strict(script_nonce: &str) -> Self {
        Self::new()
            .directive("default-src", vec![CspSource::None])
            .directive(
                "script-src",
                vec![
                    CspSource::SelfOrigin,
                    CspSource::Nonce(script_nonce.to_string()),
                ],
            )
            .directive("style-src", vec![CspSource::SelfOrigin])
            .directive("img-src", vec![CspSource::SelfOrigin, CspSource::Https])
            .directive("font-src", vec![CspSource::SelfOrigin])
            .directive("connect-src", vec![CspSource::SelfOrigin])
            .directive("object-src", vec![CspSource::None])
            .directive("base-uri", vec![CspSource::SelfOrigin])
            .directive("frame-ancestors", vec![CspSource::None])
            .directive("form-action", vec![CspSource::SelfOrigin])
            .upgrade_insecure()
    }

    /// Sources for a directive, if present.
    pub fn get(&self, name: &str) -> Option<&[CspSource]> {
        self.directives.get(name).map(|v| v.as_slice())
    }

    /// Renders the policy as a `Content-Security-Policy` header value.
    pub fn to_header_value(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        for (name, sources) in &self.directives {
            let rendered: Vec<String> = sources.iter().map(|s| s.render()).collect();
            if rendered.is_empty() {
                parts.push(name.clone());
            } else {
                parts.push(format!("{} {}", name, rendered.join(" ")));
            }
        }
        if self.upgrade_insecure_requests {
            parts.push("upgrade-insecure-requests".to_string());
        }
        parts.join("; ")
    }

    /// Whether the policy is strict enough (no inline scripts, locked-down
    /// default/object/frame-ancestors). Used by the grader.
    pub fn is_strict(&self) -> bool {
        let default_locked = self
            .get("default-src")
            .map(|s| s == [CspSource::None] || s == [CspSource::SelfOrigin])
            .unwrap_or(false);
        let object_locked = self
            .get("object-src")
            .map(|s| s == [CspSource::None])
            .unwrap_or(false);
        let frame_locked = self
            .get("frame-ancestors")
            .map(|s| s == [CspSource::None] || s == [CspSource::SelfOrigin])
            .unwrap_or(false);
        let base_locked = self.get("base-uri").is_some();
        default_locked && object_locked && frame_locked && base_locked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_policy_renders_expected_directives() {
        let csp = ContentSecurityPolicy::strict("abc123");
        let value = csp.to_header_value();
        assert!(value.contains("default-src 'none'"));
        assert!(value.contains("script-src 'self' 'nonce-abc123'"));
        assert!(value.contains("object-src 'none'"));
        assert!(value.contains("frame-ancestors 'none'"));
        assert!(value.contains("base-uri 'self'"));
        assert!(value.contains("upgrade-insecure-requests"));
    }

    #[test]
    fn strict_policy_grades_as_strict() {
        assert!(ContentSecurityPolicy::strict("n").is_strict());
    }

    #[test]
    fn loose_policy_is_not_strict() {
        let csp = ContentSecurityPolicy::new().directive("default-src", vec![CspSource::Https]);
        assert!(!csp.is_strict());
    }

    #[test]
    fn source_rendering() {
        assert_eq!(CspSource::SelfOrigin.render(), "'self'");
        assert_eq!(CspSource::Nonce("x".into()).render(), "'nonce-x'");
        assert_eq!(CspSource::Sha256("h".into()).render(), "'sha256-h'");
        assert_eq!(
            CspSource::Host("cdn.example.com".into()).render(),
            "cdn.example.com"
        );
    }

    #[test]
    fn custom_directive_round_trips() {
        let csp = ContentSecurityPolicy::new().directive(
            "script-src",
            vec![CspSource::SelfOrigin, CspSource::Host("cdn.x".into())],
        );
        assert_eq!(csp.get("script-src").unwrap().len(), 2);
        assert!(csp.to_header_value().contains("script-src 'self' cdn.x"));
    }
}
