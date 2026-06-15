//! Gas Limit Considerations for Complex Operations
//! 
//! This module provides gas limit estimation, validation, and optimization
//! for complex operations like escrow release and emergency reward distribution.

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use log::{info, warn, error};

/// Gas limit configuration for different operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasLimitConfig {
    /// Maximum gas limit for simple operations
    pub simple_operation_limit: u64,
    /// Maximum gas limit for complex operations
    pub complex_operation_limit: u64,
    /// Maximum gas limit for batch operations
    pub batch_operation_limit: u64,
    /// Safety margin percentage (e.g., 10% means use 90% of limit)
    pub safety_margin_percentage: f64,
    /// Enable gas estimation before execution
    pub enable_estimation: bool,
    /// Enable gas optimization suggestions
    pub enable_optimization: bool,
}

impl Default for GasLimitConfig {
    fn default() -> Self {
        Self {
            simple_operation_limit: 5_000_000,      // 5M gas for simple ops
            complex_operation_limit: 25_000_000,     // 25M gas for complex ops
            batch_operation_limit: 100_000_000,      // 100M gas for batch ops
            safety_margin_percentage: 10.0,          // 10% safety margin
            enable_estimation: true,
            enable_optimization: true,
        }
    }
}

/// Operation complexity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationComplexity {
    Simple,      // Basic operations like transfers, approvals
    Complex,     // Escrow release, reward distribution
    Batch,       // Multiple operations in single transaction
}

/// Gas estimation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasEstimation {
    /// Estimated gas consumption
    pub estimated_gas: u64,
    /// Operation complexity
    pub complexity: OperationComplexity,
    /// Recommended gas limit
    pub recommended_limit: u64,
    /// Safety margin applied
    pub safety_margin: u64,
    /// Optimization suggestions
    pub optimizations: Vec<GasOptimization>,
    /// Risk assessment
    pub risk_level: GasRiskLevel,
}

/// Gas optimization suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasOptimization {
    pub optimization_type: OptimizationType,
    pub description: String,
    pub potential_savings: u64,
    pub implementation_difficulty: ImplementationDifficulty,
}

/// Types of gas optimizations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationType {
    LoopOptimization,
    StorageOptimization,
    EventOptimization,
    BatchOperations,
    ConditionalExecution,
    EarlyExit,
}

/// Implementation difficulty
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImplementationDifficulty {
    Easy,
    Medium,
    Hard,
}

/// Gas risk levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GasRiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Gas limit validator and estimator
pub struct GasLimitManager {
    config: GasLimitConfig,
    operation_profiles: HashMap<String, GasProfile>,
}

/// Profile for specific operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasProfile {
    pub operation_name: String,
    pub base_gas_cost: u64,
    pub variable_gas_factors: Vec<GasFactor>,
    pub complexity: OperationComplexity,
    pub optimization_hints: Vec<OptimizationHint>,
}

/// Factors affecting gas consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasFactor {
    pub factor_name: String,
    pub gas_per_unit: u64,
    pub max_units: Option<u64>,
}

/// Optimization hints for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationHint {
    pub hint_type: OptimizationType,
    pub description: String,
    pub applicable_conditions: Vec<String>,
}

impl GasLimitManager {
    /// Create a new gas limit manager
    pub fn new(config: GasLimitConfig) -> Self {
        let mut manager = Self {
            config,
            operation_profiles: HashMap::new(),
        };
        
        manager.initialize_default_profiles();
        manager
    }

