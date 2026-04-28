//! Transaction types and data structures

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Transaction priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TransactionPriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
    Emergency = 5,
}

impl Default for TransactionPriority {
    fn default() -> Self {
        TransactionPriority::Normal
    }
}

/// Transaction states throughout their lifecycle
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionState {
    /// Transaction is queued and waiting to be processed
    Queued,
    /// Transaction is currently being processed
    Processing,
    /// Transaction was successfully processed
    Completed,
    /// Transaction failed but will be retried
    FailedRetryable,
    /// Transaction failed permanently and will not be retried
    FailedPermanent,
    /// Transaction was cancelled before processing
    Cancelled,
    /// Transaction is on hold pending external action
    OnHold,
}

/// Transaction types supported by the engine
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    /// Standard payment transaction
    Payment,
    /// Multi-signature transaction
    MultiSignature,
    /// Contract deployment
    ContractDeployment,
    /// Contract function call
    ContractCall,
    /// Batch operation
    BatchOperation,
    /// Security scan transaction
    SecurityScan,
    /// Custom transaction type
    Custom(String),
}

/// Transaction failure reasons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionFailure {
    /// Network connectivity issues
    NetworkError(String),
    /// Invalid transaction format
    InvalidFormat(String),
    /// Insufficient funds
    InsufficientFunds,
    /// Invalid signature
    InvalidSignature,
    /// Timeout occurred
    Timeout,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Stellar network error
    StellarError(String),
    /// Validation failed
    ValidationFailed(String),
    /// Unknown error
    Unknown(String),
}

/// Core transaction structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Unique transaction identifier
    pub id: Uuid,
    /// Transaction type
    pub transaction_type: TransactionType,
    /// Current state of the transaction
    pub state: TransactionState,
    /// Transaction priority
    pub priority: TransactionPriority,
    /// Raw transaction data
    pub data: Vec<u8>,
    /// Transaction metadata
    pub metadata: TransactionMetadata,
    /// Processing information
    pub processing: ProcessingInfo,
    /// Retry information
    pub retry_info: RetryInfo,
}

/// Transaction metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMetadata {
    /// Transaction creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Transaction creator/submitter
    pub submitter: String,
    /// Target network (mainnet, testnet, etc.)
    pub network: String,
    /// Optional transaction description
    pub description: Option<String>,
    /// Transaction tags for categorization
    pub tags: Vec<String>,
    /// Additional custom metadata
    pub custom_fields: HashMap<String, String>,
}

/// Processing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingInfo {
    /// Number of times this transaction has been processed
    pub attempt_count: u32,
    /// Maximum allowed attempts
    pub max_attempts: u32,
    /// Processing start time (if currently processing)
    pub started_at: Option<DateTime<Utc>>,
    /// Estimated processing duration
    pub estimated_duration: Option<std::time::Duration>,
    /// Worker ID processing this transaction
    pub worker_id: Option<String>,
    /// Processing progress (0-100)
    pub progress: u8,
}

/// Retry information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryInfo {
    /// Current retry attempt
    pub attempt: u32,
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Next retry timestamp
    pub next_retry_at: Option<DateTime<Utc>>,
    /// Retry delay strategy
    pub retry_strategy: RetryStrategy,
    /// Last failure reason
    pub last_failure: Option<TransactionFailure>,
}

/// Retry strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryStrategy {
    /// Fixed delay between retries
    Fixed { delay_ms: u64 },
    /// Exponential backoff with jitter
    ExponentialBackoff {
        base_delay_ms: u64,
        max_delay_ms: u64,
        multiplier: f64,
        jitter: bool,
    },
    /// Linear backoff
    Linear { base_delay_ms: u64, increment_ms: u64 },
    /// No retries
    NoRetry,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        RetryStrategy::ExponentialBackoff {
            base_delay_ms: 1000,
            max_delay_ms: 30000,
            multiplier: 2.0,
            jitter: true,
        }
    }
}

/// Transaction result after processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResult {
    /// Transaction ID
    pub transaction_id: Uuid,
    /// Success status
    pub success: bool,
    /// Result data (if successful)
    pub data: Option<Vec<u8>>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Processing duration
    pub duration: std::time::Duration,
    /// Gas used (if applicable)
    pub gas_used: Option<u64>,
    /// Transaction hash (if submitted to blockchain)
    pub transaction_hash: Option<String>,
    /// Block number (if confirmed)
    pub block_number: Option<u64>,
}

/// Queue configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    /// Maximum queue size
    pub max_size: usize,
    /// Number of concurrent workers
    pub concurrent_workers: usize,
    /// Worker poll interval
    pub poll_interval_ms: u64,
    /// Batch size for processing
    pub batch_size: usize,
    /// Enable priority queue
    pub enable_priority: bool,
    /// Queue timeout
    pub timeout_ms: Option<u64>,
}

impl Default for QueueConfig {
    fn default() -> Self {
        QueueConfig {
            max_size: 10000,
            concurrent_workers: 4,
            poll_interval_ms: 100,
            batch_size: 10,
            enable_priority: true,
            timeout_ms: Some(300000), // 5 minutes
        }
    }
}

/// Transaction filter for querying
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransactionFilter {
    /// Filter by transaction state
    pub state: Option<TransactionState>,
    /// Filter by transaction type
    pub transaction_type: Option<TransactionType>,
    /// Filter by priority
    pub priority: Option<TransactionPriority>,
    /// Filter by submitter
    pub submitter: Option<String>,
    /// Filter by network
    pub network: Option<String>,
    /// Filter by creation time range
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    /// Filter by tags
    pub tags: Option<Vec<String>>,
    /// Limit results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(
        transaction_type: TransactionType,
        data: Vec<u8>,
        submitter: String,
        network: String,
    ) -> Self {
        let now = Utc::now();
        let id = Uuid::new_v4();
        
        Transaction {
            id,
            transaction_type,
            state: TransactionState::Queued,
            priority: TransactionPriority::default(),
            data,
            metadata: TransactionMetadata {
                created_at: now,
                updated_at: now,
                submitter,
                network,
                description: None,
                tags: Vec::new(),
                custom_fields: HashMap::new(),
            },
            processing: ProcessingInfo {
                attempt_count: 0,
                max_attempts: 3,
                started_at: None,
                estimated_duration: None,
                worker_id: None,
                progress: 0,
            },
            retry_info: RetryInfo {
                attempt: 0,
                max_attempts: 3,
                next_retry_at: None,
                retry_strategy: RetryStrategy::default(),
                last_failure: None,
            },
        }
    }

    /// Check if transaction can be retried
    pub fn can_retry(&self) -> bool {
        match self.state {
            TransactionState::FailedRetryable => {
                self.retry_info.attempt < self.retry_info.max_attempts
            }
            _ => false,
        }
    }

    /// Check if transaction is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            TransactionState::Completed | TransactionState::FailedPermanent | TransactionState::Cancelled
        )
    }

    /// Get transaction age
    pub fn age(&self) -> chrono::Duration {
        Utc::now() - self.metadata.created_at
    }

    /// Update transaction timestamp
    pub fn touch(&mut self) {
        self.metadata.updated_at = Utc::now();
    }
}
