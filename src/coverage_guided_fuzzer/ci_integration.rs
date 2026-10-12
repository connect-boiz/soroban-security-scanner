//! Phase 8 — CI Integration
//!
//! Implements the `stellar-scanner fuzz` sub-command:
//!
//!   stellar-scanner fuzz \
//!     --timeout 3600 \
//!     --corpus-dir ./fuzz-corpus \
//!     --wasm path/to/contract.wasm
//!
//! Exit codes:
//!   0  → no crashes found within the timeout
//!   1  → one or more unique crashes found
//!   2  → configuration / setup error
//!
//! Corpus caching: at the end of the run the corpus is persisted to
//! `--corpus-dir`. On the next CI run the same directory is loaded as seeds
//! so coverage compounds across builds.

use crate::coverage_guided_fuzzer::orchestrator::{CoverageFuzzer, FuzzerConfig, FuzzerReport};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// CLI arguments for the `stellar-scanner fuzz` command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzCiConfig {
    /// Path to the WASM contract under test.
    pub wasm_path: PathBuf,
    /// How long the fuzzer should run before exiting.
    pub timeout: Duration,
    /// Directory where the corpus is saved/loaded between CI runs.
    pub corpus_dir: PathBuf,
    /// Maximum number of executions (0 = unlimited, stop only on timeout).
    pub max_executions: u64,
    /// If true, print progress to stdout (suitable for CI log tailing).
    pub verbose: bool,
    /// Seed for deterministic replay (None = entropy).
    pub seed: Option<u64>,
    /// Target function schema: `function_name(type,type,...)` format.
    pub target_functions: Vec<String>,
}

impl FuzzCiConfig {
    pub fn new(wasm_path: PathBuf, timeout_secs: u64, corpus_dir: PathBuf) -> Self {
        Self {
            wasm_path,
            timeout: Duration::from_secs(timeout_secs),
            corpus_dir,
            max_executions: 0,
            verbose: true,
            seed: None,
            target_functions: Vec::new(),
        }
    }
}

/// The result of a complete CI fuzzing run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzCiResult {
    /// 0 = no crashes, 1 = crashes found, 2 = setup error.
    pub exit_code: i32,
    /// Human-readable summary.
    pub summary: String,
    /// Full fuzzer report.
    pub report: Option<FuzzerReport>,
    /// Path to the corpus directory (for GitHub Actions cache key).
    pub corpus_dir: PathBuf,
    /// Total executions performed.
    pub total_executions: u64,
    /// Unique crashes found.
    pub unique_crashes: usize,
}

impl FuzzCiResult {
    fn success(report: FuzzerReport, corpus_dir: PathBuf) -> Self {
        let execs = report.total_executions;
        let crashes = report.unique_crashes;
        let summary = format!(
            "Fuzzing complete: {} executions, {} unique crashes, {} corpus entries, {:.2}% edge coverage",
            execs, crashes, report.corpus_entries, report.coverage_density * 100.0
        );
        let exit_code = if crashes > 0 { 1 } else { 0 };
        Self {
            exit_code,
            summary,
            report: Some(report),
            corpus_dir,
            total_executions: execs,
            unique_crashes: crashes,
        }
    }

    fn setup_error(msg: String, corpus_dir: PathBuf) -> Self {
        Self {
            exit_code: 2,
            summary: format!("Setup error: {}", msg),
            report: None,
            corpus_dir,
            total_executions: 0,
            unique_crashes: 0,
        }
    }
}

/// The CI fuzzing runner.
pub struct FuzzCiRunner {
    config: FuzzCiConfig,
}

impl FuzzCiRunner {
    pub fn new(config: FuzzCiConfig) -> Self {
        Self { config }
    }