    /// Initialize default operation profiles
    fn initialize_default_profiles(&mut self) {
        // Escrow release operation profile
        self.operation_profiles.insert("escrow_release".to_string(), GasProfile {
            operation_name: "escrow_release".to_string(),
            base_gas_cost: 50_000,
            variable_gas_factors: vec![
                GasFactor {
                    factor_name: "recipients".to_string(),
                    gas_per_unit: 10_000,
                    max_units: Some(100),
                },
                GasFactor {
                    factor_name: "amount_transfers".to_string(),
                    gas_per_unit: 5_000,
                    max_units: None,
                },
            ],
            complexity: OperationComplexity::Complex,
            optimization_hints: vec![
                OptimizationHint {
                    hint_type: OptimizationType::BatchOperations,
                    description: "Batch multiple transfers to reduce overhead".to_string(),
                    applicable_conditions: vec!["multiple_recipients".to_string()],
                },
            ],
        });

        // Reward distribution operation profile
        self.operation_profiles.insert("reward_distribution".to_string(), GasProfile {
            operation_name: "reward_distribution".to_string(),
            base_gas_cost: 75_000,
            variable_gas_factors: vec![
                GasFactor {
                    factor_name: "researchers".to_string(),
                    gas_per_unit: 15_000,
                    max_units: Some(500),
                },
                GasFactor {
                    factor_name: "reward_calculations".to_string(),
                    gas_per_unit: 2_000,
                    max_units: None,
                },
                GasFactor {
                    factor_name: "severity_checks".to_string(),
                    gas_per_unit: 1_000,
                    max_units: None,
                },
            ],
            complexity: OperationComplexity::Complex,
            optimization_hints: vec![
                OptimizationHint {
                    hint_type: OptimizationType::EarlyExit,
                    description: "Skip zero-amount rewards early".to_string(),
                    applicable_conditions: vec!["variable_amounts".to_string()],
                },
                OptimizationHint {
                    hint_type: OptimizationType::StorageOptimization,
                    description: "Cache severity reward percentages".to_string(),
                    applicable_conditions: vec!["repeated_calculations".to_string()],
                },
            ],
        });

        // Emergency reward distribution profile
        self.operation_profiles.insert("emergency_reward_distribution".to_string(), GasProfile {
            operation_name: "emergency_reward_distribution".to_string(),
            base_gas_cost: 100_000,
            variable_gas_factors: vec![
                GasFactor {
                    factor_name: "emergency_claims".to_string(),
                    gas_per_unit: 20_000,
                    max_units: Some(1000),
                },
                GasFactor {
                    factor_name: "priority_calculations".to_string(),
                    gas_per_unit: 5_000,
                    max_units: None,
                },
                GasFactor {
                    factor_name: "bypass_checks".to_string(),
                    gas_per_unit: 3_000,
                    max_units: None,
                },
            ],
            complexity: OperationComplexity::Complex,
            optimization_hints: vec![
                OptimizationHint {
                    hint_type: OptimizationType::ConditionalExecution,
                    description: "Use priority-based processing".to_string(),
                    applicable_conditions: vec!["large_claim_sets".to_string()],
                },
            ],
        });
    }

    /// Estimate gas consumption for an operation
    pub fn estimate_gas(&self, operation: &str, parameters: &HashMap<String, u64>) -> Result<GasEstimation> {
        let profile = self.operation_profiles.get(operation)
            .ok_or_else(|| anyhow::anyhow!("Unknown operation: {}", operation))?;

        let mut estimated_gas = profile.base_gas_cost;

        // Calculate variable gas costs
        for factor in &profile.variable_gas_factors {
            if let Some(&value) = parameters.get(&factor.factor_name) {
                let units = if let Some(max_units) = factor.max_units {
                    value.min(max_units)
                } else {
                    value
                };
                estimated_gas += units * factor.gas_per_unit;
            }
        }

        // Determine limit based on complexity
        let base_limit = match profile.complexity {
            OperationComplexity::Simple => self.config.simple_operation_limit,
            OperationComplexity::Complex => self.config.complex_operation_limit,
            OperationComplexity::Batch => self.config.batch_operation_limit,
        };

        // Apply safety margin
        let safety_margin = (estimated_gas as f64 * self.config.safety_margin_percentage / 100.0) as u64;
        let recommended_limit = estimated_gas + safety_margin;

        // Check if recommended limit exceeds base limit
        let final_limit = if recommended_limit > base_limit {
            warn!("Recommended gas limit {} exceeds base limit {} for operation {}", 
                  recommended_limit, base_limit, operation);
            base_limit
        } else {
            recommended_limit
        };

        // Generate optimization suggestions
        let optimizations = if self.config.enable_optimization {
            self.generate_optimizations(profile, parameters)?
        } else {
            Vec::new()
        };

        // Assess risk level
        let risk_level = self.assess_gas_risk(estimated_gas, final_limit);

        Ok(GasEstimation {
            estimated_gas,
            complexity: profile.complexity,
            recommended_limit: final_limit,
            safety_margin,
            optimizations,
            risk_level,
        })
    }

