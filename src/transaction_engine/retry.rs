//! Retry logic and backoff strategies

use crate::transaction_engine::{
    Transaction, TransactionFailure, RetryStrategy, TransactionState,
    TransactionQueue, QueueNotification
};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use chrono::{Utc, Duration};
use anyhow::Result;

/// Retry manager configuration
#[derive(Debug, Clone)]
pub struct RetryManagerConfig {
    /// Check interval for retryable transactions
    pub check_interval_ms: u64,
    /// Maximum concurrent retries
    pub max_concurrent_retries: usize,
    /// Retry queue size limit
    pub max_retry_queue_size: usize,
    /// Enable jitter in retry delays
    pub enable_jitter: bool,
    /// Maximum retry age before giving up
    pub max_retry_age_hours: u64,
}

impl Default for RetryManagerConfig {
    fn default() -> Self {
        RetryManagerConfig {
            check_interval_ms: 5000, // 5 seconds
            max_concurrent_retries: 2,
            max_retry_queue_size: 1000,
            enable_jitter: true,
            max_retry_age_hours: 24, // 1 day
        }
    }
}

/// Retry manager handles failed transactions and retry logic
pub struct RetryManager {
    queue: Arc<TransactionQueue>,
    config: RetryManagerConfig,
    retry_stats: Arc<RwLock<RetryStats>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

/// Retry statistics
#[derive(Debug, Clone, Default)]
pub struct RetryStats {
    pub total_retries_attempted: u64,
    pub successful_retries: u64,
    pub failed_retries: u64,
    pub average_retries_per_transaction: f64,
    pub oldest_retry_age_seconds: Option<u64>,
    pub retry_queue_size: usize,
}

impl RetryManager {
    /// Create a new retry manager
    pub fn new(queue: Arc<TransactionQueue>, config: RetryManagerConfig) -> Self {
        Self {
            queue,
            config,
            retry_stats: Arc::new(RwLock::new(RetryStats::default())),
            shutdown_tx: None,
        }
    }

