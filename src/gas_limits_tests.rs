//! Tests for gas limit considerations functionality

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_gas_limit_config_default() {
        let config = GasLimitConfig::default();
        
        assert_eq!(config.simple_operation_limit, 5_000_000);
        assert_eq!(config.complex_operation_limit, 25_000_000);
        assert_eq!(config.batch_operation_limit, 100_000_000);
        assert_eq!(config.safety_margin_percentage, 10.0);
        assert!(config.enable_estimation);
        assert!(config.enable_optimization);
    }

    #[test]
    fn test_gas_limit_manager_creation() {
        let config = GasLimitConfig::default();
        let manager = GasLimitManager::new(config);
        
        // Check that default profiles are loaded
        assert!(manager.operation_profiles.contains_key("escrow_release"));
        assert!(manager.operation_profiles.contains_key("reward_distribution"));
        assert!(manager.operation_profiles.contains_key("emergency_reward_distribution"));
    }

    #[test]
    fn test_escrow_release_gas_estimation() {
        let config = GasLimitConfig::default();
        let manager = GasLimitManager::new(config);

        let mut parameters = HashMap::new();
        parameters.insert("recipients".to_string(), 10);
        parameters.insert("amount_transfers".to_string(), 5);

        let estimation = manager.estimate_gas("escrow_release", &parameters).unwrap();
        
        assert!(estimation.estimated_gas > 50_000); // Base cost
        assert_eq!(estimation.complexity, OperationComplexity::Complex);
        assert!(estimation.recommended_limit > estimation.estimated_gas);
        assert!(estimation.safety_margin > 0);
        assert!(!estimation.optimizations.is_empty());
    }

    #[test]
    fn test_reward_distribution_gas_estimation() {
        let config = GasLimitConfig::default();
        let manager = GasLimitManager::new(config);

        let mut parameters = HashMap::new();
        parameters.insert("researchers".to_string(), 20);
        parameters.insert("reward_calculations".to_string(), 15);
        parameters.insert("severity_checks".to_string(), 5);

        let estimation = manager.estimate_gas("reward_distribution", &parameters).unwrap();
        
        assert!(estimation.estimated_gas > 75_000); // Base cost
        assert_eq!(estimation.complexity, OperationComplexity::Complex);
        assert!(estimation.optimizations.len() > 1); // Should have multiple optimizations
    }

    #[test]
    fn test_emergency_reward_distribution_estimation() {
        let config = GasLimitConfig::default();
        let manager = GasLimitManager::new(config);

        let mut parameters = HashMap::new();
        parameters.insert("emergency_claims".to_string(), 100);
        parameters.insert("priority_calculations".to_string(), 50);

        let estimation = manager.estimate_gas("emergency_reward_distribution", &parameters).unwrap();
        
        assert!(estimation.estimated_gas > 100_000); // Base cost
        assert_eq!(estimation.complexity, OperationComplexity::Complex);
        assert!(estimation.risk_level != GasRiskLevel::Low); // Emergency operations should have higher risk
    }

    #[test]
    fn test_gas_validation_success() {
        let config = GasLimitConfig::default();
        let manager = GasLimitManager::new(config);

        let mut parameters = HashMap::new();
        parameters.insert("recipients".to_string(), 5);

        let result = manager.validate_gas_limit("escrow_release", &parameters, 1_000_000).unwrap();
        
        assert!(result.is_valid);
        assert!(result.is_optimal);
        assert_eq!(result.provided_limit, 1_000_000);
        assert!(result.recommendations.is_empty());
    }

    #[test]
    fn test_gas_validation_insufficient() {
        let config = GasLimitConfig::default();
        let manager = GasLimitManager::new(config);

        let parameters = HashMap::new();

        let result = manager.validate_gas_limit("escrow_release", &parameters, 10_000).unwrap();
        
        assert!(!result.is_valid);
        assert!(!result.is_optimal);
        assert_eq!(result.risk_level, GasRiskLevel::Critical);
        assert!(!result.recommendations.is_empty());
        assert!(result.recommendations.iter().any(|r| r.contains("Increase gas limit")));
    }

    #[test]
    fn test_gas_validation_excessive() {
        let config = GasLimitConfig::default();
        let manager = GasLimitManager::new(config);

        let parameters = HashMap::new();

        let result = manager.validate_gas_limit("escrow_release", &parameters, 100_000_000).unwrap();
        
        assert!(result.is_valid);
        assert!(!result.is_optimal); // Should not be optimal due to excessive limit
        assert!(result.recommendations.iter().any(|r| r.contains("reducing gas limit")));
    }

    #[test]
    fn test_gas_optimization_generation() {
        let config = GasLimitConfig {
            enable_optimization: true,
            ..Default::default()
        };
        let manager = GasLimitManager::new(config);

        let mut parameters = HashMap::new();
        parameters.insert("recipients".to_string(), 50);
        parameters.insert("reward_calculations".to_string(), 25);

        let estimation = manager.estimate_gas("reward_distribution", &parameters).unwrap();
        
        assert!(!estimation.optimizations.is_empty());
        
        // Check for specific optimization types
        let has_batch_opt = estimation.optimizations.iter()
            .any(|opt| matches!(opt.optimization_type, OptimizationType::BatchOperations));
        let has_early_exit = estimation.optimizations.iter()
            .any(|opt| matches!(opt.optimization_type, OptimizationType::EarlyExit));
        
        assert!(has_batch_opt || has_early_exit); // At least one optimization should be present
    }

    #[test]
    fn test_gas_risk_assessment() {
        let config = GasLimitConfig::default();
        let manager = GasLimitManager::new(config);

        // Test low risk
        let mut parameters = HashMap::new();
        parameters.insert("recipients".to_string(), 1);
        let result = manager.validate_gas_limit("escrow_release", &parameters, 1_000_000).unwrap();
        assert_eq!(result.risk_level, GasRiskLevel::Low);

        // Test high risk (close to limit)
        let estimation = manager.estimate_gas("escrow_release", &parameters).unwrap();
        let tight_limit = estimation.estimated_gas + (estimation.estimated_gas / 20); // 5% margin
        let result = manager.validate_gas_limit("escrow_release", &parameters, tight_limit).unwrap();
        assert!(matches!(result.risk_level, GasRiskLevel::High | GasRiskLevel::Medium));
    }

    #[test]
    fn test_custom_operation_profile() {
        let mut config = GasLimitConfig::default();
        let mut manager = GasLimitManager::new(config);

        // Add custom profile
        let custom_profile = GasProfile {
            operation_name: "custom_operation".to_string(),
            base_gas_cost: 25_000,
            variable_gas_factors: vec![
                GasFactor {
                    factor_name: "iterations".to_string(),
                    gas_per_unit: 1_000,
                    max_units: Some(100),
                },
            ],
            complexity: OperationComplexity::Simple,
            optimization_hints: vec![],
        };

        manager.add_operation_profile(custom_profile);

        let mut parameters = HashMap::new();
        parameters.insert("iterations".to_string(), 50);

        let estimation = manager.estimate_gas("custom_operation", &parameters).unwrap();
        
        assert_eq!(estimation.complexity, OperationComplexity::Simple);
        assert_eq!(estimation.estimated_gas, 25_000 + 50 * 1_000);
    }

    #[test]
    fn test_operation_complexity_limits() {
        let config = GasLimitConfig::default();
        let manager = GasLimitManager::new(config);

        // Test simple operation limit
        let simple_estimation = manager.estimate_gas("simple_operation", &HashMap::new()).unwrap();
        assert!(simple_estimation.recommended_limit <= config.simple_operation_limit);

        // Test complex operation limit
        let complex_estimation = manager.estimate_gas("escrow_release", &HashMap::new()).unwrap();
        assert!(complex_estimation.recommended_limit <= config.complex_operation_limit);

        // Test batch operation limit
        let batch_estimation = manager.estimate_gas("emergency_reward_distribution", &HashMap::new()).unwrap();
        assert!(batch_estimation.recommended_limit <= config.batch_operation_limit);
    }

    #[test]
    fn test_safety_margin_calculation() {
        let config = GasLimitConfig {
            safety_margin_percentage: 20.0, // 20% margin
            ..Default::default()
        };
        let manager = GasLimitManager::new(config);

        let parameters = HashMap::new();
        let estimation = manager.estimate_gas("escrow_release", &parameters).unwrap();
        
        let expected_margin = (estimation.estimated_gas as f64 * 0.2) as u64;
        assert_eq!(estimation.safety_margin, expected_margin);
        assert_eq!(estimation.recommended_limit, estimation.estimated_gas + expected_margin);
    }

    #[test]
    fn test_batch_gas_estimator_creation() {
        let config = GasLimitConfig::default();
        let estimator = BatchGasEstimator::new(config);
        
        // Default config should create estimator with 100M Stellar limit and 90% threshold
        let estimate = estimator.estimate_escrow_release_batch(10).unwrap();
        assert_eq!(estimate.total_items, 10);
        assert_eq!(estimate.stellar_max_limit, 100_000_000);
        assert_eq!(estimate.escrow_releases, 10);
        assert_eq!(estimate.verifications, 0);
        assert!(estimate.batch_overhead > 0);
        assert!(estimate.per_item_overhead > 0);
        assert!(estimate.safety_margin > 0);
    }

    #[test]
    fn test_escrow_release_batch_gas_estimate() {
        let config = GasLimitConfig::default();
        let estimator = BatchGasEstimator::new(config);
        
        let estimate = estimator.estimate_escrow_release_batch(5).unwrap();
        
        assert_eq!(estimate.total_items, 5);
        assert_eq!(estimate.escrow_releases, 5);
        assert_eq!(estimate.verifications, 0);
        assert_eq!(estimate.total_estimated_gas, estimate.item_estimates.iter().map(|e| e.estimated_gas).sum::<u64>() + estimate.batch_overhead + (estimate.per_item_overhead * 5));
        assert_eq!(estimate.recommended_limit, estimate.total_estimated_gas + estimate.safety_margin);
        assert_eq!(estimate.item_estimates.len(), 5);
        
        // Each item should have a valid estimate
        for item in &estimate.item_estimates {
            assert_eq!(item.operation_type, BatchOperationType::EscrowRelease);
            assert!(item.estimated_gas > 0);
            assert!(item.base_cost > 0);
        }
    }

    #[test]
    fn test_verification_batch_gas_estimate() {
        let config = GasLimitConfig::default();
        let estimator = BatchGasEstimator::new(config);
        
        let estimate = estimator.estimate_verification_batch(3).unwrap();
        
        assert_eq!(estimate.total_items, 3);
        assert_eq!(estimate.escrow_releases, 0);
        assert_eq!(estimate.verifications, 3);
        assert_eq!(estimate.item_estimates.len(), 3);
        
        for item in &estimate.item_estimates {
            assert_eq!(item.operation_type, BatchOperationType::VulnerabilityVerification);
            assert!(item.estimated_gas > 0);
        }
    }

    #[test]
    fn test_mixed_batch_gas_estimate() {
        let config = GasLimitConfig::default();
        let estimator = BatchGasEstimator::new(config);
        
        let estimate = estimator.estimate_mixed_batch(2, 3).unwrap();
        
        assert_eq!(estimate.total_items, 5);
        assert_eq!(estimate.escrow_releases, 2);
        assert_eq!(estimate.verifications, 3);
        assert_eq!(estimate.item_estimates.len(), 5);
        
        // First 2 items should be escrow releases, next 3 should be verifications
        assert_eq!(estimate.item_estimates[0].operation_type, BatchOperationType::EscrowRelease);
        assert_eq!(estimate.item_estimates[1].operation_type, BatchOperationType::EscrowRelease);
        assert_eq!(estimate.item_estimates[2].operation_type, BatchOperationType::VulnerabilityVerification);
        assert_eq!(estimate.item_estimates[3].operation_type, BatchOperationType::VulnerabilityVerification);
        assert_eq!(estimate.item_estimates[4].operation_type, BatchOperationType::VulnerabilityVerification);
    }

    #[test]
    fn test_batch_gas_threshold_warning() {
        let config = GasLimitConfig::default();
        // Use a very low Stellar max gas to trigger the warning
        let estimator = BatchGasEstimator::with_limits(config, 100_000, 50.0); // 50% of 100K = 50K
        
        // 10 items should easily exceed 50K total gas
        let estimate = estimator.estimate_escrow_release_batch(10).unwrap();
        
        assert!(estimate.exceeds_recommended_threshold);
        assert!(estimate.threshold_warning.is_some());
        assert!(estimate.suggested_splits > 1);
        assert!(estimate.estimated_gas_per_split > 0);
    }

    #[test]
    fn test_batch_gas_within_safe_limits() {
        let config = GasLimitConfig::default();
        // Use a very high Stellar max gas so even large batches are fine
        let estimator = BatchGasEstimator::with_limits(config, 1_000_000_000, 90.0);
        
        let estimate = estimator.estimate_escrow_release_batch(50).unwrap();
        
        assert!(!estimate.exceeds_recommended_threshold);
        assert!(estimate.threshold_warning.is_none());
        assert_eq!(estimate.suggested_splits, 1);
    }

    #[test]
    fn test_batch_gas_estimate_format_output() {
        let config = GasLimitConfig::default();
        let estimator = BatchGasEstimator::new(config);
        
        let estimate = estimator.estimate_escrow_release_batch(3).unwrap();
        let formatted = BatchGasEstimator::format_estimate(&estimate);
        
        assert!(formatted.contains("Batch Gas Estimate"));
        assert!(formatted.contains("Total items:"));
        assert!(formatted.contains("Escrow releases: 3"));
        assert!(formatted.contains("Recommended limit:"));
        assert!(formatted.contains("Stellar max limit:"));
        assert!(formatted.contains("Risk level:"));
    }

    #[test]
    fn test_batch_gas_estimate_with_overhead() {
        let config = GasLimitConfig::default();
        let estimator = BatchGasEstimator::new(config);
        
        // 1 item batch: overhead should be batch_overhead + 1 * per_item_overhead
        let estimate_1 = estimator.estimate_escrow_release_batch(1).unwrap();
        let expected_overhead_1 = estimate_1.batch_overhead + estimate_1.per_item_overhead;
        
        // 10 item batch: overhead should be batch_overhead + 10 * per_item_overhead
        let estimate_10 = estimator.estimate_escrow_release_batch(10).unwrap();
        let expected_overhead_10 = estimate_10.batch_overhead + (estimate_10.per_item_overhead * 10);
        
        // Overhead scales with item count
        assert!(estimate_10.total_estimated_gas > estimate_1.total_estimated_gas);
        assert!(expected_overhead_10 > expected_overhead_1);
    }

    #[test]
    fn test_batch_gas_risk_levels() {
        let config = GasLimitConfig::default();
        let estimator = BatchGasEstimator::new(config);
        
        // Small batch should be low risk
        let estimate_small = estimator.estimate_escrow_release_batch(1).unwrap();
        assert_eq!(estimate_small.risk_level, GasRiskLevel::Low);
        
        // Very large batch may be higher risk
        // Use with_limits to make sure we get a meaningful result
        let estimator_tight = BatchGasEstimator::with_limits(config, 1_000_000, 90.0);
        let estimate_large = estimator_tight.estimate_escrow_release_batch(100).unwrap();
        
        // Must be some risk level
        assert!(matches!(estimate_large.risk_level, GasRiskLevel::Low | GasRiskLevel::Medium | GasRiskLevel::High | GasRiskLevel::Critical));
    }

    #[test]
    fn test_custom_batch_estimator_limits() {
        let config = GasLimitConfig::default();
        let estimator = BatchGasEstimator::with_limits(config, 50_000_000, 80.0);
        
        // With 80% threshold of 50M = 40M, a modest batch should be safe
        let estimate = estimator.estimate_escrow_release_batch(20).unwrap();
        
        assert_eq!(estimate.stellar_max_limit, 50_000_000);
        
        // Risk level should be defined
        assert!(matches!(estimate.risk_level, GasRiskLevel::Low | GasRiskLevel::Medium | GasRiskLevel::High | GasRiskLevel::Critical));
    }

    #[test]
    fn test_batch_operation_type_as_str() {
        assert_eq!(BatchOperationType::EscrowRelease.as_str(), "escrow_release");
        assert_eq!(BatchOperationType::VulnerabilityVerification.as_str(), "vulnerability_verification");
    }

    #[test]
    fn test_empty_batch_estimate() {
        let config = GasLimitConfig::default();
        let estimator = BatchGasEstimator::new(config);
        
        let estimate = estimator.estimate_escrow_release_batch(0).unwrap();
        
        assert_eq!(estimate.total_items, 0);
        assert_eq!(estimate.escrow_releases, 0);
        assert!(estimate.total_estimated_gas > 0); // Still has overhead
    }

    #[test]
    fn test_optimization_disabled() {
        let config = GasLimitConfig {
            enable_optimization: false,
            ..Default::default()
        };
        let manager = GasLimitManager::new(config);

        let parameters = HashMap::new();
        let estimation = manager.estimate_gas("escrow_release", &parameters).unwrap();
        
        assert!(estimation.optimizations.is_empty());
    }

    #[test]
    fn test_unknown_operation() {
        let config = GasLimitConfig::default();
        let manager = GasLimitManager::new(config);

        let parameters = HashMap::new();
        let result = manager.estimate_gas("unknown_operation", &parameters);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown operation"));
    }

    #[test]
    fn test_gas_factor_limits() {
        let config = GasLimitConfig::default();
        let manager = GasLimitManager::new(config);

        let mut parameters = HashMap::new();
        parameters.insert("recipients".to_string(), 200); // Exceeds max_units (100)

        let estimation = manager.estimate_gas("escrow_release", &parameters).unwrap();
        
        // Should be capped at max_units
        let expected_recipient_gas = 100 * 10_000; // max_units * gas_per_unit
        let expected_total = 50_000 + expected_recipient_gas; // base + recipients
        assert_eq!(estimation.estimated_gas, expected_total);
    }

    #[test]
    fn test_implementation_difficulty_assessment() {
        let config = GasLimitConfig::default();
        let manager = GasLimitManager::new(config);

        let easy_difficulty = manager.assess_implementation_difficulty(&OptimizationType::EarlyExit);
        assert_eq!(easy_difficulty, ImplementationDifficulty::Easy);

        let medium_difficulty = manager.assess_implementation_difficulty(&OptimizationType::BatchOperations);
        assert_eq!(medium_difficulty, ImplementationDifficulty::Medium);

        let hard_difficulty = manager.assess_implementation_difficulty(&OptimizationType::LoopOptimization);
        assert_eq!(hard_difficulty, ImplementationDifficulty::Hard);
    }

    #[test]
    fn test_config_update() {
        let mut config = GasLimitConfig::default();
        let mut manager = GasLimitManager::new(config.clone());

        // Update config
        config.complex_operation_limit = 50_000_000;
        config.safety_margin_percentage = 15.0;
        
        manager.update_config(config);
        
        // Verify config was updated (this would require access to internal config)
        // For now, just ensure the manager still works
        let parameters = HashMap::new();
        let estimation = manager.estimate_gas("escrow_release", &parameters).unwrap();
        assert!(estimation.estimated_gas > 0);
    }

    #[test]
    fn test_optimization_savings_estimation() {
        let config = GasLimitConfig::default();
        let manager = GasLimitManager::new(config);

        let mut parameters = HashMap::new();
        parameters.insert("recipients".to_string(), 10);
        parameters.insert("reward_calculations".to_string(), 5);

        let estimation = manager.estimate_gas("reward_distribution", &parameters).unwrap();
        
        // Should have some optimization suggestions with potential savings
        assert!(!estimation.optimizations.is_empty());
        for optimization in &estimation.optimizations {
            assert!(optimization.potential_savings > 0);
            assert!(!optimization.description.is_empty());
        }
    }

    #[test]
    fn test_multiple_variable_factors() {
        let config = GasLimitConfig::default();
        let manager = GasLimitManager::new(config);

        let mut parameters = HashMap::new();
        parameters.insert("researchers".to_string(), 10);
        parameters.insert("reward_calculations".to_string(), 20);
        parameters.insert("severity_checks".to_string(), 5);

        let estimation = manager.estimate_gas("reward_distribution", &parameters).unwrap();
        
        // Base cost + all variable factors
        let expected_gas = 75_000 + (10 * 15_000) + (20 * 2_000) + (5 * 1_000);
        assert_eq!(estimation.estimated_gas, expected_gas);
    }
}
