//! Discrepancy Detector for Differential Fuzzing
//! 
//! Identifies discrepancies in execution results between different SDK versions.

use crate::differential_fuzzing::{
    ExecutionResult, SdkVersion, StateChange, StateChangeType, ArgumentValue
};
use crate::Severity;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use anyhow::Result;

/// Types of discrepancies that can be detected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiscrepancyType {
    GasConsumption,
    StateChange,
    LogicDivergence,
    ReturnValue,
    ErrorDifference,
    ExecutionOrder,
    MemoryUsage,
    ExternalCallBehavior,
}

/// Detected discrepancy between SDK versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscrepancyReport {
    pub discrepancy_type: DiscrepancyType,
    pub severity: Severity,
    pub description: String,
    pub affected_versions: Vec<SdkVersion>,
    pub test_input: String,
    pub details: DiscrepancyDetails,
    pub recommendation: String,
    pub confidence: f64,
}

/// Detailed information about the discrepancy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscrepancyDetails {
    GasDiscrepancy(GasDiscrepancyDetails),
    StateDiscrepancy(StateDiscrepancyDetails),
    LogicDivergence(LogicDivergenceDetails),
    ReturnValueDifference(ReturnValueDifferenceDetails),
    ErrorDifference(ErrorDifferenceDetails),
    ExecutionOrderDifference(ExecutionOrderDifferenceDetails),
    MemoryUsageDifference(MemoryUsageDifferenceDetails),
    ExternalCallDifference(ExternalCallDifferenceDetails),
}

/// Gas consumption discrepancy details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasDiscrepancyDetails {
    pub gas_values: HashMap<String, u64>,
    pub percentage_difference: f64,
    pub absolute_difference: u64,
    pub threshold_exceeded: bool,
}

/// State change discrepancy details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDiscrepancyDetails {
    pub state_changes_by_version: HashMap<String, Vec<StateChange>>,
    pub missing_changes: HashMap<String, Vec<StateChange>>,
    pub extra_changes: HashMap<String, Vec<StateChange>>,
    pub different_values: HashMap<String, Vec<StateChangeDiff>>,
}

/// Difference in state change values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChangeDiff {
    pub key: Vec<u8>,
    pub version1_value: Option<Vec<u8>>,
    pub version2_value: Option<Vec<u8>>,
    pub change_type: StateChangeType,
}

/// Logic divergence details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicDivergenceDetails {
    pub divergence_point: Option<usize>,
    pub trace_similarity: f64,
    pub different_paths: HashMap<String, Vec<String>>,
    pub branch_differences: Vec<BranchDifference>,
}

/// Branch difference in execution flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchDifference {
    pub line_number: usize,
    pub version1_taken: bool,
    pub version2_taken: bool,
    pub condition: String,
}

/// Return value difference details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnValueDifferenceDetails {
    pub return_values: HashMap<String, Option<ArgumentValue>>,
    pub value_types_match: bool,
    pub numeric_difference: Option<i128>,
}

/// Error difference details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDifferenceDetails {
    pub errors: HashMap<String, Option<String>>,
    pub error_types_match: bool,
    pub success_mismatch: bool,
}

/// Execution order difference details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOrderDifferenceDetails {
    pub event_sequences: HashMap<String, Vec<String>>,
    pub order_mismatch: bool,
    pub missing_events: HashMap<String, Vec<String>>,
    pub extra_events: HashMap<String, Vec<String>>,
}

/// Memory usage difference details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsageDifferenceDetails {
    pub memory_usage: HashMap<String, u64>,
    pub peak_memory: HashMap<String, u64>,
    pub allocation_count: HashMap<String, usize>,
    pub percentage_difference: f64,
}

/// External call behavior difference details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalCallDifferenceDetails {
    pub external_calls: HashMap<String, Vec<ExternalCallInfo>>,
    pub call_count_mismatch: bool,
    pub call_parameter_differences: Vec<CallParameterDifference>,
}

/// External call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalCallInfo {
    pub contract_address: String,
    pub function_name: String,
    pub parameters: Vec<String>,
    pub success: bool,
    pub return_value: Option<String>,
}

/// Call parameter difference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallParameterDifference {
    pub contract_address: String,
    pub function_name: String,
    pub parameter_index: usize,
    pub version1_value: String,
    pub version2_value: String,
}

