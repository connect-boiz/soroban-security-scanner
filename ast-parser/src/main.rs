mod parser;

use axum::{
    extract::{Multipart, DefaultBodyLimit},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{post, get},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::{
    trace::TraceLayer,
    timeout::TimeoutLayer,
    cors::CorsLayer,
};
use std::time::Duration;
use tracing::{info, error, Level};
use tracing_subscriber::{FmtSubscriber, EnvFilter};

#[derive(Serialize, Deserialize)]
struct ParseResponse {
    contract_info: parser::SorobanContractInfo,
    normalized_code: String,
}

#[derive(Serialize, Deserialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Deserialize)]
struct GithubLinkRequest {
    url: String,
}

async fn handle_file_upload(mut multipart: Multipart) -> impl IntoResponse {
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("unknown").to_string();
        let file_name = field.file_name().unwrap_or("unknown.rs").to_string();
        
        if file_name.ends_with(".rs") {
            let data = match field.bytes().await {
                Ok(d) => d,
                Err(e) => return (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: format!("Failed to read field: {}", e) })).into_response(),
            };
            
            let code = String::from_utf8_lossy(&data);
            
            match parser::parse_soroban_code(&code) {
                Ok(info) => {
                    let normalized = parser::normalize_code(&code);
                    return (StatusCode::OK, Json(ParseResponse {
                        contract_info: info,
                        normalized_code: normalized,
                    })).into_response();
                }
                Err(e) => {
                    error!("Parsing failure for file {}: {}", file_name, e);
                    return (StatusCode::UNPROCESSABLE_ENTITY, Json(ErrorResponse { error: format!("Parsing failed: {}", e) })).into_response();
                }
            }
        }
    }
    
    (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "No valid .rs file found in request".to_string() })).into_response()
}

async fn handle_github_link(Json(payload): Json<GithubLinkRequest>) -> impl IntoResponse {
    let url = payload.url;
    
    // Simplification: only handle direct GitHub file URLs for now
    // In a production app, we would use GitHub API to clone or fetch the contents
    let raw_url = if url.contains("github.com") && !url.contains("raw.githubusercontent.com") {
        url.replace("github.com", "raw.githubusercontent.com").replace("/blob/", "/")
    } else {
        url
    };

    let client = reqwest::Client::new();
    let res = match client.get(&raw_url).timeout(Duration::from_secs(10)).send().await {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_GATEWAY, Json(ErrorResponse { error: format!("Failed to fetch from GitHub: {}", e) })).into_response(),
    };

    if !res.status().is_success() {
        return (StatusCode::BAD_GATEWAY, Json(ErrorResponse { error: format!("GitHub returned status: {}", res.status()) })).into_response();
    }

    let code = match res.text().await {
        Ok(t) => t,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: format!("Failed to read content: {}", e) })).into_response(),
    };

    match parser::parse_soroban_code(&code) {
        Ok(info) => {
            let normalized = parser::normalize_code(&code);
            (StatusCode::OK, Json(ParseResponse {
                contract_info: info,
                normalized_code: normalized,
            })).into_response()
        }
        Err(e) => {
            error!("Parsing failure for GitHub URL {}: {}", raw_url, e);
            (StatusCode::UNPROCESSABLE_ENTITY, Json(ErrorResponse { error: format!("Parsing failed: {}", e) })).into_response()
        }
    }
}

async fn health_check() -> &'static str {
    "AST Parser Service is healthy"
}

#[tokio::main]
async fn main() {
    // Initialize JSON logging for Datadog/ELK
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .json()  // Use JSON for structured logging
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/parse/file", post(handle_file_upload))
        .route("/parse/github", post(handle_github_link))
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(Duration::from_secs(30))) // Security: timeout constraints
        .layer(DefaultBodyLimit::max(5 * 1024 * 1024)) // Security: strict 5MB file-size limit
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("AST Parser Service starting on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
