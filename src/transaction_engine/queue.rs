//! Transaction queue management system

use crate::transaction_engine::{Transaction, TransactionPriority, TransactionState, QueueConfig, TransactionFilter};
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::sync::{RwLock, mpsc, oneshot};
use uuid::Uuid;
use chrono::Utc;
use anyhow::Result;

/// Priority queue entry
#[derive(Debug)]
struct PriorityQueueEntry {
    transaction: Transaction,
    priority: TransactionPriority,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl PartialEq for PriorityQueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.transaction.id == other.transaction.id
    }
}

impl Eq for PriorityQueueEntry {}

impl PartialOrd for PriorityQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityQueueEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse order for max-heap (higher priority first)
        other.priority.cmp(&self.priority)
            .then_with(|| self.created_at.cmp(&other.created_at))
    }
}

/// Transaction queue manager
#[derive(Clone)]
pub struct TransactionQueue {
    inner: Arc<RwLock<QueueInner>>,
    config: QueueConfig,
    notification_sender: mpsc::UnboundedSender<QueueNotification>,
}

/// Internal queue state
struct QueueInner {
    /// Priority queue for high-priority transactions
    priority_queue: BinaryHeap<PriorityQueueEntry>,
    /// FIFO queue for normal transactions
    fifo_queue: VecDeque<Transaction>,
    /// Transactions currently being processed
    processing: HashMap<Uuid, Transaction>,
    /// Completed transactions (kept for monitoring)
    completed: Vec<Transaction>,
    /// Failed transactions (kept for retry)
    failed_retryable: Vec<Transaction>,
    /// Failed permanently
    failed_permanent: Vec<Transaction>,
    /// All transactions by ID for quick lookup
    transactions: HashMap<Uuid, Transaction>,
    /// Queue statistics
    stats: QueueStats,
}

/// Queue statistics
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    pub total_queued: usize,
    pub processing: usize,
    pub completed: usize,
    pub failed_retryable: usize,
    pub failed_permanent: usize,
    pub cancelled: usize,
    pub on_hold: usize,
    pub oldest_transaction_age_seconds: Option<u64>,
    pub average_processing_time_ms: Option<f64>,
}

/// Queue notifications
#[derive(Debug, Clone)]
pub enum QueueNotification {
    TransactionQueued { transaction_id: Uuid },
    TransactionStarted { transaction_id: Uuid, worker_id: String },
    TransactionCompleted { transaction_id: Uuid, result: crate::transaction_engine::TransactionResult },
    TransactionFailed { transaction_id: Uuid, error: String, retryable: bool },
    TransactionCancelled { transaction_id: Uuid },
    QueueFull,
    WorkerAvailable,
}

impl TransactionQueue {
    /// Create a new transaction queue
    pub fn new(config: QueueConfig) -> (Self, mpsc::UnboundedReceiver<QueueNotification>) {
        let (notification_sender, notification_receiver) = mpsc::unbounded_channel();
        
        let queue = TransactionQueue {
            inner: Arc::new(RwLock::new(QueueInner {
                priority_queue: BinaryHeap::new(),
                fifo_queue: VecDeque::new(),
                processing: HashMap::new(),
                completed: Vec::new(),
                failed_retryable: Vec::new(),
                failed_permanent: Vec::new(),
                transactions: HashMap::new(),
                stats: QueueStats::default(),
            })),
            config,
            notification_sender,
        };

        (queue, notification_receiver)
    }

    /// Add a transaction to the queue
    pub async fn enqueue(&self, mut transaction: Transaction) -> Result<()> {
        let mut inner = self.inner.write().await;
        
        // Check queue size limit
        if inner.total_transactions() >= self.config.max_size {
            drop(inner);
            let _ = self.notification_sender.send(QueueNotification::QueueFull);
            return Err(anyhow::anyhow!("Queue is full"));
        }

        // Set initial state if not set
        if transaction.state == TransactionState::Processing {
            transaction.state = TransactionState::Queued;
        }
        
        transaction.touch();

        // Add to appropriate queue based on priority
        let queue_entry = PriorityQueueEntry {
            transaction: transaction.clone(),
            priority: transaction.priority,
            created_at: transaction.metadata.created_at,
        };

        if self.config.enable_priority && transaction.priority >= TransactionPriority::High {
            inner.priority_queue.push(queue_entry);
        } else {
            inner.fifo_queue.push_back(transaction.clone());
        }

        // Store in lookup map
        inner.transactions.insert(transaction.id, transaction.clone());
        
        // Update statistics
        inner.update_stats();

        // Send notification
        let _ = self.notification_sender.send(QueueNotification::TransactionQueued {
            transaction_id: transaction.id,
        });

        Ok(())
    }

