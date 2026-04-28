//! Comprehensive test suite for the Transaction Processing Engine

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction_engine::*;
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};

    /// Test transaction creation and basic properties
    #[tokio::test]
    async fn test_transaction_creation() {
        let transaction = Transaction::new(
            TransactionType::Payment,
            b"test data".to_vec(),
            "test_user".to_string(),
            "testnet".to_string(),
        );

        assert_eq!(transaction.transaction_type, TransactionType::Payment);
        assert_eq!(transaction.state, TransactionState::Queued);
        assert_eq!(transaction.metadata.submitter, "test_user");
        assert_eq!(transaction.metadata.network, "testnet");
        assert_eq!(transaction.priority, TransactionPriority::Normal);
        assert!(!transaction.id.is_nil());
    }

    /// Test transaction state transitions
    #[tokio::test]
    async fn test_transaction_state_transitions() {
        let mut transaction = Transaction::new(
            TransactionType::Payment,
            b"test".to_vec(),
            "user".to_string(),
            "testnet".to_string(),
        );

        // Initial state
        assert_eq!(transaction.state, TransactionState::Queued);
        assert!(!transaction.is_terminal());

        // Mark as processing
        transaction.state = TransactionState::Processing;
        assert!(!transaction.is_terminal());

        // Complete transaction
        transaction.state = TransactionState::Completed;
        assert!(transaction.is_terminal());

        // Test retry capability
        transaction.state = TransactionState::FailedRetryable;
        transaction.retry_info.attempt = 1;
        transaction.retry_info.max_attempts = 3;
        assert!(transaction.can_retry());

        transaction.retry_info.attempt = 3;
        assert!(!transaction.can_retry());
    }

    /// Test queue basic operations
    #[tokio::test]
    async fn test_queue_basic_operations() {
        let queue_config = QueueConfig {
            max_size: 100,
            concurrent_workers: 2,
            poll_interval_ms: 50,
            batch_size: 5,
            enable_priority: true,
            timeout_ms: Some(5000),
        };

        let (queue, _) = TransactionQueue::new(queue_config);

        // Create test transaction
        let transaction = Transaction::new(
            TransactionType::Payment,
            b"test data".to_vec(),
            "user1".to_string(),
            "testnet".to_string(),
        );

        // Enqueue transaction
        queue.enqueue(transaction.clone()).await.unwrap();

        // Check queue statistics
        let stats = queue.get_stats().await;
        assert_eq!(stats.total_queued, 1);

        // Get transaction by ID
        let retrieved = queue.get_transaction(&transaction.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, transaction.id);

        // Dequeue transaction
        let dequeued = queue.dequeue("worker1".to_string()).await.unwrap();
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().id, transaction.id);

        // Queue should be empty
        let stats = queue.get_stats().await;
        assert_eq!(stats.total_queued, 0);
        assert_eq!(stats.processing, 1);
    }

    /// Test priority queue functionality
    #[tokio::test]
    async fn test_priority_queue() {
        let queue_config = QueueConfig {
            max_size: 100,
            concurrent_workers: 1,
            poll_interval_ms: 10,
            batch_size: 1,
            enable_priority: true,
            timeout_ms: Some(1000),
        };

        let (queue, _) = TransactionQueue::new(queue_config);

        // Create transactions with different priorities
        let low_priority_tx = Transaction::new(
            TransactionType::Payment,
            b"low".to_vec(),
            "user".to_string(),
            "testnet".to_string(),
        );
        let mut high_priority_tx = Transaction::new(
            TransactionType::Payment,
            b"high".to_vec(),
            "user".to_string(),
            "testnet".to_string(),
        );
        high_priority_tx.priority = TransactionPriority::High;

        // Enqueue in order: low first, then high
        queue.enqueue(low_priority_tx).await.unwrap();
        queue.enqueue(high_priority_tx.clone()).await.unwrap();

        // High priority should be dequeued first
        let dequeued = queue.dequeue("worker1".to_string()).await.unwrap();
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().id, high_priority_tx.id);
    }

    /// Test transaction completion and failure
    #[tokio::test]
    async fn test_transaction_completion() {
        let queue_config = QueueConfig::default();
        let (queue, _) = TransactionQueue::new(queue_config);

        let transaction = Transaction::new(
            TransactionType::Payment,
            b"test".to_vec(),
            "user".to_string(),
            "testnet".to_string(),
        );

        let tx_id = transaction.id;
        queue.enqueue(transaction).await.unwrap();

        // Dequeue for processing
        let _ = queue.dequeue("worker1".to_string()).await.unwrap();

        // Complete transaction
        let result = TransactionResult {
            transaction_id: tx_id,
            success: true,
            data: Some(b"success".to_vec()),
            error: None,
            duration: Duration::from_millis(100),
            gas_used: Some(1000),
            transaction_hash: Some("hash123".to_string()),
            block_number: Some(12345),
        };

        queue.complete_transaction(tx_id, result).await.unwrap();

        // Check statistics
        let stats = queue.get_stats().await;
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.processing, 0);
    }

    /// Test transaction failure and retry
    #[tokio::test]
    async fn test_transaction_failure_and_retry() {
        let queue_config = QueueConfig::default();
        let (queue, _) = TransactionQueue::new(queue_config);

        let transaction = Transaction::new(
            TransactionType::Payment,
            b"test".to_vec(),
            "user".to_string(),
            "testnet".to_string(),
        );

        let tx_id = transaction.id;
        queue.enqueue(transaction).await.unwrap();

        // Dequeue for processing
        let _ = queue.dequeue("worker1".to_string()).await.unwrap();

        // Mark as failed (retryable)
        queue.fail_transaction(tx_id, "Network error".to_string(), true).await.unwrap();

        // Check statistics
        let stats = queue.get_stats().await;
        assert_eq!(stats.failed_retryable, 1);
        assert_eq!(stats.processing, 0);

        // Get retryable transactions
        let retryable = queue.get_retryable_transactions().await.unwrap();
        assert_eq!(retryable.len(), 1);
        assert_eq!(retryable[0].id, tx_id);

        // Requeue for retry
        queue.requeue_transaction(&tx_id).await.unwrap();

        // Should be back in queue
        let stats = queue.get_stats().await;
        assert_eq!(stats.total_queued, 1);
        assert_eq!(stats.failed_retryable, 0);
    }

    /// Test transaction cancellation
    #[tokio::test]
    async fn test_transaction_cancellation() {
        let queue_config = QueueConfig::default();
        let (queue, _) = TransactionQueue::new(queue_config);

        let transaction = Transaction::new(
            TransactionType::Payment,
            b"test".to_vec(),
            "user".to_string(),
            "testnet".to_string(),
        );

        let tx_id = transaction.id;
        queue.enqueue(transaction).await.unwrap();

        // Cancel transaction
        let cancelled = queue.cancel_transaction(tx_id).await.unwrap();
        assert!(cancelled);

        // Verify it's no longer in queue
        let stats = queue.get_stats().await;
        assert_eq!(stats.total_queued, 0);

        // Check transaction state
        let retrieved = queue.get_transaction(&tx_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().state, TransactionState::Cancelled);
    }

    /// Test transaction filtering
    #[tokio::test]
    async fn test_transaction_filtering() {
        let queue_config = QueueConfig::default();
        let (queue, _) = TransactionQueue::new(queue_config);

        // Create transactions with different properties
        let payment_tx = Transaction::new(
            TransactionType::Payment,
            b"payment".to_vec(),
            "alice".to_string(),
            "testnet".to_string(),
        );

        let multisig_tx = Transaction::new(
            TransactionType::MultiSignature,
            b"multisig".to_vec(),
            "bob".to_string(),
            "mainnet".to_string(),
        );

        // Enqueue transactions
        queue.enqueue(payment_tx).await.unwrap();
        queue.enqueue(multisig_tx).await.unwrap();

        // Filter by transaction type
        let filter = TransactionFilter {
            transaction_type: Some(TransactionType::Payment),
            ..Default::default()
        };

        let results = queue.get_transactions(&filter).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].transaction_type, TransactionType::Payment);

        // Filter by submitter
        let filter = TransactionFilter {
            submitter: Some("bob".to_string()),
            ..Default::default()
        };

        let results = queue.get_transactions(&filter).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.submitter, "bob");
    }

    /// Test retry strategy calculations
    #[tokio::test]
    async fn test_retry_strategies() {
        // Test exponential backoff
        let strategy = RetryStrategy::ExponentialBackoff {
            base_delay_ms: 1000,
            max_delay_ms: 10000,
            multiplier: 2.0,
            jitter: false,
        };

        assert_eq!(strategy.next_delay_ms(1), 1000);
        assert_eq!(strategy.next_delay_ms(2), 2000);
        assert_eq!(strategy.next_delay_ms(3), 4000);
        assert_eq!(strategy.next_delay_ms(4), 8000);
        assert_eq!(strategy.next_delay_ms(5), 10000); // Capped at max

        // Test fixed delay
        let strategy = RetryStrategy::Fixed { delay_ms: 5000 };
        assert_eq!(strategy.next_delay_ms(1), 5000);
        assert_eq!(strategy.next_delay_ms(10), 5000);

        // Test linear backoff
        let strategy = RetryStrategy::Linear {
            base_delay_ms: 1000,
            increment_ms: 500,
        };
        assert_eq!(strategy.next_delay_ms(1), 1000);
        assert_eq!(strategy.next_delay_ms(2), 1500);
        assert_eq!(strategy.next_delay_ms(3), 2000);
    }

    /// Test transaction validation
    #[tokio::test]
    async fn test_transaction_validation() {
        let validation_config = ValidationConfig {
            strict_mode: false,
            max_data_size: 1024,
            max_age_minutes: 60,
            required_fields: {
                let mut fields = std::collections::HashMap::new();
                fields.insert(TransactionType::Payment, vec!["amount".to_string(), "recipient".to_string()]);
                fields
            },
            blacklisted_submitters: vec![],
            rate_limit_per_minute: 100,
            enable_security_scan: false,
        };

        let mut validator = TransactionValidator::new(validation_config);

        // Test valid transaction
        let valid_tx = Transaction::new(
            TransactionType::Payment,
            serde_json::json!({
                "amount": "100.50",
                "recipient": "GABC123456789012345678901234567890123456789012345678901234567890"
            }).to_string().into_bytes(),
            "user".to_string(),
            "testnet".to_string(),
        );

        let result = validator.validate_transaction(&valid_tx).await;
        assert!(result.is_valid);
        assert!(result.errors.is_empty());

        // Test invalid transaction (missing required field)
        let invalid_tx = Transaction::new(
            TransactionType::Payment,
            serde_json::json!({
                "amount": "100.50"
                // Missing recipient
            }).to_string().into_bytes(),
            "user".to_string(),
            "testnet".to_string(),
        );

        let result = validator.validate_transaction(&invalid_tx).await;
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    /// Test state manager persistence
    #[tokio::test]
    async fn test_state_manager() {
        let state_config = StateConfig {
            enable_persistence: false, // Disable for testing
            persistence_file: None,
            auto_save_interval_seconds: 1,
            max_memory_transactions: 100,
            enable_compression: false,
        };

        let state_manager = StateManager::new(state_config);
        state_manager.initialize().await.unwrap();

        // Create and add transaction
        let transaction = Transaction::new(
            TransactionType::Payment,
            b"test".to_vec(),
            "user".to_string(),
            "testnet".to_string(),
        );

        state_manager.update_transaction(transaction).await.unwrap();

        // Get transaction statistics
        let stats = state_manager.get_transaction_stats().await;
        assert_eq!(stats.total, 1);
        assert_eq!(stats.payments, 1);
    }

    /// Test monitoring system
    #[tokio::test]
    async fn test_monitoring_system() {
        let monitoring_config = MonitoringConfig {
            enable_real_time: true,
            metrics_interval_seconds: 1,
            alert_thresholds: AlertThresholds::default(),
            dashboard_interval_seconds: 1,
            enable_performance_tracking: true,
            max_history_hours: 1,
        };

        let (monitor, _) = TransactionMonitor::new(monitoring_config);
        monitor.start().await.unwrap();

        // Create manual alert
        let alert_details = std::collections::HashMap::from([
            ("source".to_string(), "test".to_string()),
        ]);

        monitor.create_alert(
            AlertLevel::Warning,
            "Test alert".to_string(),
            alert_details,
        ).await.unwrap();

        // Get active alerts
        let alerts = monitor.get_active_alerts().await.unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].level, AlertLevel::Warning);

        // Resolve alert
        let resolved = monitor.resolve_alert(alerts[0].id).await.unwrap();
        assert!(resolved);

        monitor.stop().await.unwrap();
    }

    /// Test retry manager
    #[tokio::test]
    async fn test_retry_manager() {
        let queue_config = QueueConfig::default();
        let (queue, _) = TransactionQueue::new(queue_config);
        let queue = Arc::new(queue);

        let retry_config = RetryManagerConfig {
            check_interval_ms: 100,
            max_concurrent_retries: 2,
            max_retry_queue_size: 100,
            enable_jitter: false,
            max_retry_age_hours: 1,
        };

        let mut retry_manager = RetryManager::new(queue.clone(), retry_config);
        let _notifications = retry_manager.start().await.unwrap();

        // Create failed transaction
        let transaction = Transaction::new(
            TransactionType::Payment,
            b"test".to_vec(),
            "user".to_string(),
            "testnet".to_string(),
        );

        let tx_id = transaction.id;
        queue.enqueue(transaction).await.unwrap();

        // Mark as failed and retryable
        queue.fail_transaction(tx_id, "Network error".to_string(), true).await.unwrap();

        // Manual retry
        retry_manager.retry_transaction(&tx_id).await.unwrap();

        // Check retry statistics
        let stats = retry_manager.get_stats().await;
        assert_eq!(stats.total_retries_attempted, 1);

        retry_manager.stop().await.unwrap();
    }

    /// Test transaction worker
    #[tokio::test]
    async fn test_transaction_worker() {
        let queue_config = QueueConfig::default();
        let (queue, _) = TransactionQueue::new(queue_config);
        let queue = Arc::new(queue);

        let processor_config = ProcessorConfig {
            worker_id: "test-worker".to_string(),
            max_concurrent: 1,
            timeout_ms: 5000,
            health_check_interval_ms: 1000,
            enable_metrics: true,
        };

        let processor = Arc::new(DefaultTransactionProcessor::new(processor_config.clone()));
        let mut worker = TransactionWorker::new(processor, queue.clone(), processor_config);
        let notifications = worker.start().await.unwrap();

        // Create and enqueue transaction
        let transaction = Transaction::new(
            TransactionType::Payment,
            b"test".to_vec(),
            "user".to_string(),
            "testnet".to_string(),
        );

        let tx_id = transaction.id;
        queue.enqueue(transaction).await.unwrap();

        // Wait for processing
        let mut completed = false;
        let timeout = Duration::from_secs(5);
        let start_time = std::time::Instant::now();

        while !completed && start_time.elapsed() < timeout {
            if let Some(notification) = notifications.recv().await {
                match notification {
                    WorkerMessage::TransactionCompleted { transaction_id, result } => {
                        if transaction_id == tx_id {
                            assert!(result.success);
                            completed = true;
                        }
                    }
                    WorkerMessage::TransactionFailed { transaction_id, .. } => {
                        if transaction_id == tx_id {
                            panic!("Transaction should have succeeded");
                        }
                    }
                    _ => {}
                }
            }
        }

        assert!(completed, "Transaction should have completed");

        worker.stop().await.unwrap();
    }

    /// Test queue cleanup
    #[tokio::test]
    async fn test_queue_cleanup() {
        let queue_config = QueueConfig::default();
        let (queue, _) = TransactionQueue::new(queue_config);

        // Create old transaction (simulate by manipulating timestamp)
        let mut transaction = Transaction::new(
            TransactionType::Payment,
            b"old".to_vec(),
            "user".to_string(),
            "testnet".to_string(),
        );

        // Manually set old timestamp
        transaction.metadata.created_at = chrono::Utc::now() - chrono::Duration::hours(25);
        transaction.metadata.updated_at = transaction.metadata.created_at;

        // Complete transaction to move it to completed list
        queue.enqueue(transaction.clone()).await.unwrap();
        let _ = queue.dequeue("worker".to_string()).await.unwrap();
        queue.complete_transaction(transaction.id, TransactionResult::default()).await.unwrap();

        // Cleanup old transactions
        let removed = queue.cleanup_old_transactions(chrono::Duration::hours(24)).await.unwrap();
        assert_eq!(removed, 1);
    }

    /// Test comprehensive transaction flow
    #[tokio::test]
    async fn test_comprehensive_transaction_flow() {
        // Initialize all components
        let queue_config = QueueConfig::default();
        let (queue, _) = TransactionQueue::new(queue_config);
        let queue = Arc::new(queue);

        let state_config = StateConfig {
            enable_persistence: false,
            ..Default::default()
        };
        let state_manager = Arc::new(StateManager::new(state_config));
        state_manager.initialize().await.unwrap();

        let monitoring_config = MonitoringConfig {
            enable_real_time: true,
            metrics_interval_seconds: 1,
            ..Default::default()
        };
        let (monitor, _) = TransactionMonitor::new(monitoring_config);
        let monitor = Arc::new(monitor);
        monitor.start().await.unwrap();

        // Create and process multiple transactions
        let transaction_types = vec![
            TransactionType::Payment,
            TransactionType::MultiSignature,
            TransactionType::ContractDeployment,
            TransactionType::SecurityScan,
        ];

        for (i, tx_type) in transaction_types.iter().enumerate() {
            let transaction = Transaction::new(
                tx_type.clone(),
                format!("test data {}", i).into_bytes(),
                format!("user{}", i).to_string(),
                "testnet".to_string(),
            );

            let tx_id = transaction.id;
            queue.enqueue(transaction).await.unwrap();

            // Process transaction
            let mut processor_manager = ProcessorManager::new(queue.clone(), queue_config);
            processor_manager.start().await.unwrap();

            // Wait a bit for processing
            sleep(Duration::from_millis(100)).await;

            // Verify completion
            let stats = queue.get_stats().await;
            assert!(stats.completed > 0);

            processor_manager.stop().await.unwrap();
        }

        // Verify final state
        let stats = queue.get_stats().await;
        assert!(stats.completed >= transaction_types.len() as usize);

        let transaction_stats = state_manager.get_transaction_stats().await;
        assert_eq!(transaction_stats.total, transaction_types.len());

        monitor.stop().await.unwrap();
        state_manager.stop().await.unwrap();
    }

    /// Test error handling and recovery
    #[tokio::test]
    async fn test_error_handling_and_recovery() {
        let queue_config = QueueConfig::default();
        let (queue, _) = TransactionQueue::new(queue_config);

        // Test queue full error
        let small_queue_config = QueueConfig {
            max_size: 1,
            ..Default::default()
        };
        let (small_queue, _) = TransactionQueue::new(small_queue_config);

        // Fill queue
        let tx1 = Transaction::new(
            TransactionType::Payment,
            b"test1".to_vec(),
            "user".to_string(),
            "testnet".to_string(),
        );
        small_queue.enqueue(tx1).await.unwrap();

        // Try to add second transaction (should fail)
        let tx2 = Transaction::new(
            TransactionType::Payment,
            b"test2".to_vec(),
            "user".to_string(),
            "testnet".to_string(),
        );
        let result = small_queue.enqueue(tx2).await;
        assert!(result.is_err());

        // Test transaction not found error
        let fake_id = uuid::Uuid::new_v4();
        let result = queue.get_transaction(&fake_id).await.unwrap();
        assert!(result.is_none());

        // Test retry non-existent transaction
        let result = queue.requeue_transaction(&fake_id).await;
        assert!(result.is_err());
    }
}

