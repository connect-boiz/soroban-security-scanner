//! The logging facade.
//!
//! The single entry point components use to emit structured logs. It applies
//! the level filter, stamps records with the active correlation context, routes
//! them to a sink (centralized aggregator), and feeds the log-based metrics and
//! alerting in one call — giving consistent, correlated, traceable logs across
//! every component.

use crate::observability::alerting::{AlertConfig, LogAlert, LogAlerter};
use crate::observability::context::CorrelationContext;
use crate::observability::level::{LevelConfig, LogLevel};
use crate::observability::metrics::{LogMetrics, LogMetricsCollector};
use crate::observability::record::LogRecord;
use crate::observability::sink::LogSink;
use std::collections::BTreeMap;
use std::sync::Mutex;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Outcome of an emit: whether it was recorded and any alerts it triggered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EmitOutcome {
    /// False when filtered out by the level config.
    pub emitted: bool,
    /// Alerts triggered by this record.
    pub alerts: Vec<LogAlert>,
}

/// A structured logger bound to a sink, level policy and correlation context.
#[derive(Clone)]
pub struct Logger {
    target: String,
    level: LevelConfig,
    sink: Arc<dyn LogSink>,
    metrics: Arc<LogMetricsCollector>,
    alerter: Arc<Mutex<LogAlerter>>,
    context: Arc<Mutex<Option<CorrelationContext>>>,
    auto_context: bool,
    warn_on_missing_context: bool,
    missing_correlation_count: Arc<AtomicUsize>,
    emit_warning_records: bool,
}

impl Logger {
    /// Builds a logger for `target` writing to `sink`.
    pub fn new(target: impl Into<String>, level: LevelConfig, sink: Box<dyn LogSink>) -> Self {
        Self {
            target: target.into(),
            level,
            sink: Arc::from(sink),
            metrics: Arc::new(LogMetricsCollector::new()),
            alerter: Arc::new(Mutex::new(LogAlerter::new(AlertConfig::default()))),
            context: Arc::new(Mutex::new(None)),
            auto_context: false,
            warn_on_missing_context: true,
            missing_correlation_count: Arc::new(AtomicUsize::new(0)),
            emit_warning_records: false,
        }
    }

    /// Builds a logger for `target` writing to a shared `Arc<dyn LogSink>`.
    pub fn from_arc_sink(
        target: impl Into<String>,
        level: LevelConfig,
        sink: Arc<dyn LogSink>,
    ) -> Self {
        Self {
            target: target.into(),
            level,
            sink,
            metrics: Arc::new(LogMetricsCollector::new()),
            alerter: Arc::new(Mutex::new(LogAlerter::new(AlertConfig::default()))),
            context: Arc::new(Mutex::new(None)),
            auto_context: false,
            warn_on_missing_context: true,
            missing_correlation_count: Arc::new(AtomicUsize::new(0)),
            emit_warning_records: false,
        }
    }

    /// Returns a logger instance configured to automatically populate `trace_id`,
    /// `span_id`, and `request_id` from `CorrelationContext::current()` on every `LogRecord`.
    pub fn with_context(&self) -> Self {
        let mut cloned = self.clone();
        cloned.auto_context = true;
        cloned
    }

    /// Overrides the alerting configuration.
    pub fn with_alert_config(self, config: AlertConfig) -> Self {
        *self.alerter.lock().expect("alerter poisoned") = LogAlerter::new(config);
        self
    }

    /// Configures whether to emit warning log records to the sink when correlation IDs are missing.
    pub fn with_emit_warning_records(mut self, enabled: bool) -> Self {
        self.emit_warning_records = enabled;
        self
    }

    /// Configures whether to warn when logs are emitted without correlation IDs.
    pub fn with_warn_on_missing(mut self, enabled: bool) -> Self {
        self.warn_on_missing_context = enabled;
        self
    }

    /// Returns the number of times logs were emitted without correlation IDs.
    pub fn missing_correlation_warnings(&self) -> usize {
        self.missing_correlation_count.load(Ordering::Relaxed)
    }