    /// Get the next transaction to process
    pub async fn dequeue(&self, worker_id: String) -> Result<Option<Transaction>> {
        let mut inner = self.inner.write().await;
        
        // Try priority queue first
        let transaction = if self.config.enable_priority {
            inner.priority_queue.pop().map(|entry| entry.transaction)
        } else {
            None
        };

        // Fall back to FIFO queue
        let transaction = transaction.or_else(|| inner.fifo_queue.pop_front());

        if let Some(mut tx) = transaction {
            // Update transaction state
            tx.state = TransactionState::Processing;
            tx.processing.attempt_count += 1;
            tx.processing.started_at = Some(Utc::now());
            tx.processing.worker_id = Some(worker_id.clone());
            tx.touch();

            // Move to processing map
            inner.processing.insert(tx.id, tx.clone());
            inner.transactions.insert(tx.id, tx.clone());
            
            // Update statistics
            inner.update_stats();

            // Send notification
            let _ = self.notification_sender.send(QueueNotification::TransactionStarted {
                transaction_id: tx.id,
                worker_id,
            });

            Ok(Some(tx))
        } else {
            Ok(None)
        }
    }

    /// Mark transaction as completed
    pub async fn complete_transaction(&self, transaction_id: Uuid, result: crate::transaction_engine::TransactionResult) -> Result<()> {
        let mut inner = self.inner.write().await;
        
        if let Some(mut transaction) = inner.transactions.remove(&transaction_id) {
            // Remove from processing
            inner.processing.remove(&transaction_id);
            
            // Update state
            transaction.state = TransactionState::Completed;
            transaction.touch();
            
            // Move to completed
            inner.completed.push(transaction.clone());
            
            // Update statistics
            inner.update_stats();

            // Send notification
            let _ = self.notification_sender.send(QueueNotification::TransactionCompleted {
                transaction_id,
                result,
            });

            Ok(())
        } else {
            Err(anyhow::anyhow!("Transaction not found: {}", transaction_id))
        }
    }

    /// Mark transaction as failed
    pub async fn fail_transaction(&self, transaction_id: Uuid, error: String, retryable: bool) -> Result<()> {
        let mut inner = self.inner.write().await;
        
        if let Some(mut transaction) = inner.transactions.remove(&transaction_id) {
            // Remove from processing
            inner.processing.remove(&transaction_id);
            
            // Update state and retry info
            if retryable {
                transaction.state = TransactionState::FailedRetryable;
                transaction.retry_info.attempt += 1;
                transaction.retry_info.last_failure = Some(crate::transaction_engine::TransactionFailure::Unknown(error.clone()));
                
                // Calculate next retry time
                transaction.retry_info.next_retry_at = Some(
                    Utc::now() + chrono::Duration::milliseconds(
                        transaction.retry_info.retry_strategy.next_delay_ms(transaction.retry_info.attempt) as i64
                    )
                );
                
                inner.failed_retryable.push(transaction.clone());
            } else {
                transaction.state = TransactionState::FailedPermanent;
                inner.failed_permanent.push(transaction.clone());
            }
            
            transaction.touch();
            
            // Update statistics
            inner.update_stats();

            // Send notification
            let _ = self.notification_sender.send(QueueNotification::TransactionFailed {
                transaction_id,
                error,
                retryable,
            });

            Ok(())
        } else {
            Err(anyhow::anyhow!("Transaction not found: {}", transaction_id))
        }
    }