/// Discrepancy detector that analyzes execution results
pub struct DiscrepancyDetector {
    gas_threshold_percentage: f64,
    state_change_sensitivity: f64,
    trace_similarity_threshold: f64,
}

impl DiscrepancyDetector {
    pub fn new(gas_threshold_percentage: f64) -> Self {
        Self {
            gas_threshold_percentage,
            state_change_sensitivity: 0.1,
            trace_similarity_threshold: 0.95,
        }
    }

    /// Detect discrepancies across multiple execution results
    pub fn detect_discrepancies(&self, results: &[ExecutionResult]) -> Result<Vec<DiscrepancyReport>> {
        let mut discrepancies = Vec::new();
        
        if results.len() < 2 {
            return Ok(discrepancies);
        }

        // Compare each pair of results
        for i in 0..results.len() {
            for j in (i + 1)..results.len() {
                let pair_discrepancies = self.compare_pair(&results[i], &results[j])?;
                discrepancies.extend(pair_discrepancies);
            }
        }

        // Remove duplicates and merge similar discrepancies
        self.deduplicate_and_merge(&mut discrepancies);

        Ok(discrepancies)
    }

    /// Compare two execution results for discrepancies
    fn compare_pair(&self, result1: &ExecutionResult, result2: &ExecutionResult) -> Result<Vec<DiscrepancyReport>> {
        let mut discrepancies = Vec::new();

        // Check gas consumption differences
        if let Some(discrepancy) = self.detect_gas_discrepancy(result1, result2)? {
            discrepancies.push(discrepancy);
        }

        // Check state change differences
        if let Some(discrepancy) = self.detect_state_discrepancy(result1, result2)? {
            discrepancies.push(discrepancy);
        }

        // Check logic divergence
        if let Some(discrepancy) = self.detect_logic_divergence(result1, result2)? {
            discrepancies.push(discrepancy);
        }

        // Check return value differences
        if let Some(discrepancy) = self.detect_return_value_difference(result1, result2)? {
            discrepancies.push(discrepancy);
        }

        // Check error differences
        if let Some(discrepancy) = self.detect_error_difference(result1, result2)? {
            discrepancies.push(discrepancy);
        }

        // Check execution order differences
        if let Some(discrepancy) = self.detect_execution_order_difference(result1, result2)? {
            discrepancies.push(discrepancy);
        }

        // Check memory usage differences
        if let Some(discrepancy) = self.detect_memory_usage_difference(result1, result2)? {
            discrepancies.push(discrepancy);
        }

        // Check external call differences
        if let Some(discrepancy) = self.detect_external_call_difference(result1, result2)? {
            discrepancies.push(discrepancy);
        }

        Ok(discrepancies)
    }

    /// Detect gas consumption discrepancies
    fn detect_gas_discrepancy(&self, result1: &ExecutionResult, result2: &ExecutionResult) -> Result<Option<DiscrepancyReport>> {
        let gas1 = result1.gas_consumed;
        let gas2 = result2.gas_consumed;

        if gas1 == 0 && gas2 == 0 {
            return Ok(None);
        }

        let avg_gas = (gas1 + gas2) as f64 / 2.0;
        let absolute_diff = gas1.abs_diff(gas2) as f64;
        let percentage_diff = (absolute_diff / avg_gas) * 100.0;

        if percentage_diff > self.gas_threshold_percentage {
            let mut gas_values = HashMap::new();
            gas_values.insert(result1.sdk_version.version.clone(), gas1);
            gas_values.insert(result2.sdk_version.version.clone(), gas2);

            let details = GasDiscrepancyDetails {
                gas_values,
                percentage_difference: percentage_diff,
                absolute_difference: gas1.abs_diff(gas2),
                threshold_exceeded: true,
            };

            Ok(Some(DiscrepancyReport {
                discrepancy_type: DiscrepancyType::GasConsumption,
                severity: self.calculate_gas_severity(percentage_diff),
                description: format!(
                    "Gas consumption differs by {:.2}% between SDK versions",
                    percentage_diff
                ),
                affected_versions: vec![result1.sdk_version.clone(), result2.sdk_version.clone()],
                test_input: "test_input".to_string(), // Would be populated with actual input
                details: DiscrepancyDetails::GasDiscrepancy(details),
                recommendation: "Investigate gas optimization differences between SDK versions".to_string(),
                confidence: self.calculate_confidence(percentage_diff, self.gas_threshold_percentage),
            }))
        } else {
            Ok(None)
        }
    }

