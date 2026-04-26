//! Tests for the Differential Fuzzing module

#[cfg(test)]
mod tests {
    use crate::differential_fuzzing::*;
    use std::time::Duration;

    #[test]
    fn test_sdk_version_creation() {
        let version = SdkVersion::new("25.3.0");
        assert_eq!(version.version, "25.3.0");
        assert_eq!(version.git_hash, None);
        assert_eq!(version.release_date, None);

        let version_with_details = SdkVersion::new("25.2.0")
            .with_git_hash("abc123")
            .with_release_date("2024-01-15");
        assert_eq!(version_with_details.version, "25.2.0");
        assert_eq!(version_with_details.git_hash, Some("abc123".to_string()));
        assert_eq!(version_with_details.release_date, Some("2024-01-15".to_string()));
    }

    #[test]
    fn test_edge_case_types() {
        let edge_cases = vec![
            EdgeCaseType::MaxI128,
            EdgeCaseType::MinI128,
            EdgeCaseType::ZeroValue,
            EdgeCaseType::EmptyVector,
            EdgeCaseType::LargeVector,
        ];

        for edge_case in edge_cases {
            let description = edge_case.description();
            assert!(!description.is_empty());
        }
    }

    #[test]
    fn test_differential_fuzzing_config_default() {
        let config = DifferentialFuzzingConfig::default();
        
        assert_eq!(config.sdk_versions.len(), 3);
        assert_eq!(config.test_count, 1000);
        assert_eq!(config.max_execution_time, Duration::from_secs(30));
        assert!(config.enable_cross_contract_simulation);
        assert!(config.enable_ledger_snapshot_integration);
        assert!(config.enable_deterministic_detection);
        assert_eq!(config.gas_threshold_percentage, 10.0);
    }

    #[test]
    fn test_argument_value_creation() {
        let values = vec![
            ArgumentValue::I128(42),
            ArgumentValue::U64(100),
            ArgumentValue::Bool(true),
            ArgumentValue::String("test".to_string()),
            ArgumentValue::Bytes(vec![1, 2, 3]),
            ArgumentValue::Address([42u8; 32]),
            ArgumentValue::Vector(vec![
                ArgumentValue::I128(1),
                ArgumentValue::I128(2),
            ]),
            ArgumentValue::None,
        ];

        for value in values {
            // Test that values can be created and cloned
            let cloned = value.clone();
            assert_eq!(std::mem::discriminant(&value), std::mem::discriminant(&cloned));
        }
    }

    #[test]
    fn test_test_input_creation() {
        let input = TestInput {
            function_name: "transfer".to_string(),
            arguments: vec![
                TestArgument {
                    value: ArgumentValue::Address([1u8; 32]),
                    argument_type: ArgumentType::Address,
                },
                TestArgument {
                    value: ArgumentValue::I128(1000),
                    argument_type: ArgumentType::I128,
                },
            ],
            salt: Some([42u8; 32]),
            metadata: TestInputMetadata {
                edge_case_type: Some(EdgeCaseType::MaxI128),
                generation_method: "test".to_string(),
                complexity_score: 0.8,
            },
        };

        assert_eq!(input.function_name, "transfer");
        assert_eq!(input.arguments.len(), 2);
        assert!(input.salt.is_some());
        assert!(input.metadata.edge_case_type.is_some());
    }