    /// Cancel a transaction
    pub async fn cancel_transaction(&self, transaction_id: Uuid) -> Result<bool> {
        let mut inner = self.inner.write().await;
        
        // Try to find and remove from queues
        let mut found = false;
        
        // Check priority queue
        if let Some(pos) = inner.priority_queue.iter().position(|entry| entry.transaction.id == transaction_id) {
            inner.priority_queue.remove(pos);
            found = true;
        }
        
        // Check FIFO queue
        if !found {
            if let Some(pos) = inner.fifo_queue.iter().position(|tx| tx.id == transaction_id) {
                inner.fifo_queue.remove(pos);
                found = true;
            }
        }
        
        // Check processing
        if !found {
            if inner.processing.remove(&transaction_id).is_some() {
                found = true;
            }
        }
        
        if found {
            if let Some(mut transaction) = inner.transactions.remove(&transaction_id) {
                transaction.state = TransactionState::Cancelled;
                transaction.touch();
                inner.transactions.insert(transaction_id, transaction);
            }
            
            inner.update_stats();
            
            // Send notification
            let _ = self.notification_sender.send(QueueNotification::TransactionCancelled {
                transaction_id,
            });
        }
        
        Ok(found)
    }

    /// Get transaction by ID
    pub async fn get_transaction(&self, transaction_id: &Uuid) -> Result<Option<Transaction>> {
        let inner = self.inner.read().await;
        Ok(inner.transactions.get(transaction_id).cloned())
    }

    /// Get transactions matching filter
    pub async fn get_transactions(&self, filter: &TransactionFilter) -> Result<Vec<Transaction>> {
        let inner = self.inner.read().await;
        let mut transactions: Vec<Transaction> = inner.transactions.values().cloned().collect();
        
        // Apply filters
        if let Some(state) = &filter.state {
            transactions.retain(|tx| tx.state == *state);
        }
        
        if let Some(tx_type) = &filter.transaction_type {
            transactions.retain(|tx| tx.transaction_type == *tx_type);
        }
        
        if let Some(priority) = &filter.priority {
            transactions.retain(|tx| tx.priority == *priority);
        }
        
        if let Some(submitter) = &filter.submitter {
            transactions.retain(|tx| tx.metadata.submitter == *submitter);
        }
        
        if let Some(network) = &filter.network {
            transactions.retain(|tx| tx.metadata.network == *network);
        }
        
        if let Some(after) = &filter.created_after {
            transactions.retain(|tx| tx.metadata.created_at >= *after);
        }
        
        if let Some(before) = &filter.created_before {
            transactions.retain(|tx| tx.metadata.created_at <= *before);
        }
        
        if let Some(tags) = &filter.tags {
            transactions.retain(|tx| {
                tags.iter().all(|tag| tx.metadata.tags.contains(tag))
            });
        }
        
        // Sort by creation time (newest first)
        transactions.sort_by(|a, b| b.metadata.created_at.cmp(&a.metadata.created_at));
        
        // Apply pagination
        if let Some(offset) = filter.offset {
            transactions = transactions.into_iter().skip(offset).collect();
        }
        
        if let Some(limit) = filter.limit {
            transactions.truncate(limit);
        }
        
        Ok(transactions)
    }

    /// Get queue statistics
    pub async fn get_stats(&self) -> QueueStats {
        let inner = self.inner.read().await;
        inner.stats.clone()
    }

    /// Get retryable failed transactions ready for retry
    pub async fn get_retryable_transactions(&self) -> Result<Vec<Transaction>> {
        let inner = self.inner.read().await;
        let now = Utc::now();
        
        Ok(inner.failed_retryable.iter()
            .filter(|tx| {
                if let Some(retry_at) = tx.retry_info.next_retry_at {
                    retry_at <= now && tx.can_retry()
                } else {
                    false
                }
            })
            .cloned()
            .collect())
    }

