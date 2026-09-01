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