    /// Detect state change discrepancies
    fn detect_state_discrepancy(&self, result1: &ExecutionResult, result2: &ExecutionResult) -> Result<Option<DiscrepancyReport>> {
        let state1 = &result1.state_changes;
        let state2 = &result2.state_changes;

        if state1.is_empty() && state2.is_empty() {
            return Ok(None);
        }

        let mut state_changes_by_version = HashMap::new();
        state_changes_by_version.insert(result1.sdk_version.version.clone(), state1.clone());
        state_changes_by_version.insert(result2.sdk_version.version.clone(), state2.clone());

        let (missing_changes, extra_changes, different_values) = self.compare_state_changes(state1, state2);

        let total_differences = missing_changes.len() + extra_changes.len() + different_values.len();
        
        if total_differences > 0 {
            let mut missing_map = HashMap::new();
            missing_map.insert(result1.sdk_version.version.clone(), missing_changes);
            let mut extra_map = HashMap::new();
            extra_map.insert(result2.sdk_version.version.clone(), extra_changes);
            let mut diff_map = HashMap::new();
            diff_map.insert("difference".to_string(), different_values);

            let details = StateDiscrepancyDetails {
                state_changes_by_version,
                missing_changes: missing_map,
                extra_changes: extra_map,
                different_values: diff_map,
            };

            Ok(Some(DiscrepancyReport {
                discrepancy_type: DiscrepancyType::StateChange,
                severity: Severity::High,
                description: format!("{} state change differences detected", total_differences),
                affected_versions: vec![result1.sdk_version.clone(), result2.sdk_version.clone()],
                test_input: "test_input".to_string(),
                details: DiscrepancyDetails::StateDiscrepancy(details),
                recommendation: "Review state management logic for consistency across SDK versions".to_string(),
                confidence: 0.9,
            }))
        } else {
            Ok(None)
        }
    }

    /// Compare state changes between two versions
    fn compare_state_changes(
        &self,
        state1: &[StateChange],
        state2: &[StateChange],
    ) -> (Vec<StateChange>, Vec<StateChange>, Vec<StateChangeDiff>) {
        let mut missing_changes = Vec::new();
        let mut extra_changes = Vec::new();
        let mut different_values = Vec::new();

        let state1_map: HashMap<Vec<u8>, &StateChange> = state1.iter()
            .map(|sc| (sc.key.clone(), sc))
            .collect();
        
        let state2_map: HashMap<Vec<u8>, &StateChange> = state2.iter()
            .map(|sc| (sc.key.clone(), sc))
            .collect();

        // Find missing and different changes
        for (key, change1) in &state1_map {
            if let Some(change2) = state2_map.get(key) {
                if change1.new_value != change2.new_value {
                    different_values.push(StateChangeDiff {
                        key: key.clone(),
                        version1_value: change1.new_value.clone(),
                        version2_value: change2.new_value.clone(),
                        change_type: change1.change_type.clone(),
                    });
                }
            } else {
                missing_changes.push((*change1).clone());
            }
        }

        // Find extra changes
        for (key, change2) in &state2_map {
            if !state1_map.contains_key(key) {
                extra_changes.push((*change2).clone());
            }
        }

        (missing_changes, extra_changes, different_values)
    }

    /// Detect logic divergence through trace analysis
    fn detect_logic_divergence(&self, result1: &ExecutionResult, result2: &ExecutionResult) -> Result<Option<DiscrepancyReport>> {
        let similarity = result1.execution_trace.similarity_score(&result2.execution_trace);
        
        if similarity < self.trace_similarity_threshold {
            let divergence_point = result1.execution_trace.find_divergence_point(&result2.execution_trace);
            
            let mut different_paths = HashMap::new();
            different_paths.insert(result1.sdk_version.version.clone(), vec!["path1".to_string()]);
            different_paths.insert(result2.sdk_version.version.clone(), vec!["path2".to_string()]);

            let details = LogicDivergenceDetails {
                divergence_point,
                trace_similarity: similarity,
                different_paths,
                branch_differences: Vec::new(),
            };

            Ok(Some(DiscrepancyReport {
                discrepancy_type: DiscrepancyType::LogicDivergence,
                severity: self.calculate_logic_severity(similarity),
                description: format!("Logic divergence detected with {:.2}% trace similarity", similarity * 100.0),
                affected_versions: vec![result1.sdk_version.clone(), result2.sdk_version.clone()],
                test_input: "test_input".to_string(),
                details: DiscrepancyDetails::LogicDivergence(details),
                recommendation: "Review execution flow and conditional logic for SDK version compatibility".to_string(),
                confidence: 1.0 - similarity,
            }))
        } else {
            Ok(None)
        }
    }

