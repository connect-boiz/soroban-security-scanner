use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{post, get},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use tracing::{info, error, Level};
use tracing_subscriber::{FmtSubscriber, EnvFilter};

use soroban_security_scanner_core::analyzer::SecurityAnalyzer;
use soroban_security_scanner_core::vulnerabilities::VulnerabilityPattern;
use soroban_security_scanner_core::patterns::get_vulnerability_patterns;

#[derive(Serialize, Deserialize)]
struct ScanRequest {
    code: String,
    filename: String,
    format: Option<String>, // "json" or "sarif"
}

pub struct AppState {
    patterns: RwLock<Vec<VulnerabilityPattern>>,
    redis_client: Option<redis::Client>,
}

async fn handle_scan(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ScanRequest>,
) -> impl IntoResponse {
    let patterns = state.patterns.read().await;
    let analyzer = SecurityAnalyzer::new(true, true); // Deep analysis enabled
    
    match analyzer.analyze(&payload.code, &payload.filename) {
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

async fn get_patterns(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let patterns = state.patterns.read().await;
    Json(patterns.clone())
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
    });

    let app = Router::new()
        .route("/scan", post(handle_scan))
        .route("/patterns", get(get_patterns))
        .route("/patterns/refresh", post(refresh_patterns))
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    // Background task for weekly pattern refresh
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

    let addr = SocketAddr::from(([0, 0, 0, 0], 8082));
    info!("Vulnerability Scanner Service starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
