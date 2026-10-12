//! Axum middleware for correlation context propagation across HTTP services.
//!
//! Initializes `CorrelationContext` at the beginning of each HTTP request:
//! - Extracts or generates `request_id` (from `x-request-id` or UUID v4).
//! - Extracts incoming W3C `traceparent` header if present, or creates a root trace context.
//! - Stores `CorrelationContext` in request extensions for handlers to access.
//! - Executes downstream handlers within `CorrelationContext::scope`, making it available to
//!   `CorrelationContext::current()` and `Logger::with_context()`.
//! - Attaches `x-request-id` and W3C `traceparent` headers to outgoing HTTP responses.

use axum::{
    extract::Request,
    http::HeaderValue,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::observability::context::CorrelationContext;
use crate::observability::tracing::extract_traceparent;

/// Header name for incoming/outgoing request correlation IDs.
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Header name for W3C distributed trace context propagation.
pub const TRACEPARENT_HEADER: &str = "traceparent";

/// Axum middleware function that initializes `CorrelationContext` for each request.
pub async fn correlation_middleware(mut request: Request, next: Next) -> Response {
    let headers = request.headers();

    // 1. Resolve request ID
    let request_id = headers
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // 2. Extract traceparent or initialize new root context
    let context = headers
        .get(TRACEPARENT_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(|tp| extract_traceparent(tp, &request_id))
        .unwrap_or_else(|| {
            let mut root = CorrelationContext::new_root();
            root.request_id = request_id.clone();
            root
        });

    // 3. Insert into request extensions
    request.extensions_mut().insert(context.clone());

    let trace_id = context.trace_id.clone();
    let span_id = context.span_id.clone();
    let resp_request_id = context.request_id.clone();

    // 4. Run downstream handler within the scoped task-local correlation context
    let mut response = CorrelationContext::scope(context, next.run(request)).await;

    // 5. Populate response headers
    if let Ok(val) = HeaderValue::from_str(&resp_request_id) {
        response.headers_mut().insert(REQUEST_ID_HEADER, val);
    }
    let traceparent_val = format!("00-{}-{}-01", trace_id, span_id);
    if let Ok(val) = HeaderValue::from_str(&traceparent_val) {
        response.headers_mut().insert(TRACEPARENT_HEADER, val);
    }

    response
}