    #[test]
    fn test_execution_result_creation() {
        let result = ExecutionResult {
            sdk_version: SdkVersion::new("25.3.0"),
            success: true,
            return_value: Some(ArgumentValue::I128(42)),
            gas_consumed: 1000,
            state_changes: vec![],
            execution_trace: ExecutionTrace::new(),
            error: None,
            execution_time: Duration::from_millis(100),
        };

        assert!(result.success);
        assert_eq!(result.gas_consumed, 1000);
        assert!(result.return_value.is_some());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_execution_trace() {
        let mut trace = ExecutionTrace::new();
        
        trace.add_event(TraceEvent::new(
            TraceEventType::FunctionEntry,
            "test_function".to_string(),
            1,
            "Entering test function".to_string(),
        ));

        trace.add_event(TraceEvent::new(
            TraceEventType::FunctionExit,
            "test_function".to_string(),
            10,
            "Exiting test function".to_string(),
        ));

        trace.finalize();

        assert_eq!(trace.events.len(), 2);
        assert!(trace.start_time <= trace.end_time);
        assert_eq!(trace.max_stack_depth, 1);
    }

    #[test]
    fn test_trace_similarity() {
        let mut trace1 = ExecutionTrace::new();
        let mut trace2 = ExecutionTrace::new();

        // Add identical events
        for i in 0..5 {
            let event = TraceEvent::new(
                TraceEventType::VariableRead,
                format!("var_{}", i),
                i + 1,
                format!("Reading variable {}", i),
            );
            trace1.add_event(event.clone());
            trace2.add_event(event);
        }

        trace1.finalize();
        trace2.finalize();

        let similarity = trace1.similarity_score(&trace2);
        assert_eq!(similarity, 1.0); // Should be identical

        // Add a different event to trace2
        trace2.add_event(TraceEvent::new(
            TraceEventType::Error,
            "error".to_string(),
            6,
            "An error occurred".to_string(),
        ));
        trace2.finalize();

        let similarity_after_diff = trace1.similarity_score(&trace2);
        assert!(similarity_after_diff < 1.0);
        assert!(similarity_after_diff > 0.8); // Still mostly similar
    }

    #[test]
    fn test_state_change() {
        let state_change = StateChange {
            key: b"balance".to_vec(),
            old_value: Some(b"100".to_vec()),
            new_value: Some(b"200".to_vec()),
            change_type: StateChangeType::Update,
        };

        assert_eq!(state_change.key, b"balance");
        assert_eq!(state_change.old_value, Some(b"100".to_vec()));
        assert_eq!(state_change.new_value, Some(b"200".to_vec()));
        assert_eq!(state_change.change_type, StateChangeType::Update);
    }

    #[test]
    fn test_discrepancy_types() {
        let discrepancy_types = vec![
            DiscrepancyType::GasConsumption,
            DiscrepancyType::StateChange,
            DiscrepancyType::LogicDivergence,
            DiscrepancyType::ReturnValue,
            DiscrepancyType::ErrorDifference,
            DiscrepancyType::ExecutionOrder,
            DiscrepancyType::MemoryUsage,
            DiscrepancyType::ExternalCallBehavior,
        ];

        // Test that all discrepancy types can be created and cloned
        for discrepancy_type in discrepancy_types {
            let cloned = discrepancy_type.clone();
            assert_eq!(discrepancy_type, cloned);
        }
    }

    #[test]
    fn test_non_deterministic_types() {
        let types = vec![
            NonDeterministicType::RandomValueGeneration,
            NonDeterministicType::TimeDependentLogic,
            NonDeterministicType::ExternalStateDependency,
            NonDeterministicType::NetworkCallVariation,
            NonDeterministicType::BlockchainStateDependency,
            NonDeterministicType::FloatingPointArithmetic,
            NonDeterministicType::UninitializedMemory,
            NonDeterministicType::ConcurrentExecution,
            NonDeterministicType::HashCollision,
            NonDeterministicType::ProbabilisticLogic,
        ];

        // Test that all types can be created and cloned
        for nd_type in types {
            let cloned = nd_type.clone();
            assert_eq!(nd_type, cloned);
        }
    }

    #[test]
    fn test_reentrancy_patterns() {
        let patterns = vec![
            ReentrancyPattern::DirectReentrancy,
            ReentrancyPattern::IndirectReentrancy,
            ReentrancyPattern::CrossFunctionReentrancy,
            ReentrancyPattern::ReadOnlyReentrancy,
            ReentrancyPattern::StateChangeBeforeCall,
            ReentrancyPattern::StateChangeAfterCall,
            ReentrancyPattern::DelegateCallReentrancy,
            ReentrancyPattern::MultiContractReentrancy,
        ];

        // Test that all patterns can be created and cloned
        for pattern in patterns {
            let cloned = pattern.clone();
            assert_eq!(pattern, cloned);
        }
    }

    #[tokio::test]
    async fn test_input_generator() {
        let edge_cases = vec![
            EdgeCaseType::MaxI128,
            EdgeCaseType::MinI128,
            EdgeCaseType::ZeroValue,
        ];

        let mut generator = InputGenerator::new(edge_cases);
        let inputs = generator.generate_test_inputs(10).unwrap();

        assert_eq!(inputs.len(), 10);
        
        for input in &inputs {
            assert!(!input.function_name.is_empty());
            assert!(input.metadata.edge_case_type.is_some());
            assert!(input.metadata.complexity_score >= 0.0);
            assert!(input.metadata.complexity_score <= 1.0);
        }
    }

    #[test]
    fn test_deterministic_detector_config() {
        let config = DeterministicDetectorConfig::default();
        
        assert_eq!(config.execution_retries, 5);
        assert_eq!(config.variation_threshold, 0.1);
        assert_eq!(config.time_dependency_threshold, Duration::from_millis(100));
        assert_eq!(config.gas_variation_threshold, 0.05);
        assert_eq!(config.trace_similarity_threshold, 0.95);
        assert!(config.enable_timing_analysis);
        assert!(config.enable_state_analysis);
        assert!(config.enable_trace_analysis);
    }

    #[test]
    fn test_call_graph() {
        let call_graph = CallGraph {
            nodes: vec![],
            edges: vec![],
            entry_point: "main".to_string(),
            reentrancy_cycles: vec![],
        };

        assert_eq!(call_graph.entry_point, "main");
        assert_eq!(call_graph.nodes.len(), 0);
        assert_eq!(call_graph.edges.len(), 0);
        assert_eq!(call_graph.reentrancy_cycles.len(), 0);
    }

    #[test]
    fn test_execution_tracer() {
        let mut tracer = ExecutionTracer::new();
        
        tracer.start_trace();
        tracer.record_function_entry("test_function", 1);
        tracer.record_variable_read("x", ArgumentValue::I128(42), 2);
        tracer.record_variable_write("x", ArgumentValue::I128(100), 3);
        tracer.record_function_exit("test_function", 4, Some(ArgumentValue::I128(100)));
        
        let trace = tracer.stop_trace().unwrap();
        
        assert_eq!(trace.events.len(), 4);
        assert_eq!(trace.events[0].event_type, TraceEventType::FunctionEntry);
        assert_eq!(trace.events[1].event_type, TraceEventType::VariableRead);
        assert_eq!(trace.events[2].event_type, TraceEventType::VariableWrite);
        assert_eq!(trace.events[3].event_type, TraceEventType::FunctionExit);
    }

    #[test]
    fn test_memory_usage_info() {
        let mut memory_info = MemoryUsageInfo::new();
        
        assert_eq!(memory_info.peak_memory, 0);
        assert_eq!(memory_info.current_memory, 0);
        assert_eq!(memory_info.allocations.len(), 0);
        
        memory_info.record_allocation(0x1000, 1024);
        assert_eq!(memory_info.current_memory, 1024);
        assert_eq!(memory_info.peak_memory, 1024);
        assert_eq!(memory_info.allocations.len(), 1);
        
        memory_info.record_deallocation(0x1000, 1024);
        assert_eq!(memory_info.current_memory, 0);
        assert_eq!(memory_info.peak_memory, 1024);
        assert_eq!(memory_info.allocations.len(), 0);
    }

    #[test]
    fn test_snapshot_config() {
        let config = SnapshotConfig::default();
        
        assert_eq!(config.network_url, "https://horizon-futurenet.stellar.org");
        assert_eq!(config.network_passphrase, "Test SDF Future Network ; October 2022");
        assert_eq!(config.horizon_url, "https://horizon-futurenet.stellar.org");
        assert!(config.friendbot_url.is_some());
        assert!(config.cache_enabled);
        assert_eq!(config.cache_ttl, Duration::from_secs(300));
        assert_eq!(config.max_snapshots, 100);
    }

    #[test]
    fn test_trace_event_creation() {
        let event = TraceEvent::new(
            TraceEventType::FunctionEntry,
            "test_function".to_string(),
            42,
            "Test function entry".to_string(),
        );

        assert_eq!(event.event_type, TraceEventType::FunctionEntry);
        assert_eq!(event.function_name, "test_function");
        assert_eq!(event.line_number, 42);
        assert_eq!(event.description, "Test function entry");
        assert_eq!(event.gas_consumed, 0);
        assert_eq!(event.stack_depth, 0);
        assert!(event.memory_address.is_none());
        assert!(event.value.is_none());
    }

    #[test]
    fn test_trace_event_equivalence() {
        let event1 = TraceEvent::new(
            TraceEventType::FunctionEntry,
            "test_function".to_string(),
            42,
            "Test function entry".to_string(),
        );

        let event2 = TraceEvent::new(
            TraceEventType::FunctionEntry,
            "test_function".to_string(),
            42,
            "Different description".to_string(),
        );

        assert!(event1.equivalent_to(&event2)); // Should be equivalent despite different description

        let event3 = TraceEvent::new(
            TraceEventType::FunctionExit,
            "test_function".to_string(),
            42,
            "Test function exit".to_string(),
        );

        assert!(!event1.equivalent_to(&event3)); // Should not be equivalent due to different event type
    }

    #[test]
    fn test_differential_fuzzing_summary() {
        let summary = DifferentialFuzzingSummary {
            total_tests: 1000,
            total_discrepancies: 50,
            total_non_deterministic: 10,
            high_priority_issues: 5,
            gas_discrepancies: 20,
            state_discrepancies: 15,
            logic_discrepancies: 10,
        };

        assert_eq!(summary.total_tests, 1000);
        assert_eq!(summary.total_discrepancies, 50);
        assert_eq!(summary.total_non_deterministic, 10);
        assert_eq!(summary.high_priority_issues, 5);
        assert_eq!(summary.gas_discrepancies, 20);
        assert_eq!(summary.state_discrepancies, 15);
        assert_eq!(summary.logic_discrepancies, 10);
    }

    #[test]
    fn test_cross_contract_simulation_result() {
        let result = CrossContractSimulationResult {
            call_graph: CallGraph {
                nodes: vec![],
                edges: vec![],
                entry_point: "main".to_string(),
                reentrancy_cycles: vec![],
            },
            reentrancy_vulnerabilities: vec![],
            state_consistency_issues: vec![],
            gas_analysis: GasAnalysis {
                total_gas_consumed: 50000,
                gas_by_contract: std::collections::HashMap::new(),
                gas_by_function: std::collections::HashMap::new(),
                gas_griefing_potential: 0.3,
                infinite_loop_risk: 0.1,
            },
            security_score: 0.85,
        };

        assert_eq!(result.security_score, 0.85);
        assert_eq!(result.gas_analysis.total_gas_consumed, 50000);
        assert_eq!(result.gas_analysis.gas_griefing_potential, 0.3);
        assert_eq!(result.gas_analysis.infinite_loop_risk, 0.1);
    }

    #[test]
    fn test_serialization() {
        // Test that key structures can be serialized and deserialized
        let config = DifferentialFuzzingConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: DifferentialFuzzingConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.test_count, deserialized.test_count);
        assert_eq!(config.sdk_versions.len(), deserialized.sdk_versions.len());
    }

