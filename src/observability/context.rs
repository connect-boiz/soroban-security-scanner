//! Correlation context: the IDs that tie a request's logs and spans together.
//!
//! Uses W3C Trace Context identifiers — a 16-byte trace id (32 hex chars) shared
//! by every span/log of a request, and an 8-byte span id (16 hex chars) for the
//! current operation — plus an application-level request id. The context is
//! propagated across service boundaries via the `traceparent` header.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Correlation identifiers carried with every log record and span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrelationContext {
    /// 32-hex-char trace id, constant across the whole request.
    pub trace_id: String,
    /// 16-hex-char id of the current span.
    pub span_id: String,
    /// Optional parent span id (set for child spans).
    pub parent_span_id: Option<String>,
    /// Human-facing request id (e.g. surfaced in API responses).
    pub request_id: String,
}

impl CorrelationContext {
    /// Starts a fresh root context with new ids.
    pub fn new_root() -> Self {
        Self {
            trace_id: new_trace_id(),
            span_id: new_span_id(),
            parent_span_id: None,
            request_id: Uuid::new_v4().to_string(),
        }
    }

    /// Derives a child context: same trace and request id, a new span id, and
    /// the current span recorded as parent.
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: new_span_id(),
            parent_span_id: Some(self.span_id.clone()),
            request_id: self.request_id.clone(),
        }
    }

    /// Validates that the ids are well-formed W3C identifiers.
    pub fn is_valid(&self) -> bool {
        is_hex_len(&self.trace_id, 32)
            && self.trace_id.bytes().any(|b| b != b'0')
            && is_hex_len(&self.span_id, 16)
            && self.span_id.bytes().any(|b| b != b'0')
    }
}

/// Generates a 32-hex-char trace id.
pub fn new_trace_id() -> String {
    Uuid::new_v4().simple().to_string()
}

/// Generates a 16-hex-char span id.
pub fn new_span_id() -> String {
    // Use the first 16 hex chars of a fresh UUID.
    Uuid::new_v4().simple().to_string()[..16].to_string()
}

fn is_hex_len(s: &str, len: usize) -> bool {
    s.len() == len && s.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_context_is_valid_and_unique() {
        let a = CorrelationContext::new_root();
        let b = CorrelationContext::new_root();
        assert!(a.is_valid());
        assert_eq!(a.trace_id.len(), 32);
        assert_eq!(a.span_id.len(), 16);
        assert!(a.parent_span_id.is_none());
        assert_ne!(a.trace_id, b.trace_id);
    }

    #[test]
    fn child_shares_trace_and_request_but_new_span() {
        let root = CorrelationContext::new_root();
        let child = root.child();
        assert_eq!(child.trace_id, root.trace_id);
        assert_eq!(child.request_id, root.request_id);
        assert_ne!(child.span_id, root.span_id);
        assert_eq!(child.parent_span_id.as_deref(), Some(root.span_id.as_str()));
    }

    #[test]
    fn invalid_ids_are_detected() {
        let bad = CorrelationContext {
            trace_id: "xyz".to_string(),
            span_id: "0000000000000000".to_string(),
            parent_span_id: None,
            request_id: "r".to_string(),
        };
        assert!(!bad.is_valid());
    }
}