    /// Detect return value differences
    fn detect_return_value_difference(&self, result1: &ExecutionResult, result2: &ExecutionResult) -> Result<Option<DiscrepancyReport>> {
        match (&result1.return_value, &result2.return_value) {
            (Some(val1), Some(val2)) if val1 != val2 => {
                let mut return_values = HashMap::new();
                return_values.insert(result1.sdk_version.version.clone(), Some(val1.clone()));
                return_values.insert(result2.sdk_version.version.clone(), Some(val2.clone()));

                let numeric_difference = self.calculate_numeric_difference(val1, val2);

                let details = ReturnValueDifferenceDetails {
                    return_values,
                    value_types_match: self.same_value_type(val1, val2),
                    numeric_difference,
                };

                Ok(Some(DiscrepancyReport {
                    discrepancy_type: DiscrepancyType::ReturnValue,
                    severity: Severity::Medium,
                    description: "Return values differ between SDK versions".to_string(),
                    affected_versions: vec![result1.sdk_version.clone(), result2.sdk_version.clone()],
                    test_input: "test_input".to_string(),
                    details: DiscrepancyDetails::ReturnValueDifference(details),
                    recommendation: "Verify return value handling across SDK versions".to_string(),
                    confidence: 0.8,
                }))
            },
            (None, None) => Ok(None),
            (Some(_), None) | (None, Some(_)) => {
                let mut return_values = HashMap::new();
                return_values.insert(result1.sdk_version.version.clone(), result1.return_value.clone());
                return_values.insert(result2.sdk_version.version.clone(), result2.return_value.clone());

                let details = ReturnValueDifferenceDetails {
                    return_values,
                    value_types_match: false,
                    numeric_difference: None,
                };

                Ok(Some(DiscrepancyReport {
                    discrepancy_type: DiscrepancyType::ReturnValue,
                    severity: Severity::High,
                    description: "One version returns a value while another returns None".to_string(),
                    affected_versions: vec![result1.sdk_version.clone(), result2.sdk_version.clone()],
                    test_input: "test_input".to_string(),
                    details: DiscrepancyDetails::ReturnValueDifference(details),
                    recommendation: "Investigate why return value behavior differs".to_string(),
                    confidence: 0.9,
                }))
            },
            _ => Ok(None),
        }
    }

    /// Detect error differences
    fn detect_error_difference(&self, result1: &ExecutionResult, result2: &ExecutionResult) -> Result<Option<DiscrepancyReport>> {
        let success_mismatch = result1.success != result2.success;
        let errors_match = match (&result1.error, &result2.error) {
            (Some(err1), Some(err2)) => err1 == err2,
            (None, None) => true,
            _ => false,
        };

        if success_mismatch || !errors_match {
            let mut errors = HashMap::new();
            errors.insert(result1.sdk_version.version.clone(), result1.error.clone());
            errors.insert(result2.sdk_version.version.clone(), result2.error.clone());

            let details = ErrorDifferenceDetails {
                errors,
                error_types_match: errors_match,
                success_mismatch,
            };

            Ok(Some(DiscrepancyReport {
                discrepancy_type: DiscrepancyType::ErrorDifference,
                severity: if success_mismatch { Severity::High } else { Severity::Medium },
                description: if success_mismatch {
                    "Success status differs between SDK versions".to_string()
                } else {
                    "Error messages differ between SDK versions".to_string()
                },
                affected_versions: vec![result1.sdk_version.clone(), result2.sdk_version.clone()],
                test_input: "test_input".to_string(),
                details: DiscrepancyDetails::ErrorDifference(details),
                recommendation: "Review error handling and success conditions".to_string(),
                confidence: 0.85,
            }))
        } else {
            Ok(None)
        }
    }

