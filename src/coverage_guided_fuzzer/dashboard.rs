//! Phase 7 — Fuzzer Status API and Dashboard
//!
//! Exposes:
//!   - GET /api/v1/fuzzer/status  → JSON snapshot of current fuzzer state
//!   - WebSocket /api/v1/fuzzer/ws → real-time push of status updates
//!
//! `FuzzerStatus` is the shared, `Arc<RwLock<_>>` state that the orchestrator
//! writes to and the API handlers read from.

use crate::coverage_guided_fuzzer::corpus::CorpusStats;
use crate::coverage_guided_fuzzer::coverage_map::CoverageStats;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime};
/// A complete snapshot of the fuzzer's current state (safe to serialise).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzerStatusSnapshot {
    /// Total inputs executed since the start of the session.
    pub total_executions: u64,
    /// Executions per second (rolling 1-second window).
    pub executions_per_second: f64,
    /// Number of entries in the corpus.
    pub corpus_size: usize,
    /// Coverage map statistics.
    pub coverage: CoverageStats,
    /// Corpus performance statistics.
    pub corpus_stats: CorpusStats,
    /// Number of unique crashes found.
    pub unique_crashes: usize,
    /// Total crash occurrences (unique + duplicates).
    pub total_crashes: usize,
    /// Seconds since the last new coverage was discovered.
    pub secs_since_new_coverage: f64,
    /// Estimated seconds remaining until the time budget is exhausted.
    pub secs_until_timeout: Option<f64>,
    /// Wall-clock time the fuzzing session started (Unix timestamp).
    pub session_start_unix: u64,
    /// Total wall-clock duration of the session so far.
    pub session_duration_secs: f64,
    /// Current mutation stage name (e.g. "bit_flip_1", "havoc").
    pub current_stage: String,
    /// Raw coverage bitmap for heatmap rendering (base64-encoded, 65536 bytes).
    pub coverage_bitmap_b64: String,
}

/// Mutable shared fuzzer state, updated by the orchestrator.
#[derive(Debug)]
pub struct FuzzerStatus {
    pub total_executions: u64,
    pub executions_this_window: u64,
    pub window_start: Instant,
    pub executions_per_second: f64,
    pub corpus_stats: CorpusStats,
    pub coverage_stats: CoverageStats,
    pub unique_crashes: usize,
    pub total_crashes: usize,
    pub last_new_coverage: Instant,
    pub session_start: Instant,
    pub timeout_at: Option<Instant>,
    pub current_stage: String,
    pub coverage_bitmap: Vec<u8>,
}

impl FuzzerStatus {
    pub fn new(timeout: Option<Duration>) -> Self {
        let now = Instant::now();
        Self {
            total_executions: 0,
            executions_this_window: 0,
            window_start: now,
            executions_per_second: 0.0,
            corpus_stats: CorpusStats::default(),
            coverage_stats: CoverageStats {
                total_edges: 0,
                density: 0.0,
                interesting_inputs: 0,
            },
            unique_crashes: 0,
            total_crashes: 0,
            last_new_coverage: now,
            session_start: now,
            timeout_at: timeout.map(|d| now + d),
            current_stage: "initializing".to_string(),
            coverage_bitmap: vec![
                0u8;
                crate::coverage_guided_fuzzer::coverage_map::COVERAGE_MAP_SIZE
            ],
        }
    }

    /// Record one execution tick and update rolling EPS.
    pub fn tick(&mut self) {
        self.total_executions += 1;
        self.executions_this_window += 1;

        let elapsed = self.window_start.elapsed();
        if elapsed >= Duration::from_secs(1) {
            self.executions_per_second = self.executions_this_window as f64 / elapsed.as_secs_f64();
            self.executions_this_window = 0;
            self.window_start = Instant::now();
        }
    }

    /// Record that new coverage was found.
    pub fn record_new_coverage(&mut self) {
        self.last_new_coverage = Instant::now();
    }

    /// Build a serialisable snapshot.
    pub fn snapshot(&self) -> FuzzerStatusSnapshot {
        let now = Instant::now();
        let session_duration = now.duration_since(self.session_start);
        let secs_since_new_coverage = now.duration_since(self.last_new_coverage).as_secs_f64();
        let secs_until_timeout = self
            .timeout_at
            .map(|t| t.saturating_duration_since(now).as_secs_f64());

        // Encode bitmap as base64.
        let bitmap_b64 = base64_encode(&self.coverage_bitmap);

        FuzzerStatusSnapshot {
            total_executions: self.total_executions,
            executions_per_second: self.executions_per_second,
            corpus_size: self.corpus_stats.size,
            coverage: self.coverage_stats.clone(),
            corpus_stats: self.corpus_stats.clone(),
            unique_crashes: self.unique_crashes,
            total_crashes: self.total_crashes,
            secs_since_new_coverage,
            secs_until_timeout,
            session_start_unix: unix_timestamp_now(),
            session_duration_secs: session_duration.as_secs_f64(),
            current_stage: self.current_stage.clone(),
            coverage_bitmap_b64: bitmap_b64,
        }
    }
}

