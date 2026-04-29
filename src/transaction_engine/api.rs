//! Transaction processing engine API endpoints

use crate::transaction_engine::{
    Transaction, TransactionType, TransactionPriority, TransactionFilter,
    TransactionQueue, TransactionResult, QueueStats, ProcessorMetrics,
    RetryStats, TransactionMonitor, DashboardData, StateManager
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::Result;

/// API state
#[derive(Clone)]
pub struct ApiState {
    pub queue: Arc<TransactionQueue>,
    pub monitor: Arc<TransactionMonitor>,
    pub state_manager: Arc<StateManager>,
}

/// API request/response types
#[derive(Debug, Deserialize)]
pub struct CreateTransactionRequest {
    pub transaction_type: TransactionType,
    pub data: Vec<u8>,
    pub submitter: String,
    pub network: String,
    pub priority: Option<TransactionPriority>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub custom_fields: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
pub struct CreateTransactionResponse {
    pub transaction_id: Uuid,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct TransactionListQuery {
    pub state: Option<String>,
    pub transaction_type: Option<String>,
    pub priority: Option<String>,
    pub submitter: Option<String>,
    pub network: Option<String>,
    pub created_after: Option<String>,
    pub created_before: Option<String>,
    pub tags: Option<Vec<String>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct RetryTransactionRequest {
    pub transaction_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CancelTransactionRequest {
    pub transaction_id: Uuid,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: Utc::now(),
        }
    }
}

/// Create API router
pub fn create_router(state: ApiState) -> Router {
    Router::new()
        // Transaction endpoints
        .route("/transactions", post(create_transaction))
        .route("/transactions", get(list_transactions))
        .route("/transactions/:id", get(get_transaction))
        .route("/transactions/:id/retry", post(retry_transaction))
        .route("/transactions/:id/cancel", post(cancel_transaction))
        
        // Queue endpoints
        .route("/queue/stats", get(get_queue_stats))
        .route("/queue/retryable", get(get_retryable_transactions))
        .route("/queue/cleanup", post(cleanup_queue))
        
        // Monitoring endpoints
        .route("/monitoring/snapshot", get(get_monitoring_snapshot))
        .route("/monitoring/history", get(get_monitoring_history))
        .route("/monitoring/alerts", get(get_alerts))
        .route("/monitoring/health", get(get_system_health))
        .route("/monitoring/dashboard", get(get_dashboard))
        
        // Metrics endpoints
        .route("/metrics/processors", get(get_processor_metrics))
        .route("/metrics/retries", get(get_retry_stats))
        .route("/metrics/transactions", get(get_transaction_stats))
        
        // State management endpoints
        .route("/state/export", post(export_state))
        .route("/state/import", post(import_state))
        .route("/state/save", post(save_state))
        
        .with_state(state)
}

/// Create a new transaction
async fn create_transaction(
    State(state): State<ApiState>,
    Json(request): Json<CreateTransactionRequest>,
) -> Result<Json<ApiResponse<CreateTransactionResponse>>, StatusCode> {
    let mut transaction = Transaction::new(
        request.transaction_type,
        request.data,
        request.submitter,
        request.network,
    );

    // Apply optional fields
    if let Some(priority) = request.priority {
        transaction.priority = priority;
    }
    
    if let Some(description) = request.description {
        transaction.metadata.description = Some(description);
    }
    
    if let Some(tags) = request.tags {
        transaction.metadata.tags = tags;
    }
    
    if let Some(custom_fields) = request.custom_fields {
        transaction.metadata.custom_fields = custom_fields;
    }

    // Add to queue
    match state.queue.enqueue(transaction).await {
        Ok(()) => {
            let response = CreateTransactionResponse {
                transaction_id: transaction.id,
                status: "queued".to_string(),
                message: "Transaction successfully queued".to_string(),
            };
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            Ok(Json(ApiResponse::error(format!("Failed to enqueue transaction: {}", e))))
        }
    }
}

/// List transactions with filtering
async fn list_transactions(
    State(state): State<ApiState>,
    Query(query): Query<TransactionListQuery>,
) -> Result<Json<ApiResponse<Vec<Transaction>>>, StatusCode> {
    let filter = TransactionFilter {
        state: query.state.as_ref().and_then(|s| s.parse().ok()),
        transaction_type: query.transaction_type.as_ref().and_then(|t| t.parse().ok()),
        priority: query.priority.as_ref().and_then(|p| p.parse().ok()),
        submitter: query.submitter,
        network: query.network,
        created_after: query.created_after.as_ref().and_then(|d| DateTime::parse_from_rfc3339(d).map(|dt| dt.with_timezone(&Utc)).ok()),
        created_before: query.created_before.as_ref().and_then(|d| DateTime::parse_from_rfc3339(d).map(|dt| dt.with_timezone(&Utc)).ok()),
        tags: query.tags,
        limit: query.limit,
        offset: query.offset,
    };

    match state.queue.get_transactions(&filter).await {
        Ok(transactions) => Ok(Json(ApiResponse::success(transactions))),
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to get transactions: {}", e)))),
    }
}

/// Get specific transaction
async fn get_transaction(
    State(state): State<ApiState>,
    Path(transaction_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Transaction>>, StatusCode> {
    match state.queue.get_transaction(&transaction_id).await {
        Ok(Some(transaction)) => Ok(Json(ApiResponse::success(transaction))),
        Ok(None) => Ok(Json(ApiResponse::error("Transaction not found".to_string()))),
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to get transaction: {}", e)))),
    }
}

/// Retry a transaction
async fn retry_transaction(
    State(state): State<ApiState>,
    Path(transaction_id): Path<Uuid>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    match state.queue.requeue_transaction(&transaction_id).await {
        Ok(()) => Ok(Json(ApiResponse::success("Transaction queued for retry".to_string()))),
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to retry transaction: {}", e)))),
    }
}

/// Cancel a transaction
async fn cancel_transaction(
    State(state): State<ApiState>,
    Path(transaction_id): Path<Uuid>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    match state.queue.cancel_transaction(transaction_id).await {
        Ok(true) => Ok(Json(ApiResponse::success("Transaction cancelled".to_string()))),
        Ok(false) => Ok(Json(ApiResponse::error("Transaction not found or cannot be cancelled".to_string()))),
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to cancel transaction: {}", e)))),
    }
}

/// Get queue statistics
async fn get_queue_stats(
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<QueueStats>>, StatusCode> {
    let stats = state.queue.get_stats().await;
    Ok(Json(ApiResponse::success(stats)))
}

/// Get retryable transactions
async fn get_retryable_transactions(
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<Vec<Transaction>>>, StatusCode> {
    match state.queue.get_retryable_transactions().await {
        Ok(transactions) => Ok(Json(ApiResponse::success(transactions))),
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to get retryable transactions: {}", e)))),
    }
}

/// Cleanup old transactions
async fn cleanup_queue(
    State(state): State<ApiState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let max_age_hours = params.get("max_age_hours")
        .and_then(|h| h.parse::<u64>().ok())
        .unwrap_or(24);
    
    let max_age = chrono::Duration::hours(max_age_hours as i64);
    
    match state.queue.cleanup_old_transactions(max_age).await {
        Ok(removed) => Ok(Json(ApiResponse::success(format!("Removed {} old transactions", removed)))),
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to cleanup queue: {}", e)))),
    }
}

/// Get current monitoring snapshot
async fn get_monitoring_snapshot(
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<crate::transaction_engine::monitoring::MonitoringSnapshot>>, StatusCode> {
    match state.monitor.get_current_snapshot().await {
        Ok(Some(snapshot)) => Ok(Json(ApiResponse::success(snapshot))),
        Ok(None) => Ok(Json(ApiResponse::error("No monitoring data available".to_string()))),
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to get monitoring snapshot: {}", e)))),
    }
}

/// Get monitoring history
async fn get_monitoring_history(
    State(state): State<ApiState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<crate::transaction_engine::monitoring::MonitoringSnapshot>>>, StatusCode> {
    let limit = params.get("limit").and_then(|l| l.parse::<usize>().ok());
    
    match state.monitor.get_metrics_history(limit).await {
        Ok(history) => Ok(Json(ApiResponse::success(history))),
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to get monitoring history: {}", e)))),
    }
}

/// Get active alerts
async fn get_alerts(
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<Vec<crate::transaction_engine::monitoring::Alert>>>, StatusCode> {
    match state.monitor.get_active_alerts().await {
        Ok(alerts) => Ok(Json(ApiResponse::success(alerts))),
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to get alerts: {}", e)))),
    }
}

/// Get system health
async fn get_system_health(
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<crate::transaction_engine::monitoring::SystemHealth>>, StatusCode> {
    match state.monitor.get_system_health().await {
        Ok(health) => Ok(Json(ApiResponse::success(health))),
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to get system health: {}", e)))),
    }
}

/// Get dashboard data
async fn get_dashboard(
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<DashboardData>>, StatusCode> {
    let dashboard_provider = crate::transaction_engine::monitoring::DashboardProvider::new(state.monitor.clone());
    
    match dashboard_provider.get_dashboard_data().await {
        Ok(dashboard) => Ok(Json(ApiResponse::success(dashboard))),
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to get dashboard data: {}", e)))),
    }
}

/// Get processor metrics
async fn get_processor_metrics(
    State(_state): State<ApiState>,
) -> Result<Json<ApiResponse<Vec<ProcessorMetrics>>>, StatusCode> {
    // This would need to be connected to the actual processor manager
    Ok(Json(ApiResponse::success(Vec::new())))
}

/// Get retry statistics
async fn get_retry_stats(
    State(_state): State<ApiState>,
) -> Result<Json<ApiResponse<RetryStats>>, StatusCode> {
    // This would need to be connected to the actual retry manager
    Ok(Json(ApiResponse::success(RetryStats::default())))
}

/// Get transaction statistics
async fn get_transaction_stats(
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<crate::transaction_engine::state::TransactionStats>>, StatusCode> {
    let stats = state.state_manager.get_transaction_stats().await;
    Ok(Json(ApiResponse::success(stats)))
}

/// Export state
async fn export_state(
    State(state): State<ApiState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let backup_path = params.get("path")
        .cloned()
        .unwrap_or_else(|| "transaction_state_backup.json".to_string());
    
    match state.state_manager.export_backup(&backup_path).await {
        Ok(()) => Ok(Json(ApiResponse::success(format!("State exported to {}", backup_path)))),
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to export state: {}", e)))),
    }
}

/// Import state
async fn import_state(
    State(state): State<ApiState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let backup_path = params.get("path")
        .ok_or_else(|| StatusCode::BAD_REQUEST)?
        .clone();
    
    match state.state_manager.import_backup(&backup_path).await {
        Ok(()) => Ok(Json(ApiResponse::success(format!("State imported from {}", backup_path)))),
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to import state: {}", e)))),
    }
}

/// Save state
async fn save_state(
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    match state.state_manager.save_state().await {
        Ok(()) => Ok(Json(ApiResponse::success("State saved successfully".to_string()))),
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to save state: {}", e)))),
    }
}

/// API documentation
pub fn get_api_docs() -> String {
    r#"
# Transaction Processing Engine API

## Transaction Endpoints

### POST /transactions
Create a new transaction
Request Body:
```json
{
  "transaction_type": "Payment|MultiSignature|ContractDeployment|ContractCall|BatchOperation|SecurityScan|Custom",
  "data": "base64_encoded_data",
  "submitter": "string",
  "network": "string",
  "priority": "Low|Normal|High|Critical|Emergency",
  "description": "string (optional)",
  "tags": ["string"] (optional),
  "custom_fields": {"key": "value"} (optional)
}
```

### GET /transactions
List transactions with filtering
Query Parameters:
- state: Transaction state filter
- transaction_type: Transaction type filter
- priority: Priority filter
- submitter: Submitter filter
- network: Network filter
- created_after: ISO datetime filter
- created_before: ISO datetime filter
- tags: Tag filter
- limit: Result limit
- offset: Result offset

### GET /transactions/{id}
Get specific transaction by ID

### POST /transactions/{id}/retry
Retry a failed transaction

### POST /transactions/{id}/cancel
Cancel a transaction

## Queue Endpoints

### GET /queue/stats
Get queue statistics

### GET /queue/retryable
Get retryable failed transactions

### POST /queue/cleanup
Cleanup old transactions
Query Parameters:
- max_age_hours: Maximum age in hours (default: 24)

## Monitoring Endpoints

### GET /monitoring/snapshot
Get current monitoring snapshot

### GET /monitoring/history
Get monitoring history
Query Parameters:
- limit: Number of snapshots to return

### GET /monitoring/alerts
Get active alerts

### GET /monitoring/health
Get system health status

### GET /monitoring/dashboard
Get dashboard data

## Metrics Endpoints

### GET /metrics/processors
Get processor metrics

### GET /metrics/retries
Get retry statistics

### GET /metrics/transactions
Get transaction statistics

## State Management Endpoints

### POST /state/export
Export state to backup
Query Parameters:
- path: Backup file path

### POST /state/import
Import state from backup
Query Parameters:
- path: Backup file path

### POST /state/save
Save current state

## Response Format

All responses follow this format:
```json
{
  "success": boolean,
  "data": T | null,
  "error": string | null,
  "timestamp": "ISO datetime"
}
```
"#.to_string()
}
