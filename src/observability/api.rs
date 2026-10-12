//! HTTP API endpoints for observability and log correlation.
//!
//! Provides `GET /api/v1/observability/correlation-logs?request_id=<id>` to retrieve all log
//! records associated with a specific request ID, ordered by timestamp ascending.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::observability::record::LogRecord;
use crate::observability::sink::AggregatingSink;

/// Query parameters for fetching correlated logs.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CorrelationLogsQuery {
    /// Target request ID to query.
    pub request_id: String,
}

/// Shared state for the observability API router.
#[derive(Clone)]
pub struct ObservabilityApiState {
    /// Shared log sink storing structured records.
    pub sink: Arc<AggregatingSink>,
}

impl ObservabilityApiState {
    /// Creates a new state wrapping an aggregating sink.
    pub fn new(sink: Arc<AggregatingSink>) -> Self {
        Self { sink }
    }
}

/// Builds an Axum router serving `/api/v1/observability/correlation-logs`.
pub fn build_observability_routes(sink: Arc<AggregatingSink>) -> Router {
    Router::new()
        .route(
            "/api/v1/observability/correlation-logs",
            get(get_correlation_logs),
        )
        .with_state(ObservabilityApiState::new(sink))
}

/// Handler for `GET /api/v1/observability/correlation-logs?request_id=<id>`.
pub async fn get_correlation_logs(
    State(state): State<ObservabilityApiState>,
    Query(query): Query<CorrelationLogsQuery>,
) -> Result<Json<Vec<LogRecord>>, StatusCode> {
    if query.request_id.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let records = state.sink.snapshot();
    let mut matching: Vec<LogRecord> = records
        .into_iter()
        .filter(|rec| rec.request_id.as_deref() == Some(query.request_id.trim()))
        .collect();

    matching.sort_by_key(|r| r.timestamp);

    Ok(Json(matching))
}