    /// Detect execution order differences
    fn detect_execution_order_difference(&self, result1: &ExecutionResult, result2: &ExecutionResult) -> Result<Option<DiscrepancyReport>> {
        let trace1_events: Vec<String> = result1.execution_trace.events.iter()
            .map(|e| format!("{:?}:{:?}", e.event_type, e.function_name))
            .collect();
        
        let trace2_events: Vec<String> = result2.execution_trace.events.iter()
            .map(|e| format!("{:?}:{:?}", e.event_type, e.function_name))
            .collect();

        if trace1_events != trace2_events {
            let mut event_sequences = HashMap::new();
            event_sequences.insert(result1.sdk_version.version.clone(), trace1_events);
            event_sequences.insert(result2.sdk_version.version.clone(), trace2_events);

            let details = ExecutionOrderDifferenceDetails {
                event_sequences,
                order_mismatch: true,
                missing_events: HashMap::new(),
                extra_events: HashMap::new(),
            };

            Ok(Some(DiscrepancyReport {
                discrepancy_type: DiscrepancyType::ExecutionOrder,
                severity: Severity::Medium,
                description: "Execution order differs between SDK versions".to_string(),
                affected_versions: vec![result1.sdk_version.clone(), result2.sdk_version.clone()],
                test_input: "test_input".to_string(),
                details: DiscrepancyDetails::ExecutionOrderDifference(details),
                recommendation: "Review execution order dependencies and timing".to_string(),
                confidence: 0.7,
            }))
        } else {
            Ok(None)
        }
    }

    /// Detect memory usage differences
    fn detect_memory_usage_difference(&self, result1: &ExecutionResult, result2: &ExecutionResult) -> Result<Option<DiscrepancyReport>> {
        let mem1 = result1.execution_trace.memory_usage.peak_memory;
        let mem2 = result2.execution_trace.memory_usage.peak_memory;

        if mem1 == 0 && mem2 == 0 {
            return Ok(None);
        }

        let avg_mem = (mem1 + mem2) as f64 / 2.0;
        let absolute_diff = mem1.abs_diff(mem2) as f64;
        let percentage_diff = (absolute_diff / avg_mem) * 100.0;

        if percentage_diff > 20.0 { // 20% threshold for memory differences
            let mut memory_usage = HashMap::new();
            memory_usage.insert(result1.sdk_version.version.clone(), mem1);
            memory_usage.insert(result2.sdk_version.version.clone(), mem2);

            let mut peak_memory = HashMap::new();
            peak_memory.insert(result1.sdk_version.version.clone(), mem1);
            peak_memory.insert(result2.sdk_version.version.clone(), mem2);

            let mut allocation_count = HashMap::new();
            allocation_count.insert(result1.sdk_version.version.clone(), result1.execution_trace.memory_usage.allocations.len());
            allocation_count.insert(result2.sdk_version.version.clone(), result2.execution_trace.memory_usage.allocations.len());

            let details = MemoryUsageDifferenceDetails {
                memory_usage,
                peak_memory,
                allocation_count,
                percentage_difference: percentage_diff,
            };

            Ok(Some(DiscrepancyReport {
                discrepancy_type: DiscrepancyType::MemoryUsage,
                severity: Severity::Low,
                description: format!("Memory usage differs by {:.2}%", percentage_diff),
                affected_versions: vec![result1.sdk_version.clone(), result2.sdk_version.clone()],
                test_input: "test_input".to_string(),
                details: DiscrepancyDetails::MemoryUsageDifference(details),
                recommendation: "Review memory management patterns".to_string(),
                confidence: 0.6,
            }))
        } else {
            Ok(None)
        }
    }