/// Integration tests
#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::transaction_engine::*;

    /// Test full integration with all components
    #[tokio::test]
    async fn test_full_integration() {
        // Initialize all components
        let queue_config = QueueConfig::default();
        let (queue, mut queue_notifications) = TransactionQueue::new(queue_config);
        let queue = Arc::new(queue);

        let state_config = StateConfig {
            enable_persistence: false,
            ..Default::default()
        };
        let state_manager = Arc::new(StateManager::new(state_config));
        state_manager.initialize().await.unwrap();

        let monitoring_config = MonitoringConfig {
            enable_real_time: true,
            metrics_interval_seconds: 1,
            ..Default::default()
        };
        let (monitor, mut monitoring_notifications) = TransactionMonitor::new(monitoring_config);
        let monitor = Arc::new(monitor);
        monitor.start().await.unwrap();

        let retry_config = RetryManagerConfig::default();
        let mut retry_manager = RetryManager::new(queue.clone(), retry_config);
        let mut retry_notifications = retry_manager.start().await.unwrap();

        // Create processor manager
        let mut processor_manager = ProcessorManager::new(queue.clone(), queue_config);
        let worker_notifications = processor_manager.start().await.unwrap();

        // Submit transaction
        let transaction = Transaction::new(
            TransactionType::Payment,
            serde_json::json!({
                "amount": "100.00",
                "recipient": "GTEST123456789012345678901234567890123456789012345678901234567890"
            }).to_string().into_bytes(),
            "integration_user".to_string(),
            "testnet".to_string(),
        );

        let tx_id = transaction.id;
        queue.enqueue(transaction).await.unwrap();

        // Monitor processing
        let mut completed = false;
        let timeout = Duration::from_secs(10);
        let start_time = std::time::Instant::now();

        while !completed && start_time.elapsed() < timeout {
            tokio::select! {
                notification = queue_notifications.recv() => {
                    if let Some(notification) = notification {
                        match notification {
                            QueueNotification::TransactionCompleted { transaction_id, .. } => {
                                if transaction_id == tx_id {
                                    completed = true;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ = sleep(Duration::from_millis(50)) => {
                    if start_time.elapsed() >= timeout {
                        break;
                    }
                }
            }
        }

        assert!(completed, "Transaction should have completed");

        // Cleanup
        processor_manager.stop().await.unwrap();
        retry_manager.stop().await.unwrap();
        monitor.stop().await.unwrap();
        state_manager.stop().await.unwrap();
    }
}
