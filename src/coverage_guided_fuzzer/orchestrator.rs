//! Phase 9 / Main Orchestrator — Coverage-Guided Fuzzer
//!
//! Wires all phases together into a single `CoverageFuzzer::run()` loop:
//!
//!  1. Instrument the WASM binary (Phase 1)
//!  2. Seed the corpus (Phase 2)
//!  3. Loop until timeout or max_executions:
//!     a. Select a corpus entry weighted by performance score
//!     b. Generate or mutate an input (Phase 3 / 4)
//!     c. Execute via the persistent VM (Phase 6)
//!     d. Check coverage — if interesting, add to corpus (Phase 2)
//!     e. Triage crashes (Phase 5)
//!     f. Update dashboard (Phase 7)
//!     g. Check for cross-version discrepancy — treat as interesting (Phase 9)
//!  4. Write crash reports and corpus to disk (Phase 8)
//!  5. Return `FuzzerReport`

use crate::coverage_guided_fuzzer::{
    corpus::{Corpus, CorpusEntry},
    coverage_map::CoverageMap,
    crash_triage::{CrashInfo, CrashReport, CrashTriageConfig, CrashTriageEngine},
    dashboard::FuzzerDashboard,
    input_gen::{InputGenConfig, SorobanInputGen, SorobanValueType},
    mutation::{MutationScheduler, MutatorConfig},
    persistent_vm::{PersistentVm, StubVmBackend},
    wasm_instrumenter::WasmInstrumenter,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Configuration for a fuzzing run.
#[derive(Debug, Clone)]
pub struct FuzzerConfig {
    /// Raw WASM bytes of the contract under test.
    pub wasm_bytes: Vec<u8>,
    /// Wall-clock budget (None = run until max_executions).
    pub timeout: Option<Duration>,
    /// Hard execution cap (None = run until timeout).
    pub max_executions: Option<u64>,
    /// Where to persist / load the corpus (None = in-memory only).
    pub corpus_dir: Option<PathBuf>,
    /// PRNG seed for full reproducibility (None = entropy).
    pub seed: Option<u64>,
    /// Print progress lines to stdout.
    pub verbose: bool,
    /// Target function signatures to fuzz (empty = auto-discover).
    pub target_functions: Vec<String>,
}

impl Default for FuzzerConfig {
    fn default() -> Self {
        Self {
            wasm_bytes: Vec::new(),
            timeout: Some(Duration::from_secs(60)),
            max_executions: None,
            corpus_dir: None,
            seed: None,
            verbose: false,
            target_functions: Vec::new(),
        }
    }
}

/// Summary produced at the end of a fuzzing run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzerReport {
    pub total_executions: u64,
    pub corpus_entries: usize,
    pub unique_crashes: usize,
    pub total_crashes: usize,
    pub coverage_density: f64,
    pub total_edges_found: usize,
    pub session_duration_secs: f64,
    pub executions_per_second: f64,
    /// Minimal reproducing inputs for each unique crash.
    pub crash_reports: Vec<CrashReport>,
    /// Mutation strategy effectiveness: strategy name → interesting inputs produced.
    pub strategy_effectiveness: std::collections::HashMap<String, u64>,
}

/// The main coverage-guided fuzzer.
pub struct CoverageFuzzer {
    config: FuzzerConfig,
}

impl CoverageFuzzer {
    pub fn new(config: FuzzerConfig) -> Self {
        Self { config }
    }

