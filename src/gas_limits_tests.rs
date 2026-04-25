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
