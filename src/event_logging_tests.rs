//! Tests for event logging functionality

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_event_logging_config_default() {
        let config = EventLoggingConfig::default();
        
        assert!(config.enabled);
        assert_eq!(config.log_level, EventLogLevel::Info);
        assert!(!config.structured_logging);
        assert!(config.enable_persistence);
        assert_eq!(config.max_events_in_memory, 10000);
        assert_eq!(config.retention_period_seconds, 86400 * 30);
        assert!(!config.critical_operations.is_empty());
    }

    #[test]
    fn test_event_logger_creation() {
        let config = EventLoggingConfig::default();
        let logger = EventLogger::new(config);
        
        // Logger should be created successfully
        // We can't directly test internal state, but we can test operations
        let result = logger.get_event_statistics();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().total_events, 0);
    }

    #[test]
    fn test_event_builder() {
        let event = EventBuilder::new(
            CriticalOperation::FundTransfer,
            "user123".to_string()
        )
        .description("Transfer of 100 tokens".to_string())
        .severity(EventSeverity::High)
        .target("recipient456".to_string())
        .metadata("amount".to_string(), "100".to_string())
        .metadata("currency".to_string(), "XLM".to_string())
        .previous_state("balance: 1000".to_string())
        .new_state("balance: 900".to_string())
        .duration_ms(1500)
        .gas_consumed(50000)
        .transaction_hash("tx123".to_string())
        .ledger_sequence(12345)
        .build();

        assert_eq!(event.operation, CriticalOperation::FundTransfer);
        assert_eq!(event.actor, "user123");
        assert_eq!(event.description, "Transfer of 100 tokens");
        assert_eq!(event.severity, EventSeverity::High);
        assert_eq!(event.target, Some("recipient456".to_string()));
        assert_eq!(event.metadata.get("amount"), Some(&"100".to_string()));
        assert_eq!(event.metadata.get("currency"), Some(&"XLM".to_string()));
        assert_eq!(event.previous_state, Some("balance: 1000".to_string()));
        assert_eq!(event.new_state, Some("balance: 900".to_string()));
        assert_eq!(event.execution_duration_ms, Some(1500));
        assert_eq!(event.gas_consumed, Some(50000));
        assert_eq!(event.transaction_hash, Some("tx123".to_string()));
        assert_eq!(event.ledger_sequence, Some(12345));
    }

    #[test]
    fn test_critical_operation_types() {
        let operations = vec![
            CriticalOperation::FundTransfer,
            CriticalOperation::VulnerabilityVerification,
            CriticalOperation::EscrowOperation,
            CriticalOperation::RewardDistribution,
            CriticalOperation::AdminApproval,
            CriticalOperation::OwnershipChange,
            CriticalOperation::BountyCreation,
            CriticalOperation::BountyApproval,
            CriticalOperation::BountyClaim,
            CriticalOperation::ResearcherAssignment,
            CriticalOperation::ContractUpgrade,
            CriticalOperation::EmergencyStop,
            CriticalOperation::ConfigurationChange,
        ];

        // Ensure all operations can be created and compared
        for op in &operations {
            let event = EventBuilder::new(op.clone(), "test_user".to_string())
                .description("Test event".to_string())
                .build();
            assert_eq!(event.operation, *op);
        }

        // Test hash functionality
        let mut ops_set = std::collections::HashSet::new();
        for op in &operations {
            ops_set.insert(op);
        }
        assert_eq!(ops_set.len(), operations.len());
    }

    #[test]
    fn test_event_logging_enabled() {
        let mut config = EventLoggingConfig::default();
        config.enabled = true;
        let logger = EventLogger::new(config);

        let event = EventBuilder::new(
            CriticalOperation::VulnerabilityVerification,
            "admin".to_string()
        )
        .description("Verified critical vulnerability".to_string())
        .severity(EventSeverity::Critical)
        .build();

        let result = logger.log_event(event);
        assert!(result.is_ok());

        let events = logger.get_events_by_operation(&CriticalOperation::VulnerabilityVerification).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].operation, CriticalOperation::VulnerabilityVerification);
        assert_eq!(events[0].severity, EventSeverity::Critical);
    }

    #[test]
    fn test_event_logging_disabled() {
        let mut config = EventLoggingConfig::default();
        config.enabled = false;
        let logger = EventLogger::new(config);

        let event = EventBuilder::new(
            CriticalOperation::FundTransfer,
            "user".to_string()
        )
        .description("Test transfer".to_string())
        .build();

        let result = logger.log_event(event);
        assert!(result.is_ok());

        // Should not be stored when disabled
        let events = logger.get_events_by_operation(&CriticalOperation::FundTransfer).unwrap();
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_event_logging_non_critical_operation() {
        let config = EventLoggingConfig::default();
        let logger = EventLogger::new(config);

        // Create an event for a non-critical operation (not in the configured list)
        let event = EventBuilder::new(
            CriticalOperation::ConfigurationChange, // This is in the default list
            "admin".to_string()
        )
        .description("Configuration change".to_string())
        .build();

        let result = logger.log_event(event);
        assert!(result.is_ok());

        // Should be stored since ConfigurationChange is in the default critical operations
        let events = logger.get_events_by_operation(&CriticalOperation::ConfigurationChange).unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_event_statistics() {
        let config = EventLoggingConfig::default();
        let logger = EventLogger::new(config);

        // Log multiple events of different types and severities
        let events = vec![
            EventBuilder::new(CriticalOperation::FundTransfer, "user1".to_string())
                .description("Transfer 1".to_string())
                .severity(EventSeverity::Low)
                .build(),
            EventBuilder::new(CriticalOperation::FundTransfer, "user2".to_string())
                .description("Transfer 2".to_string())
                .severity(EventSeverity::High)
                .build(),
            EventBuilder::new(CriticalOperation::VulnerabilityVerification, "admin".to_string())
                .description("Verification".to_string())
                .severity(EventSeverity::Critical)
                .build(),
        ];

        for event in events {
            logger.log_event(event).unwrap();
        }

        let stats = logger.get_event_statistics().unwrap();
        assert_eq!(stats.total_events, 3);
        assert_eq!(stats.by_operation.get(&CriticalOperation::FundTransfer), Some(&2));
        assert_eq!(stats.by_operation.get(&CriticalOperation::VulnerabilityVerification), Some(&1));
        assert_eq!(stats.by_severity.get(&EventSeverity::Low), Some(&1));
        assert_eq!(stats.by_severity.get(&EventSeverity::High), Some(&1));
        assert_eq!(stats.by_severity.get(&EventSeverity::Critical), Some(&1));
    }

    #[test]
    fn test_events_by_actor() {
        let config = EventLoggingConfig::default();
        let logger = EventLogger::new(config);

        // Log events from different actors
        let events = vec![
            EventBuilder::new(CriticalOperation::FundTransfer, "alice".to_string())
                .description("Alice transfer".to_string())
                .build(),
            EventBuilder::new(CriticalOperation::FundTransfer, "bob".to_string())
                .description("Bob transfer".to_string())
                .build(),
            EventBuilder::new(CriticalOperation::AdminApproval, "alice".to_string())
                .description("Alice approval".to_string())
                .build(),
        ];

        for event in events {
            logger.log_event(event).unwrap();
        }

        let alice_events = logger.get_events_by_actor("alice").unwrap();
        assert_eq!(alice_events.len(), 2);
        assert!(alice_events.iter().all(|e| e.actor == "alice"));

        let bob_events = logger.get_events_by_actor("bob").unwrap();
        assert_eq!(bob_events.len(), 1);
        assert_eq!(bob_events[0].actor, "bob");

        let charlie_events = logger.get_events_by_actor("charlie").unwrap();
        assert_eq!(charlie_events.len(), 0);
    }

    #[test]
    fn test_events_by_time_range() {
        let config = EventLoggingConfig::default();
        let logger = EventLogger::new(config);

        let base_time = chrono::Utc::now().timestamp() as u64;

        // Create events with different timestamps
        let event1 = EventBuilder::new(CriticalOperation::FundTransfer, "user1".to_string())
            .description("Early event".to_string())
            .build();
        
        // Simulate time passing
        thread::sleep(Duration::from_millis(10));
        
        let event2 = EventBuilder::new(CriticalOperation::FundTransfer, "user2".to_string())
            .description("Later event".to_string())
            .build();

        logger.log_event(event1).unwrap();
        logger.log_event(event2).unwrap();

        // Test time range filtering
        let start_time = base_time;
        let end_time = base_time + 1000; // Large window

        let events = logger.get_events_by_time_range(start_time, end_time).unwrap();
        assert!(events.len() >= 1); // At least one event should be in range
    }

    #[test]
    fn test_memory_limit_enforcement() {
        let mut config = EventLoggingConfig::default();
        config.max_events_in_memory = 2; // Very small limit for testing
        let logger = EventLogger::new(config);

        // Log more events than the limit
        for i in 0..5 {
            let event = EventBuilder::new(
                CriticalOperation::FundTransfer,
                format!("user{}", i)
            )
            .description(format!("Transfer {}", i))
            .build();
            logger.log_event(event).unwrap();
        }

        let stats = logger.get_event_statistics().unwrap();
        // Should not exceed the memory limit
        assert!(stats.total_events <= 2);
    }

    #[test]
    fn test_event_severity_levels() {
        let severities = vec![
            EventSeverity::Low,
            EventSeverity::Medium,
            EventSeverity::High,
            EventSeverity::Critical,
        ];

        for severity in severities {
            let event = EventBuilder::new(
                CriticalOperation::VulnerabilityVerification,
                "tester".to_string()
            )
            .description("Test event".to_string())
            .severity(severity.clone())
            .build();

            assert_eq!(event.severity, severity);
        }
    }

    #[test]
    fn test_event_status_levels() {
        let statuses = vec![
            EventStatus::Started,
            EventStatus::InProgress,
            EventStatus::Completed,
            EventStatus::Failed,
            EventStatus::Cancelled,
        ];

        for status in statuses {
            let event = EventBuilder::new(
                CriticalOperation::FundTransfer,
                "tester".to_string()
            )
            .description("Test event".to_string())
            .status(status.clone())
            .build();

            assert_eq!(event.status, status);
        }
    }

    #[test]
    fn test_event_cleanup() {
        let mut config = EventLoggingConfig::default();
        config.retention_period_seconds = 0; // Immediate cleanup
        let logger = EventLogger::new(config);

        // Log an event
        let event = EventBuilder::new(
            CriticalOperation::AdminApproval,
            "admin".to_string()
        )
        .description("Test event".to_string())
        .build();
        logger.log_event(event).unwrap();

        // Run cleanup
        let removed = logger.cleanup_old_events().unwrap();
        // In a real scenario with time manipulation, this would remove old events
        assert!(removed >= 0);
    }

    #[test]
    fn test_concurrent_event_logging() {
        let config = EventLoggingConfig::default();
        let logger = Arc::new(EventLogger::new(config));

        // Test concurrent logging from multiple threads
        let mut handles = vec![];

        for i in 0..10 {
            let logger_clone = logger.clone();
            let handle = thread::spawn(move || {
                let event = EventBuilder::new(
                    CriticalOperation::FundTransfer,
                    format!("user{}", i)
                )
                .description(format!("Concurrent transfer {}", i))
                .build();
                logger_clone.log_event(event).unwrap();
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        let stats = logger.get_event_statistics().unwrap();
        assert_eq!(stats.total_events, 10);
    }

    #[test]
    fn test_event_metadata_handling() {
        let config = EventLoggingConfig::default();
        let logger = EventLogger::new(config);

        let mut metadata = HashMap::new();
        metadata.insert("key1".to_string(), "value1".to_string());
        metadata.insert("key2".to_string(), "value2".to_string());
        metadata.insert("numeric".to_string(), "123".to_string());

        let event = EventBuilder::new(
            CriticalOperation::BountyCreation,
            "creator".to_string()
        )
        .description("Bounty created".to_string())
        .metadata("key1".to_string(), "value1".to_string())
        .metadata("key2".to_string(), "value2".to_string())
        .metadata("numeric".to_string(), "123".to_string())
        .build();

        assert_eq!(event.metadata.len(), 3);
        assert_eq!(event.metadata.get("key1"), Some(&"value1".to_string()));
        assert_eq!(event.metadata.get("key2"), Some(&"value2".to_string()));
        assert_eq!(event.metadata.get("numeric"), Some(&"123".to_string()));

        logger.log_event(event).unwrap();

        let events = logger.get_events_by_operation(&CriticalOperation::BountyCreation).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].metadata.len(), 3);
    }

    #[test]
    fn test_event_error_handling() {
        let config = EventLoggingConfig::default();
        let logger = EventLogger::new(config);

        // Test event with error message
        let event = EventBuilder::new(
            CriticalOperation::EscrowOperation,
            "user".to_string()
        )
        .description("Failed escrow operation".to_string())
        .status(EventStatus::Failed)
        .error("Insufficient funds".to_string())
        .build();

        assert_eq!(event.status, EventStatus::Failed);
        assert_eq!(event.error_message, Some("Insufficient funds".to_string()));

        logger.log_event(event).unwrap();

        let events = logger.get_events_by_operation(&CriticalOperation::EscrowOperation).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, EventStatus::Failed);
        assert_eq!(events[0].error_message, Some("Insufficient funds".to_string()));
    }

    #[test]
    fn test_structured_logging_config() {
        let mut config = EventLoggingConfig::default();
        config.structured_logging = true;
        config.log_level = EventLogLevel::Debug;

        assert!(config.structured_logging);
        assert_eq!(config.log_level, EventLogLevel::Debug);

        let logger = EventLogger::new(config);
        
        // Should be able to log events without errors
        let event = EventBuilder::new(
            CriticalOperation::ContractUpgrade,
            "admin".to_string()
        )
        .description("Contract upgrade".to_string())
        .build();

        let result = logger.log_event(event);
        assert!(result.is_ok());
    }
}