    #[test]
    fn test_edge_case_input_generation() {
        let mut generator = InputGenerator::new(vec![
            EdgeCaseType::MaxI128,
            EdgeCaseType::MinI128,
            EdgeCaseType::ZeroValue,
        ]);

        let inputs = generator.generate_boundary_inputs().unwrap();
        
        assert!(!inputs.is_empty());
        
        // Verify that boundary inputs contain expected edge cases
        let has_max_i128 = inputs.iter().any(|input| {
            input.metadata.edge_case_type == Some(EdgeCaseType::MaxI128)
        });
        let has_min_i128 = inputs.iter().any(|input| {
            input.metadata.edge_case_type == Some(EdgeCaseType::MinI128)
        });
        let has_zero = inputs.iter().any(|input| {
            input.metadata.edge_case_type == Some(EdgeCaseType::ZeroValue)
        });

        assert!(has_max_i128);
        assert!(has_min_i128);
        assert!(has_zero);
    }

    #[test]
    fn test_cross_contract_input_generation() {
        let mut generator = InputGenerator::new(vec![]);
        let inputs = generator.generate_cross_contract_inputs(5).unwrap();
        
        assert_eq!(inputs.len(), 5);
        
        for input in &inputs {
            assert!(input.function_name.contains("call") || 
                   input.function_name.contains("external") ||
                   input.function_name.contains("delegate"));
            assert_eq!(input.metadata.edge_case_type, 
                      Some(EdgeCaseType::Custom("cross_contract".to_string())));
        }
    }
}
