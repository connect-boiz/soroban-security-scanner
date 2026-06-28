//! Structured log records (JSON).
//!
//! Every component emits the same shape: timestamp, level, target, message,
//! correlation ids, and arbitrary structured fields — serialized as a single
//! JSON line suitable for ingestion by ELK/CloudWatch.

use crate::observability::context::CorrelationContext;
use crate::observability::level::LogLevel;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A structured log record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogRecord {
    /// Event time (unix seconds).
    pub timestamp: i64,
    /// Severity level.
    pub level: LogLevel,
    /// Emitting component/module.
    pub target: String,
    /// Human-readable message.
    pub message: String,
    /// Trace id (32 hex), if a correlation context is set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// Span id (16 hex), if set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
    /// Request id, if set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Arbitrary structured fields (ordered for stable output).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

impl LogRecord {
    /// Builds a record with no correlation context.
    pub fn new(
        timestamp: i64,
        level: LogLevel,
        target: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            timestamp,
            level,
            target: target.into(),
            message: message.into(),
            trace_id: None,
            span_id: None,
            request_id: None,
            fields: BTreeMap::new(),
        }
    }

    /// Attaches correlation ids from a context.
    pub fn with_context(mut self, ctx: &CorrelationContext) -> Self {
        self.trace_id = Some(ctx.trace_id.clone());
        self.span_id = Some(ctx.span_id.clone());
        self.request_id = Some(ctx.request_id.clone());
        self
    }

    /// Adds a structured field.
    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    /// Whether this record carries full correlation (is traceable).
    pub fn is_traceable(&self) -> bool {
        self.trace_id.is_some() && self.span_id.is_some() && self.request_id.is_some()
    }

    /// Serializes the record as a single JSON line.
    pub fn to_json(&self) -> String {
        // Serialization of this struct never fails; fall back defensively.
        serde_json::to_string(self).unwrap_or_else(|_| {
            format!(
                r#"{{"timestamp":{},"level":"{}","message":"<unserializable>"}}"#,
                self.timestamp,
                self.level.as_str()
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_contains_core_fields() {
        let rec = LogRecord::new(1700, LogLevel::Info, "api", "request handled")
            .with_field("status", "200")
            .with_field("path", "/api/scan");
        let json = rec.to_json();
        assert!(
            json.contains("\"level\":\"Info\"")
                || json.contains("\"level\":\"INFO\"")
                || json.contains("Info")
        );
        assert!(json.contains("request handled"));
        assert!(json.contains("\"status\":\"200\""));
        assert!(json.contains("/api/scan"));
    }

    #[test]
    fn context_makes_record_traceable() {
        let ctx = CorrelationContext::new_root();
        let rec = LogRecord::new(1700, LogLevel::Info, "api", "hi").with_context(&ctx);
        assert!(rec.is_traceable());
        let json = rec.to_json();
        assert!(json.contains(&ctx.trace_id));
        assert!(json.contains(&ctx.request_id));
    }

    #[test]
    fn record_without_context_is_not_traceable() {
        let rec = LogRecord::new(1700, LogLevel::Warn, "x", "y");
        assert!(!rec.is_traceable());
        // Absent correlation fields are omitted from JSON.
        assert!(!rec.to_json().contains("trace_id"));
    }

    #[test]
    fn json_round_trips_through_serde() {
        let ctx = CorrelationContext::new_root();
        let rec = LogRecord::new(1700, LogLevel::Error, "db", "boom")
            .with_context(&ctx)
            .with_field("code", "500");
        let json = rec.to_json();
        let parsed: LogRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, rec);
    }
}
