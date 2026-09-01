//! Differential Fuzzing Module for Soroban Security Scanner
//! 
//! This module implements differential testing against multiple versions of the Soroban SDK
//! to detect discrepancies in execution behavior, gas consumption, and state changes.

pub mod test_runner;
pub mod input_generator;
pub mod execution_tracer;
pub mod discrepancy_detector;
pub mod cross_contract_simulator;
pub mod ledger_snapshot_integration;
pub mod deterministic_detector;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use anyhow::Result;

pub use test_runner::{DifferentialTestRunner, TestConfig, TestResult};
pub use input_generator::{InputGenerator, EdgeCaseType};
pub use execution_tracer::{ExecutionTracer, ExecutionTrace, TraceEvent};
pub use discrepancy_detector::{DiscrepancyDetector, DiscrepancyType, DiscrepancyReport};
pub use cross_contract_simulator::{CrossContractSimulator, ReentrancyPattern, CallGraph};
pub use ledger_snapshot_integration::{LedgerSnapshotIntegration, NetworkState};
pub use deterministic_detector::{DeterministicDetector, NonDeterministicBehavior};

/// SDK version information for differential testing
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SdkVersion {
    pub version: String,
    pub git_hash: Option<String>,
    pub release_date: Option<String>,
}

impl SdkVersion {
    pub fn new(version: &str) -> Self {
        Self {
            version: version.to_string(),
            git_hash: None,
            release_date: None,
        }
    }

    pub fn with_git_hash(mut self, hash: &str) -> Self {
        self.git_hash = Some(hash.to_string());
        self
    }

    pub fn with_release_date(mut self, date: &str) -> Self {
        self.release_date = Some(date.to_string());
        self
    }
}

