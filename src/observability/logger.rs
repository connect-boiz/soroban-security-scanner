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

/// Outcome of an emit: whether it was recorded and any alerts it triggered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EmitOutcome {
    /// False when filtered out by the level config.
    pub emitted: bool,
    /// Alerts triggered by this record.
    pub alerts: Vec<LogAlert>,
}

/// A structured logger bound to a sink, level policy and correlation context.
pub struct Logger {
    target: String,
    level: LevelConfig,
    sink: Box<dyn LogSink>,
    metrics: LogMetricsCollector,
    alerter: Mutex<LogAlerter>,
    context: Mutex<Option<CorrelationContext>>,
}

impl Logger {
    /// Builds a logger for `target` writing to `sink`.
    pub fn new(target: impl Into<String>, level: LevelConfig, sink: Box<dyn LogSink>) -> Self {
        Self {
            target: target.into(),
            level,
            sink,
            metrics: LogMetricsCollector::new(),
            alerter: Mutex::new(LogAlerter::new(AlertConfig::default())),
            context: Mutex::new(None),
        }
    }

    /// Overrides the alerting configuration.
    pub fn with_alert_config(self, config: AlertConfig) -> Self {
        *self.alerter.lock().expect("alerter poisoned") = LogAlerter::new(config);
        self
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
        if let Some(ctx) = self.context.lock().expect("context poisoned").as_ref() {
            record = record.with_context(ctx);
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
    pub fn info(&self, ts: i64, message: impl Into<String>) -> EmitOutcome {
        self.log(LogLevel::Info, ts, message, &[])
    }

    /// Convenience: ERROR.
    pub fn error(&self, ts: i64, message: impl Into<String>) -> EmitOutcome {
        self.log(LogLevel::Error, ts, message, &[])
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
}
