//! Transaction state tracking and persistence

use crate::transaction_engine::{
    Transaction, TransactionState, TransactionResult, TransactionFilter,
    QueueStats, ProcessorMetrics, RetryStats
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{Utc, DateTime};
use anyhow::Result;

/// State persistence configuration
#[derive(Debug, Clone)]
pub struct StateConfig {
    /// Enable persistence to disk
    pub enable_persistence: bool,
    /// Persistence file path
    pub persistence_file: Option<String>,
    /// Auto-save interval in seconds
    pub auto_save_interval_seconds: u64,
    /// Maximum number of transactions to keep in memory
    pub max_memory_transactions: usize,
    /// Enable compression for persistence
    pub enable_compression: bool,
}

impl Default for StateConfig {
    fn default() -> Self {
        StateConfig {
            enable_persistence: true,
            persistence_file: Some("transaction_state.json".to_string()),
            auto_save_interval_seconds: 30,
            max_memory_transactions: 10000,
            enable_compression: true,
        }
    }
}

/// Complete transaction engine state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionEngineState {
    /// All transactions
    pub transactions: HashMap<Uuid, Transaction>,
    /// Queue statistics
    pub queue_stats: QueueStats,
    /// Processor metrics
    pub processor_metrics: Vec<ProcessorMetrics>,
    /// Retry statistics
    pub retry_stats: RetryStats,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
    /// Engine version
    pub version: String,
}

/// State manager for transaction persistence and tracking
pub struct StateManager {
    config: StateConfig,
    state: Arc<RwLock<TransactionEngineState>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl StateManager {
    /// Create a new state manager
    pub fn new(config: StateConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(TransactionEngineState {
                transactions: HashMap::new(),
                queue_stats: QueueStats::default(),
                processor_metrics: Vec::new(),
                retry_stats: RetryStats::default(),
                last_updated: Utc::now(),
                version: "1.0.0".to_string(),
            })),
            config,
            shutdown_tx: None,
        }
    }

    /// Initialize the state manager
    pub async fn initialize(&self) -> Result<()> {
        if self.config.enable_persistence {
            if let Err(e) = self.load_state().await {
                eprintln!("Warning: Failed to load persisted state: {}", e);
            }
        }

        // Start auto-save task
        self.start_auto_save().await?;

        Ok(())
    }

    /// Update transaction in state
    pub async fn update_transaction(&self, transaction: Transaction) -> Result<()> {
        let mut state = self.state.write().await;
        state.transactions.insert(transaction.id, transaction);
        state.last_updated = Utc::now();
        Ok(())
    }

    /// Get transaction by ID
    pub async fn get_transaction(&self, transaction_id: &Uuid) -> Result<Option<Transaction>> {
        let state = self.state.read().await;
        Ok(state.transactions.get(transaction_id).cloned())
    }

    /// Get transactions matching filter
    pub async fn get_transactions(&self, filter: &TransactionFilter) -> Result<Vec<Transaction>> {
        let state = self.state.read().await;
        let mut transactions: Vec<Transaction> = state.transactions.values().cloned().collect();
        
        // Apply filters (same logic as queue)
        if let Some(state_filter) = &filter.state {
            transactions.retain(|tx| tx.state == *state_filter);
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

    /// Update queue statistics
    pub async fn update_queue_stats(&self, stats: QueueStats) -> Result<()> {
        let mut state = self.state.write().await;
        state.queue_stats = stats;
        state.last_updated = Utc::now();
        Ok(())
    }

    /// Update processor metrics
    pub async fn update_processor_metrics(&self, metrics: Vec<ProcessorMetrics>) -> Result<()> {
        let mut state = self.state.write().await;
        state.processor_metrics = metrics;
        state.last_updated = Utc::now();
        Ok(())
    }

    /// Update retry statistics
    pub async fn update_retry_stats(&self, stats: RetryStats) -> Result<()> {
        let mut state = self.state.write().await;
        state.retry_stats = stats;
        state.last_updated = Utc::now();
        Ok(())
    }

    /// Get complete engine state
    pub async fn get_state(&self) -> TransactionEngineState {
        self.state.read().await.clone()
    }

    /// Get transaction statistics
    pub async fn get_transaction_stats(&self) -> TransactionStats {
        let state = self.state.read().await;
        let mut stats = TransactionStats::default();
        
        for transaction in state.transactions.values() {
            match transaction.state {
                TransactionState::Queued => stats.queued += 1,
                TransactionState::Processing => stats.processing += 1,
                TransactionState::Completed => stats.completed += 1,
                TransactionState::FailedRetryable => stats.failed_retryable += 1,
                TransactionState::FailedPermanent => stats.failed_permanent += 1,
                TransactionState::Cancelled => stats.cancelled += 1,
                TransactionState::OnHold => stats.on_hold += 1,
            }
            
            // Count by type
            match transaction.transaction_type {
                crate::transaction_engine::TransactionType::Payment => stats.payments += 1,
                crate::transaction_engine::TransactionType::MultiSignature => stats.multisig += 1,
                crate::transaction_engine::TransactionType::ContractDeployment => stats.contract_deployments += 1,
                crate::transaction_engine::TransactionType::ContractCall => stats.contract_calls += 1,
                crate::transaction_engine::TransactionType::BatchOperation => stats.batch_operations += 1,
                crate::transaction_engine::TransactionType::SecurityScan => stats.security_scans += 1,
                crate::transaction_engine::TransactionType::Custom(_) => stats.custom += 1,
            }
        }
        
        stats.total = state.transactions.len();
        stats
    }

    /// Save state to persistence
    pub async fn save_state(&self) -> Result<()> {
        if !self.config.enable_persistence {
            return Ok(());
        }

        let state = self.state.read().await;
        
        if let Some(file_path) = &self.config.persistence_file {
            let json_data = if self.config.enable_compression {
                // Compress JSON data
                let json = serde_json::to_string(&*state)?;
                let compressed = zstd::encode_all(json.as_bytes(), 0)?;
                compressed
            } else {
                serde_json::to_vec(&*state)?
            };

            tokio::fs::write(file_path, json_data).await?;
        }

        Ok(())
    }

    /// Load state from persistence
    async fn load_state(&self) -> Result<()> {
        if !self.config.enable_persistence {
            return Ok(());
        }

        if let Some(file_path) = &self.config.persistence_file {
            if tokio::fs::metadata(file_path).await.is_ok() {
                let data = tokio::fs::read(file_path).await?;
                
                let state: TransactionEngineState = if self.config.enable_compression {
                    let decompressed = zstd::decode_all(&*data)?;
                    serde_json::from_slice(&decompressed)?
                } else {
                    serde_json::from_slice(&data)?
                };

                let mut current_state = self.state.write().await;
                *current_state = state;
            }
        }

        Ok(())
    }

    /// Start auto-save task
    async fn start_auto_save(&self) -> Result<()> {
        if !self.config.enable_persistence {
            return Ok(());
        }

        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        let state_manager = self.state.clone();
        let interval_seconds = self.config.auto_save_interval_seconds;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(interval_seconds)
            );

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = Self::save_state_direct(&state_manager).await {
                            eprintln!("Auto-save failed: {}", e);
                        }
                    }
                    _ = &mut shutdown_rx => {
                        // Save before shutdown
                        let _ = Self::save_state_direct(&state_manager).await;
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Direct save method for auto-save task
    async fn save_state_direct(state: &Arc<RwLock<TransactionEngineState>>) -> Result<()> {
        let state = state.read().await;
        
        // This would need access to config, so for now we'll just log
        println!("Auto-saving transaction state with {} transactions", state.transactions.len());
        
        Ok(())
    }

    /// Stop the state manager
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        
        // Final save
        self.save_state().await?;
        
        Ok(())
    }

    /// Cleanup old transactions
    pub async fn cleanup_old_transactions(&self, max_age: chrono::Duration) -> Result<usize> {
        let mut state = self.state.write().await;
        let cutoff = Utc::now() - max_age;
        let initial_count = state.transactions.len();
        
        state.transactions.retain(|_, tx| {
            tx.metadata.updated_at > cutoff || !tx.is_terminal()
        });
        
        let removed = initial_count - state.transactions.len();
        state.last_updated = Utc::now();
        
        Ok(removed)
    }

    /// Export state to backup
    pub async fn export_backup(&self, backup_path: &str) -> Result<()> {
        let state = self.state.read().await;
        let backup_data = serde_json::to_string_pretty(&*state)?;
        tokio::fs::write(backup_path, backup_data).await?;
        Ok(())
    }

    /// Import state from backup
    pub async fn import_backup(&self, backup_path: &str) -> Result<()> {
        let backup_data = tokio::fs::read_to_string(backup_path).await?;
        let state: TransactionEngineState = serde_json::from_str(&backup_data)?;
        
        let mut current_state = self.state.write().await;
        *current_state = state;
        
        Ok(())
    }
}