    /// Detect external call differences
    fn detect_external_call_difference(&self, result1: &ExecutionResult, result2: &ExecutionResult) -> Result<Option<DiscrepancyReport>> {
        let external_calls1 = self.extract_external_calls(&result1.execution_trace);
        let external_calls2 = self.extract_external_calls(&result2.execution_trace);

        if external_calls1 != external_calls2 {
            let mut external_calls = HashMap::new();
            external_calls.insert(result1.sdk_version.version.clone(), external_calls1.clone());
            external_calls.insert(result2.sdk_version.version.clone(), external_calls2.clone());

            let details = ExternalCallDifferenceDetails {
                external_calls,
                call_count_mismatch: external_calls1.len() != external_calls2.len(),
                call_parameter_differences: Vec::new(),
            };

            Ok(Some(DiscrepancyReport {
                discrepancy_type: DiscrepancyType::ExternalCallBehavior,
                severity: Severity::High,
                description: "External call behavior differs between SDK versions".to_string(),
                affected_versions: vec![result1.sdk_version.clone(), result2.sdk_version.clone()],
                test_input: "test_input".to_string(),
                details: DiscrepancyDetails::ExternalCallDifference(details),
                recommendation: "Review external contract call handling".to_string(),
                confidence: 0.9,
            }))
        } else {
            Ok(None)
        }
    }

    /// Extract external calls from execution trace
    fn extract_external_calls(&self, trace: &crate::differential_fuzzing::ExecutionTrace) -> Vec<ExternalCallInfo> {
        trace.events.iter()
            .filter(|e| e.event_type == crate::differential_fuzzing::TraceEventType::ExternalCall)
            .map(|e| ExternalCallInfo {
                contract_address: "unknown".to_string(), // Would be extracted from trace
                function_name: e.function_name.clone(),
                parameters: Vec::new(),
                success: true,
                return_value: None,
            })
            .collect()
    }

    /// Helper methods for severity calculation
    fn calculate_gas_severity(&self, percentage_diff: f64) -> Severity {
        if percentage_diff > 50.0 {
            Severity::Critical
        } else if percentage_diff > 25.0 {
            Severity::High
        } else if percentage_diff > 15.0 {
            Severity::Medium
        } else {
            Severity::Low
        }
    }

    fn calculate_logic_severity(&self, similarity: f64) -> Severity {
        if similarity < 0.5 {
            Severity::Critical
        } else if similarity < 0.7 {
            Severity::High
        } else if similarity < 0.9 {
            Severity::Medium
        } else {
            Severity::Low
        }
    }

    fn calculate_confidence(&self, actual_diff: f64, threshold: f64) -> f64 {
        (actual_diff / threshold).min(1.0)
    }

    fn calculate_numeric_difference(&self, val1: &ArgumentValue, val2: &ArgumentValue) -> Option<i128> {
        match (val1, val2) {
            (ArgumentValue::I128(n1), ArgumentValue::I128(n2)) => Some(n1.abs_diff(*n2) as i128),
            (ArgumentValue::U64(n1), ArgumentValue::U64(n2)) => Some(n1.abs_diff(*n2) as i128),
            _ => None,
        }
    }

    fn same_value_type(&self, val1: &ArgumentValue, val2: &ArgumentValue) -> bool {
        std::mem::discriminant(val1) == std::mem::discriminant(val2)
    }

    /// Remove duplicate discrepancies and merge similar ones
    fn deduplicate_and_merge(&self, discrepancies: &mut Vec<DiscrepancyReport>) {
        // Sort by type and affected versions
        discrepancies.sort_by(|a, b| {
            a.discrepancy_type.cmp(&b.discrepancy_type)
                .then_with(|| a.affected_versions.cmp(&b.affected_versions))
        });

        // Merge similar discrepancies (simplified approach)
        let mut i = 0;
        while i < discrepancies.len() {
            let mut j = i + 1;
            while j < discrepancies.len() {
                if self.should_merge(&discrepancies[i], &discrepancies[j]) {
                    // Merge discrepancies[j] into discrepancies[i] and remove j
                    self.merge_discrepancies(&mut discrepancies[i], &discrepancies[j]);
                    discrepancies.remove(j);
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
    }

    fn should_merge(&self, disc1: &DiscrepancyReport, disc2: &DiscrepancyReport) -> bool {
        disc1.discrepancy_type == disc2.discrepancy_type &&
        disc1.affected_versions == disc2.affected_versions
    }

    fn merge_discrepancies(&self, target: &mut DiscrepancyReport, source: &DiscrepancyReport) {
        target.description = format!("{}; {}", target.description, source.description);
        target.confidence = target.confidence.max(source.confidence);
        if source.severity > target.severity {
            target.severity = source.severity.clone();
        }
    }
}