    /// Validate gas limit for an operation
    pub fn validate_gas_limit(&self, operation: &str, parameters: &HashMap<String, u64>, provided_limit: u64) -> Result<GasValidationResult> {
        let estimation = self.estimate_gas(operation, parameters)?;

        let is_valid = provided_limit >= estimation.estimated_gas;
        let is_optimal = provided_limit <= estimation.recommended_limit;
        let risk_level = if provided_limit < estimation.estimated_gas {
            GasRiskLevel::Critical
        } else if provided_limit > estimation.recommended_limit * 2 {
            GasRiskLevel::High
        } else if !is_optimal {
            GasRiskLevel::Medium
        } else {
            estimation.risk_level.clone()
        };

        Ok(GasValidationResult {
            is_valid,
            is_optimal,
            provided_limit,
            estimation,
            risk_level,
            recommendations: self.generate_recommendations(&estimation, provided_limit),
        })
    }

    /// Generate optimization suggestions
    fn generate_optimizations(&self, profile: &GasProfile, parameters: &HashMap<String, u64>) -> Result<Vec<GasOptimization>> {
        let mut optimizations = Vec::new();

        for hint in &profile.optimization_hints {
            if self.is_optimization_applicable(hint, parameters) {
                let potential_savings = self.estimate_optimization_savings(hint, parameters);
                
                optimizations.push(GasOptimization {
                    optimization_type: hint.hint_type.clone(),
                    description: hint.description.clone(),
                    potential_savings,
                    implementation_difficulty: self.assess_implementation_difficulty(&hint.hint_type),
                });
            }
        }

        Ok(optimizations)
    }

    /// Check if optimization is applicable
    fn is_optimization_applicable(&self, hint: &OptimizationHint, parameters: &HashMap<String, u64>) -> bool {
        hint.applicable_conditions.iter().any(|condition| {
            parameters.contains_key(condition)
        })
    }

    /// Estimate potential savings from optimization
    fn estimate_optimization_savings(&self, hint: &OptimizationHint, parameters: &HashMap<String, u64>) -> u64 {
        match hint.hint_type {
            OptimizationType::BatchOperations => {
                // Estimate savings from batching operations
                if let Some(&recipients) = parameters.get("recipients") {
                    (recipients - 1) * 5_000 // Assume 5k gas savings per batched operation
                } else {
                    0
                }
            }
            OptimizationType::EarlyExit => {
                // Estimate savings from early exits
                if let Some(&calculations) = parameters.get("reward_calculations") {
                    calculations * 1_000 // Assume 1k gas savings per early exit
                } else {
                    0
                }
            }
            OptimizationType::StorageOptimization => {
                // Estimate savings from storage optimization
                parameters.values().sum::<u64>() / 10 // 10% savings estimate
            }
            _ => 5_000, // Default savings estimate
        }
    }

    /// Assess implementation difficulty
    fn assess_implementation_difficulty(&self, optimization_type: &OptimizationType) -> ImplementationDifficulty {
        match optimization_type {
            OptimizationType::EarlyExit => ImplementationDifficulty::Easy,
            OptimizationType::BatchOperations => ImplementationDifficulty::Medium,
            OptimizationType::StorageOptimization => ImplementationDifficulty::Medium,
            OptimizationType::LoopOptimization => ImplementationDifficulty::Hard,
            OptimizationType::ConditionalExecution => ImplementationDifficulty::Medium,
            OptimizationType::EventOptimization => ImplementationDifficulty::Easy,
        }
    }