    /// Requeue failed transaction for retry
    pub async fn requeue_transaction(&self, transaction_id: &Uuid) -> Result<()> {
        let mut inner = self.inner.write().await;
        
        // Find in failed_retryable
        if let Some(pos) = inner.failed_retryable.iter().position(|tx| tx.id == *transaction_id) {
            let mut transaction = inner.failed_retryable.remove(pos);
            
            // Reset for retry
            transaction.state = TransactionState::Queued;
            transaction.processing.started_at = None;
            transaction.processing.worker_id = None;
            transaction.processing.progress = 0;
            transaction.touch();
            
            // Re-add to queue
            let queue_entry = PriorityQueueEntry {
                transaction: transaction.clone(),
                priority: transaction.priority,
                created_at: transaction.metadata.created_at,
            };
            
            if self.config.enable_priority && transaction.priority >= TransactionPriority::High {
                inner.priority_queue.push(queue_entry);
            } else {
                inner.fifo_queue.push_back(transaction.clone());
            }
            
            // Update lookup
            inner.transactions.insert(*transaction_id, transaction);
            inner.update_stats();
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Retryable transaction not found: {}", transaction_id))
        }
    }

    /// Clear old completed transactions
    pub async fn cleanup_old_transactions(&self, max_age: chrono::Duration) -> Result<usize> {
        let mut inner = self.inner.write().await;
        let cutoff = Utc::now() - max_age;
        let mut removed = 0;
        
        // Clean completed transactions
        let initial_len = inner.completed.len();
        inner.completed.retain(|tx| tx.metadata.updated_at > cutoff);
        removed += initial_len - inner.completed.len();
        
        // Clean permanently failed transactions
        let initial_len = inner.failed_permanent.len();
        inner.failed_permanent.retain(|tx| tx.metadata.updated_at > cutoff);
        removed += initial_len - inner.failed_permanent.len();
        
        // Update lookup map
        inner.transactions.retain(|_, tx| tx.metadata.updated_at > cutoff);
        
        inner.update_stats();
        
        Ok(removed)
    }
}

impl QueueInner {
    fn total_transactions(&self) -> usize {
        self.priority_queue.len() + self.fifo_queue.len() + self.processing.len()
    }

    fn update_stats(&mut self) {
        self.stats.total_queued = self.priority_queue.len() + self.fifo_queue.len();
        self.stats.processing = self.processing.len();
        self.stats.completed = self.completed.len();
        self.stats.failed_retryable = self.failed_retryable.len();
        self.stats.failed_permanent = self.failed_permanent.len();
        
        // Calculate oldest transaction age
        let mut oldest_age = None;
        
        for entry in &self.priority_queue {
            let age = (Utc::now() - entry.created_at).num_seconds();
            if oldest_age.is_none() || age < oldest_age.unwrap() {
                oldest_age = Some(age);
            }
        }
        
        if let Some(tx) = self.fifo_queue.front() {
            let age = (Utc::now() - tx.metadata.created_at).num_seconds();
            if oldest_age.is_none() || age < oldest_age.unwrap() {
                oldest_age = Some(age);
            }
        }
        
        self.stats.oldest_transaction_age_seconds = oldest_age.map(|age| age.max(0) as u64);
    }
}

impl crate::transaction_engine::RetryStrategy {
    /// Calculate next delay in milliseconds
    pub fn next_delay_ms(&self, attempt: u32) -> u64 {
        match self {
            crate::transaction_engine::RetryStrategy::Fixed { delay_ms } => *delay_ms,
            crate::transaction_engine::RetryStrategy::ExponentialBackoff { base_delay_ms, max_delay_ms, multiplier, jitter } => {
                let delay = (*base_delay_ms as f64) * multiplier.powi(attempt as i32 - 1);
                let delay = delay.min(*max_delay_ms as f64) as u64;
                
                if *jitter {
                    // Add ±25% jitter
                    let jitter_range = delay / 4;
                    delay + (rand::random::<u64>() % (2 * jitter_range + 1)) - jitter_range
                } else {
                    delay
                }
            },
            crate::transaction_engine::RetryStrategy::Linear { base_delay_ms, increment_ms } => {
                base_delay_ms + (increment_ms * (attempt - 1))
            },
            crate::transaction_engine::RetryStrategy::NoRetry => 0,
        }
    }
}
