//! Transaction processor implementation

use crate::transaction_engine::{
    Transaction, TransactionResult, TransactionFailure, TransactionQueue,
    QueueNotification, QueueConfig, TransactionType, TransactionState
};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, RwLock};
use uuid::Uuid;
use chrono::Utc;
use anyhow::Result;
use std::time::Duration;

/// Transaction processor configuration
#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    /// Worker ID
    pub worker_id: String,
    /// Maximum concurrent transactions per worker
    pub max_concurrent: usize,
    /// Processing timeout per transaction
    pub timeout_ms: u64,
    /// Health check interval
    pub health_check_interval_ms: u64,
    /// Enable metrics collection
    pub enable_metrics: bool,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        ProcessorConfig {
            worker_id: format!("worker-{}", uuid::Uuid::new_v4()),
            max_concurrent: 1,
            timeout_ms: 30000, // 30 seconds
            health_check_interval_ms: 5000, // 5 seconds
            enable_metrics: true,
        }
    }
}

/// Transaction processor metrics
#[derive(Debug, Clone, Default)]
pub struct ProcessorMetrics {
    pub total_processed: u64,
    pub successful: u64,
    pub failed: u64,
    pub average_processing_time_ms: f64,
    pub current_processing: usize,
    pub uptime_ms: u64,
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,
}

/// Transaction processor trait
#[async_trait::async_trait]
pub trait TransactionProcessor: Send + Sync {
    /// Process a single transaction
    async fn process_transaction(&self, transaction: &Transaction) -> Result<TransactionResult>;
    
    /// Validate transaction before processing
    async fn validate_transaction(&self, transaction: &Transaction) -> Result<()> {
        // Default validation - can be overridden
        if transaction.data.is_empty() {
            return Err(anyhow::anyhow!("Transaction data is empty"));
        }
        Ok(())
    }
    
    /// Get processor name
    fn name(&self) -> &str;
}

/// Default transaction processor implementation
pub struct DefaultTransactionProcessor {
    name: String,
    config: ProcessorConfig,
    metrics: Arc<RwLock<ProcessorMetrics>>,
    start_time: chrono::DateTime<chrono::Utc>,
}

impl DefaultTransactionProcessor {
    pub fn new(config: ProcessorConfig) -> Self {
        Self {
            name: format!("DefaultProcessor-{}", config.worker_id),
            config,
            metrics: Arc::new(RwLock::new(ProcessorMetrics::default())),
            start_time: Utc::now(),
        }
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> ProcessorMetrics {
        self.metrics.read().await.clone()
    }

    /// Reset metrics
    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        *metrics = ProcessorMetrics::default();
    }
}

#[async_trait::async_trait]
impl TransactionProcessor for DefaultTransactionProcessor {
    async fn process_transaction(&self, transaction: &Transaction) -> Result<TransactionResult> {
        let start_time = std::time::Instant::now();
        
        // Simulate processing based on transaction type
        let result = match transaction.transaction_type {
            TransactionType::Payment => self.process_payment_transaction(transaction).await,
            TransactionType::MultiSignature => self.process_multisig_transaction(transaction).await,
            TransactionType::ContractDeployment => self.process_contract_deployment(transaction).await,
            TransactionType::ContractCall => self.process_contract_call(transaction).await,
            TransactionType::BatchOperation => self.process_batch_operation(transaction).await,
            TransactionType::SecurityScan => self.process_security_scan(transaction).await,
            TransactionType::Custom(_) => self.process_custom_transaction(transaction).await,
        };

        let duration = start_time.elapsed();
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_processed += 1;
            metrics.last_activity = Some(Utc::now());
            
            match &result {
                Ok(_) => metrics.successful += 1,
                Err(_) => metrics.failed += 1,
            }
            
            // Update average processing time
            let total_time = metrics.average_processing_time_ms * (metrics.total_processed - 1) as f64;
            metrics.average_processing_time_ms = (total_time + duration.as_millis() as f64) / metrics.total_processed as f64;
        }