    /// Assess gas risk level
    fn assess_gas_risk(&self, estimated_gas: u64, limit: u64) -> GasRiskLevel {
        let usage_percentage = (estimated_gas as f64 / limit as f64) * 100.0;

        match usage_percentage {
            x if x >= 95.0 => GasRiskLevel::Critical,
            x if x >= 80.0 => GasRiskLevel::High,
            x if x >= 60.0 => GasRiskLevel::Medium,
            _ => GasRiskLevel::Low,
        }
    }

    /// Generate recommendations for gas usage
    fn generate_recommendations(&self, estimation: &GasEstimation, provided_limit: u64) -> Vec<String> {
        let mut recommendations = Vec::new();

        if provided_limit < estimation.estimated_gas {
            recommendations.push(format!(
                "Increase gas limit to at least {} (current: {})",
                estimation.recommended_limit, provided_limit
            ));
        }

        if provided_limit > estimation.recommended_limit * 2 {
            recommendations.push(format!(
                "Consider reducing gas limit to {} (current: {})",
                estimation.recommended_limit, provided_limit
            ));
        }

        if !estimation.optimizations.is_empty() {
            recommendations.push("Consider implementing gas optimization suggestions".to_string());
        }

        match estimation.risk_level {
            GasRiskLevel::Critical => {
                recommendations.push("CRITICAL: Gas limit is insufficient for this operation".to_string());
            }
            GasRiskLevel::High => {
                recommendations.push("WARNING: Gas limit is close to estimated consumption".to_string());
            }
            _ => {}
        }

        recommendations
    }

    /// Add custom operation profile
    pub fn add_operation_profile(&mut self, profile: GasProfile) {
        self.operation_profiles.insert(profile.operation_name.clone(), profile);
    }

    /// Update configuration
    pub fn update_config(&mut self, config: GasLimitConfig) {
        self.config = config;
    }
}

/// Gas validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasValidationResult {
    pub is_valid: bool,
    pub is_optimal: bool,
    pub provided_limit: u64,
    pub estimation: GasEstimation,
    pub risk_level: GasRiskLevel,
    pub recommendations: Vec<String>,
}

/// Batch operation types for gas estimation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatchOperationType {
    EscrowRelease,
    VulnerabilityVerification,
}

impl BatchOperationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BatchOperationType::EscrowRelease => "escrow_release",
            BatchOperationType::VulnerabilityVerification => "vulnerability_verification",
        }
    }
}

/// Per-item gas estimate breakdown for batch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemGasEstimate {
    /// Item index in the batch
    pub item_index: u64,
    /// Operation type for this item
    pub operation_type: BatchOperationType,
    /// Estimated gas for this single item
    pub estimated_gas: u64,
    /// Base gas cost for this operation type
    pub base_cost: u64,
    /// Variable gas cost based on item complexity
    pub variable_cost: u64,
}

/// Batch gas estimate with breakdown and recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchGasEstimate {
    /// Total items in the batch
    pub total_items: u64,
    /// Breakdown of items by operation type
    pub escrow_releases: u64,
    pub verifications: u64,
    /// Per-item gas estimates
    pub item_estimates: Vec<ItemGasEstimate>,
    /// Total estimated gas (sum of all items + overhead)
    pub total_estimated_gas: u64,
    /// Fixed overhead for batch processing
    pub batch_overhead: u64,
    /// Per-item overhead
    pub per_item_overhead: u64,
    /// Recommended gas limit (estimated + safety margin)
    pub recommended_limit: u64,
    /// Safety margin applied
    pub safety_margin: u64,
    /// Stellar maximum transaction gas limit
    pub stellar_max_limit: u64,
    /// Whether estimated gas exceeds 90% of Stellar limit
    pub exceeds_recommended_threshold: bool,
    /// Warning message if threshold is exceeded
    pub threshold_warning: Option<String>,
    /// Risk level for this batch
    pub risk_level: GasRiskLevel,
    /// Suggested number of splits if threshold is exceeded
    pub suggested_splits: u64,
    /// Estimated gas per split
    pub estimated_gas_per_split: u64,
}

