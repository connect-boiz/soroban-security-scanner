//! Prometheus metrics, structured request logging, and `/health` endpoint.
//!
//! Exposes:
//! - `GET /health`   — liveness / readiness probe (JSON)
//! - `GET /metrics`  — Prometheus text exposition
//!
//! Tracked counters / histograms:
//! - `http_requests_total{method, path, status}`
//! - `http_request_duration_seconds{method, path}`
//! - `scan_operations_total{type, status}`

use axum::{
    body::Body,
    extract::MatchedPath,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use metrics::{counter, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use serde::Serialize;
use std::time::Instant;
use tokio::time::Duration;

// ---------------------------------------------------------------------------
// Prometheus setup
// ---------------------------------------------------------------------------

/// Install the global Prometheus recorder and return the scrape handle.
///
/// Call once at startup before building the Axum router.
pub fn install_prometheus_recorder() -> PrometheusHandle {
    PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install Prometheus recorder")
}

/// Axum handler: `GET /metrics`
pub async fn metrics_handler(
    axum::Extension(handle): axum::Extension<PrometheusHandle>,
) -> impl IntoResponse {
    handle.render()
}

// ---------------------------------------------------------------------------
// Health endpoint
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status:  &'static str,
    pub version: &'static str,
    pub uptime_seconds: u64,
}

/// Axum handler: `GET /health`
pub async fn health_handler(
    axum::Extension(started_at): axum::Extension<std::time::Instant>,
) -> impl IntoResponse {
    let uptime = started_at.elapsed().as_secs();
    Json(HealthResponse {
        status:         "ok",
        version:        env!("CARGO_PKG_VERSION"),
        uptime_seconds: uptime,
    })
}

// ---------------------------------------------------------------------------
// Request logging + metrics middleware
// ---------------------------------------------------------------------------

/// Tower middleware that:
/// 1. Increments `http_requests_total`
/// 2. Records `http_request_duration_seconds`
/// 3. Emits a structured `tracing::info!` log for every request
pub async fn track_request_metrics(
    req:  Request<Body>,
    next: Next,
) -> Response {
    let method = req.method().to_string();
    let path   = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_owned())
        .unwrap_or_else(|| req.uri().path().to_owned());

    let start = Instant::now();
    let resp  = next.run(req).await;
    let elapsed = start.elapsed();

    let status = resp.status().as_u16().to_string();
    let duration_secs = elapsed.as_secs_f64();

    // Prometheus counters
    counter!("http_requests_total",
        "method" => method.clone(),
        "path"   => path.clone(),
        "status" => status.clone()
    ).increment(1);

    histogram!("http_request_duration_seconds",
        "method" => method.clone(),
        "path"   => path.clone()
    ).record(duration_secs);

    // Structured access log
    tracing::info!(
        method  = %method,
        path    = %path,
        status  = %status,
        duration_ms = duration_secs * 1000.0,
        "request"
    );

    resp
}

/// Increment the `scan_operations_total` counter from scan-related handlers.
pub fn record_scan_op(scan_type: &'static str, status: &'static str) {
    counter!("scan_operations_total",
        "type"   => scan_type,
        "status" => status
    ).increment(1);
}

/// Increment the `wallet_operations_total` counter from wallet handlers.
pub fn record_wallet_op() {
    counter!("wallet_operations_total").increment(1);
}
