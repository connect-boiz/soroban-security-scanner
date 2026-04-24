use axum::{
    extract::{Json, Path, State},
    http::Request,
    http::StatusCode,
    middleware::{self, Next},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::time::Duration;
use tokio::sync::RwLock;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::trace::TraceLayer;
use tracing::{error, info, Level};
use tracing_subscriber::{EnvFilter, FmtSubscriber};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

use soroban_security_scanner_core::analyzer::SecurityAnalyzer;
use soroban_security_scanner_core::patterns::get_vulnerability_patterns;
use soroban_security_scanner_core::vulnerabilities::VulnerabilityPattern;
use soroban_security_scanner_core::scan_controller::{ScanController, ScanCommand, ScanStatus, ScanControl};

lazy_static! {
    static ref DEV_API_KEY: String = std::env::var("DEV_API_KEY").unwrap_or_else(|_| "dummy-key".to_string());
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
struct ScanRequest {
    code: String,
    filename: String,
    format: Option<String>, // "json" or "sarif"
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
struct CiScanRequest {
    repository_url: String,
    commit_hash: String,
    branch_name: String,
    code: String,
    filename: String,
    #[schema(example = "critical")]
    failure_threshold: Option<String>, // e.g., "critical", "high"
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
struct CiScanResponse {
    tracking_id: String,
    status_url: String,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
enum ScanStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
struct ScanResult {
    status: ScanStatus,
    ci_pass: Option<bool>, // true if passed, false if failed based on threshold
    report: Option<serde_json::Value>, // Can be JSON or SARIF report
    error: Option<String>,
    commit_hash: String,
    branch_name: String,
    repository_url: String,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
struct ScanControlRequest {
    scan_id: String,
    command: ScanCommand,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
struct ScanControlResponse {
    success: bool,
    message: String,
    scan_status: Option<ScanControl>,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
struct ScanStatusResponse {
    scan_id: String,
    status: Option<ScanControl>,
    active_scans: Vec<ScanControl>,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        handle_scan,
        handle_ci_scan,
        get_scan_result,
        get_patterns,
        refresh_patterns,
        handle_scan_control,
        get_scan_status,
        get_active_scans,
    ),
    components(
        schemas(ScanRequest, CiScanRequest, CiScanResponse, ScanResult, ScanStatus, VulnerabilityPattern, ScanControlRequest, ScanControlResponse, ScanStatusResponse, ScanCommand)
    ),
    tags(
        (name = "soroban-security-scanner", description = "Soroban Security Scanner API")
    )
)]
struct ApiDoc;

pub struct AppState {
    patterns: RwLock<Vec<VulnerabilityPattern>>,
    redis_client: Option<redis::Client>,
    scan_controller: ScanController,
}

async fn api_key_auth<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    if let Some(auth_header) = auth_header {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            // In a real app, you'd look up the key in a database
            // and check subscription tiers for rate limiting.
            if token == *DEV_API_KEY {
                return next.run(req).await;
            }
        }
    }

    (StatusCode::UNAUTHORIZED, "Invalid or missing API key").into_response()
}

/// Check if the number of vulnerabilities exceeds a defined threshold.
fn check_failure_threshold(report: &serde_json::Value, threshold: &str) -> bool {
    let severities_to_check: &[&str] = match threshold {
        "critical" => &["error"],
        "high" => &["error"],
        "medium" => &["error", "warning"],
        "low" => &["error", "warning", "note"],
        _ => &["error", "warning", "note", "none"], // Default to any issue causing failure
    };

    if let Some(runs) = report.get("runs").and_then(|r| r.as_array()) {
        for run in runs {
            if let Some(results) = run.get("results").and_then(|r| r.as_array()) {
                for result in results {
                    if let Some(level) = result.get("level").and_then(|l| l.as_str()) {
                        if severities_to_check.contains(&level) {
                            return true; // Failure condition met
                        }
                    }
                }
            }
        }
    }
    false // No failure condition met
}

#[utoipa::path(
    post,
    path = "/scan",
    request_body = ScanRequest,
    responses(
        (status = 200, description = "Scan successful", body = String),
        (status = 500, description = "Scan failed", body = String)
    )
)]
async fn handle_scan(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ScanRequest>,
) -> impl IntoResponse {
    let patterns = state.patterns.read().await;
    let analyzer = SecurityAnalyzer::new(true, true, Some(state.scan_controller.clone())); // Deep analysis enabled with control
    
    match analyzer.analyze_with_control(&payload.code, &payload.filename, true, 5).await {
        Ok(report) => {
            let format = payload.format.unwrap_or_else(|| "json".to_string());
            if format == "sarif" {
                match report.export_sarif() {
                    Ok(sarif) => (StatusCode::OK, sarif).into_response(),
                    Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
                }
            } else {
                match report.export_json() {
                    Ok(json) => (StatusCode::OK, json).into_response(),
                    Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
                }
            }
        }
        Err(e) => {
            error!("Scan failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/ci/scan",
    request_body = CiScanRequest,
    responses(
        (status = 202, description = "Scan accepted for processing", body = CiScanResponse),
        (status = 500, description = "Failed to start scan", body = String)
    ),
    security(
        ("api_key" = [])
    )
)]
async fn handle_ci_scan(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CiScanRequest>,
) -> impl IntoResponse {
    let tracking_id = Uuid::new_v4().to_string();
    let status_url = format!("/api/v1/ci/results/{}", tracking_id);

    let initial_result = ScanResult {
        status: ScanStatus::Pending,
        ci_pass: None,
        report: None,
        error: None,
        commit_hash: payload.commit_hash.clone(),
        branch_name: payload.branch_name.clone(),
        repository_url: payload.repository_url.clone(),
    };

    if let Some(client) = &state.redis_client {
        if let Ok(mut con) = client.get_async_connection().await {
            let _: Result<(), _> = redis::cmd("SET")
                .arg(&tracking_id)
                .arg(serde_json::to_string(&initial_result).unwrap())
                .arg("EX")
                .arg(3600) // Expire in 1 hour
                .query_async(&mut con)
                .await;
        } else {
            error!("Could not connect to Redis to queue scan job.");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Could not connect to Redis").into_response();
        }
    } else {
        error!("Redis not configured, cannot process CI scan.");
        return (StatusCode::INTERNAL_SERVER_ERROR, "Redis not configured").into_response();
    }

    let scan_state = state.clone();
    tokio::spawn(async move {
        info!("Starting background scan for tracking ID: {}", tracking_id);
        // Update status to Running in Redis
        if let Ok(mut con) = scan_state.redis_client.as_ref().unwrap().get_async_connection().await {
            let mut result: ScanResult = initial_result;
            result.status = ScanStatus::Running;
            let _: Result<(), _> = redis::cmd("SET")
                .arg(&tracking_id)
                .arg(serde_json::to_string(&result).unwrap())
                .arg("EX").arg(3600)
                .query_async(&mut con).await;
        }

        let analyzer = SecurityAnalyzer::new(true, true, Some(scan_state.scan_controller.clone()));
        let scan_output = analyzer.analyze_with_control(
            &payload.code, 
            &payload.filename,
            true, // auto_stop_enabled
            3     // auto_stop_threshold (stop after 3 critical vulnerabilities)
        ).await;

        let final_result = match scan_output {
            Ok(report) => {
                let sarif_report = report.export_sarif().unwrap_or_default();
                let report_json: serde_json::Value = serde_json::from_str(&sarif_report).unwrap_or_default();
                let threshold_str = payload.failure_threshold.as_deref().unwrap_or("critical");
                let failed_threshold = check_failure_threshold(&report_json, threshold_str);

                ScanResult {
                    status: ScanStatus::Completed,
                    ci_pass: Some(!failed_threshold),
                    report: Some(report_json),
                    error: None,
                    commit_hash: payload.commit_hash,
                    branch_name: payload.branch_name,
                    repository_url: payload.repository_url,
                }
            }
            Err(e) => {
                error!("Scan failed for tracking ID {}: {}", tracking_id, e);
                ScanResult {
                    status: ScanStatus::Failed,
                    ci_pass: Some(false),
                    report: None,
                    error: Some(e.to_string()),
                    commit_hash: payload.commit_hash,
                    branch_name: payload.branch_name,
                    repository_url: payload.repository_url,
                }
            }
        };

        if let Ok(mut con) = scan_state.redis_client.as_ref().unwrap().get_async_connection().await {
            let _: Result<(), _> = redis::cmd("SET")
                .arg(&tracking_id)
                .arg(serde_json::to_string(&final_result).unwrap())
                .arg("EX").arg(3600)
                .query_async(&mut con).await;
            info!("Finished background scan for tracking ID: {}", tracking_id);
        }
    });

    (StatusCode::ACCEPTED, Json(CiScanResponse { tracking_id, status_url })).into_response()
}

#[utoipa::path(
    get,
    path = "/api/v1/ci/results/{tracking_id}",
    responses(
        (status = 200, description = "Scan result", body = ScanResult),
        (status = 404, description = "Scan not found")
    ),
)]
async fn refresh_patterns(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    info!("Refreshing vulnerability patterns from central repository...");
    // Simulated GitHub pull
    let new_patterns = get_vulnerability_patterns();
    let mut patterns_lock = state.patterns.write().await;
    *patterns_lock = new_patterns.clone();

    // Cache in Redis if available
    if let Some(client) = &state.redis_client {
        if let Ok(mut con) = client.get_async_connection().await {
            let json = serde_json::to_string(&new_patterns).unwrap();
            let _: Result<(), _> = redis::cmd("SET")
                .arg("vulnerability_patterns")
                .arg(json)
                .query_async(&mut con)
                .await;
        }
    }

    StatusCode::OK
}

#[utoipa::path(
    get,
    path = "/patterns",
    responses(
        (status = 200, description = "List of vulnerability patterns", body = [VulnerabilityPattern])
    )
)]
async fn get_patterns(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let patterns = state.patterns.read().await;
    Json(patterns.clone())
}