/// Test input for contract execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestInput {
    pub function_name: String,
    pub arguments: Vec<TestArgument>,
    pub salt: Option<[u8; 32]>,
    pub metadata: TestInputMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestArgument {
    pub value: ArgumentValue,
    pub argument_type: ArgumentType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArgumentValue {
    I128(i128),
    U64(u64),
    U32(u32),
    Bool(bool),
    Bytes(Vec<u8>),
    String(String),
    Address([u8; 32]),
    Vector(Vec<ArgumentValue>),
    Map(HashMap<String, ArgumentValue>),
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArgumentType {
    I128,
    U64,
    U32,
    Bool,
    Bytes,
    String,
    Address,
    Vector(Box<ArgumentType>),
    Map,
    Option(Box<ArgumentType>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestInputMetadata {
    pub edge_case_type: Option<EdgeCaseType>,
    pub generation_method: String,
    pub complexity_score: f64,
}

/// Execution result from a single SDK version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub sdk_version: SdkVersion,
    pub success: bool,
    pub return_value: Option<ArgumentValue>,
    pub gas_consumed: u64,
    pub state_changes: Vec<StateChange>,
    pub execution_trace: ExecutionTrace,
    pub error: Option<String>,
    pub execution_time: std::time::Duration,
}

/// State change detected during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    pub key: Vec<u8>,
    pub old_value: Option<Vec<u8>>,
    pub new_value: Option<Vec<u8>>,
    pub change_type: StateChangeType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateChangeType {
    Insert,
    Update,
    Delete,
    NoChange,
}

/// Differential fuzzing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferentialFuzzingConfig {
    pub sdk_versions: Vec<SdkVersion>,
    pub contract_path: String,
    pub test_count: usize,
    pub max_execution_time: std::time::Duration,
    pub enable_cross_contract_simulation: bool,
    pub enable_ledger_snapshot_integration: bool,
    pub enable_deterministic_detection: bool,
    pub edge_case_types: Vec<EdgeCaseType>,
    pub gas_threshold_percentage: f64,
}

impl Default for DifferentialFuzzingConfig {
    fn default() -> Self {
        Self {
            sdk_versions: vec![
                SdkVersion::new("25.3.0"),
                SdkVersion::new("25.2.0"),
                SdkVersion::new("25.1.0"),
            ],
            contract_path: "./target/wasm32-unknown-unknown/release".to_string(),
            test_count: 1000,
            max_execution_time: std::time::Duration::from_secs(30),
            enable_cross_contract_simulation: true,
            enable_ledger_snapshot_integration: true,
            enable_deterministic_detection: true,
            edge_case_types: vec![
                EdgeCaseType::MaxI128,
                EdgeCaseType::MinI128,
                EdgeCaseType::EmptyVector,
                EdgeCaseType::LargeVector,
                EdgeCaseType::ZeroValue,
            ],
            gas_threshold_percentage: 10.0,
        }
    }
}

/// Main differential fuzzing orchestrator
pub struct DifferentialFuzzer {
    config: DifferentialFuzzingConfig,
    test_runner: DifferentialTestRunner,
    input_generator: InputGenerator,
    execution_tracer: ExecutionTracer,
    discrepancy_detector: DiscrepancyDetector,
    cross_contract_simulator: Option<CrossContractSimulator>,
    ledger_integration: Option<LedgerSnapshotIntegration>,
    deterministic_detector: DeterministicDetector,
}

impl DifferentialFuzzer {
    pub fn new(config: DifferentialFuzzingConfig) -> Result<Self> {
        let test_runner = DifferentialTestRunner::new(config.sdk_versions.clone())?;
        let input_generator = InputGenerator::new(config.edge_case_types.clone());
        let execution_tracer = ExecutionTracer::new();
        let discrepancy_detector = DiscrepancyDetector::new(config.gas_threshold_percentage);
        let cross_contract_simulator = if config.enable_cross_contract_simulation {
            Some(CrossContractSimulator::new()?)
        } else {
            None
        };
        let ledger_integration = if config.enable_ledger_snapshot_integration {
            Some(LedgerSnapshotIntegration::new()?)
        } else {
            None
        };
        let deterministic_detector = DeterministicDetector::new();

        Ok(Self {
            config,
            test_runner,
            input_generator,
            execution_tracer,
            discrepancy_detector,
            cross_contract_simulator,
            ledger_integration,
            deterministic_detector,
        })
    }

    /// Run differential fuzzing analysis
    pub async fn run_fuzzing(&mut self) -> Result<DifferentialFuzzingReport> {
        let start_time = Instant::now();
        let mut all_results = Vec::new();
        let mut discrepancies = Vec::new();
        let mut non_deterministic_behaviors = Vec::new();

        // Generate test inputs
        let test_inputs = self.input_generator.generate_test_inputs(self.config.test_count)?;

        for (index, input) in test_inputs.into_iter().enumerate() {
            if index % 100 == 0 {
                println!("Running test {}/{}", index + 1, self.config.test_count);
            }

            // Execute test across all SDK versions
            let test_results = self.test_runner.execute_test(&input, &self.config).await?;

            // Detect discrepancies
            let detected_discrepancies = self.discrepancy_detector.detect_discrepancies(&test_results)?;
            discrepancies.extend(detected_discrepancies);

            // Detect non-deterministic behavior
            if self.config.enable_deterministic_detection {
                let non_deterministic = self.deterministic_detector.detect(&test_results)?;
                non_deterministic_behaviors.extend(non_deterministic);
            }

            // Run cross-contract simulation if enabled
            if let (Some(simulator), true) = (&mut self.cross_contract_simulator, self.config.enable_cross_contract_simulation) {
                let reentrancy_results = simulator.simulate_cross_contract_calls(&input, &self.config).await?;
                // Process reentrancy results...
            }

            all_results.push(test_results);
        }

        let execution_time = start_time.elapsed();

        Ok(DifferentialFuzzingReport {
            config: self.config.clone(),
            execution_time,
            test_results: all_results,
            discrepancies,
            non_deterministic_behaviors,
            summary: self.generate_summary(&all_results, &discrepancies, &non_deterministic_behaviors),
        })
    }

    fn generate_summary(
        &self,
        test_results: &[Vec<ExecutionResult>],
        discrepancies: &[DiscrepancyReport],
        non_deterministic_behaviors: &[NonDeterministicBehavior],
    ) -> DifferentialFuzzingSummary {
        DifferentialFuzzingSummary {
            total_tests: test_results.len(),
            total_discrepancies: discrepancies.len(),
            total_non_deterministic: non_deterministic_behaviors.len(),
            high_priority_issues: discrepancies.iter()
                .filter(|d| d.severity == crate::Severity::High || d.severity == crate::Severity::Critical)
                .count(),
            gas_discrepancies: discrepancies.iter()
                .filter(|d| matches!(d.discrepancy_type, DiscrepancyType::GasConsumption))
                .count(),
            state_discrepancies: discrepancies.iter()
                .filter(|d| matches!(d.discrepancy_type, DiscrepancyType::StateChange))
                .count(),
            logic_discrepancies: discrepancies.iter()
                .filter(|d| matches!(d.discrepancy_type, DiscrepancyType::LogicDivergence))
                .count(),
        }
    }
}

/// Comprehensive differential fuzzing report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferentialFuzzingReport {
    pub config: DifferentialFuzzingConfig,
    pub execution_time: std::time::Duration,
    pub test_results: Vec<Vec<ExecutionResult>>,
    pub discrepancies: Vec<DiscrepancyReport>,
    pub non_deterministic_behaviors: Vec<NonDeterministicBehavior>,
    pub summary: DifferentialFuzzingSummary,
}

/// Summary statistics for the fuzzing run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferentialFuzzingSummary {
    pub total_tests: usize,
    pub total_discrepancies: usize,
    pub total_non_deterministic: usize,
    pub high_priority_issues: usize,
    pub gas_discrepancies: usize,
    pub state_discrepancies: usize,
    pub logic_discrepancies: usize,
}

/// Edge case types for input generation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeCaseType {
    MaxI128,
    MinI128,
    ZeroValue,
    EmptyVector,
    LargeVector,
    EmptyString,
    LongString,
    NullAddress,
    MaxU64,
    ZeroU64,
    NegativeValue,
    BoundaryValue,
    RandomValue,
    Custom(String),
}

impl EdgeCaseType {
    pub fn description(&self) -> &'static str {
        match self {
            EdgeCaseType::MaxI128 => "Maximum i128 value",
            EdgeCaseType::MinI128 => "Minimum i128 value",
            EdgeCaseType::ZeroValue => "Zero value",
            EdgeCaseType::EmptyVector => "Empty vector",
            EdgeCaseType::LargeVector => "Large vector",
            EdgeCaseType::EmptyString => "Empty string",
            EdgeCaseType::LongString => "Long string",
            EdgeCaseType::NullAddress => "Null address",
            EdgeCaseType::MaxU64 => "Maximum u64 value",
            EdgeCaseType::ZeroU64 => "Zero u64 value",
            EdgeCaseType::NegativeValue => "Negative value",
            EdgeCaseType::BoundaryValue => "Boundary value",
            EdgeCaseType::RandomValue => "Random value",
            EdgeCaseType::Custom(_) => "Custom edge case",
        }
    }
}