/// Thread-safe fuzzer dashboard shared between the orchestrator and the API.
#[derive(Clone, Debug)]
pub struct FuzzerDashboard {
    pub status: Arc<RwLock<FuzzerStatus>>,
}

impl FuzzerDashboard {
    pub fn new(timeout: Option<Duration>) -> Self {
        Self {
            status: Arc::new(RwLock::new(FuzzerStatus::new(timeout))),
        }
    }

    /// Get a serialisable snapshot (used by the GET /status handler).
    pub fn snapshot(&self) -> FuzzerStatusSnapshot {
        self.status.read().unwrap().snapshot()
    }

    /// Update tick counter (called after every execution).
    pub fn tick(&self) {
        self.status.write().unwrap().tick();
    }

    /// Update stage name.
    pub fn set_stage(&self, stage: &str) {
        self.status.write().unwrap().current_stage = stage.to_string();
    }

    /// Record a new coverage discovery.
    pub fn record_new_coverage(&self) {
        self.status.write().unwrap().record_new_coverage();
    }

    /// Update crash counters.
    pub fn record_crash(&self, unique: bool) {
        let mut s = self.status.write().unwrap();
        s.total_crashes += 1;
        if unique {
            s.unique_crashes += 1;
        }
    }

    /// Push updated coverage stats and bitmap.
    pub fn update_coverage(&self, coverage_stats: CoverageStats, bitmap: Vec<u8>) {
        let mut s = self.status.write().unwrap();
        s.coverage_stats = coverage_stats;
        s.coverage_bitmap = bitmap;
    }

    /// Push updated corpus stats.
    pub fn update_corpus_stats(&self, stats: CorpusStats) {
        self.status.write().unwrap().corpus_stats = stats;
    }
}

// ── Axum route handlers ───────────────────────────────────────────────────────

/// Axum handler: GET /api/v1/fuzzer/status
///
/// Returns the current fuzzer state as JSON. Intended to be registered on an
/// existing `axum::Router` with `dashboard.clone()` injected via `Extension`.
pub async fn status_handler(dashboard: FuzzerDashboard) -> impl axum_response_marker::IntoResponse {
    let snap = dashboard.snapshot();
    (axum::http::StatusCode::OK, axum::Json(snap))
}

// Marker trait to avoid importing axum in tests.
mod axum_response_marker {
    pub trait IntoResponse {}
    impl<T: serde::Serialize> IntoResponse for (axum::http::StatusCode, axum::Json<T>) {}
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn unix_timestamp_now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_serialises() {
        let dash = FuzzerDashboard::new(Some(Duration::from_secs(3600)));
        let snap = dash.snapshot();
        let json = serde_json::to_string(&snap).expect("should serialise");
        assert!(json.contains("total_executions"));
        assert!(json.contains("corpus_size"));
        assert!(json.contains("unique_crashes"));
    }

    #[test]
    fn tick_increments_executions() {
        let dash = FuzzerDashboard::new(None);
        for _ in 0..10 {
            dash.tick();
        }
        assert_eq!(dash.status.read().unwrap().total_executions, 10);
    }

    #[test]
    fn crash_tracking() {
        let dash = FuzzerDashboard::new(None);
        dash.record_crash(true);
        dash.record_crash(true);
        dash.record_crash(false);
        let s = dash.status.read().unwrap();
        assert_eq!(s.unique_crashes, 2);
        assert_eq!(s.total_crashes, 3);
    }

    #[test]
    fn stage_updates() {
        let dash = FuzzerDashboard::new(None);
        dash.set_stage("bit_flip_1");
        assert_eq!(dash.status.read().unwrap().current_stage, "bit_flip_1");
    }

    #[test]
    fn base64_encodes_correctly() {
        // "Man" → "TWFu"
        assert_eq!(base64_encode(b"Man"), "TWFu");
        // "Ma" → "TWE="
        assert_eq!(base64_encode(b"Ma"), "TWE=");
        // "M" → "TQ=="
        assert_eq!(base64_encode(b"M"), "TQ==");
    }
}