/// Batch gas estimator for pre-execution gas estimation
pub struct BatchGasEstimator {
    gas_limit_manager: GasLimitManager,
    stellar_transaction_max_gas: u64,
    threshold_percentage: f64,
}

impl BatchGasEstimator {
    /// Create a new batch gas estimator
    pub fn new(gas_limit_config: GasLimitConfig) -> Self {
        let gas_limit_manager = GasLimitManager::new(gas_limit_config);
        Self {
            gas_limit_manager,
            stellar_transaction_max_gas: 100_000_000, // ~100M max gas per Stellar transaction
            threshold_percentage: 90.0, // Warn at 90% of Stellar limit
        }
    }

    /// Create with custom Stellar max gas and threshold
    pub fn with_limits(
        gas_limit_config: GasLimitConfig,
        stellar_max_gas: u64,
        threshold_pct: f64,
    ) -> Self {
        let gas_limit_manager = GasLimitManager::new(gas_limit_config);
        Self {
            gas_limit_manager,
            stellar_transaction_max_gas: stellar_max_gas,
            threshold_percentage: threshold_pct,
        }
    }

    /// Estimate gas for a batch of escrow releases
    pub fn estimate_escrow_release_batch(&self, item_count: u64) -> Result<BatchGasEstimate> {
        let operation_type = BatchOperationType::EscrowRelease;
        let mut item_estimates = Vec::with_capacity(item_count as usize);

        for i in 0..item_count {
            // Each escrow release has 1 recipient (the beneficiary) and 1 amount transfer
            let mut params = std::collections::HashMap::new();
            params.insert("recipients".to_string(), 1);
            params.insert("amount_transfers".to_string(), 1);

            let estimation = self.gas_limit_manager
                .estimate_gas(operation_type.as_str(), &params)?;

            item_estimates.push(ItemGasEstimate {
                item_index: i,
                operation_type: operation_type.clone(),
                estimated_gas: estimation.estimated_gas,
                base_cost: 50_000,
                variable_cost: estimation.estimated_gas - 50_000.min(estimation.estimated_gas),
            });
        }

        self.build_batch_estimate(item_count, item_estimates, 0)
    }

    /// Estimate gas for a batch of vulnerability verifications
    pub fn estimate_verification_batch(&self, item_count: u64) -> Result<BatchGasEstimate> {
        let operation_type = BatchOperationType::VulnerabilityVerification;
        let mut item_estimates = Vec::with_capacity(item_count as usize);

        for i in 0..item_count {
            // Each verification checks vulnerability data and updates status
            let mut params = std::collections::HashMap::new();
            params.insert("recipients".to_string(), 1);
            params.insert("amount_transfers".to_string(), 1);
            params.insert("reward_calculations".to_string(), 1);
            params.insert("severity_checks".to_string(), 1);

            let estimation = self.gas_limit_manager
                .estimate_gas("reward_distribution", &params)?;

            item_estimates.push(ItemGasEstimate {
                item_index: i,
                operation_type: operation_type.clone(),
                estimated_gas: estimation.estimated_gas,
                base_cost: 75_000,
                variable_cost: estimation.estimated_gas - 75_000.min(estimation.estimated_gas),
            });
        }

        self.build_batch_estimate(0, item_estimates, item_count)
    }