    /// Start the retry manager
    pub async fn start(&mut self) -> Result<mpsc::UnboundedReceiver<RetryNotification>> {
        let (notification_tx, notification_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        let queue = self.queue.clone();
        let config = self.config.clone();
        let retry_stats = self.retry_stats.clone();
        let notification_tx_clone = notification_tx.clone();

        // Spawn retry manager task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_millis(config.check_interval_ms)
            );
            let mut shutdown_rx = shutdown_rx;

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = Self::process_retryable_transactions(
                            &queue,
                            &config,
                            &retry_stats,
                            &notification_tx_clone,
                        ).await {
                            let _ = notification_tx.send(RetryNotification::Error {
                                error: format!("Retry manager error: {}", e),
                            });
                        }
                    }
                    _ = &mut shutdown_rx => {
                        let _ = notification_tx.send(RetryNotification::Shutdown);
                        break;
                    }
                }
            }
        });

        Ok(notification_rx)
    }

    /// Stop the retry manager
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        Ok(())
    }

    /// Get retry statistics
    pub async fn get_stats(&self) -> RetryStats {
        self.retry_stats.read().await.clone()
    }

    /// Manually retry a specific transaction
    pub async fn retry_transaction(&self, transaction_id: &Uuid) -> Result<()> {
        // Get the transaction
        let transaction = self.queue.get_transaction(transaction_id).await?
            .ok_or_else(|| anyhow::anyhow!("Transaction not found: {}", transaction_id))?;

        // Check if it can be retried
        if !transaction.can_retry() {
            return Err(anyhow::anyhow!("Transaction cannot be retried: {}", transaction_id));
        }

        // Requeue the transaction
        self.queue.requeue_transaction(transaction_id).await?;

        // Update stats
        {
            let mut stats = self.retry_stats.write().await;
            stats.total_retries_attempted += 1;
        }

        Ok(())
    }

    /// Process retryable transactions
    async fn process_retryable_transactions(
        queue: &Arc<TransactionQueue>,
        config: &RetryManagerConfig,
        retry_stats: &Arc<RwLock<RetryStats>>,
        notification_tx: &mpsc::UnboundedSender<RetryNotification>,
    ) -> Result<()> {
        // Get retryable transactions
        let retryable_transactions = queue.get_retryable_transactions().await?;
        
        if retryable_transactions.is_empty() {
            return Ok(());
        }

        // Check retry queue size limit
        if retryable_transactions.len() > config.max_retry_queue_size {
            let _ = notification_tx.send(RetryNotification::RetryQueueFull {
                size: retryable_transactions.len(),
                limit: config.max_retry_queue_size,
            });
            return Ok(());
        }

        let mut retried_count = 0;
        let now = Utc::now();

        for transaction in retryable_transactions {
            // Check if transaction is too old
            let age = now - transaction.metadata.created_at;
            if age > Duration::hours(config.max_retry_age_hours as i64) {
                // Mark as permanently failed
                let _ = queue.fail_transaction(
                    transaction.id,
                    "Transaction exceeded maximum retry age".to_string(),
                    false,
                ).await;
                
                let _ = notification_tx.send(RetryNotification::TransactionExpired {
                    transaction_id: transaction.id,
                    age_hours: age.num_hours(),
                });
                continue;
            }

            // Check if it's time to retry
            if let Some(retry_at) = transaction.retry_info.next_retry_at {
                if retry_at <= now {
                    // Retry the transaction
                    match queue.requeue_transaction(&transaction.id).await {
                        Ok(()) => {
                            retried_count += 1;
                            
                            let _ = notification_tx.send(RetryNotification::TransactionRetried {
                                transaction_id: transaction.id,
                                attempt: transaction.retry_info.attempt + 1,
                                next_retry_delay_ms: transaction.retry_info.retry_strategy.next_delay_ms(transaction.retry_info.attempt + 1),
                            });
                        }
                        Err(e) => {
                            let _ = notification_tx.send(RetryNotification::RetryFailed {
                                transaction_id: transaction.id,
                                error: e.to_string(),
                            });
                        }
                    }
                }
            }

            // Limit concurrent retries
            if retried_count >= config.max_concurrent_retries {
                break;
            }
        }

        // Update statistics
        {
            let mut stats = retry_stats.write().await;
            stats.total_retries_attempted += retried_count as u64;
            stats.retry_queue_size = queue.get_retryable_transactions().await?.len();
            
            // Calculate oldest retry age
            let retryable = queue.get_retryable_transactions().await?;
            if let Some(oldest) = retryable.iter().min_by_key(|tx| tx.metadata.created_at) {
                let age = (now - oldest.metadata.created_at).num_seconds();
                stats.oldest_retry_age_seconds = Some(age.max(0) as u64);
            }
        }

        Ok(())
    }
}

/// Retry notifications
#[derive(Debug, Clone)]
pub enum RetryNotification {
    TransactionRetried {
        transaction_id: Uuid,
        attempt: u32,
        next_retry_delay_ms: u64,
    },
    TransactionExpired {
        transaction_id: Uuid,
        age_hours: i64,
    },
    RetryFailed {
        transaction_id: Uuid,
        error: String,
    },
    RetryQueueFull {
        size: usize,
        limit: usize,
    },
    Error {
        error: String,
    },
    Shutdown,
}

/// Advanced retry strategies
pub struct RetryStrategies;

impl RetryStrategies {
    /// Create exponential backoff strategy with jitter
    pub fn exponential_backoff(
        base_delay_ms: u64,
        max_delay_ms: u64,
        multiplier: f64,
        jitter: bool,
    ) -> RetryStrategy {
        RetryStrategy::ExponentialBackoff {
            base_delay_ms,
            max_delay_ms,
            multiplier,
            jitter,
        }
    }

    /// Create linear backoff strategy
    pub fn linear_backoff(base_delay_ms: u64, increment_ms: u64) -> RetryStrategy {
        RetryStrategy::Linear {
            base_delay_ms,
            increment_ms,
        }
    }

    /// Create fixed delay strategy
    pub fn fixed_delay(delay_ms: u64) -> RetryStrategy {
        RetryStrategy::Fixed { delay_ms }
    }

    /// Create no retry strategy
    pub fn no_retry() -> RetryStrategy {
        RetryStrategy::NoRetry
    }