#[utoipa::path(
    post,
    path = "/patterns/refresh",
    responses(
        (status = 200, description = "Patterns refreshed successfully")
    )
)]
async fn get_scan_result(
    State(state): State<Arc<AppState>>,
    Path(tracking_id): Path<String>,
) -> impl IntoResponse {
    if let Some(client) = &state.redis_client {
        if let Ok(mut con) = client.get_async_connection().await {
            let result: Result<String, _> = redis::cmd("GET").arg(&tracking_id).query_async(&mut con).await;
            return match result {
                Ok(val) => (StatusCode::OK, val).into_response(),
                Err(_) => (StatusCode::NOT_FOUND, "Scan result not found or expired.".to_string()).into_response(),
            };
        } else {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Could not connect to Redis".to_string()).into_response();
        }
    }

    (StatusCode::INTERNAL_SERVER_ERROR, "Redis not configured".to_string()).into_response()
}

#[utoipa::path(
    post,
    path = "/scan/control",
    request_body = ScanControlRequest,
    responses(
        (status = 200, description = "Scan control command processed", body = ScanControlResponse),
        (status = 404, description = "Scan not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    )
)]
async fn handle_scan_control(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ScanControlRequest>,
) -> impl IntoResponse {
    info!("Received scan control command: scan_id={}, command={:?}", payload.scan_id, payload.command);
    
    match state.scan_controller.issue_command(&payload.scan_id, payload.command).await {
        Ok(_) => {
            let scan_status = state.scan_controller.get_scan_status(&payload.scan_id).await.unwrap_or(None);
            (StatusCode::OK, Json(ScanControlResponse {
                success: true,
                message: "Command processed successfully".to_string(),
                scan_status,
            })).into_response()
        }
        Err(e) => {
            error!("Failed to process scan control command: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ScanControlResponse {
                success: false,
                message: format!("Failed to process command: {}", e),
                scan_status: None,
            })).into_response()
        }
    }
}

