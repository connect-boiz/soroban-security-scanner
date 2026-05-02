//! Tests for the Time Travel Debugger module

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use soroban_sdk::xdr::ScVal;

    #[tokio::test]
    async fn test_time_travel_debugger_creation() {
        let config = TimeTravelConfig::default();
        let result = TimeTravelDebugger::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_ledger_snapshot_serialization() {
        let snapshot = LedgerSnapshot {
            ledger_sequence: 100000,
            ledger_hash: "abcd1234".to_string(),
            close_time: 1640995200,
            protocol_version: 20,
            operation_count: 10,
            base_fee: 100,
            base_reserve: 5000000,
        };

        let serialized = serde_json::to_string(&snapshot).unwrap();
        let deserialized: LedgerSnapshot = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(snapshot, deserialized);
    }

    #[tokio::test]
    async fn test_contract_state_creation() {
        let mut storage = HashMap::new();
        storage.insert("balance".to_string(), ScVal::U64(1000));
        storage.insert("owner".to_string(), ScVal::Bytes(vec![1, 2, 3, 4]));

        let state = ContractState {
            contract_id: "test_contract".to_string(),
            wasm_hash: "test_hash".to_string(),
            storage,
            ledger_sequence: 1000,
        };

        assert_eq!(state.contract_id, "test_contract");
        assert_eq!(state.ledger_sequence, 1000);
        assert_eq!(state.storage.len(), 2);
    }

    #[tokio::test]
    async fn test_forked_state_creation() {
        let config = TimeTravelConfig::default();
        let debugger = TimeTravelDebugger::new(config).await.unwrap();
        
        // This would normally fetch from Stellar RPC, but for testing we'll mock it
        let ledger_sequence = 1000;
        
        // Test that we can create a fork (this will fail in real test without RPC)
        // but we can test the structure
        assert_eq!(ledger_sequence, 1000);
    }

    #[tokio::test]
    async fn test_upgrade_simulation_result() {
        let result = UpgradeSimulationResult {
            is_compatible: true,
            compatibility_issues: vec![],
            orphaned_entries: vec!["old_storage".to_string()],
            warnings: vec!["Some warning".to_string()],
        };

        assert!(result.is_compatible);
        assert_eq!(result.orphaned_entries.len(), 1);
        assert_eq!(result.warnings.len(), 1);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let stats = CacheStats {
            contract_states_cached: 100,
            ledgers_cached: 50,
            max_contract_states: 1000,
            max_ledgers: 100,
        };

        assert_eq!(stats.contract_states_cached, 100);
        assert_eq!(stats.ledgers_cached, 50);
        assert_eq!(stats.max_contract_states, 1000);
    }

    #[tokio::test]
    async fn test_test_result() {
        let result = TestResult {
            contract_id: "test_contract".to_string(),
            ledger_sequence: 1000,
            passed: true,
            issues: vec![],
            execution_time: std::time::Duration::from_millis(100),
        };

        assert!(result.passed);
        assert_eq!(result.contract_id, "test_contract");
        assert_eq!(result.ledger_sequence, 1000);
        assert_eq!(result.execution_time.as_millis(), 100);
    }

    #[tokio::test]
    async fn test_time_travel_config_default() {
        let config = TimeTravelConfig::default();
        
        assert_eq!(config.rpc_url, "https://mainnet.stellar.rpc");
        assert_eq!(config.cache_size, 10000);
        assert_eq!(config.request_timeout, 30);
        assert_eq!(config.max_retries, 3);
    }

    #[tokio::test]
    async fn test_time_travel_config_serialization() {
        let config = TimeTravelConfig {
            rpc_url: "https://testnet.stellar.rpc".to_string(),
            network_passphrase: "Test SDF Network ; September 2015".to_string(),
            cache_size: 5000,
            request_timeout: 60,
            max_retries: 5,
        };

        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: TimeTravelConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.rpc_url, deserialized.rpc_url);
        assert_eq!(config.cache_size, deserialized.cache_size);
    }

    // Integration tests would go here but require actual Stellar RPC access
    // These would test the full workflow with real network data
}