    /// Estimate gas for a mixed batch (escrow releases + verifications)
    pub fn estimate_mixed_batch(
        &self,
        escrow_count: u64,
        verification_count: u64,
    ) -> Result<BatchGasEstimate> {
        let mut item_estimates = Vec::with_capacity((escrow_count + verification_count) as usize);

        // Estimate escrow items first
        for i in 0..escrow_count {
            let mut params = std::collections::HashMap::new();
            params.insert("recipients".to_string(), 1);
            params.insert("amount_transfers".to_string(), 1);

            let estimation = self.gas_limit_manager
                .estimate_gas("escrow_release", &params)?;

            item_estimates.push(ItemGasEstimate {
                item_index: i,
                operation_type: BatchOperationType::EscrowRelease,
                estimated_gas: estimation.estimated_gas,
                base_cost: 50_000,
                variable_cost: estimation.estimated_gas - 50_000.min(estimation.estimated_gas),
            });
        }

        // Estimate verification items
        for i in 0..verification_count {
            let mut params = std::collections::HashMap::new();
            params.insert("recipients".to_string(), 1);
            params.insert("amount_transfers".to_string(), 1);
            params.insert("reward_calculations".to_string(), 1);
            params.insert("severity_checks".to_string(), 1);

            let estimation = self.gas_limit_manager
                .estimate_gas("reward_distribution", &params)?;

            item_estimates.push(ItemGasEstimate {
                item_index: escrow_count + i,
                operation_type: BatchOperationType::VulnerabilityVerification,
                estimated_gas: estimation.estimated_gas,
                base_cost: 75_000,
                variable_cost: estimation.estimated_gas - 75_000.min(estimation.estimated_gas),
            });
        }

        self.build_batch_estimate(
            escrow_count,
            item_estimates,
            verification_count,
        )
    }

    /// Build the final batch estimate from item estimates
    fn build_batch_estimate(
        &self,
        escrow_count: u64,
        item_estimates: Vec<ItemGasEstimate>,
        verification_count: u64,
    ) -> Result<BatchGasEstimate> {
        let total_items = escrow_count + verification_count;

        // Calculate overhead: fixed batch overhead + per-item overhead
        let batch_overhead = 10_000u64; // Fixed overhead for dispatching (10K gas)
        let per_item_overhead = 2_000u64; // Per-item processing overhead (2K gas)

        // Sum item gas estimates
        let items_total: u64 = item_estimates.iter().map(|e| e.estimated_gas).sum();
        let total_overhead = batch_overhead + (per_item_overhead * total_items);
        let total_estimated_gas = items_total + total_overhead;

        // Apply safety margin
        let safety_margin = (total_estimated_gas as f64 * 0.10) as u64; // 10% safety
        let recommended_limit = total_estimated_gas + safety_margin;

        // Check if exceeds threshold
        let threshold_gas = (self.stellar_transaction_max_gas as f64 * self.threshold_percentage / 100.0) as u64;
        let exceeds_threshold = recommended_limit > threshold_gas;

        let threshold_warning = if exceeds_threshold {
            Some(format!(
                "Estimated gas ({}) exceeds {:.0}% of Stellar transaction limit ({}). Consider splitting the batch.",
                total_estimated_gas,
                self.threshold_percentage,
                self.stellar_transaction_max_gas
            ))
        } else {
            None
        };

        // Calculate suggested splits
        let suggested_splits = if exceeds_threshold {
            let gas_per_item = total_estimated_gas / total_items.max(1);
            let max_items_per_split = threshold_gas / gas_per_item.max(1);
            let splits = (total_items + max_items_per_split - 1) / max_items_per_split.max(1);
            splits.max(1)
        } else {
            1
        };

        let estimated_gas_per_split = if exceeds_threshold {
            total_estimated_gas / suggested_splits
        } else {
            total_estimated_gas
        };

        // Assess risk level
        let risk_level = self.assess_batch_risk(total_estimated_gas, recommended_limit);

        Ok(BatchGasEstimate {
            total_items,
            escrow_releases: escrow_count,
            verifications: verification_count,
            item_estimates,
            total_estimated_gas,
            batch_overhead,
            per_item_overhead,
            recommended_limit,
            safety_margin,
            stellar_max_limit: self.stellar_transaction_max_gas,
            exceeds_recommended_threshold: exceeds_threshold,
            threshold_warning,
            risk_level,
            suggested_splits,
            estimated_gas_per_split,
        })
    }

    /// Assess gas risk for the batch
    fn assess_batch_risk(&self, estimated_gas: u64, limit: u64) -> GasRiskLevel {
        let usage_pct = (estimated_gas as f64 / limit as f64) * 100.0;
        match usage_pct {
            x if x >= 95.0 => GasRiskLevel::Critical,
            x if x >= 80.0 => GasRiskLevel::High,
            x if x >= 60.0 => GasRiskLevel::Medium,
            _ => GasRiskLevel::Low,
        }
    }

