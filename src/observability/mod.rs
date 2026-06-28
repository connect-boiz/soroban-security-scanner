//! Structured logging and distributed tracing (issue #337).
//!
//! A self-contained observability layer: consistent JSON structured logging
//! across components, W3C-compatible distributed tracing with correlation IDs
//! that propagate across services, centralized aggregation with access control
//! and audit, retention/archival, an inverted search index, log-based metrics,
//! and error-pattern/anomaly alerting.
//!
//! # Acceptance-criteria mapping
//!
//! | Requirement | Where |
//! |-------------|-------|
//! | Structured logging, consistent JSON format | [`record::LogRecord`], [`logger::Logger`] |
//! | Distributed tracing (OpenTelemetry/Jaeger-compatible) | [`tracing::Span`], W3C `traceparent` |
//! | Correlation IDs propagated across services | [`context::CorrelationContext`], [`tracing::extract_traceparent`] |
//! | Centralized log aggregation (ELK/CloudWatch) | [`sink::LogSink`], [`sink::AggregatingSink`] |
//! | Level config with env defaults (INFO prod / DEBUG dev) | [`level::LevelConfig`] |
//! | Retention policies with automatic archival | [`retention::apply_retention`] |
//! | Parsing & indexing for search | [`search::LogIndex`] |
//! | Log-based metrics extraction | [`metrics::LogMetricsCollector`] |
//! | Alerting for error patterns & anomalies | [`alerting::LogAlerter`] |
//! | Log security: access controls + audit | [`sink::AggregatingSink::read`] |
//! | 100% request traceability | [`record::LogRecord::is_traceable`], shared trace ids |
//! | Comprehensive testing | per-module tests + [`tests`] |
//!
//! # Example
//!
//! ```
//! use soroban_security_scanner::observability::*;
//!
//! let cfg = LevelConfig::for_environment(Environment::Production);
//! // (in production, pass a real centralized sink instead)
//! let span = Span::root("handle_request", 1_700_000_000);
//! assert!(span.traceparent().starts_with("00-"));
//! assert_eq!(cfg.min_level, LogLevel::Info);
//! ```

pub mod alerting;
pub mod context;
pub mod level;
pub mod logger;
pub mod metrics;
pub mod record;
pub mod retention;
pub mod search;
pub mod sink;
pub mod tracing;

#[cfg(test)]
mod tests;

pub use alerting::{AlertConfig, LogAlert, LogAlerter, PatternRule};
pub use context::{new_span_id, new_trace_id, CorrelationContext};
pub use level::{Environment, LevelConfig, LogLevel};
pub use logger::{EmitOutcome, Logger};
pub use metrics::{LogMetrics, LogMetricsCollector};
pub use record::LogRecord;
pub use retention::{apply_retention, Archive, InMemoryArchive, RetentionPolicy};
pub use search::{LogIndex, LogQuery};
pub use sink::{AggregatingSink, LogAccessRecord, LogReaderRole, LogSink};
pub use tracing::{extract_traceparent, Span, SpanStatus};
