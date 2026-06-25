//! Distributed tracing and structured logging setup.
//!
//! Configures OpenTelemetry + tracing-subscriber so every request
//! carries a `correlation_id` (W3C TraceContext or generated UUID)
//! through the full call stack, across async boundaries, and into
//! any child services.
//!
//! Emits JSON-structured logs compatible with log aggregation platforms
//! (Loki, Datadog, Cloud Logging).

use opentelemetry::{
    global,
    sdk::{trace as sdktrace, Resource},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Initialisation
// ---------------------------------------------------------------------------

/// Initialise the global tracing subscriber with JSON output and
/// optional OpenTelemetry OTLP export.
///
/// # Arguments
/// * `service_name`  – Reported in OTLP spans (e.g. `"soroban-scanner"`).
/// * `otlp_endpoint` – OTLP gRPC endpoint (e.g. `"http://collector:4317"`).
///                     Pass `None` to disable OTLP export.
pub fn init_tracing(
    service_name:  &'static str,
    otlp_endpoint: Option<&str>,
) -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let json_layer = fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(true);

    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(json_layer);

    if let Some(endpoint) = otlp_endpoint {
        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(endpoint),
            )
            .with_trace_config(
                sdktrace::config().with_resource(Resource::new(vec![
                    KeyValue::new("service.name", service_name),
                ])),
            )
            .install_batch(opentelemetry::runtime::Tokio)?;

        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        subscriber.with(otel_layer).try_init()?;
    } else {
        subscriber.try_init()?;
    }

    Ok(())
}

/// Shuts down the OTLP tracer, flushing all pending spans.
pub fn shutdown_tracing() {
    global::shutdown_tracer_provider();
}

// ---------------------------------------------------------------------------
// Correlation ID middleware
// ---------------------------------------------------------------------------

use axum::{
    body::Body,
    http::{HeaderValue, Request},
    middleware::Next,
    response::Response,
};

pub const CORRELATION_ID_HEADER: &str = "x-correlation-id";

/// Axum middleware: extracts or generates a `x-correlation-id` and
/// attaches it to the `tracing::Span` for the duration of the request.
pub async fn correlation_id_middleware(
    mut req: Request<Body>,
    next:    Next,
) -> Response {
    let correlation_id = req
        .headers()
        .get(CORRELATION_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Make the correlation ID available to downstream handlers.
    req.extensions_mut().insert(CorrelationId(correlation_id.clone()));

    let span = tracing::info_span!("request", correlation_id = %correlation_id);
    let _guard = span.enter();

    let mut resp = next.run(req).await;

    // Echo the correlation ID back in the response.
    if let Ok(val) = HeaderValue::from_str(&correlation_id) {
        resp.headers_mut().insert(CORRELATION_ID_HEADER, val);
    }

    resp
}

/// Extension type for extracting the correlation ID in handlers.
#[derive(Debug, Clone)]
pub struct CorrelationId(pub String);
