//! Deterministic Behavior Detector for Differential Fuzzing
//! 
//! Flags non-deterministic behavior as high-priority security vulnerabilities.

use crate::differential_fuzzing::{
    ExecutionResult, SdkVersion, TestInput, ArgumentValue
};
use crate::Severity;
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use anyhow::Result;

/// Types of non-deterministic behavior
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NonDeterministicType {
    RandomValueGeneration,
    TimeDependentLogic,
    ExternalStateDependency,
    NetworkCallVariation,
    BlockchainStateDependency,
    FloatingPointArithmetic,
    UninitializedMemory,
    ConcurrentExecution,
    HashCollision,
    ProbabilisticLogic,
}

/// Non-deterministic behavior detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonDeterministicBehavior {
    pub behavior_type: NonDeterministicType,
    pub severity: Severity,
    pub description: String,
    pub affected_versions: Vec<SdkVersion>,
    pub test_input: String,
    pub detection_method: DetectionMethod,
    pub confidence: f64,
    pub reproducibility_score: f64,
    pub impact_assessment: ImpactAssessment,
    pub mitigation_suggestions: Vec<String>,
}

/// Method used to detect the non-deterministic behavior
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetectionMethod {
    MultipleExecutionComparison,
    StateChangeAnalysis,
    TraceDivergenceAnalysis,
    GasVariationAnalysis,
    ReturnValueVariation,
    ErrorRateAnalysis,
    TimingAnalysis,
}

/// Impact assessment of the non-deterministic behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAssessment {
    pub security_impact: SecurityImpact,
    pub financial_impact: FinancialImpact,
    pub reliability_impact: ReliabilityImpact,
    pub exploitability: f64,
}

/// Security impact level
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityImpact {
    Critical,
    High,
    Medium,
    Low,
    None,
}

/// Financial impact level
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinancialImpact {
    Critical,
    High,
    Medium,
    Low,
    None,
}

/// Reliability impact level
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReliabilityImpact {
    Critical,
    High,
    Medium,
    Low,
    None,
}

/// Deterministic behavior detector configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeterministicDetectorConfig {
    pub execution_retries: usize,
    pub variation_threshold: f64,
    pub time_dependency_threshold: Duration,
    pub gas_variation_threshold: f64,
    pub trace_similarity_threshold: f64,
    pub enable_timing_analysis: bool,
    pub enable_state_analysis: bool,
    pub enable_trace_analysis: bool,
}

impl Default for DeterministicDetectorConfig {
    fn default() -> Self {
        Self {
            execution_retries: 5,
            variation_threshold: 0.1,
            time_dependency_threshold: Duration::from_millis(100),
            gas_variation_threshold: 0.05,
            trace_similarity_threshold: 0.95,
            enable_timing_analysis: true,
            enable_state_analysis: true,
            enable_trace_analysis: true,
        }
    }
}

/// Deterministic behavior detector
pub struct DeterministicDetector {
    config: DeterministicDetectorConfig,
    execution_history: HashMap<String, Vec<ExecutionResult>>,
    known_deterministic_patterns: HashSet<String>,
}

impl DeterministicDetector {
    pub fn new() -> Self {
        Self::with_config(DeterministicDetectorConfig::default())
    }

    pub fn with_config(config: DeterministicDetectorConfig) -> Self {
        Self {
            config,
            execution_history: HashMap::new(),
            known_deterministic_patterns: HashSet::new(),
        }
    }

    /// Detect non-deterministic behavior across multiple execution results
    pub fn detect(&mut self, results: &[ExecutionResult]) -> Result<Vec<NonDeterministicBehavior>> {
        let mut behaviors = Vec::new();

        // Group results by SDK version
        let results_by_version: HashMap<String, Vec<&ExecutionResult>> = results.iter()
            .fold(HashMap::new(), |mut acc, result| {
                acc.entry(result.sdk_version.version.clone())
                    .or_insert_with(Vec::new)
                    .push(result);
                acc
            });

        // Check for non-determinism within each version
        for (version, version_results) in results_by_version {
            if version_results.len() > 1 {
                let version_behaviors = self.detect_non_determinism_within_version(&version, version_results)?;
                behaviors.extend(version_behaviors);
            }
        }

        // Check for non-determinism across versions
        if results.len() > 1 {
            let cross_version_behaviors = self.detect_cross_version_non_determinism(results)?;
            behaviors.extend(cross_version_behaviors);
        }

        // Analyze specific patterns
        behaviors.extend(self.analyze_random_patterns(results)?);
        behaviors.extend(self.analyze_time_dependencies(results)?);
        behaviors.extend(self.analyze_external_dependencies(results)?);
        behaviors.extend(self.analyze_concurrent_execution(results)?);

        Ok(behaviors)
    }