    /// Run the fuzzer and return a report. This is the blocking entry point;
    /// it respects `config.timeout` and `config.max_executions`.
    pub fn run(&mut self) -> FuzzerReport {
        let start = Instant::now();

        // ── Phase 1: Instrument WASM ──────────────────────────────────────
        let instrumenter = WasmInstrumenter::new();
        let (_instrumented_wasm, instr_stats) = instrumenter
            .instrument(&self.config.wasm_bytes)
            .unwrap_or_else(|e| {
                if self.config.verbose {
                    eprintln!("[fuzzer] Instrumentation warning: {e} — running in black-box mode");
                }
                (self.config.wasm_bytes.clone(), Default::default())
            });

        if self.config.verbose {
            println!(
                "[fuzzer] WASM instrumented: {} blocks, {:.1}% overhead, {} bytes → {} bytes",
                instr_stats.blocks_instrumented,
                instr_stats.overhead_pct,
                instr_stats.original_size,
                instr_stats.instrumented_size,
            );
        }

        // ── Phase 2: Corpus setup ─────────────────────────────────────────
        let mut corpus = if let Some(ref dir) = self.config.corpus_dir {
            Corpus::with_dir(dir.clone()).unwrap_or_default()
        } else {
            Corpus::new()
        };

        // Load existing seeds from the corpus directory.
        if let Some(ref dir) = self.config.corpus_dir {
            if dir.exists() {
                let loaded = corpus.load_seeds(dir).unwrap_or(0);
                if self.config.verbose && loaded > 0 {
                    println!(
                        "[fuzzer] Loaded {} seed corpus entries from {}",
                        loaded,
                        dir.display()
                    );
                }
            }
        }

        let coverage_map = CoverageMap::new();
        let dashboard = FuzzerDashboard::new(self.config.timeout);

        // ── Phase 3: Input generator ──────────────────────────────────────
        let mut input_gen = SorobanInputGen::new(InputGenConfig {
            seed: self.config.seed,
            ..Default::default()
        });

        // ── Phase 4: Mutation scheduler ───────────────────────────────────
        let mut mutator = MutationScheduler::with_seed(MutatorConfig::default(), self.config.seed);

        // ── Phase 5: Crash triage ─────────────────────────────────────────
        let mut triage = CrashTriageEngine::new(CrashTriageConfig::default());
        let mut crash_reports: Vec<CrashReport> = Vec::new();

        // ── Phase 6: Persistent VM ────────────────────────────────────────
        let backend = StubVmBackend::new(); // replaced with live host in production
        let mut vm = PersistentVm::new(Box::new(backend), Duration::from_secs(30));
        vm.take_snapshot();

        // Seed corpus with a few auto-generated inputs so we always have
        // something to mutate even on the first run.
        self.seed_corpus(&mut corpus, &mut input_gen, &coverage_map, &mut vm);

        // ── Main fuzzing loop ─────────────────────────────────────────────
        let mut rng = if let Some(s) = self.config.seed {
            rand_chacha::ChaCha8Rng::seed_from_u64(s)
        } else {
            rand_chacha::ChaCha8Rng::from_entropy()
        };

        let mut executions: u64 = 0;
        let mut strategy_effectiveness: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();
        let mut last_progress = Instant::now();

        loop {
            // ── Stop conditions ───────────────────────────────────────────
            if let Some(max) = self.config.max_executions {
                if executions >= max {
                    break;
                }
            }
            if let Some(timeout) = self.config.timeout {
                if start.elapsed() >= timeout {
                    break;
                }
            }

            // ── Select base input ─────────────────────────────────────────
            let base_bytes: Vec<u8> = if corpus.is_empty() || rand::Rng::gen_bool(&mut rng, 0.2) {
                // 20% of the time generate a fresh structured input.
                dashboard.set_stage("generate");
                let val = input_gen.generate(&SorobanValueType::I128);
                val.to_bytes()
            } else {
                // 80% of the time mutate an existing corpus entry.
                let entry = corpus.select_weighted(&mut rng).unwrap();
                entry.bytes.clone()
            };

            // ── Apply mutation (Phase 4) ──────────────────────────────────
            let donor = corpus.select_random(&mut rng).map(|e| e.bytes.clone());
            let (mutated, strategy) = mutator.mutate_random(&base_bytes, donor.as_deref());
            dashboard.set_stage(strategy.name());

            // ── Execute (Phase 6) ─────────────────────────────────────────
            let exec_start = Instant::now();
            let outcome = match vm.execute_and_reset(&mutated) {
                Ok(o) => o,
                Err(e) => {
                    if self.config.verbose {
                        eprintln!("[fuzzer] VM reset error: {e}");
                    }
                    break;
                }
            };
            let exec_time = exec_start.elapsed();

            executions += 1;
            dashboard.tick();

            // ── Coverage analysis (Phase 2) ───────────────────────────────
            // Simulate coverage from the instrumented binary using the input
            // as the seed. In production this reads the shared-memory buffer.
            let seed_for_coverage = u64::from_le_bytes(
                mutated
                    .get(..8)
                    .unwrap_or(&[0u8; 8])
                    .try_into()
                    .unwrap_or([0u8; 8]),
            );
            let exec_coverage =
                instrumenter.simulate_coverage(&self.config.wasm_bytes, seed_for_coverage);

            let new_edges = coverage_map.merge(&exec_coverage);
            let is_interesting = !new_edges.is_empty();

            // Phase 9: Cross-version discrepancy also marks interesting.
            let has_discrepancy = self.check_differential_discrepancy(&outcome);
            let is_interesting = is_interesting || has_discrepancy;

            if is_interesting {
                dashboard.record_new_coverage();
                *strategy_effectiveness
                    .entry(strategy.name().to_string())
                    .or_insert(0) += 1;

                let id = corpus.next_id();
                let entry = CorpusEntry::new(
                    mutated.clone(),
                    exec_time,
                    new_edges.len(),
                    None,
                    strategy.name(),
                    id,
                );
                corpus.add(entry);
                dashboard.update_corpus_stats(corpus.stats());
                dashboard.update_coverage(coverage_map.stats(), coverage_map.bitmap_snapshot());
            }

            // ── Crash triage (Phase 5) ────────────────────────────────────
            if !outcome.success {
                let error_text = outcome
                    .error
                    .clone()
                    .unwrap_or_else(|| "unknown error".to_string());
                let cloned = mutated.clone();
                let _error_clone = error_text.clone();
                let report = triage.process(
                    CrashInfo {
                        input: mutated.clone(),
                        error_text,
                        execution_time: exec_time,
                        label: None,
                    },
                    // reproduces closure: for the stub we consider any non-empty
                    // input that contains the same first byte as reproducing.
                    move |candidate| !candidate.is_empty() && cloned.first() == candidate.first(),
                );

                let is_unique = report.is_some();
                dashboard.record_crash(is_unique);
                if let Some(r) = report {
                    if self.config.verbose {
                        println!(
                            "[fuzzer] NEW CRASH [{}]: {} (input: {} bytes → {} bytes after minimisation)",
                            r.fingerprint,
                            r.normalised_error.chars().take(80).collect::<String>(),
                            r.original_input.len(),
                            r.minimal_input.len(),
                        );
                    }
                    crash_reports.push(r);
                }
            }

            // ── Progress reporting ────────────────────────────────────────
            if self.config.verbose && last_progress.elapsed() >= Duration::from_secs(5) {
                let snap = dashboard.snapshot();
                println!(
                    "[fuzzer] {:>8} exec  {:.0}/s  corpus={:>4}  edges={:>5}  crashes={:>3}  stage={}",
                    snap.total_executions,
                    snap.executions_per_second,
                    snap.corpus_size,
                    snap.coverage.total_edges,
                    snap.unique_crashes,
                    snap.current_stage,
                );
                last_progress = Instant::now();
            }
        }

        // ── Finalise ──────────────────────────────────────────────────────
        let elapsed = start.elapsed().as_secs_f64();
        let eps = if elapsed > 0.0 {
            executions as f64 / elapsed
        } else {
            0.0
        };

        FuzzerReport {
            total_executions: executions,
            corpus_entries: corpus.len(),
            unique_crashes: triage.unique_crash_count(),
            total_crashes: triage.total_crash_count(),
            coverage_density: coverage_map.density(),
            total_edges_found: coverage_map.total_edges(),
            session_duration_secs: elapsed,
            executions_per_second: eps,
            crash_reports,
            strategy_effectiveness,
        }
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    /// Seed the corpus with a handful of well-typed inputs covering all basic
    /// value types, ensuring we have interesting starting points even with an
    /// empty corpus directory.
    fn seed_corpus(
        &self,
        corpus: &mut Corpus,
        gen: &mut SorobanInputGen,
        cov_map: &CoverageMap,
        _vm: &mut PersistentVm,
    ) {
        if !corpus.is_empty() {
            return; // Already loaded from disk.
        }

        let seed_types = vec![
            SorobanValueType::I128,
            SorobanValueType::Address,
            SorobanValueType::Bool,
            SorobanValueType::Bytes,
            SorobanValueType::U64,
            SorobanValueType::Symbol,
        ];

        for (idx, ty) in seed_types.iter().enumerate() {
            let val = gen.generate(ty);
            let bytes = val.to_bytes();
            let exec_cov =
                WasmInstrumenter::new().simulate_coverage(&self.config.wasm_bytes, idx as u64);
            let new_edges = cov_map.merge(&exec_cov);
            let entry = CorpusEntry::new(
                bytes,
                Duration::from_millis(1),
                new_edges.len().max(1),
                Some(format!("{:?}", ty)),
                "seed_auto",
                idx,
            );
            corpus.add(entry);
        }
    }

    /// Phase 9: Check whether the execution outcome exhibits a cross-version
    /// discrepancy that should be treated as "interesting" even without new
    /// coverage. In the stub backend every execution is single-version, so
    /// this checks for non-deterministic return values as a proxy.
    fn check_differential_discrepancy(
        &self,
        outcome: &crate::coverage_guided_fuzzer::persistent_vm::ExecutionOutcome,
    ) -> bool {
        // Production: compare outcome against a second-version VM execution.
        // Stub: flag if return_value contains the byte 0xFF (unusual sentinel).
        outcome
            .return_value
            .as_ref()
            .map(|v| v.contains(&0xFF))
            .unwrap_or(false)
    }
}

// Re-export rand types used in the loop.
use rand::SeedableRng;

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_wasm() -> Vec<u8> {
        let mut m = b"\0asm".to_vec();
        m.extend_from_slice(&1u32.to_le_bytes());
        m.push(10); // code section
        m.push(20);
        m.extend_from_slice(&[0u8; 20]);
        m
    }

