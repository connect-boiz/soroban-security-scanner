//! Phase 6 — Persistent Fuzzing Mode
//!
//! Instead of spawning a new VM instance for every input (fork+exec ≈ 50 ms
//! overhead), the persistent fuzzer keeps a single long-lived VM and resets
//! its observable state between inputs:
//!
//!   Storage  → cleared / restored from snapshot
//!   Memory   → re-initialised from the original WASM data segments
//!   Globals  → restored from snapshot
//!   Call stack → unwound / reset
//!
//! This reduces per-input overhead from ~50 ms to ~1 ms, enabling 1000+
//! executions/second.
//!
//! # Architecture
//!
//! `VmSnapshot` captures the baseline state (taken once after contract
//! deployment). `PersistentVm` wraps the Soroban host and exposes:
//!   - `execute(input)` → runs one input, then calls `reset()`
//!   - `reset()` → restores state from the snapshot in O(storage_entries)
//!
//! The actual Soroban host integration is abstracted behind `VmBackend` so
//! the rest of the fuzzer can be tested without a live Soroban runtime.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// A key-value entry in the contract storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageEntry {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

/// A snapshot of the VM state taken at a known-good baseline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmSnapshot {
    /// Contract storage contents.
    pub storage: Vec<StorageEntry>,
    /// WASM global variable values (index → little-endian bytes).
    pub globals: HashMap<u32, Vec<u8>>,
    /// WASM linear memory content (page index → page bytes).
    pub memory_pages: HashMap<u32, Vec<u8>>,
    /// WASM binary (for re-initialisation of data segments if needed).
    pub wasm_bytes: Vec<u8>,
}

impl VmSnapshot {
    /// Create an empty snapshot (used in testing / stub mode).
    pub fn empty(wasm_bytes: Vec<u8>) -> Self {
        Self {
            storage: Vec::new(),
            globals: HashMap::new(),
            memory_pages: HashMap::new(),
            wasm_bytes,
        }
    }

    /// Estimated memory footprint of the snapshot in bytes.
    pub fn size_bytes(&self) -> usize {
        let storage: usize = self
            .storage
            .iter()
            .map(|e| e.key.len() + e.value.len())
            .sum();
        let globals: usize = self.globals.values().map(|v| v.len()).sum();
        let memory: usize = self.memory_pages.values().map(|p| p.len()).sum();
        storage + globals + memory + self.wasm_bytes.len()
    }
}

/// Errors that can occur when resetting the VM state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmResetError {
    StorageRestoreFailed(String),
    GlobalRestoreFailed(String),
    MemoryRestoreFailed(String),
    SnapshotMissing,
}

impl std::fmt::Display for VmResetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StorageRestoreFailed(m) => write!(f, "storage restore failed: {}", m),
            Self::GlobalRestoreFailed(m) => write!(f, "global restore failed: {}", m),
            Self::MemoryRestoreFailed(m) => write!(f, "memory restore failed: {}", m),
            Self::SnapshotMissing => write!(f, "no snapshot available"),
        }
    }
}

/// The outcome of executing one input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutcome {
    /// Whether the execution succeeded (no panic / trap / timeout).
    pub success: bool,
    /// Return value as raw bytes (serialised Soroban value).
    pub return_value: Option<Vec<u8>>,
    /// Error text if the execution failed.
    pub error: Option<String>,
    /// CPU units consumed.
    pub cpu_units: u64,
    /// Wall-clock time.
    pub execution_time: Duration,
    /// Storage entries written during this execution.
    pub storage_writes: Vec<StorageEntry>,
    /// Which WASM basic blocks were hit (block IDs, for Phase 2).
    pub blocks_hit: Vec<u32>,
}

impl ExecutionOutcome {
    pub fn crashed(error: String, elapsed: Duration) -> Self {
        Self {
            success: false,
            return_value: None,
            error: Some(error),
            cpu_units: 0,
            execution_time: elapsed,
            storage_writes: Vec::new(),
            blocks_hit: Vec::new(),
        }
    }
}

/// Trait abstracting the actual Soroban VM backend.
/// A production implementation wraps `soroban-env-host`; the in-process
/// implementation is used for tests.
pub trait VmBackend: Send {
    /// Take a snapshot of the current state.
    fn snapshot(&self) -> VmSnapshot;

    /// Restore state from a snapshot. Returns `Err` if restoration fails.
    fn restore(&mut self, snapshot: &VmSnapshot) -> Result<(), VmResetError>;

    /// Execute `input` bytes as a serialised function call.
    fn execute_raw(&mut self, input: &[u8], timeout: Duration) -> ExecutionOutcome;
}

/// The persistent fuzzer VM wrapper.
///
/// Keeps a single `VmBackend` instance and a `VmSnapshot` for fast resets.
pub struct PersistentVm {
    backend: Box<dyn VmBackend>,
    snapshot: Option<VmSnapshot>,
    pub reset_count: u64,
    pub total_executions: u64,
    pub total_reset_time: Duration,
    pub timeout: Duration,
}

impl PersistentVm {
    /// Wrap a VM backend. Call `take_snapshot()` once after contract
    /// deployment to capture the baseline state.
    pub fn new(backend: Box<dyn VmBackend>, timeout: Duration) -> Self {
        Self {
            backend,
            snapshot: None,
            reset_count: 0,
            total_executions: 0,
            total_reset_time: Duration::ZERO,
            timeout,
        }
    }

    /// Capture the current VM state as the baseline snapshot.
    /// Must be called once after the contract is fully deployed and
    /// before any fuzzing input is executed.
    pub fn take_snapshot(&mut self) {
        self.snapshot = Some(self.backend.snapshot());
    }