    /// Detect non-determinism within a single SDK version
    fn detect_non_determinism_within_version(
        &self,
        version: &str,
        results: &[&ExecutionResult],
    ) -> Result<Vec<NonDeterministicBehavior>> {
        let mut behaviors = Vec::new();

        if results.len() < 2 {
            return Ok(behaviors);
        }

        // Check for return value variations
        if let Some(behavior) = self.check_return_value_variations(version, results)? {
            behaviors.push(behavior);
        }

        // Check for gas consumption variations
        if let Some(behavior) = self.check_gas_variations(version, results)? {
            behaviors.push(behavior);
        }

        // Check for state change variations
        if let Some(behavior) = self.check_state_variations(version, results)? {
            behaviors.push(behavior);
        }

        // Check for execution trace variations
        if let Some(behavior) = self.check_trace_variations(version, results)? {
            behaviors.push(behavior);
        }

        // Check for error rate variations
        if let Some(behavior) = self.check_error_rate_variations(version, results)? {
            behaviors.push(behavior);
        }

        Ok(behaviors)
    }

    /// Detect cross-version non-determinism
    fn detect_cross_version_non_determinism(&self, results: &[ExecutionResult]) -> Result<Vec<NonDeterministicBehavior>> {
        let mut behaviors = Vec::new();

        // Check if different versions produce different results for the same input
        let return_values_by_version: HashMap<String, Option<&ArgumentValue>> = results.iter()
            .map(|result| (result.sdk_version.version.clone(), result.return_value.as_ref()))
            .collect();

        let unique_return_values: HashSet<Option<&ArgumentValue>> = return_values_by_version.values().cloned().collect();
        
        if unique_return_values.len() > 1 {
            let behavior = NonDeterministicBehavior {
                behavior_type: NonDeterministicType::ExternalStateDependency,
                severity: Severity::High,
                description: "Different SDK versions produce different return values for the same input".to_string(),
                affected_versions: results.iter().map(|r| r.sdk_version.clone()).collect(),
                test_input: "cross_version_test".to_string(),
                detection_method: DetectionMethod::ReturnValueVariation,
                confidence: 0.9,
                reproducibility_score: 0.8,
                impact_assessment: ImpactAssessment {
                    security_impact: SecurityImpact::High,
                    financial_impact: FinancialImpact::Medium,
                    reliability_impact: ReliabilityImpact::High,
                    exploitability: 0.7,
                },
                mitigation_suggestions: vec![
                    "Standardize behavior across SDK versions".to_string(),
                    "Add version compatibility checks".to_string(),
                    "Document version-specific behavior".to_string(),
                ],
            };
            behaviors.push(behavior);
        }

        Ok(behaviors)
    }

    /// Check for return value variations
    fn check_return_value_variations(&self, version: &str, results: &[&ExecutionResult]) -> Result<Option<NonDeterministicBehavior>> {
        let return_values: Vec<Option<&ArgumentValue>> = results.iter()
            .map(|r| r.return_value.as_ref())
            .collect();

        let unique_values: HashSet<Option<&ArgumentValue>> = return_values.iter().cloned().collect();
        
        if unique_values.len() > 1 {
            Ok(Some(NonDeterministicBehavior {
                behavior_type: NonDeterministicType::RandomValueGeneration,
                severity: Severity::High,
                description: format!("Return value varies between executions in SDK version {}", version),
                affected_versions: vec![SdkVersion::new(version)],
                test_input: "return_value_test".to_string(),
                detection_method: DetectionMethod::ReturnValueVariation,
                confidence: 0.85,
                reproducibility_score: 0.3,
                impact_assessment: ImpactAssessment {
                    security_impact: SecurityImpact::High,
                    financial_impact: FinancialImpact::High,
                    reliability_impact: ReliabilityImpact::Critical,
                    exploitability: 0.8,
                },
                mitigation_suggestions: vec![
                    "Remove randomness from critical functions".to_string(),
                    "Use deterministic pseudo-random generators".to_string(),
                    "Seed random values with deterministic input".to_string(),
                ],
            }))
        } else {
            Ok(None)
        }
    }

