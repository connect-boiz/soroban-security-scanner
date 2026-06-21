//! Transaction Processing Engine Example
//! 
//! This example demonstrates how to use the Transaction Processing Engine
//! for handling various types of transactions with queue management,
//! retry logic, failure handling, and state tracking.

use crate::transaction_engine::*;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

/// Example usage of the Transaction Processing Engine
pub async fn run_transaction_engine_example() -> Result<()> {
    println!("🚀 Starting Transaction Processing Engine Example\n");

    // 1. Initialize the transaction queue
    let queue_config = QueueConfig {
        max_size: 1000,
        concurrent_workers: 4,
        poll_interval_ms: 100,
        batch_size: 10,
        enable_priority: true,
        timeout_ms: Some(30000),
    };

    let (queue, mut queue_notifications) = TransactionQueue::new(queue_config);
    let queue = Arc::new(queue);

    // 2. Initialize the state manager
    let state_config = StateConfig::default();
    let state_manager = Arc::new(StateManager::new(state_config));
    state_manager.initialize().await?;

    // 3. Initialize the monitoring system
    let monitoring_config = MonitoringConfig::default();
    let (monitor, mut monitoring_notifications) = TransactionMonitor::new(monitoring_config);
    let monitor = Arc::new(monitor);
    monitor.start().await?;

    // 4. Initialize the retry manager
    let retry_config = RetryManagerConfig::default();
    let mut retry_manager = RetryManager::new(queue.clone(), retry_config);
    let mut retry_notifications = retry_manager.start().await?;

    // 5. Initialize the processor manager
    let mut processor_manager = ProcessorManager::new(queue.clone(), queue_config);
    let worker_notifications = processor_manager.start().await?;

    // 6. Initialize the validator
    let validation_config = ValidationConfig::default();
    let mut validator = TransactionValidator::new(validation_config);

    println!("✅ Transaction Processing Engine initialized successfully!\n");

    // 7. Create and submit example transactions
    println!("📝 Creating example transactions...");
    
    // Payment transaction
    let payment_tx = Transaction::new(
        TransactionType::Payment,
        serde_json::json!({
            "amount": "100.50",
            "recipient": "GABC123456789012345678901234567890123456789012345678901234567890",
            "currency": "XLM"
        }).to_string().into_bytes(),
        "user1".to_string(),
        "testnet".to_string(),
    );
    payment_tx.priority = TransactionPriority::High;
    queue.enqueue(payment_tx).await?;
    println!("  ✅ Payment transaction queued");

    // Multi-signature transaction
    let multisig_tx = Transaction::new(
        TransactionType::MultiSignature,
        serde_json::json!({
            "threshold": 2,
            "signers": [
                {
                    "public_key": "GDEF123456789012345678901234567890123456789012345678901234567890",
                    "weight": 1
                },
                {
                    "public_key": "GHIJ123456789012345678901234567890123456789012345678901234567890",
                    "weight": 1
                }
            ]
        }).to_string().into_bytes(),
        "user2".to_string(),
        "mainnet".to_string(),
    );
    queue.enqueue(multisig_tx).await?;
    println!("  ✅ Multi-signature transaction queued");

    // Contract deployment transaction
    let contract_tx = Transaction::new(
        TransactionType::ContractDeployment,
        serde_json::json!({
            "contract_code": "example_contract_wasm_bytes",
            "contract_name": "MyContract",
            "initial_balance": "1000"
        }).to_string().into_bytes(),
        "deployer".to_string(),
        "testnet".to_string(),
    );
    contract_tx.priority = TransactionPriority::Normal;
    queue.enqueue(contract_tx).await?;
    println!("  ✅ Contract deployment transaction queued");

    // Security scan transaction
    let scan_tx = Transaction::new(
        TransactionType::SecurityScan,
        serde_json::json!({
            "target_contract": "SCAN123456789012345678901234567890123456789012345678901234567890",
            "scan_level": "comprehensive",
            "vulnerability_check": true
        }).to_string().into_bytes(),
        "security_admin".to_string(),
        "mainnet".to_string(),
    );
    scan_tx.priority = TransactionPriority::Critical;
    queue.enqueue(scan_tx).await?;
    println!("  ✅ Security scan transaction queued");

    println!("\n📊 Current queue statistics:");
    let stats = queue.get_stats().await;
    println!("  Total queued: {}", stats.total_queued);
    println!("  Processing: {}", stats.processing);
    println!("  Completed: {}", stats.completed);

    // 8. Monitor transactions processing
    println!("\n🔄 Monitoring transaction processing...");
    
    let mut processed_count = 0;
    let timeout = Duration::from_secs(30);
    let start_time = std::time::Instant::now();

    while processed_count < 4 && start_time.elapsed() < timeout {
        tokio::select! {
            // Monitor queue notifications
            notification = queue_notifications.recv() => {
                if let Some(notification) = notification {
                    match notification {
                        QueueNotification::TransactionQueued { transaction_id } => {
                            println!("  📋 Transaction {} queued", transaction_id);
                        }
                        QueueNotification::TransactionStarted { transaction_id, worker_id } => {
                            println!("  ⚡ Transaction {} started processing on worker {}", transaction_id, worker_id);
                        }
                        QueueNotification::TransactionCompleted { transaction_id, result } => {
                            println!("  ✅ Transaction {} completed successfully", transaction_id);
                            if result.success {
                                println!("    Gas used: {:?}", result.gas_used);
                                println!("    Duration: {:?}", result.duration);
                            }
                            processed_count += 1;
                        }
                        QueueNotification::TransactionFailed { transaction_id, error, retryable } => {
                            println!("  ❌ Transaction {} failed: {}", transaction_id, error);
                            if retryable {
                                println!("    🔄 Will retry automatically");
                            }
                        }
                        _ => {}
                    }
                }
            }
            
            // Monitor retry notifications
            notification = retry_notifications.recv() => {
                if let Some(notification) = notification {
                    match notification {
                        RetryNotification::TransactionRetried { transaction_id, attempt, next_retry_delay_ms } => {
                            println!("  🔄 Transaction {} retried (attempt {}), next retry in {}ms", 
                                transaction_id, attempt, next_retry_delay_ms);
                        }
                        _ => {}
                    }
                }
            }
            
            // Monitor system notifications
            notification = monitoring_notifications.recv() => {
                if let Some(notification) = notification {
                    match notification {
                        MonitoringNotification::MetricsUpdated { snapshot } => {
                            println!("  📈 Metrics updated - Queue depth: {}", 
                                snapshot.performance_metrics.queue_depth);
                        }
                        MonitoringNotification::AlertTriggered { alert } => {
                            println!("  🚨 Alert triggered: {} - {}", 
                                alert.level, alert.message);
                        }
                        _ => {}
                    }
                }
            }
            
            // Check timeout
            _ = sleep(Duration::from_millis(100)) => {
                if start_time.elapsed() >= timeout {
                    break;
                }
            }
        }
    }

    // 9. Show final statistics
    println!("\n📊 Final Statistics:");
    let final_stats = queue.get_stats().await;
    println!("  Total queued: {}", final_stats.total_queued);
    println!("  Processing: {}", final_stats.processing);
    println!("  Completed: {}", final_stats.completed);
    println!("  Failed (retryable): {}", final_stats.failed_retryable);
    println!("  Failed (permanent): {}", final_stats.failed_permanent);

    // 10. Show monitoring data
    if let Some(snapshot) = monitor.get_current_snapshot().await? {
        println!("\n📈 Monitoring Snapshot:");
        println!("  Throughput: {:.2} TPS", snapshot.performance_metrics.throughput_tps);
        println!("  Success rate: {:.1}%", snapshot.performance_metrics.success_rate);
        println!("  Average processing time: {:.2}ms", snapshot.performance_metrics.average_processing_time_ms);
        
        if !snapshot.alerts.is_empty() {
            println!("  Active alerts: {}", snapshot.alerts.len());
            for alert in &snapshot.alerts {
                println!("    - {}: {}", alert.level, alert.message);
            }
        }
    }

    // 11. Show transaction statistics
    let transaction_stats = state_manager.get_transaction_stats().await;
    println!("\n📋 Transaction Statistics:");
    println!("  Total transactions: {}", transaction_stats.total);
    println!("  Payments: {}", transaction_stats.payments);
    println!("  Multi-signature: {}", transaction_stats.multisig);
    println!("  Contract deployments: {}", transaction_stats.contract_deployments);
    println!("  Security scans: {}", transaction_stats.security_scans);

    // 12. Cleanup
    println!("\n🧹 Cleaning up...");
    processor_manager.stop().await?;
    retry_manager.stop().await?;
    monitor.stop().await?;
    state_manager.stop().await?;
    println!("✅ Cleanup completed");

    println!("\n🎉 Transaction Processing Engine example completed successfully!");
    
    Ok(())
}

