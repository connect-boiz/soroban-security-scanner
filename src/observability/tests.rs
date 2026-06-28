//! End-to-end integration tests: a request flows across two services with full
//! trace correlation, structured logs are aggregated/indexed/searched, metrics
//! and alerts are derived, retention archives old logs, and log reads are
//! access-controlled.

use super::*;
use std::sync::Arc;

struct SharedSink(Arc<AggregatingSink>);
impl LogSink for SharedSink {
    fn emit(&self, record: &LogRecord) -> Result<(), String> {
        self.0.emit(record)
    }
}

fn logger(sink: Arc<AggregatingSink>) -> Logger {
    Logger::new(
        "service-a",
        LevelConfig::for_environment(Environment::Development),
        Box::new(SharedSink(sink)),
    )
}

#[test]
fn full_request_is_traceable_across_services() {
    let sink = Arc::new(AggregatingSink::new());
    let log = logger(Arc::clone(&sink));

    // Service A starts a root span and logs under its context.
    let span_a = Span::root("POST /api/scan", 1000);
    log.set_context(span_a.context.clone());
    log.info(1000, "received scan request");

    // A calls service B, propagating context via traceparent.
    let header = span_a.traceparent();
    let ctx_b = extract_traceparent(&header, span_a.context.request_id.clone()).unwrap();
    let span_b = Span::with_context("analyze_contract", ctx_b, 1001);

    // Service B logs under the propagated context (same logger here for the test).
    log.set_context(span_b.context.clone());
    log.info(1001, "analyzing contract");
    log.info(1002, "analysis complete");

    // Every emitted log is traceable, and all share ONE trace id → 100%
    // traceability across the request, even across the service hop.
    let recs = sink.snapshot();
    assert_eq!(recs.len(), 3);
    assert!(recs.iter().all(|r| r.is_traceable()));
    let trace = span_a.context.trace_id.clone();
    assert!(recs
        .iter()
        .all(|r| r.trace_id.as_deref() == Some(trace.as_str())));
}

#[test]
fn aggregated_logs_are_indexed_and_searchable() {
    let sink = Arc::new(AggregatingSink::new());
    let log = logger(Arc::clone(&sink));
    let ctx = CorrelationContext::new_root();
    log.set_context(ctx.clone());
    log.log(LogLevel::Info, 1000, "ok", &[("status", "200")]);
    log.log(LogLevel::Error, 1001, "db failure", &[("status", "500")]);

    // Build a search index from the aggregated records.
    let mut index = LogIndex::new();
    for rec in sink.snapshot() {
        index.index(rec);
    }
    // Query by trace → entire request; by level → just the error.
    assert_eq!(
        index
            .search(&LogQuery::new().trace(ctx.trace_id.clone()))
            .len(),
        2
    );
    let errors = index.search(&LogQuery::new().level(LogLevel::Error));
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].fields.get("status").map(|s| s.as_str()),
        Some("500")
    );
}

#[test]
fn metrics_and_alerting_from_log_stream() {
    let sink = Arc::new(AggregatingSink::new());
    let log = logger(Arc::clone(&sink));

    log.info(1000, "healthy");
    let out = log.error(1001, "thread panic during scan");
    // Error metric incremented and the panic pattern alert fired.
    assert_eq!(log.metrics().error, 1);
    assert!(out.alerts.iter().any(|a| a.rule == "pattern:panic"));
}

#[test]
fn retention_archives_old_logs() {
    let sink = AggregatingSink::new();
    let archive = InMemoryArchive::new();
    let policy = RetentionPolicy::default();
    let now = 100 * 24 * 3600;

    sink.emit(&LogRecord::new(now - 1000, LogLevel::Info, "t", "fresh"))
        .unwrap();
    sink.emit(&LogRecord::new(
        now - 30 * 24 * 3600,
        LogLevel::Info,
        "t",
        "old",
    ))
    .unwrap();

    let (archived, _) = apply_retention(&sink, &archive, &policy, now);
    assert_eq!(archived, 1);
    assert_eq!(sink.len(), 1);
    assert_eq!(archive.len(), 1);
}

#[test]
fn log_reads_are_access_controlled_and_audited() {
    let sink = AggregatingSink::new();
    sink.emit(&LogRecord::new(1000, LogLevel::Info, "t", "ok"))
        .unwrap();
    sink.emit(&LogRecord::new(
        1001,
        LogLevel::Error,
        "t",
        "secret error detail",
    ))
    .unwrap();

    // Operators see everything; developers are shielded from error detail;
    // unauthorized readers get nothing. Every read is audited.
    assert_eq!(sink.read("op", LogReaderRole::Operator).len(), 2);
    assert_eq!(sink.read("dev", LogReaderRole::Developer).len(), 1);
    assert_eq!(sink.read("nobody", LogReaderRole::Unauthorized).len(), 0);
    assert_eq!(sink.access_log().len(), 3);
    assert!(sink.access_log().iter().any(|a| !a.granted));
}

#[test]
fn production_defaults_suppress_debug_noise() {
    let sink = Arc::new(AggregatingSink::new());
    let log = Logger::new(
        "svc",
        LevelConfig::for_environment(Environment::Production),
        Box::new(SharedSink(Arc::clone(&sink))),
    );
    assert!(!log.log(LogLevel::Debug, 1000, "debug noise", &[]).emitted);
    assert!(log.log(LogLevel::Info, 1001, "info kept", &[]).emitted);
    assert_eq!(sink.len(), 1);
}