    /// Check for gas consumption variations
    fn check_gas_variations(&self, version: &str, results: &[&ExecutionResult]) -> Result<Option<NonDeterministicBehavior>> {
        let gas_values: Vec<u64> = results.iter().map(|r| r.gas_consumed).collect();
        
        if gas_values.len() < 2 {
            return Ok(None);
        }

        let avg_gas = gas_values.iter().sum::<u64>() as f64 / gas_values.len() as f64;
        let variance = gas_values.iter()
            .map(|&gas| ((gas as f64 - avg_gas).powi(2) / avg_gas).abs())
            .sum::<f64>() / gas_values.len() as f64;

        if variance > self.config.gas_variation_threshold {
            Ok(Some(NonDeterministicBehavior {
                behavior_type: NonDeterministicType::BlockchainStateDependency,
                severity: Severity::Medium,
                description: format!("Gas consumption varies significantly (variance: {:.2}) in SDK version {}", version, variance),
                affected_versions: vec![SdkVersion::new(version)],
                test_input: "gas_variation_test".to_string(),
                detection_method: DetectionMethod::GasVariationAnalysis,
                confidence: 0.7,
                reproducibility_score: 0.5,
                impact_assessment: ImpactAssessment {
                    security_impact: SecurityImpact::Low,
                    financial_impact: FinancialImpact::Medium,
                    reliability_impact: ReliabilityImpact::Medium,
                    exploitability: 0.4,
                },
                mitigation_suggestions: vec![
                    "Optimize gas consumption consistency".to_string(),
                    "Avoid blockchain state dependencies in gas calculation".to_string(),
                    "Use fixed gas limits where possible".to_string(),
                ],
            }))
        } else {
            Ok(None)
        }
    }