    /// Get recommended retry strategy based on transaction type
    pub fn recommended_strategy(transaction_type: &crate::transaction_engine::TransactionType) -> RetryStrategy {
        match transaction_type {
            crate::transaction_engine::TransactionType::Payment => {
                Self::exponential_backoff(1000, 10000, 2.0, true)
            }
            crate::transaction_engine::TransactionType::MultiSignature => {
                Self::exponential_backoff(2000, 30000, 2.5, true)
            }
            crate::transaction_engine::TransactionType::ContractDeployment => {
                Self::exponential_backoff(5000, 60000, 2.0, true)
            }
            crate::transaction_engine::TransactionType::ContractCall => {
                Self::exponential_backoff(1000, 15000, 2.0, true)
            }
            crate::transaction_engine::TransactionType::BatchOperation => {
                Self::exponential_backoff(3000, 45000, 2.0, true)
            }
            crate::transaction_engine::TransactionType::SecurityScan => {
                Self::exponential_backoff(10000, 120000, 1.5, true)
            }
            crate::transaction_engine::TransactionType::Custom(_) => {
                Self::exponential_backoff(2000, 20000, 2.0, true)
            }
        }
    }
}

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Retry strategy
    pub strategy: RetryStrategy,
    /// Conditions under which to retry
    pub retry_conditions: Vec<RetryCondition>,
    /// Maximum age for retries
    pub max_age_hours: Option<u64>,
}

/// Retry condition
#[derive(Debug, Clone)]
pub enum RetryCondition {
    /// Retry on network errors
    NetworkError,
    /// Retry on timeout
    Timeout,
    /// Retry on rate limit
    RateLimit,
    /// Retry on specific error codes
    ErrorCode(String),
    /// Custom retry condition
    Custom(Box<dyn Fn(&TransactionFailure) -> bool + Send + Sync>),
}

impl RetryPolicy {
    /// Create a new retry policy
    pub fn new(max_attempts: u32, strategy: RetryStrategy) -> Self {
        Self {
            max_attempts,
            strategy,
            retry_conditions: vec![
                RetryCondition::NetworkError,
                RetryCondition::Timeout,
                RetryCondition::RateLimit,
            ],
            max_age_hours: Some(24),
        }
    }

    /// Check if a transaction failure should be retried
    pub fn should_retry(&self, failure: &TransactionFailure, attempt: u32) -> bool {
        if attempt >= self.max_attempts {
            return false;
        }

        // Check retry conditions
        for condition in &self.retry_conditions {
            if self.matches_condition(failure, condition) {
                return true;
            }
        }

        false
    }

    /// Check if failure matches retry condition
    fn matches_condition(&self, failure: &TransactionFailure, condition: &RetryCondition) -> bool {
        match condition {
            RetryCondition::NetworkError => {
                matches!(failure, TransactionFailure::NetworkError(_))
            }
            RetryCondition::Timeout => {
                matches!(failure, TransactionFailure::Timeout)
            }
            RetryCondition::RateLimit => {
                matches!(failure, TransactionFailure::RateLimitExceeded)
            }
            RetryCondition::ErrorCode(code) => {
                // This would need to be implemented based on actual error codes
                false
            }
            RetryCondition::Custom(func) => {
                func(failure)
            }
        }
    }
}

/// Default retry policies for different transaction types
pub struct DefaultRetryPolicies;

impl DefaultRetryPolicies {
    /// Get retry policy for transaction type
    pub fn for_transaction_type(transaction_type: &crate::transaction_engine::TransactionType) -> RetryPolicy {
        let strategy = RetryStrategies::recommended_strategy(transaction_type);
        
        match transaction_type {
            crate::transaction_engine::TransactionType::Payment => {
                RetryPolicy::new(3, strategy)
            }
            crate::transaction_engine::TransactionType::MultiSignature => {
                RetryPolicy::new(5, strategy)
            }
            crate::transaction_engine::TransactionType::ContractDeployment => {
                RetryPolicy::new(2, strategy) // Less retries for expensive operations
            }
            crate::transaction_engine::TransactionType::ContractCall => {
                RetryPolicy::new(3, strategy)
            }
            crate::transaction_engine::TransactionType::BatchOperation => {
                RetryPolicy::new(4, strategy)
            }
            crate::transaction_engine::TransactionType::SecurityScan => {
                RetryPolicy::new(2, strategy) // Security scans might be time-sensitive
            }
            crate::transaction_engine::TransactionType::Custom(_) => {
                RetryPolicy::new(3, strategy)
            }
        }
    }
}