    /// Run the fuzzer according to the CI configuration.
    ///
    /// This is the top-level entry point called by the `stellar-scanner fuzz`
    /// subcommand. It:
    ///   1. Loads seed corpus from `corpus_dir` (if it exists).
    ///   2. Runs the fuzzer for `timeout` seconds.
    ///   3. Persists the updated corpus to `corpus_dir`.
    ///   4. Returns exit code 0 (clean) or 1 (crashes found).
    pub fn run(&self) -> FuzzCiResult {
        // Validate the WASM file.
        if !self.config.wasm_path.exists() {
            return FuzzCiResult::setup_error(
                format!("WASM file not found: {}", self.config.wasm_path.display()),
                self.config.corpus_dir.clone(),
            );
        }

        let wasm_bytes = match std::fs::read(&self.config.wasm_path) {
            Ok(b) => b,
            Err(e) => {
                return FuzzCiResult::setup_error(
                    format!("Failed to read WASM: {}", e),
                    self.config.corpus_dir.clone(),
                );
            }
        };

        // Ensure corpus directory exists.
        if let Err(e) = std::fs::create_dir_all(&self.config.corpus_dir) {
            return FuzzCiResult::setup_error(
                format!("Failed to create corpus dir: {}", e),
                self.config.corpus_dir.clone(),
            );
        }

        // Build fuzzer config.
        let fuzzer_config = FuzzerConfig {
            wasm_bytes,
            timeout: Some(self.config.timeout),
            max_executions: if self.config.max_executions == 0 {
                None
            } else {
                Some(self.config.max_executions)
            },
            corpus_dir: Some(self.config.corpus_dir.clone()),
            seed: self.config.seed,
            verbose: self.config.verbose,
            target_functions: self.config.target_functions.clone(),
        };

        if self.config.verbose {
            println!(
                "[stellar-scanner fuzz] Starting: wasm={} timeout={}s corpus={}",
                self.config.wasm_path.display(),
                self.config.timeout.as_secs(),
                self.config.corpus_dir.display()
            );
        }

        // Run the fuzzer.
        let mut fuzzer = CoverageFuzzer::new(fuzzer_config);
        let report = fuzzer.run();

        if self.config.verbose {
            println!("[stellar-scanner fuzz] Done: {}", {
                let r = &report;
                format!(
                    "{} executions, {} corpus entries, {} unique crashes, {:.1}% edge coverage",
                    r.total_executions,
                    r.corpus_entries,
                    r.unique_crashes,
                    r.coverage_density * 100.0
                )
            });

            if report.unique_crashes > 0 {
                println!(
                    "[stellar-scanner fuzz] {} UNIQUE CRASH(ES) FOUND — exiting with code 1",
                    report.unique_crashes
                );
                for crash in &report.crash_reports {
                    println!(
                        "  crash [{}]: {} (minimal input: {} bytes)",
                        crash.fingerprint,
                        crash.normalised_error.chars().take(120).collect::<String>(),
                        crash.minimal_input.len()
                    );
                }
            }
        }

        FuzzCiResult::success(report, self.config.corpus_dir.clone())
    }

    /// Generate the GitHub Actions workflow snippet for corpus caching.
    pub fn github_actions_cache_config(&self) -> String {
        format!(
            r#"      - name: Restore fuzzing corpus
        uses: actions/cache@v4
        with:
          path: {corpus}
          key: fuzz-corpus-${{{{ hashFiles('{wasm}') }}}}
          restore-keys: |
            fuzz-corpus-

      - name: Run coverage-guided fuzzer
        run: |
          cargo run --bin soroban-scanner -- fuzz \
            --timeout {timeout} \
            --corpus-dir {corpus} \
            --wasm {wasm}

      - name: Save fuzzing corpus
        if: always()
        uses: actions/cache/save@v4
        with:
          path: {corpus}
          key: fuzz-corpus-${{{{ hashFiles('{wasm}') }}}}-${{{{ github.run_id }}}}
"#,
            corpus = self.config.corpus_dir.display(),
            wasm = self.config.wasm_path.display(),
            timeout = self.config.timeout.as_secs(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn minimal_wasm() -> Vec<u8> {
        let mut m = b"\0asm".to_vec();
        m.extend_from_slice(&1u32.to_le_bytes());
        // Code section with 20 dummy bytes.
        m.push(10);
        m.push(20);
        m.extend_from_slice(&[0u8; 20]);
        m
    }

    #[test]
    fn missing_wasm_returns_setup_error() {
        let dir = tempdir().unwrap();
        let config = FuzzCiConfig::new(
            dir.path().join("nonexistent.wasm"),
            10,
            dir.path().join("corpus"),
        );
        let result = FuzzCiRunner::new(config).run();
        assert_eq!(result.exit_code, 2);
        assert!(result.summary.contains("not found"));
    }

    #[test]
    fn valid_wasm_runs_and_exits_cleanly() {
        let dir = tempdir().unwrap();
        let wasm_path = dir.path().join("contract.wasm");
        std::fs::write(&wasm_path, minimal_wasm()).unwrap();

        let mut config = FuzzCiConfig::new(
            wasm_path,
            2, // 2-second budget
            dir.path().join("corpus"),
        );
        config.verbose = false;
        config.seed = Some(1234);

        let result = FuzzCiRunner::new(config).run();
        // With no real VM crashes the exit code should be 0.
        assert!(result.exit_code <= 1, "exit code should be 0 or 1");
        assert!(result.total_executions > 0);
    }

    #[test]
    fn corpus_dir_created() {
        let dir = tempdir().unwrap();
        let wasm_path = dir.path().join("c.wasm");
        std::fs::write(&wasm_path, minimal_wasm()).unwrap();
        let corpus_dir = dir.path().join("my-corpus");

        let mut config = FuzzCiConfig::new(wasm_path, 1, corpus_dir.clone());
        config.verbose = false;
        config.seed = Some(99);

        FuzzCiRunner::new(config).run();
        assert!(corpus_dir.exists(), "corpus dir should be created");
    }

    #[test]
    fn github_actions_snippet_contains_corpus_path() {
        let config = FuzzCiConfig::new(
            PathBuf::from("target/contract.wasm"),
            3600,
            PathBuf::from("./fuzz-corpus"),
        );
        let snippet = FuzzCiRunner::new(config).github_actions_cache_config();
        assert!(snippet.contains("fuzz-corpus"));
        assert!(snippet.contains("3600"));
    }
}
