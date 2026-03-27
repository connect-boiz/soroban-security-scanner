mod executor;
mod ledger;
mod fuzzer;

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{post, get},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tower_http::trace::TraceLayer;
use tracing::{info, error, Level};
use tracing_subscriber::{FmtSubscriber, EnvFilter};
use crate::fuzzer::{FuzzValue, InputGenerator};
use crate::executor::{WasmExecutor, CoverageData, FuzzerInput};
use crate::ledger::MockLedger;
use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose};

#[derive(Serialize, Deserialize)]
struct FuzzJobRequest {
    wasm_base64: String,
    function: String,
    iterations: usize,
    arg_count: Option<usize>,
}

#[derive(Serialize, Deserialize, Clone)]
struct FuzzResult {
    success: bool,
    iterations_completed: usize,
    failure_input_sequence: Option<Vec<FuzzValue>>,
    error_message: Option<String>,
    coverage_data: Option<CoverageData>,
}

pub struct AppState {
    status: Mutex<WorkerStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
enum WorkerStatus {
    Idle,
    Busy,
}

async fn handle_fuzz_job(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<FuzzJobRequest>,
) -> impl IntoResponse {
    let mut status = state.status.lock().unwrap();
    if matches!(*status, WorkerStatus::Busy) {
        return (StatusCode::CONFLICT, Json("Worker is busy")).into_response();
    }
    *status = WorkerStatus::Busy;
    drop(status);

    let wasm_bytes = match general_purpose::STANDARD.decode(&payload.wasm_base64) {
        Ok(b) => b,
        Err(_) => {
            *state.status.lock().unwrap() = WorkerStatus::Idle;
            return (StatusCode::BAD_REQUEST, Json("Invalid base64 WASM")).into_response();
        }
    };
    
    let executor = WasmExecutor::new().unwrap();
    let arg_count = payload.arg_count.unwrap_or(0);
    
    // Reset coverage data for new job
    executor.reset_coverage();
    
    info!("Starting fuzz job for function '{}' ({} args) with {} iterations", 
        payload.function, arg_count, payload.iterations);

    let mut failure_sequence = None;
    let mut success = true;
    let mut error_message = None;
    let mut iterations_completed = 0;

    for i in 0..payload.iterations {
        let mut inputs = Vec::new();
        for _ in 0..arg_count {
            inputs.push(InputGenerator::generate_random_input());
        }
        
        // Define invariants for this iteration
        let invariants: Vec<Box<dyn Fn(&MockLedger) -> Result<()>>> = vec![
             Box::new(|ledger| {
                // Example: Invariant for total supply (simplified)
                // In a real scenario, this would check specific keys in the ledger
                if ledger.get_storage(&[0]).map(|v| v.len()).unwrap_or(0) > 1000 {
                    return Err(anyhow!("Storage overflow invariant failed"));
                }
                Ok(())
            })
        ];

        // Execute with invariants check and coverage tracking
        if let Err(e) = executor.execute_with_invariants(&wasm_bytes, &payload.function, inputs.clone(), invariants, i) {
           error!("Invariant violation at iteration {}: {}", i, e);
           failure_sequence = Some(inputs);
           success = false;
           error_message = Some(e.to_string());
           iterations_completed = i + 1;
           break;
        }
        iterations_completed = i + 1;
    }

    // Get coverage data after fuzzing completes
    let coverage_data = executor.get_coverage_data();

    *state.status.lock().unwrap() = WorkerStatus::Idle;

    (StatusCode::OK, Json(FuzzResult {
        success,
        iterations_completed,
        failure_input_sequence: failure_sequence,
        error_message,
        coverage_data: Some(coverage_data),
    })).into_response()
}

async fn health_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let status = state.status.lock().unwrap();
    Json(serde_json::json!({
        "status": *status,
        "worker_id": uuid::Uuid::new_v4().to_string(),
        "ready": matches!(*status, WorkerStatus::Idle),
    }))
}

#[derive(Serialize, Deserialize)]
struct FuzzerInputRequest {
    wasm_base64: String,
    line_number: u32,
}

async fn get_fuzzer_inputs_for_line(
    Json(payload): Json<FuzzerInputRequest>,
) -> impl IntoResponse {
    let wasm_bytes = match general_purpose::STANDARD.decode(&payload.wasm_base64) {
        Ok(b) => b,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json("Invalid base64 WASM")).into_response();
        }
    };
    
    let executor = WasmExecutor::new().unwrap();
    let inputs = executor.get_fuzzer_inputs_for_line(payload.line_number);
    
    (StatusCode::OK, Json(serde_json::json!({
        "line_number": payload.line_number,
        "fuzzer_inputs": inputs,
        "input_count": inputs.len()
    }))).into_response()
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .json()
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

    let state = Arc::new(AppState {
        status: Mutex::new(WorkerStatus::Idle),
    });

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/fuzz", post(handle_fuzz_job))
        .route("/fuzzer-inputs", post(get_fuzzer_inputs_for_line))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8081));
    info!("Invariant Fuzzer Worker starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