/// Advanced example with custom transaction processor
pub async fn run_custom_processor_example() -> Result<()> {
    println!("🚀 Starting Custom Processor Example\n");

    // Create a custom transaction processor
    struct CustomPaymentProcessor;

    #[async_trait::async_trait]
    impl TransactionProcessor for CustomPaymentProcessor {
        async fn process_transaction(&self, transaction: &Transaction) -> Result<TransactionResult> {
            println!("  💳 Processing custom payment transaction: {}", transaction.id);
            
            // Simulate custom payment processing logic
            sleep(Duration::from_millis(200)).await;
            
            // Parse payment data
            let data_str = String::from_utf8_lossy(&transaction.data);
            if let Ok(payment_data) = serde_json::from_str::<serde_json::Value>(&data_str) {
                let amount = payment_data.get("amount")
                    .and_then(|a| a.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                
                let recipient = payment_data.get("recipient")
                    .and_then(|r| r.as_str())
                    .unwrap_or("unknown");

                // Custom validation logic
                if amount > 10000.0 {
                    return Ok(TransactionResult {
                        transaction_id: transaction.id,
                        success: false,
                        data: None,
                        error: Some("Amount exceeds custom limit".to_string()),
                        duration: Duration::from_millis(200),
                        gas_used: Some(500),
                        transaction_hash: None,
                        block_number: None,
                    });
                }

                println!("    💰 Processing payment of {} XLM to {}", amount, recipient);

                Ok(TransactionResult {
                    transaction_id: transaction.id,
                    success: true,
                    data: Some(format!("Payment of {} XLM processed", amount).into_bytes()),
                    error: None,
                    duration: Duration::from_millis(200),
                    gas_used: Some(1000 + (amount as u64)),
                    transaction_hash: Some(format!("custom_tx_{}", Uuid::new_v4())),
                    block_number: Some(99999),
                })
            } else {
                Ok(TransactionResult {
                    transaction_id: transaction.id,
                    success: false,
                    data: None,
                    error: Some("Invalid payment data format".to_string()),
                    duration: Duration::from_millis(50),
                    gas_used: Some(100),
                    transaction_hash: None,
                    block_number: None,
                })
            }
        }

        fn name(&self) -> &str {
            "CustomPaymentProcessor"
        }
    }

    // Initialize queue with custom processor
    let queue_config = QueueConfig::default();
    let (queue, _) = TransactionQueue::new(queue_config);
    let queue = Arc::new(queue);

    // Create worker with custom processor
    let processor_config = ProcessorConfig {
        worker_id: "custom-worker".to_string(),
        max_concurrent: 1,
        timeout_ms: 30000,
        health_check_interval_ms: 5000,
        enable_metrics: true,
    };

    let custom_processor = Arc::new(CustomPaymentProcessor);
    let mut worker = TransactionWorker::new(custom_processor, queue.clone(), processor_config);
    let mut worker_notifications = worker.start().await?;

    // Create and submit a payment transaction
    let payment_tx = Transaction::new(
        TransactionType::Payment,
        serde_json::json!({
            "amount": "500.75",
            "recipient": "GCUSTOM123456789012345678901234567890123456789012345678901234567890",
            "currency": "XLM"
        }).to_string().into_bytes(),
        "custom_user".to_string(),
        "testnet".to_string(),
    );
    
    let tx_id = payment_tx.id;
    queue.enqueue(payment_tx).await?;
    println!("💳 Custom payment transaction submitted: {}", tx_id);

    // Monitor processing
    let mut completed = false;
    let timeout = Duration::from_secs(10);
    let start_time = std::time::Instant::now();

    while !completed && start_time.elapsed() < timeout {
        if let Some(notification) = worker_notifications.recv().await {
            match notification {
                WorkerMessage::TransactionCompleted { transaction_id, result } => {
                    if transaction_id == tx_id {
                        println!("✅ Custom payment transaction completed!");
                        println!("  Success: {}", result.success);
                        if let Some(error) = result.error {
                            println!("  Error: {}", error);
                        }
                        if let Some(data) = result.data {
                            println!("  Result: {}", String::from_utf8_lossy(&data));
                        }
                        completed = true;
                    }
                }
                WorkerMessage::TransactionFailed { transaction_id, error, .. } => {
                    if transaction_id == tx_id {
                        println!("❌ Custom payment transaction failed: {}", error);
                        completed = true;
                    }
                }
                _ => {}
            }
        }
        
        if !completed {
            sleep(Duration::from_millis(100)).await;
        }
    }

    worker.stop().await?;
    println!("\n🎉 Custom Processor example completed!");

    Ok(())
}

/// Example demonstrating validation and security scanning
pub async fn run_validation_example() -> Result<()> {
    println!("🔒 Starting Validation and Security Example\n");

    // Create validator with strict settings
    let validation_config = ValidationConfig {
        strict_mode: true,
        max_data_size: 1024 * 1024,
        max_age_minutes: 60,
        required_fields: {
            let mut fields = std::collections::HashMap::new();
            fields.insert(TransactionType::Payment, vec!["amount".to_string(), "recipient".to_string()]);
            fields
        },
        blacklisted_submitters: vec!["malicious_user".to_string()],
        rate_limit_per_minute: 10,
        enable_security_scan: true,
    };

    let mut validator = TransactionValidator::new(validation_config);

    // Test cases
    let test_cases = vec![
        ("Valid payment", serde_json::json!({
            "amount": "100.50",
            "recipient": "GVALID123456789012345678901234567890123456789012345678901234567890"
        })),
        
        ("Invalid amount", serde_json::json!({
            "amount": "-50.00",
            "recipient": "GVALID123456789012345678901234567890123456789012345678901234567890"
        })),
        
        ("Missing recipient", serde_json::json!({
            "amount": "100.50"
        })),
        
        ("Suspicious pattern", serde_json::json!({
            "amount": "100.50",
            "recipient": "GVALID123456789012345678901234567890123456789012345678901234567890",
            "note": "DROP TABLE users"
        })),
    ];

    for (test_name, test_data) in test_cases {
        println!("🧪 Testing: {}", test_name);
        
        let transaction = Transaction::new(
            TransactionType::Payment,
            test_data.to_string().into_bytes(),
            "test_user".to_string(),
            "testnet".to_string(),
        );

        let validation_result = validator.validate_transaction(&transaction).await;
        
        println!("  Valid: {}", validation_result.is_valid);
        println!("  Overall Score: {}/100", validation_result.score.overall);
        println!("  Security Score: {}/100", validation_result.score.security);
        println!("  Performance Score: {}/100", validation_result.score.performance);
        println!("  Reliability Score: {}/100", validation_result.score.reliability);
        
        if !validation_result.errors.is_empty() {
            println!("  Errors:");
            for error in &validation_result.errors {
                println!("    - {}: {} ({:?})", error.code, error.message, error.severity);
            }
        }
        
        if !validation_result.warnings.is_empty() {
            println!("  Warnings:");
            for warning in &validation_result.warnings {
                println!("    - {}: {}", warning.code, warning.message);
                if let Some(rec) = &warning.recommendation {
                    println!("      Recommendation: {}", rec);
                }
            }
        }
        
        println!();
    }

    println!("🎉 Validation and Security example completed!");

    Ok(())
}

/// Main example runner
pub async fn run_all_examples() -> Result<()> {
    println!("🎯 Running all Transaction Processing Engine examples\n");
    
    // Run basic example
    if let Err(e) = run_transaction_engine_example().await {
        eprintln!("❌ Basic example failed: {}", e);
    }
    
    println!("\n" + "=".repeat(60).as_str() + "\n");
    
    // Run custom processor example
    if let Err(e) = run_custom_processor_example().await {
        eprintln!("❌ Custom processor example failed: {}", e);
    }
    
    println!("\n" + "=".repeat(60).as_str() + "\n");
    
    // Run validation example
    if let Err(e) = run_validation_example().await {
        eprintln!("❌ Validation example failed: {}", e);
    }
    
    println!("\n🎉 All examples completed!");
    
    Ok(())
}