        result.map(|mut r| {
            r.duration = duration;
            r
        })
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl DefaultTransactionProcessor {
    async fn process_payment_transaction(&self, transaction: &Transaction) -> Result<TransactionResult> {
        // Simulate payment processing
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        Ok(TransactionResult {
            transaction_id: transaction.id,
            success: true,
            data: Some(b"payment_processed".to_vec()),
            error: None,
            duration: Duration::default(),
            gas_used: Some(1000),
            transaction_hash: Some(format!("tx_hash_{}", uuid::Uuid::new_v4())),
            block_number: Some(12345),
        })
    }

    async fn process_multisig_transaction(&self, transaction: &Transaction) -> Result<TransactionResult> {
        // Simulate multi-signature processing
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Simulate validation
        if transaction.data.len() < 10 {
            return Ok(TransactionResult {
                transaction_id: transaction.id,
                success: false,
                data: None,
                error: Some("Insufficient signature data".to_string()),
                duration: Duration::default(),
                gas_used: Some(500),
                transaction_hash: None,
                block_number: None,
            });
        }
        
        Ok(TransactionResult {
            transaction_id: transaction.id,
            success: true,
            data: Some(b"multisig_processed".to_vec()),
            error: None,
            duration: Duration::default(),
            gas_used: Some(2000),
            transaction_hash: Some(format!("multisig_tx_{}", uuid::Uuid::new_v4())),
            block_number: Some(12346),
        })
    }

    async fn process_contract_deployment(&self, transaction: &Transaction) -> Result<TransactionResult> {
        // Simulate contract deployment
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        Ok(TransactionResult {
            transaction_id: transaction.id,
            success: true,
            data: Some(b"contract_deployed".to_vec()),
            error: None,
            duration: Duration::default(),
            gas_used: Some(50000),
            transaction_hash: Some(format!("contract_{}", uuid::Uuid::new_v4())),
            block_number: Some(12347),
        })
    }

    async fn process_contract_call(&self, transaction: &Transaction) -> Result<TransactionResult> {
        // Simulate contract call
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        Ok(TransactionResult {
            transaction_id: transaction.id,
            success: true,
            data: Some(b"call_executed".to_vec()),
            error: None,
            duration: Duration::default(),
            gas_used: Some(3000),
            transaction_hash: Some(format!("call_{}", uuid::Uuid::new_v4())),
            block_number: Some(12348),
        })
    }

    async fn process_batch_operation(&self, transaction: &Transaction) -> Result<TransactionResult> {
        // Simulate batch operation
        tokio::time::sleep(Duration::from_millis(300)).await;
        
        Ok(TransactionResult {
            transaction_id: transaction.id,
            success: true,
            data: Some(b"batch_completed".to_vec()),
            error: None,
            duration: Duration::default(),
            gas_used: Some(10000),
            transaction_hash: Some(format!("batch_{}", uuid::Uuid::new_v4())),
            block_number: Some(12349),
        })
    }

    async fn process_security_scan(&self, transaction: &Transaction) -> Result<TransactionResult> {
        // Simulate security scan
        tokio::time::sleep(Duration::from_millis(1000)).await;
        
        Ok(TransactionResult {
            transaction_id: transaction.id,
            success: true,
            data: Some(b"security_scan_complete".to_vec()),
            error: None,
            duration: Duration::default(),
            gas_used: None,
            transaction_hash: Some(format!("scan_{}", uuid::Uuid::new_v4())),
            block_number: None,
        })
    }

    async fn process_custom_transaction(&self, transaction: &Transaction) -> Result<TransactionResult> {
        // Simulate custom transaction processing
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        Ok(TransactionResult {
            transaction_id: transaction.id,
            success: true,
            data: Some(b"custom_processed".to_vec()),
            error: None,
            duration: Duration::default(),
            gas_used: Some(1500),
            transaction_hash: Some(format!("custom_{}", uuid::Uuid::new_v4())),
            block_number: Some(12350),
        })
    }
}

/// Transaction worker that processes transactions from the queue
pub struct TransactionWorker {
    worker_id: String,
    processor: Arc<dyn TransactionProcessor>,
    queue: Arc<TransactionQueue>,
    config: ProcessorConfig,
    shutdown_tx: Option<oneshot::Sender<()>>,
    metrics: Arc<RwLock<ProcessorMetrics>>,
}

impl TransactionWorker {
    /// Create a new transaction worker
    pub fn new(
        processor: Arc<dyn TransactionProcessor>,
        queue: Arc<TransactionQueue>,
        config: ProcessorConfig,
    ) -> Self {
        Self {
            worker_id: config.worker_id.clone(),
            processor,
            queue,
            config,
            shutdown_tx: None,
            metrics: Arc::new(RwLock::new(ProcessorMetrics::default())),
        }
    }

