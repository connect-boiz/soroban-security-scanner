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

#[tokio::test]
async fn http_handler_populates_correlation_context_and_emits_correlated_logs() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use tower::ServiceExt;

    let sink = Arc::new(AggregatingSink::new());
    let log = logger(Arc::clone(&sink));
    let log_state = Arc::new(log);

    let app = axum::Router::new()
        .route(
            "/api/test-scan",
            get({
                let l = Arc::clone(&log_state);
                move || {
                    let l = Arc::clone(&l);
                    async move {
                        l.with_context().info(1001, "scan processing in handler");
                        "ok"
                    }
                }
            }),
        )
        .layer(axum::middleware::from_fn(correlation_middleware));

    let traceparent_input = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
    let req = Request::builder()
        .uri("/api/test-scan")
        .header("x-request-id", "req-xyz-789")
        .header("traceparent", traceparent_input)
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify response headers
    let resp_req_id = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok());
    assert_eq!(resp_req_id, Some("req-xyz-789"));

    let resp_tp = resp
        .headers()
        .get("traceparent")
        .and_then(|v| v.to_str().ok())
        .expect("traceparent header on response");
    assert!(resp_tp.starts_with("00-4bf92f3577b34da6a3ce929d0e0e4736-"));

    // Verify record in sink was correlated with request
    let recs = sink.snapshot();
    assert_eq!(recs.len(), 1);
    let record = &recs[0];
    assert!(record.is_traceable());
    assert_eq!(
        record.trace_id.as_deref(),
        Some("4bf92f3577b34da6a3ce929d0e0e4736")
    );
    assert_eq!(record.request_id.as_deref(), Some("req-xyz-789"));
    assert_eq!(record.message, "scan processing in handler");
}

#[tokio::test]
async fn spawned_async_tasks_inherit_parent_correlation_context() {
    let sink = Arc::new(AggregatingSink::new());
    let log = Arc::new(logger(Arc::clone(&sink)));

    let parent_ctx = CorrelationContext::new_root();
    let expected_trace = parent_ctx.trace_id.clone();
    let expected_req = parent_ctx.request_id.clone();

    CorrelationContext::scope(parent_ctx, async {
        // Log 1: In parent
        log.with_context().info(1000, "parent log");

        // Log 2: In spawn_correlated
        let log_clone2 = Arc::clone(&log);
        let h1 = spawn_correlated(async move {
            log_clone2.with_context().info(1001, "task 1 log");
        });

        // Log 3: In tokio::spawn with CorrelatedFutureExt
        let log_clone3 = Arc::clone(&log);
        let fut = async move {
            log_clone3.with_context().info(1002, "task 2 log");
        };
        let h2 = tokio::spawn(fut.with_current_context());

        // Log 4: In spawn_blocking_correlated
        let log_clone4 = Arc::clone(&log);
        let h3 = spawn_blocking_correlated(move || {
            log_clone4.with_context().info(1003, "blocking task log");
        });

        h1.await.unwrap();
        h2.await.unwrap();
        h3.await.unwrap();
    })
    .await;

    let recs = sink.snapshot();
    assert_eq!(recs.len(), 4);
    for r in &recs {
        assert!(r.is_traceable(), "all task logs must be traceable");
        assert_eq!(
            r.trace_id.as_deref(),
            Some(expected_trace.as_str()),
            "trace_id must match parent"
        );
        assert_eq!(
            r.request_id.as_deref(),
            Some(expected_req.as_str()),
            "request_id must match parent"
        );
    }
}

#[tokio::test]
async fn correlation_logs_endpoint_returns_ordered_matching_records() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let sink = Arc::new(AggregatingSink::new());

    // Inject out-of-order records for req-alpha
    let mut rec_alpha_2 = LogRecord::new(2000, LogLevel::Info, "engine", "step 2 finished");
    rec_alpha_2.request_id = Some("req-alpha".to_string());
    rec_alpha_2.trace_id = Some("trace-alpha".to_string());
    rec_alpha_2.span_id = Some("span-2".to_string());
    sink.emit(&rec_alpha_2).unwrap();

    let mut rec_alpha_1 = LogRecord::new(1000, LogLevel::Info, "engine", "step 1 started");
    rec_alpha_1.request_id = Some("req-alpha".to_string());
    rec_alpha_1.trace_id = Some("trace-alpha".to_string());
    rec_alpha_1.span_id = Some("span-1".to_string());
    sink.emit(&rec_alpha_1).unwrap();

    // Inject record for different request
    let mut rec_beta = LogRecord::new(1500, LogLevel::Info, "engine", "other request");
    rec_beta.request_id = Some("req-beta".to_string());
    sink.emit(&rec_beta).unwrap();

    let app = build_observability_routes(Arc::clone(&sink));

    // 1. Query req-alpha: should return 2 records ordered by timestamp (1000 then 2000)
    let req = Request::builder()
        .uri("/api/v1/observability/correlation-logs?request_id=req-alpha")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let records: Vec<LogRecord> = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].timestamp, 1000);
    assert_eq!(records[0].message, "step 1 started");
    assert_eq!(records[1].timestamp, 2000);
    assert_eq!(records[1].message, "step 2 finished");

    // 2. Query unknown request ID: returns empty list
    let req_unknown = Request::builder()
        .uri("/api/v1/observability/correlation-logs?request_id=req-unknown")
        .body(Body::empty())
        .unwrap();

    let resp_unknown = app.clone().oneshot(req_unknown).await.unwrap();
    assert_eq!(resp_unknown.status(), StatusCode::OK);
    let body_unknown = resp_unknown.into_body().collect().await.unwrap().to_bytes();
    let records_unknown: Vec<LogRecord> = serde_json::from_slice(&body_unknown).unwrap();
    assert!(records_unknown.is_empty());

    // 3. Query without request_id or empty: returns 400 Bad Request
    let req_bad = Request::builder()
        .uri("/api/v1/observability/correlation-logs?request_id=")
        .body(Body::empty())
        .unwrap();

    let resp_bad = app.oneshot(req_bad).await.unwrap();
    assert_eq!(resp_bad.status(), StatusCode::BAD_REQUEST);
}