    /// Sets the active correlation context for subsequent logs (e.g. at the
    /// start of handling a request).
    pub fn set_context(&self, ctx: CorrelationContext) {
        *self.context.lock().expect("context poisoned") = Some(ctx);
    }

    /// Clears the active correlation context.
    pub fn clear_context(&self) {
        *self.context.lock().expect("context poisoned") = None;
    }

    /// Current metrics snapshot.
    pub fn metrics(&self) -> LogMetrics {
        self.metrics.snapshot()
    }

    /// Emits a log record at `level` with optional structured fields.
    #[track_caller]
    pub fn log(
        &self,
        level: LogLevel,
        timestamp: i64,
        message: impl Into<String>,
        fields: &[(&str, &str)],
    ) -> EmitOutcome {
        if !self.level.is_enabled(level) {
            return EmitOutcome::default();
        }

        let mut record = LogRecord::new(timestamp, level, self.target.clone(), message);

        let explicit_ctx = self.context.lock().expect("context poisoned").clone();
        let effective_ctx = explicit_ctx.or_else(|| {
            if self.auto_context {
                CorrelationContext::current()
            } else {
                None
            }
        });

        if let Some(ref ctx) = effective_ctx {
            record = record.with_context(ctx);
        } else if self.warn_on_missing_context
            && level != LogLevel::Debug
            && level != LogLevel::Trace
        {
            self.missing_correlation_count
                .fetch_add(1, Ordering::Relaxed);
            let caller = std::panic::Location::caller();
            tracing::warn!(
                file = caller.file(),
                line = caller.line(),
                "LogRecord emitted without correlation IDs at {}:{}",
                caller.file(),
                caller.line()
            );

            if self.emit_warning_records {
                let warn_rec = LogRecord::new(
                    timestamp,
                    LogLevel::Warn,
                    "observability::correlation",
                    format!(
                        "LogRecord emitted without correlation IDs at {}:{}",
                        caller.file(),
                        caller.line()
                    ),
                )
                .with_field("call_site_file", caller.file())
                .with_field("call_site_line", caller.line().to_string());
                let _ = self.sink.emit(&warn_rec);
            }
        }

        let mut map = BTreeMap::new();
        for (k, v) in fields {
            map.insert((*k).to_string(), (*v).to_string());
        }
        record.fields = map;

        // Route + observe.
        let _ = self.sink.emit(&record);
        self.metrics.observe(&record);
        let alerts = self
            .alerter
            .lock()
            .expect("alerter poisoned")
            .observe(&record);

        EmitOutcome {
            emitted: true,
            alerts,
        }
    }

    /// Convenience: INFO.
    #[track_caller]
    pub fn info(&self, ts: i64, message: impl Into<String>) -> EmitOutcome {
        self.log(LogLevel::Info, ts, message, &[])
    }

    /// Convenience: ERROR.
    #[track_caller]
    pub fn error(&self, ts: i64, message: impl Into<String>) -> EmitOutcome {
        self.log(LogLevel::Error, ts, message, &[])
    }

    /// Convenience: WARN.
    #[track_caller]
    pub fn warn(&self, ts: i64, message: impl Into<String>) -> EmitOutcome {
        self.log(LogLevel::Warn, ts, message, &[])
    }

    /// Convenience: DEBUG.
    #[track_caller]
    pub fn debug(&self, ts: i64, message: impl Into<String>) -> EmitOutcome {
        self.log(LogLevel::Debug, ts, message, &[])
    }

