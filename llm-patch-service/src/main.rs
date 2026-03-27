use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, error, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use llm_patch_service::{
    models::{PatchRequest, PatchResponse, ServiceConfig},
    error::{ServiceError, ServiceResult},
    LLMClient, CodeSanitizer, VerificationSandbox, ConfidenceScorer,
    RemediationDB, GitDiffFormatter, FallbackProvider,
};

struct AppState {
    llm_client: LLMClient,
    sanitizer: CodeSanitizer,
    verifier: VerificationSandbox,
    confidence_scorer: ConfidenceScorer,
    database: RemediationDB,
    git_formatter: GitDiffFormatter,
    fallback_provider: FallbackProvider,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "llm_patch_service=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = load_configuration().await?;
    
    info!("Starting LLM Patch Service on {}:{}", config.host, config.port);
    
    // Initialize components
    let state = Arc::new(AppState {
        llm_client: LLMClient::new(config.llm.clone()),
        sanitizer: CodeSanitizer::new(),
        verifier: VerificationSandbox::new(),
        confidence_scorer: ConfidenceScorer::new(),
        database: RemediationDB::new(&config.database.url).await?,
        git_formatter: GitDiffFormatter::new(),
        fallback_provider: FallbackProvider::new(),
    });

    // Test LLM connection
    if let Err(e) = state.llm_client.test_connection().await {
        warn!("LLM connection test failed: {}", e);
    } else {
        info!("LLM connection test passed");
    }

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/patch", post(generate_patch))
        .route("/patch/:id/apply", post(apply_patch))
        .route("/history/:vulnerability_id", get(get_remediation_history))
        .route("/stats", get(get_remediation_stats))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    info!("Server listening on {}", addr);
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn load_configuration() -> ServiceResult<ServiceConfig> {
    // Try to load from environment variables first
    let config = ServiceConfig {
        host: std::env::var("SERVICE_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
        port: std::env::var("SERVICE_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .unwrap_or(8080),
        llm: crate::models::LLMConfig {
            provider: std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "openai".to_string()),
            api_key: std::env::var("LLM_API_KEY")
                .map_err(|_| ServiceError::ConfigurationError("LLM_API_KEY environment variable is required".to_string()))?,
            model: std::env::var("LLM_MODEL").unwrap_or_else(|_| "gpt-4".to_string()),
            max_tokens: std::env::var("LLM_MAX_TOKENS")
                .unwrap_or_else(|_| "2000".to_string())
                .parse()
                .unwrap_or(2000),
            temperature: std::env::var("LLM_TEMPERATURE")
                .unwrap_or_else(|_| "0.3".to_string())
                .parse()
                .unwrap_or(0.3),
        },
        database: crate::models::DatabaseConfig {
            url: std::env::var("DATABASE_URL")
                .map_err(|_| ServiceError::ConfigurationError("DATABASE_URL environment variable is required".to_string()))?,
            max_connections: std::env::var("DB_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
        },
    };
    
    Ok(config)
}

async fn health_check() -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now(),
        "service": "llm-patch-service"
    })))
}