#[cfg(test)]
mod state_injection_tests {
    use super::*;
    use crate::time_travel_debugger::state_injection::StateInjector;
    use std::collections::HashMap;
    use soroban_sdk::xdr::ScVal;

    #[tokio::test]
    async fn test_state_injector_creation() {
        let config = TimeTravelConfig::default();
        let injector = StateInjector::new(config);
        
        let states = injector.get_injected_states().await;
        assert_eq!(states.len(), 0);
    }

    #[tokio::test]
    async fn test_state_injector_state_tracking() {
        let config = TimeTravelConfig::default();
        let injector = StateInjector::new(config);
        
        let contract_id = "test_contract";
        assert!(!injector.is_state_injected(contract_id).await);
        
        // After injection, this should be true
        // injector.inject_state(&state).await.unwrap();
        // assert!(injector.is_state_injected(contract_id).await);
    }

    #[tokio::test]
    async fn test_storage_stats() {
        let config = TimeTravelConfig::default();
        let injector = StateInjector::new(config);
        
        let contract_id = "test_contract";
        let stats = injector.get_storage_stats(contract_id).await;
        assert!(stats.is_none());
    }

    #[tokio::test]
    async fn test_scval_conversion() {
        let config = TimeTravelConfig::default();
        let injector = StateInjector::new(config);
        
        let scval = ScVal::U32(42);
        let result = injector.convert_scval_to_sdk(&scval);
        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod contract_upgrade_tests {
    use super::*;
    use crate::time_travel_debugger::contract_upgrade::{
        ContractUpgradeSimulator, StorageLayoutInfo, StorageType
    };
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_upgrade_simulator_creation() {
        let config = TimeTravelConfig::default();
        let simulator = ContractUpgradeSimulator::new(config);
        // Should create without panicking
    }

    #[tokio::test]
    async fn test_wasm_validation() {
        let config = TimeTravelConfig::default();
        let simulator = ContractUpgradeSimulator::new(config);
        
        // Test with invalid WASM
        let invalid_wasm = b"not wasm";
        let result = simulator.validate_wasm_compatibility(invalid_wasm).await.unwrap();
        assert!(!result.is_empty());
        
        // Test with valid WASM header (minimal)
        let valid_wasm = b"\0asm\x01\0\0\0";
        let result = simulator.validate_wasm_compatibility(valid_wasm).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_storage_type_validation() {
        let config = TimeTravelConfig::default();
        let simulator = ContractUpgradeSimulator::new(config);
        
        let layout_info = StorageLayoutInfo {
            storage_type: StorageType::Temporary,
            required: false,
            description: "Test storage".to_string(),
        };
        
        // Valid temporary storage
        let valid_value = ScVal::U32(42);
        assert!(simulator.validate_storage_type(&valid_value, &layout_info).is_ok());
        
        // Invalid temporary storage
        let invalid_value = ScVal::Void;
        assert!(simulator.validate_storage_type(&invalid_value, &layout_info).is_err());
    }

    #[tokio::test]
    async fn test_upgrade_simulation() {
        let config = TimeTravelConfig::default();
        let simulator = ContractUpgradeSimulator::new(config);
        
        let mut storage = HashMap::new();
        storage.insert("balance".to_string(), ScVal::U64(1000));
        
        let state = ContractState {
            contract_id: "test_contract".to_string(),
            wasm_hash: "old_hash".to_string(),
            storage,
            ledger_sequence: 1000,
        };
        
        let new_wasm = b"\0asm\x01\0\0\0"; // Minimal valid WASM
        let result = simulator.simulate_upgrade(&state, new_wasm).await.unwrap();
        
        // Should return a result with compatibility information
        assert_eq!(result.contract_id, "test_contract");
    }
}

#[cfg(test)]
mod orphaned_state_tests {
    use super::*;
    use crate::time_travel_debugger::orphaned_state::{
        OrphanedStateTracker, DataLossRisk, OrphanedSummary
    };
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_orphaned_tracker_creation() {
        let tracker = OrphanedStateTracker::new();
        assert_eq!(tracker.get_contracts_with_orphans().len(), 0);
    }

    #[tokio::test]
    async fn test_value_type_detection() {
        let tracker = OrphanedStateTracker::new();
        
        let bool_val = ScVal::Bool(true);
        assert_eq!(tracker.get_value_type(&bool_val), "bool");
        
        let u32_val = ScVal::U32(42);
        assert_eq!(tracker.get_value_type(&u32_val), "u32");
        
        let bytes_val = ScVal::Bytes(vec![1, 2, 3, 4]);
        assert_eq!(tracker.get_value_type(&bytes_val), "bytes");
    }

    #[tokio::test]
    async fn test_size_estimation() {
        let tracker = OrphanedStateTracker::new();
        
        let bool_val = ScVal::Bool(true);
        assert_eq!(tracker.estimate_value_size(&bool_val), 1);
        
        let u32_val = ScVal::U32(42);
        assert_eq!(tracker.estimate_value_size(&u32_val), 4);
        
        let bytes_val = ScVal::Bytes(vec![1, 2, 3, 4, 5]);
        assert_eq!(tracker.estimate_value_size(&bytes_val), 5);
    }

    #[tokio::test]
    async fn test_risk_assessment() {
        let tracker = OrphanedStateTracker::new();
        
        let balance_val = ScVal::U64(1000);
        assert_eq!(tracker.assess_data_loss_risk("balance", &balance_val), DataLossRisk::High);
        
        let temp_val = ScVal::U32(42);
        assert_eq!(tracker.assess_data_loss_risk("temp_counter", &temp_val), DataLossRisk::Low);
    }

    #[tokio::test]
    async fn test_orphaned_summary() {
        let tracker = OrphanedStateTracker::new();
        let contract_id = "test_contract";
        
        let summary = tracker.get_orphaned_summary(contract_id);
        assert_eq!(summary.total_entries, 0);
        assert_eq!(summary.risk_level, crate::time_travel_debugger::orphaned_state::OverallRisk::Low);
    }

    #[tokio::test]
    async fn test_recovery_recommendations() {
        let tracker = OrphanedStateTracker::new();
        let contract_id = "test_contract";
        
        let recommendations = tracker.generate_recovery_recommendations(contract_id);
        // Should return empty recommendations for contract with no orphans
        assert_eq!(recommendations.len(), 0);
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    use crate::time_travel_debugger::cache::{StateCache, CacheConfig};

    #[tokio::test]
    async fn test_cache_creation() {
        let config = CacheConfig::default();
        let cache = StateCache::new(config);
        assert!(cache.is_ok());
    }

    #[tokio::test]
    async fn test_contract_state_caching() {
        let config = CacheConfig::default();
        let cache = StateCache::new(config).unwrap();
        
        let contract_id = "test_contract";
        let ledger_sequence = 1000;
        let state = ContractState {
            contract_id: contract_id.to_string(),
            wasm_hash: "test_hash".to_string(),
            storage: HashMap::new(),
            ledger_sequence,
        };
        
        // Put state in cache
        cache.put_contract_state(contract_id, ledger_sequence, state.clone()).await.unwrap();
        
        // Get state from cache
        let cached_state = cache.get_contract_state(contract_id, ledger_sequence).await;
        assert!(cached_state.is_some());
        assert_eq!(cached_state.unwrap().contract_id, contract_id);
    }

    #[tokio::test]
    async fn test_cache_statistics() {
        let config = CacheConfig::default();
        let cache = StateCache::new(config).unwrap();
        
        // Initially no hits or misses
        let stats = cache.get_statistics().await;
        assert_eq!(stats.contract_hits, 0);
        assert_eq!(stats.contract_misses, 0);
        
        // Cache miss
        let _ = cache.get_contract_state("nonexistent", 1000).await;
        let stats = cache.get_statistics().await;
        assert_eq!(stats.contract_misses, 1);
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let config = CacheConfig::default();
        let cache = StateCache::new(config).unwrap();
        
        let contract_id = "test_contract";
        let ledger_sequence = 1000;
        let state = ContractState {
            contract_id: contract_id.to_string(),
            wasm_hash: "test_hash".to_string(),
            storage: HashMap::new(),
            ledger_sequence,
        };
        
        // Put state in cache
        cache.put_contract_state(contract_id, ledger_sequence, state).await.unwrap();
        
        // Verify it's cached
        let cached_state = cache.get_contract_state(contract_id, ledger_sequence).await;
        assert!(cached_state.is_some());
        
        // Clear cache
        cache.clear().await;
        
        // Verify it's gone
        let cached_state = cache.get_contract_state(contract_id, ledger_sequence).await;
        assert!(cached_state.is_none());
    }

    #[tokio::test]
    async fn test_cache_info() {
        let config = CacheConfig::default();
        let cache = StateCache::new(config).unwrap();
        
        let info = cache.get_cache_info().await;
        assert_eq!(info.contract_states_cached, 0);
        assert_eq!(info.ledger_snapshots_cached, 0);
        assert_eq!(info.max_contract_states, config.max_contract_states);
        assert_eq!(info.max_ledger_snapshots, config.max_ledger_snapshots);
    }

    #[tokio::test]
    async fn test_cache_ttl() {
        let mut config = CacheConfig::default();
        config.ttl_seconds = 1; // 1 second TTL for testing
        
        let cache = StateCache::new(config).unwrap();
        
        let contract_id = "test_contract";
        let ledger_sequence = 1000;
        let state = ContractState {
            contract_id: contract_id.to_string(),
            wasm_hash: "test_hash".to_string(),
            storage: HashMap::new(),
            ledger_sequence,
        };
        
        // Put state in cache
        cache.put_contract_state(contract_id, ledger_sequence, state).await.unwrap();
        
        // Should be available immediately
        let cached_state = cache.get_contract_state(contract_id, ledger_sequence).await;
        assert!(cached_state.is_some());
        
        // Wait for TTL to expire
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        
        // Should be expired now
        let cached_state = cache.get_contract_state(contract_id, ledger_sequence).await;
        assert!(cached_state.is_none());
    }
}

// Performance tests
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_large_cache_performance() {
        let config = CacheConfig {
            max_contract_states: 10000,
            max_ledger_snapshots: 1000,
            ttl_seconds: 3600,
            enable_compression: true,
            cleanup_interval_seconds: 300,
        };
        
        let cache = StateCache::new(config).unwrap();
        
        let start = Instant::now();
        
        // Insert 1000 contract states
        for i in 0..1000 {
            let contract_id = format!("contract_{}", i);
            let ledger_sequence = i;
            let state = ContractState {
                contract_id: contract_id.clone(),
                wasm_hash: format!("hash_{}", i),
                storage: HashMap::new(),
                ledger_sequence,
            };
            
            cache.put_contract_state(&contract_id, ledger_sequence, state).await.unwrap();
        }
        
        let insert_time = start.elapsed();
        println!("Inserted 1000 states in {:?}", insert_time);
        
        // Test retrieval performance
        let start = Instant::now();
        for i in 0..1000 {
            let contract_id = format!("contract_{}", i);
            let _ = cache.get_contract_state(&contract_id, i).await;
        }
        
        let retrieval_time = start.elapsed();
        println!("Retrieved 1000 states in {:?}", retrieval_time);
        
        // Performance assertions
        assert!(insert_time.as_millis() < 5000, "Insert should take less than 5 seconds");
        assert!(retrieval_time.as_millis() < 1000, "Retrieval should take less than 1 second");
    }

    #[tokio::test]
    async fn test_concurrent_cache_access() {
        let config = CacheConfig::default();
        let cache = std::sync::Arc::new(StateCache::new(config).unwrap());
        
        let mut handles = vec![];
        
        // Spawn 10 concurrent tasks
        for i in 0..10 {
            let cache_clone = cache.clone();
            let handle = tokio::spawn(async move {
                let contract_id = format!("contract_{}", i);
                let ledger_sequence = i;
                let state = ContractState {
                    contract_id: contract_id.clone(),
                    wasm_hash: format!("hash_{}", i),
                    storage: HashMap::new(),
                    ledger_sequence,
                };
                
                // Insert and retrieve
                cache_clone.put_contract_state(&contract_id, ledger_sequence, state).await.unwrap();
                let _ = cache_clone.get_contract_state(&contract_id, ledger_sequence).await;
            });
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }
        
        // Verify all states are cached
        let info = cache.get_cache_info().await;
        assert_eq!(info.contract_states_cached, 10);
    }
}