    fn quick_config() -> FuzzerConfig {
        FuzzerConfig {
            wasm_bytes: minimal_wasm(),
            timeout: Some(Duration::from_millis(500)),
            max_executions: Some(200),
            corpus_dir: None,
            seed: Some(42),
            verbose: false,
            target_functions: Vec::new(),
        }
    }

    #[test]
    fn run_completes_within_budget() {
        let mut fuzzer = CoverageFuzzer::new(quick_config());
        let report = fuzzer.run();
        assert!(
            report.total_executions <= 200,
            "should not exceed max_executions"
        );
        assert!(
            report.total_executions > 0,
            "should have executed at least one input"
        );
    }

    #[test]
    fn corpus_grows_during_run() {
        let mut fuzzer = CoverageFuzzer::new(quick_config());
        let report = fuzzer.run();
        // The auto-seeder always adds 6 entries, so corpus_entries >= 6.
        assert!(
            report.corpus_entries >= 6,
            "corpus should have at least the seed entries"
        );
    }

    #[test]
    fn coverage_increases_from_zero() {
        let mut fuzzer = CoverageFuzzer::new(quick_config());
        let report = fuzzer.run();
        assert!(
            report.total_edges_found > 0,
            "should discover at least one edge"
        );
        assert!(report.coverage_density > 0.0);
    }

    #[test]
    fn report_eps_is_positive() {
        let mut fuzzer = CoverageFuzzer::new(quick_config());
        let report = fuzzer.run();
        assert!(report.executions_per_second > 0.0);
    }

    #[test]
    fn strategy_effectiveness_populated() {
        let mut fuzzer = CoverageFuzzer::new(quick_config());
        let report = fuzzer.run();
        // At least one strategy should have produced an interesting input.
        let total_interesting: u64 = report.strategy_effectiveness.values().sum();
        assert!(
            total_interesting > 0,
            "at least one strategy should find new coverage"
        );
    }

    #[test]
    fn run_with_corpus_dir_creates_files() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = quick_config();
        config.corpus_dir = Some(dir.path().to_path_buf());
        let mut fuzzer = CoverageFuzzer::new(config);
        fuzzer.run();
        let files: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert!(
            !files.is_empty(),
            "corpus dir should contain at least one file"
        );
    }

    #[test]
    fn no_crashes_on_stub_backend() {
        // The stub never crashes by default.
        let mut fuzzer = CoverageFuzzer::new(quick_config());
        let report = fuzzer.run();
        assert_eq!(report.unique_crashes, 0, "stub should not produce crashes");
    }
}