    /// Check for state change variations
    fn check_state_variations(&self, version: &str, results: &[&ExecutionResult]) -> Result<Option<NonDeterministicBehavior>> {
        let state_changes_by_execution: Vec<Vec<&crate::differential_fuzzing::StateChange>> = results.iter()
            .map(|r| r.state_changes.iter().collect())
            .collect();

        // Check if state changes are consistent across executions
        let first_execution_state = &state_changes_by_execution[0];
        
        for (i, execution_state) in state_changes_by_execution.iter().enumerate().skip(1) {
            if execution_state.len() != first_execution_state.len() {
                return Ok(Some(NonDeterministicBehavior {
                    behavior_type: NonDeterministicType::ExternalStateDependency,
                    severity: Severity::High,
                    description: format!("State changes differ between executions in SDK version {}", version),
                    affected_versions: vec![SdkVersion::new(version)],
                    test_input: "state_variation_test".to_string(),
                    detection_method: DetectionMethod::StateChangeAnalysis,
                    confidence: 0.8,
                    reproducibility_score: 0.4,
                    impact_assessment: ImpactAssessment {
                        security_impact: SecurityImpact::Critical,
                        financial_impact: FinancialImpact::High,
                        reliability_impact: ReliabilityImpact::Critical,
                        exploitability: 0.9,
                    },
                    mitigation_suggestions: vec![
                        "Ensure state changes are deterministic".to_string(),
                        "Remove external state dependencies".to_string(),
                        "Use atomic operations for state changes".to_string(),
                    ],
                }));
            }

            // Check if individual state changes are consistent
            for (j, change1) in first_execution_state.iter().enumerate() {
                if let Some(change2) = execution_state.get(j) {
                    if change1.key != change2.key || change1.new_value != change2.new_value {
                        return Ok(Some(NonDeterministicBehavior {
                            behavior_type: NonDeterministicType::ExternalStateDependency,
                            severity: Severity::High,
                            description: format!("State change values differ between executions in SDK version {}", version),
                            affected_versions: vec![SdkVersion::new(version)],
                            test_input: "state_value_variation_test".to_string(),
                            detection_method: DetectionMethod::StateChangeAnalysis,
                            confidence: 0.85,
                            reproducibility_score: 0.3,
                            impact_assessment: ImpactAssessment {
                                security_impact: SecurityImpact::Critical,
                                financial_impact: FinancialImpact::High,
                                reliability_impact: ReliabilityImpact::Critical,
                                exploitability: 0.9,
                            },
                            mitigation_suggestions: vec![
                                "Ensure state change values are deterministic".to_string(),
                                "Validate external state before using it".to_string(),
                                "Use consistent state update patterns".to_string(),
                            ],
                        }));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Check for execution trace variations
    fn check_trace_variations(&self, version: &str, results: &[&ExecutionResult]) -> Result<Option<NonDeterministicBehavior>> {
        let traces: Vec<&crate::differential_fuzzing::ExecutionTrace> = results.iter()
            .map(|r| &r.execution_trace)
            .collect();

        if traces.len() < 2 {
            return Ok(None);
        }

        // Compare traces pairwise
        for i in 0..traces.len() {
            for j in (i + 1)..traces.len() {
                let similarity = traces[i].similarity_score(traces[j]);
                if similarity < self.config.trace_similarity_threshold {
                    return Ok(Some(NonDeterministicBehavior {
                        behavior_type: NonDeterministicType::ConcurrentExecution,
                        severity: Severity::Medium,
                        description: format!("Execution traces differ (similarity: {:.2}) in SDK version {}", version, similarity),
                        affected_versions: vec![SdkVersion::new(version)],
                        test_input: "trace_variation_test".to_string(),
                        detection_method: DetectionMethod::TraceDivergenceAnalysis,
                        confidence: 0.75,
                        reproducibility_score: 0.6,
                        impact_assessment: ImpactAssessment {
                            security_impact: SecurityImpact::Medium,
                            financial_impact: FinancialImpact::Low,
                            reliability_impact: ReliabilityImpact::High,
                            exploitability: 0.5,
                        },
                        mitigation_suggestions: vec![
                            "Ensure execution flow is deterministic".to_string(),
                            "Avoid conditional logic based on external factors".to_string(),
                            "Use consistent error handling".to_string(),
                        ],
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Check for error rate variations
    fn check_error_rate_variations(&self, version: &str, results: &[&ExecutionResult]) -> Result<Option<NonDeterministicBehavior>> {
        let success_count = results.iter().filter(|r| r.success).count();
        let error_count = results.len() - success_count;
        
        if results.len() >= 3 && (success_count > 0 && error_count > 0) {
            let error_rate = error_count as f64 / results.len() as f64;
            
            if error_rate > 0.1 && error_rate < 0.9 { // Intermittent errors
                return Ok(Some(NonDeterministicBehavior {
                    behavior_type: NonDeterministicType::NetworkCallVariation,
                    severity: Severity::Medium,
                    description: format!("Intermittent errors detected (error rate: {:.2}) in SDK version {}", version, error_rate),
                    affected_versions: vec![SdkVersion::new(version)],
                    test_input: "error_rate_test".to_string(),
                    detection_method: DetectionMethod::ErrorRateAnalysis,
                    confidence: 0.7,
                    reproducibility_score: 0.4,
                    impact_assessment: ImpactAssessment {
                        security_impact: SecurityImpact::Medium,
                        financial_impact: FinancialImpact::Medium,
                        reliability_impact: ReliabilityImpact::High,
                        exploitability: 0.6,
                    },
                    mitigation_suggestions: vec![
                        "Handle network errors gracefully".to_string(),
                        "Implement retry mechanisms".to_string(),
                        "Add circuit breakers for external calls".to_string(),
                    ],
                }));
            }
        }

        Ok(None)
    }

    /// Analyze random patterns in execution
    fn analyze_random_patterns(&self, results: &[ExecutionResult]) -> Result<Vec<NonDeterministicBehavior>> {
        let mut behaviors = Vec::new();

        for result in results {
            // Look for patterns that suggest randomness
            if self.contains_random_patterns(&result.execution_trace) {
                let behavior = NonDeterministicBehavior {
                    behavior_type: NonDeterministicType::RandomValueGeneration,
                    severity: Severity::High,
                    description: "Random value generation detected in execution trace".to_string(),
                    affected_versions: vec![result.sdk_version.clone()],
                    test_input: "random_pattern_test".to_string(),
                    detection_method: DetectionMethod::TraceDivergenceAnalysis,
                    confidence: 0.8,
                    reproducibility_score: 0.2,
                    impact_assessment: ImpactAssessment {
                        security_impact: SecurityImpact::High,
                        financial_impact: FinancialImpact::High,
                        reliability_impact: ReliabilityImpact::Medium,
                        exploitability: 0.7,
                    },
                    mitigation_suggestions: vec![
                        "Replace random values with deterministic alternatives".to_string(),
                        "Use commit-reveal schemes for randomness".to_string(),
                        "Seed randomness with block hash or timestamp".to_string(),
                    ],
                };
                behaviors.push(behavior);
            }
        }

        Ok(behaviors)
    }

    /// Analyze time-dependent logic
    fn analyze_time_dependencies(&self, results: &[ExecutionResult]) -> Result<Vec<NonDeterministicBehavior>> {
        let mut behaviors = Vec::new();

        for result in results {
            // Look for time-dependent patterns
            if self.contains_time_dependencies(&result.execution_trace) {
                let behavior = NonDeterministicBehavior {
                    behavior_type: NonDeterministicType::TimeDependentLogic,
                    severity: Severity::Medium,
                    description: "Time-dependent logic detected in execution".to_string(),
                    affected_versions: vec![result.sdk_version.clone()],
                    test_input: "time_dependency_test".to_string(),
                    detection_method: DetectionMethod::TimingAnalysis,
                    confidence: 0.7,
                    reproducibility_score: 0.5,
                    impact_assessment: ImpactAssessment {
                        security_impact: SecurityImpact::Medium,
                        financial_impact: FinancialImpact::Low,
                        reliability_impact: ReliabilityImpact::Medium,
                        exploitability: 0.5,
                    },
                    mitigation_suggestions: vec![
                        "Use block timestamp instead of system time".to_string(),
                        "Add time windows instead of exact timestamps".to_string(),
                        "Consider time-based attacks in security analysis".to_string(),
                    ],
                };
                behaviors.push(behavior);
            }
        }

        Ok(behaviors)
    }

    /// Analyze external dependencies
    fn analyze_external_dependencies(&self, results: &[ExecutionResult]) -> Result<Vec<NonDeterministicBehavior>> {
        let mut behaviors = Vec::new();

        for result in results {
            // Look for external call patterns
            if self.contains_external_dependencies(&result.execution_trace) {
                let behavior = NonDeterministicBehavior {
                    behavior_type: NonDeterministicType::ExternalStateDependency,
                    severity: Severity::High,
                    description: "External state dependency detected in execution".to_string(),
                    affected_versions: vec![result.sdk_version.clone()],
                    test_input: "external_dependency_test".to_string(),
                    detection_method: DetectionMethod::TraceDivergenceAnalysis,
                    confidence: 0.8,
                    reproducibility_score: 0.3,
                    impact_assessment: ImpactAssessment {
                        security_impact: SecurityImpact::High,
                        financial_impact: FinancialImpact::Medium,
                        reliability_impact: ReliabilityImpact::High,
                        exploitability: 0.8,
                    },
                    mitigation_suggestions: vec![
                        "Cache external state when possible".to_string(),
                        "Validate external state consistency".to_string(),
                        "Handle external state changes gracefully".to_string(),
                    ],
                };
                behaviors.push(behavior);
            }
        }

        Ok(behaviors)
    }

    /// Analyze concurrent execution patterns
    fn analyze_concurrent_execution(&self, results: &[ExecutionResult]) -> Result<Vec<NonDeterministicBehavior>> {
        let mut behaviors = Vec::new();

        for result in results {
            // Look for concurrent execution patterns
            if self.contains_concurrent_patterns(&result.execution_trace) {
                let behavior = NonDeterministicBehavior {
                    behavior_type: NonDeterministicType::ConcurrentExecution,
                    severity: Severity::High,
                    description: "Concurrent execution pattern detected".to_string(),
                    affected_versions: vec![result.sdk_version.clone()],
                    test_input: "concurrent_execution_test".to_string(),
                    detection_method: DetectionMethod::TraceDivergenceAnalysis,
                    confidence: 0.75,
                    reproducibility_score: 0.4,
                    impact_assessment: ImpactAssessment {
                        security_impact: SecurityImpact::High,
                        financial_impact: FinancialImpact::High,
                        reliability_impact: ReliabilityImpact::Critical,
                        exploitability: 0.8,
                    },
                    mitigation_suggestions: vec![
                        "Implement proper synchronization mechanisms".to_string(),
                        "Use atomic operations for shared state".to_string(),
                        "Consider mutex or lock patterns".to_string(),
                    ],
                };
                behaviors.push(behavior);
            }
        }

        Ok(behaviors)
    }

    /// Check if execution trace contains random patterns
    fn contains_random_patterns(&self, trace: &crate::differential_fuzzing::ExecutionTrace) -> bool {
        let random_indicators = vec![
            "rand", "random", "rng", "entropy", "nonce", "seed",
        ];

        for event in &trace.events {
            let description = event.description.to_lowercase();
            if random_indicators.iter().any(|indicator| description.contains(indicator)) {
                return true;
            }
        }

        false
    }

    /// Check if execution trace contains time dependencies
    fn contains_time_dependencies(&self, trace: &crate::differential_fuzzing::ExecutionTrace) -> bool {
        let time_indicators = vec![
            "time", "timestamp", "now", "blocktime", "deadline", "duration",
        ];

        for event in &trace.events {
            let description = event.description.to_lowercase();
            if time_indicators.iter().any(|indicator| description.contains(indicator)) {
                return true;
            }
        }

        false
    }

    /// Check if execution trace contains external dependencies
    fn contains_external_dependencies(&self, trace: &crate::differential_fuzzing::ExecutionTrace) -> bool {
        let external_indicators = vec![
            "external", "oracle", "api", "network", "http", "rpc",
        ];

        for event in &trace.events {
            let description = event.description.to_lowercase();
            if external_indicators.iter().any(|indicator| description.contains(indicator)) {
                return true;
            }
        }

        false
    }

    /// Check if execution trace contains concurrent patterns
    fn contains_concurrent_patterns(&self, trace: &crate::differential_fuzzing::ExecutionTrace) -> bool {
        let concurrent_indicators = vec![
            "concurrent", "parallel", "thread", "async", "await", "spawn",
        ];

        for event in &trace.events {
            let description = event.description.to_lowercase();
            if concurrent_indicators.iter().any(|indicator| description.contains(indicator)) {
                return true;
            }
        }

        false
    }

    /// Store execution result for historical analysis
    pub fn store_execution_result(&mut self, test_input: &str, result: ExecutionResult) {
        let key = format!("{}:{}", test_input, result.sdk_version.version);
        self.execution_history.entry(key)
            .or_insert_with(Vec::new)
            .push(result);
    }

    /// Get execution history for a specific test input
    pub fn get_execution_history(&self, test_input: &str, version: &str) -> Option<&[ExecutionResult]> {
        let key = format!("{}:{}", test_input, version);
        self.execution_history.get(&key).map(|results| results.as_slice())
    }

    /// Clear execution history
    pub fn clear_history(&mut self) {
        self.execution_history.clear();
    }

    /// Get detector statistics
    pub fn get_statistics(&self) -> DetectorStats {
        DetectorStats {
            total_executions_stored: self.execution_history.values()
                .map(|results| results.len())
                .sum(),
            unique_test_inputs: self.execution_history.len(),
            known_patterns: self.known_deterministic_patterns.len(),
            config: self.config.clone(),
        }
    }

    /// Configure detector settings
    pub fn configure(&mut self, config: DeterministicDetectorConfig) {
        self.config = config;
    }
}

/// Detector statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorStats {
    pub total_executions_stored: usize,
    pub unique_test_inputs: usize,
    pub known_patterns: usize,
    pub config: DeterministicDetectorConfig,
}