    /// Execute one fuzzer input, then reset the VM to the baseline snapshot.
    ///
    /// Returns `Err(VmResetError)` only if the *reset* fails (not if the
    /// execution itself crashes — crashes are returned via `ExecutionOutcome`).
    pub fn execute_and_reset(&mut self, input: &[u8]) -> Result<ExecutionOutcome, VmResetError> {
        let outcome = self.backend.execute_raw(input, self.timeout);
        self.total_executions += 1;

        // Always reset, even on crash — state may be partially modified.
        self.reset()?;

        Ok(outcome)
    }

    /// Reset the VM to the baseline snapshot.
    pub fn reset(&mut self) -> Result<(), VmResetError> {
        let snapshot = self
            .snapshot
            .as_ref()
            .ok_or(VmResetError::SnapshotMissing)?;
        let t0 = Instant::now();
        self.backend.restore(snapshot)?;
        self.total_reset_time += t0.elapsed();
        self.reset_count += 1;
        Ok(())
    }

    /// Average reset latency.
    pub fn avg_reset_time(&self) -> Duration {
        if self.reset_count == 0 {
            return Duration::ZERO;
        }
        self.total_reset_time / self.reset_count as u32
    }

    /// Estimated executions per second (based on average reset + exec time).
    pub fn estimated_eps(&self) -> f64 {
        let avg_reset = self.avg_reset_time().as_secs_f64();
        if avg_reset == 0.0 {
            return 0.0;
        }
        // Rough estimate: dominated by reset latency in persistent mode.
        1.0 / avg_reset
    }
}

// ── In-process stub backend (for tests and CI without a live Soroban host) ───

/// A minimal in-memory VM backend for testing.
pub struct StubVmBackend {
    storage: HashMap<Vec<u8>, Vec<u8>>,
    /// If true, every execution returns a crash.
    pub always_crash: bool,
    /// If set, executions taking longer than this return a timeout error.
    pub simulated_exec_time: Duration,
}

impl StubVmBackend {
    pub fn new() -> Self {
        Self {
            storage: HashMap::new(),
            always_crash: false,
            simulated_exec_time: Duration::from_micros(100),
        }
    }
}

impl Default for StubVmBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl VmBackend for StubVmBackend {
    fn snapshot(&self) -> VmSnapshot {
        let entries: Vec<StorageEntry> = self
            .storage
            .iter()
            .map(|(k, v)| StorageEntry {
                key: k.clone(),
                value: v.clone(),
            })
            .collect();
        VmSnapshot {
            storage: entries,
            globals: HashMap::new(),
            memory_pages: HashMap::new(),
            wasm_bytes: vec![],
        }
    }

    fn restore(&mut self, snapshot: &VmSnapshot) -> Result<(), VmResetError> {
        self.storage.clear();
        for entry in &snapshot.storage {
            self.storage.insert(entry.key.clone(), entry.value.clone());
        }
        Ok(())
    }

    fn execute_raw(&mut self, input: &[u8], timeout: Duration) -> ExecutionOutcome {
        let t0 = Instant::now();

        if self.always_crash {
            return ExecutionOutcome::crashed("stub: forced crash".to_string(), t0.elapsed());
        }

        if self.simulated_exec_time > timeout {
            return ExecutionOutcome::crashed("stub: execution timeout".to_string(), timeout);
        }

        // Simulate a storage write using the input bytes as both key and value.
        let key = input.get(..4).unwrap_or(input).to_vec();
        let value = input.to_vec();
        self.storage.insert(key.clone(), value.clone());

        ExecutionOutcome {
            success: true,
            return_value: Some(input.get(..8).unwrap_or(input).to_vec()),
            error: None,
            cpu_units: (input.len() as u64) * 10,
            execution_time: t0.elapsed(),
            storage_writes: vec![StorageEntry { key, value }],
            blocks_hit: vec![1, 2, 3],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vm_with_snapshot() -> PersistentVm {
        let mut vm = PersistentVm::new(Box::new(StubVmBackend::new()), Duration::from_secs(5));
        vm.take_snapshot();
        vm
    }

    #[test]
    fn execute_succeeds() {
        let mut vm = vm_with_snapshot();
        let out = vm.execute_and_reset(b"hello world").unwrap();
        assert!(out.success);
        assert!(out.return_value.is_some());
    }

    #[test]
    fn reset_count_increments() {
        let mut vm = vm_with_snapshot();
        for _ in 0..5 {
            vm.execute_and_reset(b"test").unwrap();
        }
        assert_eq!(vm.reset_count, 5);
        assert_eq!(vm.total_executions, 5);
    }

    #[test]
    fn crash_propagated_correctly() {
        let mut backend = StubVmBackend::new();
        backend.always_crash = true;
        let mut vm = PersistentVm::new(Box::new(backend), Duration::from_secs(5));
        vm.take_snapshot();
        let out = vm.execute_and_reset(b"input").unwrap();
        assert!(!out.success);
        assert!(out.error.is_some());
    }

    #[test]
    fn snapshot_missing_returns_error() {
        // No take_snapshot() called.
        let mut vm = PersistentVm::new(Box::new(StubVmBackend::new()), Duration::from_secs(5));
        let err = vm.reset().unwrap_err();
        assert_eq!(err, VmResetError::SnapshotMissing);
    }

    #[test]
    fn state_restored_after_reset() {
        let mut vm = vm_with_snapshot();
        // Execute writes key=b"hell" into storage.
        vm.execute_and_reset(b"hello").unwrap();
        // After reset, storage should be empty (snapshot had no entries).
        let snap = vm.backend.snapshot();
        assert!(
            snap.storage.is_empty(),
            "storage should be empty after reset"
        );
    }
}
