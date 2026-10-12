//! Coverage-Guided Fuzzer for Soroban Smart Contracts
//!
//! A full AFL/libFuzzer-style coverage-guided fuzzing infrastructure adapted for
//! the Soroban WASM runtime. Implements all 9 phases:
//!
//! - Phase 1: WASM instrumentation with basic-block and edge counters
//! - Phase 2: Coverage map (65536-entry bitmap) and corpus management
//! - Phase 3: Structured Soroban input generation (Address, Symbol, i128, Vec, Map)
//! - Phase 4: Multi-strategy mutation scheduler (bit-flip, arithmetic, splice, havoc, structure-aware)
//! - Phase 5: Crash triage with deduplication and delta-debugging minimization
//! - Phase 6: Persistent fuzzing mode (VM state reset between inputs)
//! - Phase 7: Real-time fuzzer status API endpoint and WebSocket dashboard
//! - Phase 8: CI integration (`stellar-scanner fuzz` command with corpus caching)
//! - Phase 9: Differential fuzzing integration (cross-version discrepancies = interesting)

pub mod ci_integration;
pub mod corpus;
pub mod coverage_map;
pub mod crash_triage;
pub mod dashboard;
pub mod input_gen;
pub mod mutation;
pub mod orchestrator;
pub mod persistent_vm;
pub mod wasm_instrumenter;

pub use ci_integration::{FuzzCiConfig, FuzzCiResult, FuzzCiRunner};
pub use corpus::{Corpus, CorpusEntry, CorpusStats};
pub use coverage_map::{CoverageMap, EdgeId, COVERAGE_MAP_SIZE};
pub use crash_triage::{CrashGroup, CrashReport, CrashTriageEngine};
pub use dashboard::{FuzzerDashboard, FuzzerStatus, FuzzerStatusSnapshot};
pub use input_gen::{SorobanInputGen, SorobanValue, SorobanValueType};
pub use mutation::{MutationScheduler, MutationStrategy, MutatorConfig};
pub use orchestrator::{CoverageFuzzer, FuzzerConfig, FuzzerReport};
pub use persistent_vm::{PersistentVm, VmResetError, VmSnapshot};
pub use wasm_instrumenter::{InstrumentationStats, WasmInstrumenter};
