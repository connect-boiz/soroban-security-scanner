//! WebSocket endpoint for real-time scan progress updates.
//!
//! Exposes `GET /ws/scan/:scan_id` as an Axum WebSocket handler.
//! The server broadcasts `ScanEvent` frames to all connected clients
//! for a given `scan_id`, replacing the previous HTTP polling approach.

use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Path, State},
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Events emitted during a scan run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScanEvent {
    /// Scan has started.
    Started { scan_id: String, total_files: u64 },
    /// Progress update.
    Progress { scan_id: String, scanned: u64, total: u64, current_file: String },
    /// A vulnerability was found.
    VulnerabilityFound {
        scan_id:       String,
        file_path:     String,
        severity:      String,
        description:   String,
    },
    /// Scan completed successfully.
    Completed { scan_id: String, total_vulnerabilities: u64, duration_ms: u64 },
    /// Scan failed.
    Failed { scan_id: String, error: String },
    /// Periodic heartbeat to keep the connection alive.
    Heartbeat { scan_id: String },
}

/// Shared state: one broadcast channel per active scan_id.
#[derive(Clone, Default)]
pub struct WsState {
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<ScanEvent>>>>,
}

impl WsState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create a broadcast channel for `scan_id`.
    pub async fn channel(&self, scan_id: &str) -> broadcast::Sender<ScanEvent> {
        let mut map = self.channels.write().await;
        map.entry(scan_id.to_owned())
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(128);
                tx
            })
            .clone()
    }

    /// Publish an event to all subscribers of `scan_id`.
    pub async fn publish(&self, scan_id: &str, event: ScanEvent) {
        let map = self.channels.read().await;
        if let Some(tx) = map.get(scan_id) {
            let _ = tx.send(event); // ignore "no receivers" error
        }
    }

    /// Remove a channel when a scan finishes (cleanup).
    pub async fn remove(&self, scan_id: &str) {
        self.channels.write().await.remove(scan_id);
    }
}

// ---------------------------------------------------------------------------
// Axum handler
// ---------------------------------------------------------------------------

/// `GET /ws/scan/:scan_id`
///
/// Upgrades the HTTP connection to WebSocket and streams `ScanEvent` JSON
/// frames until the scan completes or the client disconnects.
pub async fn ws_scan_handler(
    ws: WebSocketUpgrade,
    Path(scan_id): Path<String>,
    State(state): State<WsState>,
) -> impl IntoResponse {
    let scan_id_clone = scan_id.clone();
    ws.on_upgrade(move |socket| handle_socket(socket, scan_id_clone, state))
}

async fn handle_socket(socket: WebSocket, scan_id: String, state: WsState) {
    let (mut sender, mut receiver) = socket.split();
    let tx = state.channel(&scan_id).await;
    let mut rx = tx.subscribe();

    // Spawn task: forward broadcast events -> WebSocket client.
    let forward = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let json = match serde_json::to_string(&event) {
                Ok(j) => j,
                Err(_) => continue,
            };
            if sender.send(Message::Text(json)).await.is_err() {
                break; // client disconnected
            }
            // Auto-close on terminal events.
            match &event {
                ScanEvent::Completed { .. } | ScanEvent::Failed { .. } => break,
                _ => {}
            }
        }
    });

    // Drain incoming frames (ping/close) to keep the connection healthy.
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }

    forward.abort();
}