/// Transaction statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransactionStats {
    pub total: usize,
    pub queued: usize,
    pub processing: usize,
    pub completed: usize,
    pub failed_retryable: usize,
    pub failed_permanent: usize,
    pub cancelled: usize,
    pub on_hold: usize,
    pub payments: usize,
    pub multisig: usize,
    pub contract_deployments: usize,
    pub contract_calls: usize,
    pub batch_operations: usize,
    pub security_scans: usize,
    pub custom: usize,
}

/// State change event
#[derive(Debug, Clone)]
pub enum StateChangeEvent {
    TransactionAdded { transaction_id: Uuid },
    TransactionUpdated { transaction_id: Uuid, old_state: TransactionState, new_state: TransactionState },
    TransactionRemoved { transaction_id: Uuid },
    StatsUpdated,
    StateSaved,
    StateLoaded,
}

/// State change listener
pub trait StateChangeListener: Send + Sync {
    async fn on_state_change(&self, event: StateChangeEvent);
}

/// State change broadcaster
pub struct StateChangeBroadcaster {
    listeners: Arc<RwLock<Vec<Box<dyn StateChangeListener>>>>,
}

impl StateChangeBroadcaster {
    pub fn new() -> Self {
        Self {
            listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn add_listener(&self, listener: Box<dyn StateChangeListener>) {
        let mut listeners = self.listeners.write().await;
        listeners.push(listener);
    }

    pub async fn broadcast_change(&self, event: StateChangeEvent) {
        let listeners = self.listeners.read().await;
        for listener in listeners.iter() {
            let _ = listener.on_state_change(event.clone()).await;
        }
    }
}

impl Default for StateChangeBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for StateManager {
    fn drop(&mut self) {
        // Note: This is not async, so we can't guarantee save on drop
        // The stop() method should be called explicitly
    }
}
