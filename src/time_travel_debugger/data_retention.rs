use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub contract_state_retention_days: u32,
    pub ledger_snapshot_retention_days: u32,
    pub audit_log_retention_days: u32,
    pub orphaned_state_retention_days: u32,
    pub max_stored_states_per_contract: usize,
    pub auto_cleanup_enabled: bool,
    pub cleanup_interval_hours: u32,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            contract_state_retention_days: 30,
            ledger_snapshot_retention_days: 14,
            audit_log_retention_days: 90,
            orphaned_state_retention_days: 7,
            max_stored_states_per_contract: 100,
            auto_cleanup_enabled: true,
            cleanup_interval_hours: 24,
        }
    }
}

impl RetentionPolicy {
    pub fn strict() -> Self {
        Self {
            contract_state_retention_days: 7,
            ledger_snapshot_retention_days: 3,
            audit_log_retention_days: 30,
            orphaned_state_retention_days: 1,
            max_stored_states_per_contract: 10,
            auto_cleanup_enabled: true,
            cleanup_interval_hours: 6,
        }
    }

    pub fn enterprise() -> Self {
        Self {
            contract_state_retention_days: 90,
            ledger_snapshot_retention_days: 30,
            audit_log_retention_days: 365,
            orphaned_state_retention_days: 30,
            max_stored_states_per_contract: 500,
            auto_cleanup_enabled: true,
            cleanup_interval_hours: 12,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRetentionMetadata {
    pub contract_id: String,
    pub ledger_sequence: u32,
    pub stored_at: Instant,
    pub data_type: StoredDataType,
    pub size_bytes: usize,
    pub encrypted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoredDataType {
    ContractState,
    LedgerSnapshot,
    OrphanedStateEntry,
}

pub struct DataRetentionManager {
    policy: Arc<RwLock<RetentionPolicy>>,
    metadata: Arc<RwLock<Vec<DataRetentionMetadata>>>,
    last_cleanup: Arc<RwLock<Instant>>,
}

impl DataRetentionManager {
    pub fn new(policy: RetentionPolicy) -> Self {
        Self {
            policy: Arc::new(RwLock::new(policy)),
            metadata: Arc::new(RwLock::new(Vec::new())),
            last_cleanup: Arc::new(RwLock::new(Instant::now())),
        }
    }

    pub async fn record_storage(
        &self,
        contract_id: &str,
        ledger_sequence: u32,
        data_type: StoredDataType,
        size_bytes: usize,
        encrypted: bool,
    ) -> Result<()> {
        let policy = self.policy.read().await;

        if data_type == StoredDataType::ContractState {
            let count = {
                let meta = self.metadata.read().await;
                meta.iter()
                    .filter(|m| {
                        m.contract_id == contract_id
                            && m.data_type == StoredDataType::ContractState
                    })
                    .count()
            };

            if count >= policy.max_stored_states_per_contract {
                return Err(anyhow!(
                    "Maximum stored states per contract ({}) exceeded",
                    policy.max_stored_states_per_contract
                ));
            }
        }

        let entry = DataRetentionMetadata {
            contract_id: contract_id.to_string(),
            ledger_sequence,
            stored_at: Instant::now(),
            data_type,
            size_bytes,
            encrypted,
        };

        let mut meta = self.metadata.write().await;
        meta.push(entry);
        Ok(())
    }

    pub async fn perform_cleanup(&self) -> RetentionCleanupReport {
        let mut dirty = false;

        let needs_cleanup = {
            let last = self.last_cleanup.read().await;
            let policy = self.policy.read().await;
            last.elapsed() > Duration::from_secs(policy.cleanup_interval_hours as u64 * 3600)
        };

        if !needs_cleanup {
            return RetentionCleanupReport {
                entries_removed: 0,
                bytes_freed: 0,
                details: "Cleanup not yet needed".to_string(),
            };
        }

        let policy = self.policy.read().await;
        let now = Instant::now();
        let mut meta = self.metadata.write().await;

        let retention_durations: HashMap<StoredDataType, Duration> = [
            (
                StoredDataType::ContractState,
                Duration::from_secs(policy.contract_state_retention_days as u64 * 86400),
            ),
            (
                StoredDataType::LedgerSnapshot,
                Duration::from_secs(policy.ledger_snapshot_retention_days as u64 * 86400),
            ),
            (
                StoredDataType::OrphanedStateEntry,
                Duration::from_secs(policy.orphaned_state_retention_days as u64 * 86400),
            ),
        ]
        .into();

        let original_len = meta.len();
        let original_size: usize = meta.iter().map(|m| m.size_bytes).sum();

        meta.retain(|entry| {
            let retention = retention_durations.get(&entry.data_type).unwrap();
            now.duration_since(entry.stored_at) < *retention
        });

        let new_size: usize = meta.iter().map(|m| m.size_bytes).sum();
        let removed = original_len - meta.len();
        let freed = original_size - new_size;
        dirty = dirty || removed > 0;

        if dirty {
            let mut last = self.last_cleanup.write().await;
            *last = Instant::now();
        }

        RetentionCleanupReport {
            entries_removed: removed,
            bytes_freed: freed,
            details: format!(
                "Cleaned up {} entries, freed {} bytes",
                removed, freed
            ),
        }
    }

    pub async fn get_retention_policy(&self) -> RetentionPolicy {
        self.policy.read().await.clone()
    }

    pub async fn update_retention_policy(&self, policy: RetentionPolicy) -> Result<()> {
        let mut current = self.policy.write().await;
        *current = policy;
        Ok(())
    }

    pub async fn get_storage_usage(&self) -> StorageUsage {
        let meta = self.metadata.read().await;
        let mut usage = StorageUsage::default();

        for entry in meta.iter() {
            match entry.data_type {
                StoredDataType::ContractState => {
                    usage.contract_state_count += 1;
                    usage.contract_state_bytes += entry.size_bytes;
                }
                StoredDataType::LedgerSnapshot => {
                    usage.ledger_snapshot_count += 1;
                    usage.ledger_snapshot_bytes += entry.size_bytes;
                }
                StoredDataType::OrphanedStateEntry => {
                    usage.orphaned_state_count += 1;
                    usage.orphaned_state_bytes += entry.size_bytes;
                }
            }
        }

        usage.total_bytes =
            usage.contract_state_bytes + usage.ledger_snapshot_bytes + usage.orphaned_state_bytes;
        usage
    }

    pub async fn get_contract_state_count(&self, contract_id: &str) -> usize {
        let meta = self.metadata.read().await;
        meta.iter()
            .filter(|m| {
                m.contract_id == contract_id && m.data_type == StoredDataType::ContractState
            })
            .count()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageUsage {
    pub contract_state_count: usize,
    pub contract_state_bytes: usize,
    pub ledger_snapshot_count: usize,
    pub ledger_snapshot_bytes: usize,
    pub orphaned_state_count: usize,
    pub orphaned_state_bytes: usize,
    pub total_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionCleanupReport {
    pub entries_removed: usize,
    pub bytes_freed: usize,
    pub details: String,
}
