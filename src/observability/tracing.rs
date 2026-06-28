//! Distributed tracing: spans and W3C Trace Context propagation.
//!
//! A [`Span`] represents one timed operation within a trace and carries the
//! [`CorrelationContext`]. Child spans inherit the trace id, giving end-to-end
//! request traceability. The `traceparent` header (W3C Trace Context, the wire
//! format OpenTelemetry/Jaeger interoperate with) is used to propagate context
//! across service boundaries.

use crate::observability::context::CorrelationContext;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Completion status of a span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanStatus {
    /// Still running.
    Active,
    /// Completed successfully.
    Ok,
    /// Completed with an error.
    Error,
}

/// A single traced operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    /// Operation name.
    pub name: String,
    /// Correlation ids for this span.
    pub context: CorrelationContext,
    /// Start time (unix seconds).
    pub start: i64,
    /// End time (unix seconds), once finished.
    pub end: Option<i64>,
    /// Status.
    pub status: SpanStatus,
    /// Span attributes/tags.
    pub attributes: BTreeMap<String, String>,
}

impl Span {
    /// Starts a root span (new trace).
    pub fn root(name: impl Into<String>, start: i64) -> Self {
        Self::with_context(name, CorrelationContext::new_root(), start)
    }

    /// Starts a span with an explicit context.
    pub fn with_context(name: impl Into<String>, context: CorrelationContext, start: i64) -> Self {
        Self {
            name: name.into(),
            context,
            start,
            end: None,
            status: SpanStatus::Active,
            attributes: BTreeMap::new(),
        }
    }

    /// Starts a child span of this one (same trace, new span id).
    pub fn child(&self, name: impl Into<String>, start: i64) -> Span {
        Span::with_context(name, self.context.child(), start)
    }

    /// Adds an attribute.
    pub fn set_attribute(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.attributes.insert(key.into(), value.into());
    }

    /// Finishes the span with a status.
    pub fn finish(&mut self, end: i64, status: SpanStatus) {
        self.end = Some(end);
        self.status = status;
    }

    /// Duration in seconds, if finished.
    pub fn duration_secs(&self) -> Option<i64> {
        self.end.map(|e| (e - self.start).max(0))
    }

    /// The `traceparent` header value for propagating this span downstream.
    /// Format: `version-traceid-spanid-flags` (e.g. `00-<32hex>-<16hex>-01`).
    pub fn traceparent(&self) -> String {
        format!("00-{}-{}-01", self.context.trace_id, self.context.span_id)
    }
}

/// Parses a `traceparent` header into a continuation context (the remote span
/// becomes our parent). Returns `None` for malformed input.
pub fn extract_traceparent(
    header: &str,
    request_id: impl Into<String>,
) -> Option<CorrelationContext> {
    let parts: Vec<&str> = header.split('-').collect();
    if parts.len() != 4 {
        return None;
    }
    let (version, trace_id, parent_span, _flags) = (parts[0], parts[1], parts[2], parts[3]);
    if version != "00" || trace_id.len() != 32 || parent_span.len() != 16 {
        return None;
    }
    if !trace_id.bytes().all(|b| b.is_ascii_hexdigit())
        || !parent_span.bytes().all(|b| b.is_ascii_hexdigit())
        || trace_id.bytes().all(|b| b == b'0')
        || parent_span.bytes().all(|b| b == b'0')
    {
        return None;
    }
    Some(CorrelationContext {
        trace_id: trace_id.to_string(),
        span_id: crate::observability::context::new_span_id(),
        parent_span_id: Some(parent_span.to_string()),
        request_id: request_id.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_lifecycle_and_duration() {
        let mut span = Span::root("handle_request", 1000);
        span.set_attribute("http.method", "POST");
        assert_eq!(span.status, SpanStatus::Active);
        assert!(span.duration_secs().is_none());
        span.finish(1002, SpanStatus::Ok);
        assert_eq!(span.duration_secs(), Some(2));
        assert_eq!(span.status, SpanStatus::Ok);
    }

    #[test]
    fn child_spans_share_trace_id() {
        let root = Span::root("a", 1000);
        let child = root.child("b", 1001);
        let grandchild = child.child("c", 1002);
        assert_eq!(child.context.trace_id, root.context.trace_id);
        assert_eq!(grandchild.context.trace_id, root.context.trace_id);
        assert_eq!(
            child.context.parent_span_id.as_deref(),
            Some(root.context.span_id.as_str())
        );
    }

    #[test]
    fn traceparent_round_trips_across_a_service_boundary() {
        // Service A starts a span and emits a traceparent header.
        let span_a = Span::root("service-a", 1000);
        let header = span_a.traceparent();
        assert!(header.starts_with("00-"));

        // Service B extracts it and continues the same trace.
        let ctx_b = extract_traceparent(&header, "req-2").unwrap();
        assert_eq!(ctx_b.trace_id, span_a.context.trace_id);
        assert_eq!(
            ctx_b.parent_span_id.as_deref(),
            Some(span_a.context.span_id.as_str())
        );
        // New local span, same trace → full traceability.
        let span_b = Span::with_context("service-b", ctx_b, 1003);
        assert_eq!(span_b.context.trace_id, span_a.context.trace_id);
    }

    #[test]
    fn malformed_traceparent_is_rejected() {
        assert!(extract_traceparent("garbage", "r").is_none());
        assert!(extract_traceparent("00-tooshort-abcdef0000000000-01", "r").is_none());
        assert!(extract_traceparent(
            "99-00000000000000000000000000000000-0000000000000000-01",
            "r"
        )
        .is_none());
        // All-zero ids are invalid per spec.
        assert!(extract_traceparent(
            "00-00000000000000000000000000000000-0000000000000000-01",
            "r"
        )
        .is_none());
    }
}