#[utoipa::path(
    get,
    path = "/scan/status/{scan_id}",
    responses(
        (status = 200, description = "Scan status retrieved", body = ScanStatusResponse),
        (status = 404, description = "Scan not found")
    )
)]
async fn get_scan_status(
    State(state): State<Arc<AppState>>,
    Path(scan_id): Path<String>,
) -> impl IntoResponse {
    let scan_status = state.scan_controller.get_scan_status(&scan_id).await.unwrap_or(None);
    let active_scans = state.scan_controller.get_active_scans().await.unwrap_or_default();
    
    (StatusCode::OK, Json(ScanStatusResponse {
        scan_id,
        status: scan_status,
        active_scans,
    })).into_response()
}

#[utoipa::path(
    get,
    path = "/scan/active",
    responses(
        (status = 200, description = "Active scans retrieved", body = Vec<ScanControl>)
    )
)]
async fn get_active_scans(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let active_scans = state.scan_controller.get_active_scans().await.unwrap_or_default();
    (StatusCode::OK, Json(active_scans)).into_response()
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string());
    let redis_client = redis::Client::open(redis_url).ok();

    let state = Arc::new(AppState {
        patterns: RwLock::new(get_vulnerability_patterns()),
        redis_client,
        scan_controller: ScanController::new(),
    });

    // In a real application, this would be based on the API key's subscription tier.
    // For now, we'll use a global rate limit.
    let governor_conf = Box::new(
        GovernorConfigBuilder::default()
            .per_second(10) // Allow 10 requests per second
            .burst_size(30) // Allow bursts of up to 30 requests
            .finish()
            .unwrap(),
    );

    let ci_router = Router::new()
        .route("/scan", post(handle_ci_scan))
        .route("/results/:tracking_id", get(get_scan_result))
        .route_layer(middleware::from_fn(api_key_auth));

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/scan", post(handle_scan))
        .route("/scan/control", post(handle_scan_control))
        .route("/scan/status/:scan_id", get(get_scan_status))
        .route("/scan/active", get(get_active_scans))
        .route("/patterns", get(get_patterns))
        .route("/patterns/refresh", post(refresh_patterns))
        .nest("/api/v1/ci", ci_router)
        .layer(TraceLayer::new_for_http())
        .layer(GovernorLayer { config: Box::leak(governor_conf) })
        .with_state(state.clone());

    let cron_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(7 * 24 * 3600));
        loop {
            interval.tick().await;
            info!("Running scheduled vulnerability pattern refresh...");
            // We can't easily call the async handler here without some refactoring,
            // so we just call the logic directly or trigger the endpoint.
            let new_patterns = get_vulnerability_patterns();
            let mut patterns_lock = cron_state.patterns.write().await;
            *patterns_lock = new_patterns.clone();
            info!("Scheduled refresh complete.");
        }
    });

    let cleanup_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600)); // Every hour
        loop {
            interval.tick().await;
            info!("Running scan cleanup task...");
            if let Ok(cleaned_count) = cleanup_state.scan_controller.cleanup_old_scans(
                chrono::Duration::hours(24) // Clean scans older than 24 hours
            ).await {
                if cleaned_count > 0 {
                    info!("Cleaned up {} old scan records", cleaned_count);
                }
            }
        }
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], 8082));
    info!("Vulnerability Scanner Service starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