    /// Start the worker
    pub async fn start(&mut self) -> Result<mpsc::UnboundedReceiver<WorkerMessage>> {
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        let worker_id = self.worker_id.clone();
        let processor = self.processor.clone();
        let queue = self.queue.clone();
        let config = self.config.clone();
        let metrics = self.metrics.clone();

        // Spawn worker task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100)); // Poll every 100ms
            let mut shutdown_rx = shutdown_rx;

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Try to get next transaction
                        match queue.dequeue(worker_id.clone()).await {
                            Ok(Some(transaction)) => {
                                let processor = processor.clone();
                                let queue = queue.clone();
                                let message_tx = message_tx.clone();
                                let metrics = metrics.clone();

                                // Process transaction in background
                                tokio::spawn(async move {
                                    let _ = Self::process_single_transaction(
                                        processor,
                                        queue,
                                        transaction,
                                        message_tx,
                                        metrics,
                                    ).await;
                                });
                            }
                            Ok(None) => {
                                // No transactions available
                                let _ = message_tx.send(WorkerMessage::NoTransactionsAvailable);
                            }
                            Err(e) => {
                                let _ = message_tx.send(WorkerMessage::Error {
                                    error: format!("Failed to dequeue transaction: {}", e),
                                });
                            }
                        }
                    }
                    _ = &mut shutdown_rx => {
                        let _ = message_tx.send(WorkerMessage::Shutdown);
                        break;
                    }
                }
            }
        });

        Ok(message_rx)
    }

    /// Stop the worker
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        Ok(())
    }

    /// Get worker metrics
    pub async fn get_metrics(&self) -> ProcessorMetrics {
        self.metrics.read().await.clone()
    }

    /// Process a single transaction
    async fn process_single_transaction(
        processor: Arc<dyn TransactionProcessor>,
        queue: Arc<TransactionQueue>,
        transaction: Transaction,
        message_tx: mpsc::UnboundedSender<WorkerMessage>,
        metrics: Arc<RwLock<ProcessorMetrics>>,
    ) -> Result<()> {
        let transaction_id = transaction.id;
        
        // Update metrics
        {
            let mut m = metrics.write().await;
            m.current_processing += 1;
            m.last_activity = Some(Utc::now());
        }

        // Send started message
        let _ = message_tx.send(WorkerMessage::TransactionStarted {
            transaction_id,
            worker_id: transaction.processing.worker_id.clone().unwrap_or_default(),
        });

        // Validate transaction
        if let Err(e) = processor.validate_transaction(&transaction).await {
            let _ = queue.fail_transaction(transaction_id, e.to_string(), false).await;
            let _ = message_tx.send(WorkerMessage::TransactionFailed {
                transaction_id,
                error: e.to_string(),
                retryable: false,
            });
            
            let mut m = metrics.write().await;
            m.current_processing -= 1;
            return Ok(());
        }

        // Process transaction with timeout
        let result = tokio::time::timeout(
            Duration::from_millis(30000), // 30 second timeout
            processor.process_transaction(&transaction)
        ).await;

        match result {
            Ok(Ok(transaction_result)) => {
                // Success
                let _ = queue.complete_transaction(transaction_id, transaction_result.clone()).await;
                let _ = message_tx.send(WorkerMessage::TransactionCompleted {
                    transaction_id,
                    result: transaction_result,
                });
            }
            Ok(Err(e)) => {
                // Processing error
                let error_msg = e.to_string();
                let retryable = !error_msg.contains("invalid") && !error_msg.contains("permission");
                
                let _ = queue.fail_transaction(transaction_id, error_msg.clone(), retryable).await;
                let _ = message_tx.send(WorkerMessage::TransactionFailed {
                    transaction_id,
                    error: error_msg,
                    retryable,
                });
            }
            Err(_) => {
                // Timeout
                let _ = queue.fail_transaction(transaction_id, "Processing timeout".to_string(), true).await;
                let _ = message_tx.send(WorkerMessage::TransactionFailed {
                    transaction_id,
                    error: "Processing timeout".to_string(),
                    retryable: true,
                });
            }
        }

        // Update metrics
        {
            let mut m = metrics.write().await;
            m.current_processing -= 1;
        }

        Ok(())
    }
}

/// Worker messages
#[derive(Debug, Clone)]
pub enum WorkerMessage {
    TransactionStarted {
        transaction_id: Uuid,
        worker_id: String,
    },
    TransactionCompleted {
        transaction_id: Uuid,
        result: TransactionResult,
    },
    TransactionFailed {
        transaction_id: Uuid,
        error: String,
        retryable: bool,
    },
    NoTransactionsAvailable,
    Error {
        error: String,
    },
    Shutdown,
}

/// Transaction processor manager
pub struct ProcessorManager {
    workers: Vec<TransactionWorker>,
    queue: Arc<TransactionQueue>,
    config: QueueConfig,
}

impl ProcessorManager {
    /// Create a new processor manager
    pub fn new(queue: Arc<TransactionQueue>, config: QueueConfig) -> Self {
        Self {
            workers: Vec::new(),
            queue,
            config,
        }
    }

    /// Start all workers
    pub async fn start(&mut self) -> Result<Vec<mpsc::UnboundedReceiver<WorkerMessage>>> {
        let mut message_receivers = Vec::new();

        for i in 0..self.config.concurrent_workers {
            let processor_config = ProcessorConfig {
                worker_id: format!("worker-{}", i),
                max_concurrent: 1,
                timeout_ms: 30000,
                health_check_interval_ms: 5000,
                enable_metrics: true,
            };

            let processor = Arc::new(DefaultTransactionProcessor::new(processor_config.clone()));
            let mut worker = TransactionWorker::new(
                processor,
                self.queue.clone(),
                processor_config,
            );

            let receiver = worker.start().await?;
            message_receivers.push(receiver);
            self.workers.push(worker);
        }

        Ok(message_receivers)
    }

    /// Stop all workers
    pub async fn stop(&mut self) -> Result<()> {
        for worker in &mut self.workers {
            worker.stop().await?;
        }
        Ok(())
    }

    /// Get all worker metrics
    pub async fn get_metrics(&self) -> Vec<ProcessorMetrics> {
        let mut metrics = Vec::new();
        for worker in &self.workers {
            metrics.push(worker.get_metrics().await);
        }
        metrics
    }
}