async fn generate_patch(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PatchRequest>,
) -> Result<Json<PatchResponse>, StatusCode> {
    info!("Received patch request for vulnerability: {}", request.vulnerability.id);
    
    match process_patch_request(state, &request).await {
        Ok(response) => {
            info!("Successfully generated patch for vulnerability: {}", request.vulnerability.id);
            Ok(Json(response))
        },
        Err(e) => {
            error!("Failed to generate patch for vulnerability {}: {}", request.vulnerability.id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn process_patch_request(
    state: Arc<AppState>,
    request: &PatchRequest,
) -> ServiceResult<PatchResponse> {
    // Step 1: Sanitize the input
    let sanitized_code = state.sanitizer.sanitize_code(&request.original_code)?;
    let sanitized_vulnerability = sanitize_vulnerability(&state.sanitizer, &request.vulnerability)?;
    
    // Step 2: Validate code safety
    state.sanitizer.validate_code_safety(&sanitized_code)?;
    
    // Step 3: Generate patch using LLM
    let patch_result = state.llm_client.generate_patch(&PatchRequest {
        vulnerability: sanitized_vulnerability.clone(),
        original_code: sanitized_code.clone(),
        context: request.context.clone(),
    }).await;
    
    let (patch, fallback_provided) = match patch_result {
        Ok(patch) => (patch, false),
        Err(e) => {
            warn!("LLM patch generation failed, using fallback: {}", e);
            let fallback_patch = state.fallback_provider.get_fallback_patch(
                &sanitized_vulnerability,
                &sanitized_code
            ).await?;
            (fallback_patch, true)
        }
    };
    
    // Step 4: Verify the patch
    let verification_status = state.verifier.verify_patch(&patch).await?;
    
    // Step 5: Calculate confidence score
    let confidence_score = state.confidence_scorer.calculate_confidence(
        &patch,
        &sanitized_vulnerability,
        verification_status.clone(),
    ).await?;
    
    // Step 6: Generate Git diff
    let git_diff = state.git_formatter.create_patch_diff(&patch, &sanitized_vulnerability)?;
    
    // Step 7: Store in database
    let remediation_id = state.database.store_remediation(
        &sanitized_vulnerability.id,
        &patch,
        confidence_score,
        verification_status.clone(),
    ).await?;
    
    // Step 8: Check if we should use fallback due to low confidence
    let final_patch = if confidence_score < 0.4 && !fallback_provided {
        warn!("Low confidence score ({}), using fallback", confidence_score);
        let fallback_patch = state.fallback_provider.get_fallback_patch(
            &sanitized_vulnerability,
            &sanitized_code
        ).await?;
        fallback_patch
    } else {
        patch
    };
    
    Ok(PatchResponse {
        id: remediation_id,
        vulnerability_id: sanitized_vulnerability.id,
        patch: final_patch,
        confidence_score,
        verification_status,
        git_diff,
        fallback_provided,
        created_at: chrono::Utc::now(),
    })
}

fn sanitize_vulnerability(
    sanitizer: &CodeSanitizer,
    vulnerability: &crate::models::VulnerabilityReport,
) -> ServiceResult<crate::models::VulnerabilityReport> {
    let mut sanitized = vulnerability.clone();
    
    // Sanitize code snippet
    sanitized.code_snippet = sanitizer.sanitize_code(&vulnerability.code_snippet)?;
    
    // Sanitize SARIF report if present
    if let Some(sarif) = &vulnerability.sarif_report {
        let sarif_json = serde_json::to_string(sarif)?;
        let sanitized_sarif_json = sanitizer.sanitize_sarif_report(&sarif_json)?;
        sanitized.sarif_report = Some(serde_json::from_str(&sanitized_sarif_json)?);
    }
    
    Ok(sanitized)
}

async fn apply_patch(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(remediation_id): axum::extract::Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let target_dir = payload.get("target_dir")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    
    // Get remediation from database
    let history = state.database.get_remediation_history(&remediation_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let remediation = history.first()
        .ok_or(StatusCode::NOT_FOUND)?;
    
    // Apply the patch
    match state.git_formatter.apply_patch(&remediation.patch.patched_code, target_dir) {
        Ok(message) => {
            // Update success rate
            let _ = state.database.update_remediation_success(&remediation_id, 1.0).await;
            
            Ok(Json(json!({
                "success": true,
                "message": message,
                "remediation_id": remediation_id
            })))
        },
        Err(e) => {
            error!("Failed to apply patch {}: {}", remediation_id, e);
            // Update success rate
            let _ = state.database.update_remediation_success(&remediation_id, 0.0).await;
            
            Ok(Json(json!({
                "success": false,
                "error": e.to_string(),
                "remediation_id": remediation_id
            })))
        }
    }
}

async fn get_remediation_history(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(vulnerability_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.database.get_remediation_history(&vulnerability_id).await {
        Ok(history) => Ok(Json(json!({
            "vulnerability_id": vulnerability_id,
            "history": history,
            "count": history.len()
        }))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn get_remediation_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.database.get_remediation_stats().await {
        Ok(stats) => Ok(Json(json!({
            "total_remediations": stats.total_remediations,
            "applied_remediations": stats.applied_remediations,
            "avg_confidence": stats.avg_confidence,
            "avg_success_rate": stats.avg_success_rate,
            "passed_verifications": stats.passed_verifications
        }))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