    /// Convenience: TRACE.
    #[track_caller]
    pub fn trace(&self, ts: i64, message: impl Into<String>) -> EmitOutcome {
        self.log(LogLevel::Trace, ts, message, &[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::level::Environment;
    use crate::observability::sink::AggregatingSink;
    use std::sync::Arc;

    struct SharedSink(Arc<AggregatingSink>);
    impl LogSink for SharedSink {
        fn emit(&self, record: &LogRecord) -> Result<(), String> {
            self.0.emit(record)
        }
    }

    fn logger(sink: Arc<AggregatingSink>, env: Environment) -> Logger {
        Logger::new(
            "test",
            LevelConfig::for_environment(env),
            Box::new(SharedSink(sink)),
        )
    }

    #[test]
    fn below_min_level_is_filtered() {
        let sink = Arc::new(AggregatingSink::new());
        let log = logger(Arc::clone(&sink), Environment::Production); // INFO min
        let out = log.log(LogLevel::Debug, 1000, "noisy", &[]);
        assert!(!out.emitted);
        assert_eq!(sink.len(), 0);
    }

    #[test]
    fn emits_structured_record_with_fields() {
        let sink = Arc::new(AggregatingSink::new());
        let log = logger(Arc::clone(&sink), Environment::Production);
        let out = log.log(LogLevel::Info, 1000, "handled", &[("status", "200")]);
        assert!(out.emitted);
        let recs = sink.snapshot();
        assert_eq!(recs.len(), 1);
        assert_eq!(
            recs[0].fields.get("status").map(|s| s.as_str()),
            Some("200")
        );
    }

    #[test]
    fn context_makes_logs_traceable() {
        let sink = Arc::new(AggregatingSink::new());
        let log = logger(Arc::clone(&sink), Environment::Development);
        let ctx = CorrelationContext::new_root();
        log.set_context(ctx.clone());
        log.info(1000, "a");
        log.info(1001, "b");
        let recs = sink.snapshot();
        assert!(recs.iter().all(|r| r.is_traceable()));
        // Both logs share the request's trace id → traceable across the flow.
        assert!(recs
            .iter()
            .all(|r| r.trace_id.as_deref() == Some(ctx.trace_id.as_str())));
    }

    #[test]
    fn metrics_and_alerts_are_driven_by_logs() {
        let sink = Arc::new(AggregatingSink::new());
        let log = logger(Arc::clone(&sink), Environment::Development);
        log.error(1000, "thread panic in worker");
        assert_eq!(log.metrics().error, 1);
        // The default panic pattern alert should have fired.
        let out = log.error(1001, "another panic occurred");
        assert!(out.alerts.iter().any(|a| a.rule == "pattern:panic"));
    }

    #[tokio::test]
    async fn with_context_populates_ids_from_current_correlation_context() {
        let sink = Arc::new(AggregatingSink::new());
        let log = logger(Arc::clone(&sink), Environment::Development);
        let ctx = CorrelationContext::new_root();
        let expected_trace = ctx.trace_id.clone();
        let expected_span = ctx.span_id.clone();
        let expected_req = ctx.request_id.clone();

        CorrelationContext::scope(ctx, async {
            // Emitting using logger.with_context() should automatically bind context from task-local
            log.with_context().info(1000, "auto correlated message");
        })
        .await;

        let recs = sink.snapshot();
        assert_eq!(recs.len(), 1);
        let rec = &recs[0];
        assert!(rec.is_traceable());
        assert_eq!(rec.trace_id.as_deref(), Some(expected_trace.as_str()));
        assert_eq!(rec.span_id.as_deref(), Some(expected_span.as_str()));
        assert_eq!(rec.request_id.as_deref(), Some(expected_req.as_str()));
    }

    #[test]
    fn missing_correlation_ids_increments_warning_and_emits_call_site_warning() {
        let sink = Arc::new(AggregatingSink::new());
        let log = logger(Arc::clone(&sink), Environment::Development)
            .with_emit_warning_records(true);

        assert_eq!(log.missing_correlation_warnings(), 0);
        log.info(1000, "untraceable log");
        assert_eq!(log.missing_correlation_warnings(), 1);

        let recs = sink.snapshot();
        // 1 original log + 1 warning record
        assert_eq!(recs.len(), 2);
        let warn_rec = recs.iter().find(|r| r.level == LogLevel::Warn).expect("warning record emitted");
        assert!(warn_rec.message.contains("LogRecord emitted without correlation IDs"));
        assert!(warn_rec.fields.contains_key("call_site_file"));
        assert!(warn_rec.fields.contains_key("call_site_line"));
    }
}