    /// Format the batch estimate as a human-readable string for CLI output
    pub fn format_estimate(estimate: &BatchGasEstimate) -> String {
        let mut output = String::new();
        output.push_str(&format!("📊 Batch Gas Estimate\n"));
        output.push_str(&format!("═══════════════════════════════════\n"));
        output.push_str(&format!("  Total items:     {}\n", estimate.total_items));
        output.push_str(&format!("  Escrow releases: {}\n", estimate.escrow_releases));
        output.push_str(&format!("  Verifications:   {}\n", estimate.verifications));
        output.push_str(&format!("\n"));
        output.push_str(&format!("  Item gas total:     {}\n", estimate.total_estimated_gas - estimate.batch_overhead - (estimate.per_item_overhead * estimate.total_items)));
        output.push_str(&format!("  Batch overhead:     {} (fixed: {}, per-item: {} x {})\n",
            estimate.batch_overhead + (estimate.per_item_overhead * estimate.total_items),
            estimate.batch_overhead,
            estimate.per_item_overhead,
            estimate.total_items
        ));
        output.push_str(&format!("  Total estimated:    {}\n", estimate.total_estimated_gas));
        output.push_str(&format!("  Safety margin:      {}\n", estimate.safety_margin));
        output.push_str(&format!("  Recommended limit:  {}\n", estimate.recommended_limit));
        output.push_str(&format!("  Stellar max limit:  {}\n", estimate.stellar_max_limit));
        output.push_str(&format!("\n"));
        output.push_str(&format!("  Risk level: {}\n", match estimate.risk_level {
            GasRiskLevel::Critical => "🔴 CRITICAL".to_string(),
            GasRiskLevel::High => "🟠 HIGH".to_string(),
            GasRiskLevel::Medium => "🟡 MEDIUM".to_string(),
            GasRiskLevel::Low => "🟢 LOW".to_string(),
        }));

        if estimate.exceeds_recommended_threshold {
            output.push_str(&format!("  ⚠️  WARNING: Estimated gas exceeds 90% of Stellar limit!\n"));
            output.push_str(&format!("  💡  Suggested splits: {} ({} items, ~{} gas each)\n",
                estimate.suggested_splits,
                if estimate.total_items > 0 { estimate.total_items / estimate.suggested_splits } else { 0 },
                estimate.estimated_gas_per_split
            ));
        }

        if let Some(ref warning) = estimate.threshold_warning {
            output.push_str(&format!("  ⚠️  {}\n", warning));
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_estimation() {
        let config = GasLimitConfig::default();
        let manager = GasLimitManager::new(config);

        let mut parameters = HashMap::new();
        parameters.insert("recipients".to_string(), 10);
        parameters.insert("amount_transfers".to_string(), 5);

        let estimation = manager.estimate_gas("escrow_release", &parameters).unwrap();
        
        assert!(estimation.estimated_gas > 50_000); // Base cost
        assert_eq!(estimation.complexity, OperationComplexity::Complex);
        assert!(estimation.recommended_limit > estimation.estimated_gas);
    }

    #[test]
    fn test_gas_validation() {
        let config = GasLimitConfig::default();
        let manager = GasLimitManager::new(config);

        let mut parameters = HashMap::new();
        parameters.insert("researchers".to_string(), 5);

        let result = manager.validate_gas_limit("reward_distribution", &parameters, 1_000_000).unwrap();
        
        assert!(result.is_valid);
        assert!(result.recommendations.is_empty());
    }

    #[test]
    fn test_insufficient_gas_validation() {
        let config = GasLimitConfig::default();
        let manager = GasLimitManager::new(config);

        let parameters = HashMap::new();

        let result = manager.validate_gas_limit("escrow_release", &parameters, 10_000).unwrap();
        
        assert!(!result.is_valid);
        assert_eq!(result.risk_level, GasRiskLevel::Critical);
        assert!(!result.recommendations.is_empty());
    }
}
