//! Orphaned State Tracker
//! 
//! This module tracks storage entries that are no longer accessible by contract code
//! after upgrades or changes.

use std::collections::{HashMap, HashSet};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use soroban_sdk::xdr::ScVal;
use crate::time_travel_debugger::{ContractState, StorageLayoutInfo};

/// Tracks orphaned state entries that become inaccessible after contract changes
pub struct OrphanedStateTracker {
    orphaned_entries: HashMap<String, Vec<OrphanedEntry>>,
}

impl OrphanedStateTracker {
    /// Create a new orphaned state tracker
    pub fn new() -> Self {
        Self {
            orphaned_entries: HashMap::new(),
        }
    }

    /// Find orphaned storage entries for a contract after upgrade
    pub async fn find_orphaned_entries(
        &self,
        current_state: &ContractState,
        new_wasm: &[u8],
    ) -> Result<Vec<String>> {
        // Parse new WASM to get expected storage layout
        let new_layout = self.parse_new_wasm_layout(new_wasm).await?;
        
        // Find entries that exist in current state but not in new layout
        let mut orphaned_keys = Vec::new();
        
        for (key, value) in &current_state.storage {
            if !self.is_key_accessible_in_new_layout(key, &new_layout) {
                orphaned_keys.push(key.clone());
                
                // Store detailed information about the orphaned entry
                let orphaned_entry = OrphanedEntry {
                    key: key.clone(),
                    value_type: self.get_value_type(value),
                    size_bytes: self.estimate_value_size(value),
                    last_accessed: None, // Would need to track this historically
                    recovery_possible: self.is_recovery_possible(key, value),
                    data_loss_risk: self.assess_data_loss_risk(key, value),
                };
                
                self.orphaned_entries
                    .entry(current_state.contract_id.clone())
                    .or_insert_with(Vec::new)
                    .push(orphaned_entry);
            }
        }
        
        Ok(orphaned_keys)
    }

    /// Find orphaned entries given current state and new layout
    pub async fn find_orphaned_entries_with_layout(
        &self,
        current_state: &ContractState,
        new_layout: &HashMap<String, StorageLayoutInfo>,
    ) -> Result<Vec<String>> {
        let mut orphaned_keys = Vec::new();
        
        for (key, value) in &current_state.storage {
            if !new_layout.contains_key(key) && key != "instance" {
                orphaned_keys.push(key.clone());
                
                // Store detailed information
                let orphaned_entry = OrphanedEntry {
                    key: key.clone(),
                    value_type: self.get_value_type(value),
                    size_bytes: self.estimate_value_size(value),
                    last_accessed: None,
                    recovery_possible: self.is_recovery_possible(key, value),
                    data_loss_risk: self.assess_data_loss_risk(key, value),
                };
                
                self.orphaned_entries
                    .entry(current_state.contract_id.clone())
                    .or_insert_with(Vec::new)
                    .push(orphaned_entry);
            }
        }
        
        Ok(orphaned_keys)
    }

    /// Parse new WASM to extract expected storage layout
    async fn parse_new_wasm_layout(&self, wasm: &[u8]) -> Result<HashMap<String, StorageLayoutInfo>> {
        let mut layout = HashMap::new();
        
        // In a real implementation, this would parse the WASM to extract storage patterns
        // For now, we'll simulate with some common patterns
        let wasm_string = String::from_utf8_lossy(wasm);
        
        // Look for storage-related patterns
        if wasm_string.contains("balance") {
            layout.insert("balance".to_string(), StorageLayoutInfo {
                storage_type: crate::time_travel_debugger::contract_upgrade::StorageType::Persistent,
                required: false,
                description: "Token balance".to_string(),
            });
        }
        
        if wasm_string.contains("allowance") {
            layout.insert("allowance".to_string(), StorageLayoutInfo {
                storage_type: crate::time_travel_debugger::contract_upgrade::StorageType::Persistent,
                required: false,
                description: "Token allowance".to_string(),
            });
        }
        
        if wasm_string.contains("owner") {
            layout.insert("owner".to_string(), StorageLayoutInfo {
                storage_type: crate::time_travel_debugger::contract_upgrade::StorageType::Persistent,
                required: true,
                description: "Contract owner".to_string(),
            });
        }
        
        // Always include instance storage
        layout.insert("instance".to_string(), StorageLayoutInfo {
            storage_type: crate::time_travel_debugger::contract_upgrade::StorageType::Instance,
            required: true,
            description: "Contract instance data".to_string(),
        });
        
        Ok(layout)
    }

    /// Check if a storage key is accessible in the new layout
    fn is_key_accessible_in_new_layout(
        &self,
        key: &str,
        new_layout: &HashMap<String, StorageLayoutInfo>,
    ) -> bool {
        // Instance storage is always accessible
        if key == "instance" {
            return true;
        }
        
        // Check if the key exists in the new layout
        new_layout.contains_key(key)
    }

    /// Get the type of a storage value
    fn get_value_type(&self, value: &ScVal) -> String {
        match value {
            ScVal::Void => "void".to_string(),
            ScVal::Bool(_) => "bool".to_string(),
            ScVal::U32(_) => "u32".to_string(),
            ScVal::I32(_) => "i32".to_string(),
            ScVal::U64(_) => "u64".to_string(),
            ScVal::I64(_) => "i64".to_string(),
            ScVal::Bytes(_) => "bytes".to_string(),
            ScVal::String(_) => "string".to_string(),
            ScVal::Symbol(_) => "symbol".to_string(),
            ScVal::Map(_) => "map".to_string(),
            ScVal::Vec(_) => "vec".to_string(),
            _ => "unknown".to_string(),
        }
    }

    /// Estimate the size of a storage value in bytes
    fn estimate_value_size(&self, value: &ScVal) -> usize {
        match value {
            ScVal::Void => 1,
            ScVal::Bool(_) => 1,
            ScVal::U32(_) => 4,
            ScVal::I32(_) => 4,
            ScVal::U64(_) => 8,
            ScVal::I64(_) => 8,
            ScVal::Bytes(bytes) => bytes.len(),
            ScVal::String(string) => string.len(),
            ScVal::Symbol(symbol) => symbol.len(),
            ScVal::Map(map) => {
                let mut size = 8; // Base map size
                for _ in map {
                    size += 16; // Rough estimate per entry
                }
                size
            }
            ScVal::Vec(vec) => {
                let mut size = 8; // Base vec size
                for _ in vec {
                    size += 8; // Rough estimate per element
                }
                size
            }
            _ => 32, // Default estimate for complex types
        }
    }

    /// Check if recovery of orphaned data is possible
    fn is_recovery_possible(&self, key: &str, value: &ScVal) -> bool {
        // Recovery is possible for simple, serializable data
        match value {
            ScVal::U32(_) | ScVal::I32(_) | ScVal::U64(_) | ScVal::I64(_) | ScVal::Bool(_) => true,
            ScVal::Bytes(bytes) if bytes.len() <= 1024 => true, // Small byte arrays
            ScVal::String(string) if string.len() <= 256 => true, // Short strings
            _ => false,
        }
    }

    /// Assess the risk of data loss for an orphaned entry
    fn assess_data_loss_risk(&self, key: &str, value: &ScVal) -> DataLossRisk {
        // High-risk keys
        if key.contains("balance") || key.contains("total_supply") || key.contains("owner") {
            return DataLossRisk::High;
        }
        
        // Medium-risk keys
        if key.contains("allowance") || key.contains("approval") || key.contains("metadata") {
            return DataLossRisk::Medium;
        }
        
        // Low-risk keys
        if key.contains("temp") || key.contains("cache") || key.contains("counter") {
            return DataLossRisk::Low;
        }
        
        // Default to medium for unknown keys
        DataLossRisk::Medium
    }

    /// Get detailed orphaned entries for a contract
    pub fn get_orphaned_entries(&self, contract_id: &str) -> Vec<OrphanedEntry> {
        self.orphaned_entries
            .get(contract_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get orphaned entries summary for a contract
    pub fn get_orphaned_summary(&self, contract_id: &str) -> OrphanedSummary {
        let entries = self.get_orphaned_entries(contract_id);
        
        let total_entries = entries.len();
        let total_size_bytes: usize = entries.iter().map(|e| e.size_bytes).sum();
        let high_risk_count = entries.iter().filter(|e| e.data_loss_risk == DataLossRisk::High).count();
        let recoverable_count = entries.iter().filter(|e| e.recovery_possible).count();
        
        let risk_level = if high_risk_count > 0 {
            OverallRisk::High
        } else if total_entries > 10 {
            OverallRisk::Medium
        } else {
            OverallRisk::Low
        };
        
        OrphanedSummary {
            total_entries,
            total_size_bytes,
            high_risk_count,
            recoverable_count,
            risk_level,
        }
    }

    /// Generate recovery recommendations for orphaned entries
    pub fn generate_recovery_recommendations(&self, contract_id: &str) -> Vec<RecoveryRecommendation> {
        let entries = self.get_orphaned_entries(contract_id);
        let mut recommendations = Vec::new();
        
        // Group entries by type for batch recommendations
        let mut balance_entries = Vec::new();
        let mut metadata_entries = Vec::new();
        let mut other_entries = Vec::new();
        
        for entry in &entries {
            if entry.key.contains("balance") {
                balance_entries.push(entry);
            } else if entry.key.contains("metadata") || entry.key.contains("config") {
                metadata_entries.push(entry);
            } else {
                other_entries.push(entry);
            }
        }
        
        // Generate recommendations for each group
        if !balance_entries.is_empty() {
            recommendations.push(RecoveryRecommendation {
                priority: RecommendationPriority::Critical,
                category: "Balance Data".to_string(),
                description: format!("{} balance entries will be lost. Consider implementing a migration function to preserve user funds.", balance_entries.len()),
                action: "Create migration function to transfer balances to new storage format".to_string(),
                estimated_effort: "High".to_string(),
            });
        }
        
        if !metadata_entries.is_empty() {
            recommendations.push(RecoveryRecommendation {
                priority: RecommendationPriority::High,
                category: "Metadata".to_string(),
                description: format!("{} metadata entries will be lost. This may affect contract functionality.", metadata_entries.len()),
                action: "Export metadata before upgrade and re-import after".to_string(),
                estimated_effort: "Medium".to_string(),
            });
        }
        
        if !other_entries.is_empty() {
            recommendations.push(RecoveryRecommendation {
                priority: RecommendationPriority::Medium,
                category: "Other Data".to_string(),
                description: format!("{} other storage entries will be orphaned.", other_entries.len()),
                action: "Review and determine if data needs to be preserved".to_string(),
                estimated_effort: "Low".to_string(),
            });
        }
        
        recommendations
    }

    /// Clear orphaned entries for a contract
    pub fn clear_orphaned_entries(&mut self, contract_id: &str) {
        self.orphaned_entries.remove(contract_id);
    }

    /// Get all contracts with orphaned entries
    pub fn get_contracts_with_orphans(&self) -> Vec<String> {
        self.orphaned_entries.keys().cloned().collect()
    }

    /// Analyze orphaned patterns across multiple contracts
    pub fn analyze_orphaned_patterns(&self) -> OrphanedPatterns {
        let mut common_keys = HashMap::new();
        let mut risk_distribution = HashMap::new();
        let mut total_orphans = 0;
        
        for entries in self.orphaned_entries.values() {
            total_orphans += entries.len();
            
            for entry in entries {
                // Track common orphaned keys
                *common_keys.entry(entry.key.clone()).or_insert(0) += 1;
                
                // Track risk distribution
                let risk_str = format!("{:?}", entry.data_loss_risk);
                *risk_distribution.entry(risk_str).or_insert(0) += 1;
            }
        }
        
        // Find most common orphaned keys
        let mut common_keys_vec: Vec<_> = common_keys.into_iter().collect();
        common_keys_vec.sort_by(|a, b| b.1.cmp(&a.1));
        
        OrphanedPatterns {
            total_contracts_with_orphans: self.orphaned_entries.len(),
            total_orphaned_entries: total_orphans,
            most_common_orphaned_keys: common_keys_vec.into_iter().take(10).collect(),
            risk_distribution,
        }
    }
}

/// Represents an orphaned storage entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrphanedEntry {
    pub key: String,
    pub value_type: String,
    pub size_bytes: usize,
    pub last_accessed: Option<u64>, // Timestamp
    pub recovery_possible: bool,
    pub data_loss_risk: DataLossRisk,
}

/// Risk level for data loss
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DataLossRisk {
    Low,
    Medium,
    High,
    Critical,
}

/// Summary of orphaned entries for a contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrphanedSummary {
    pub total_entries: usize,
    pub total_size_bytes: usize,
    pub high_risk_count: usize,
    pub recoverable_count: usize,
    pub risk_level: OverallRisk,
}

/// Overall risk assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OverallRisk {
    Low,
    Medium,
    High,
}

/// Recovery recommendation for orphaned data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryRecommendation {
    pub priority: RecommendationPriority,
    pub category: String,
    pub description: String,
    pub action: String,
    pub estimated_effort: String,
}

/// Priority of recovery recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Patterns analysis for orphaned entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrphanedPatterns {
    pub total_contracts_with_orphans: usize,
    pub total_orphaned_entries: usize,
    pub most_common_orphaned_keys: Vec<(String, usize)>,
    pub risk_distribution: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::xdr::ScVal;

    #[tokio::test]
    async fn test_orphaned_tracker_creation() {
        let tracker = OrphanedStateTracker::new();
        assert_eq!(tracker.get_contracts_with_orphans().len(), 0);
    }

    #[tokio::test]
    async fn test_value_type_detection() {
        let tracker = OrphanedStateTracker::new();
        
        let bool_val = ScVal::Bool(true);
        assert_eq!(tracker.get_value_type(&bool_val), "bool");
        
        let u32_val = ScVal::U32(42);
        assert_eq!(tracker.get_value_type(&u32_val), "u32");
        
        let bytes_val = ScVal::Bytes(vec![1, 2, 3, 4]);
        assert_eq!(tracker.get_value_type(&bytes_val), "bytes");
    }

    #[tokio::test]
    async fn test_size_estimation() {
        let tracker = OrphanedStateTracker::new();
        
        let bool_val = ScVal::Bool(true);
        assert_eq!(tracker.estimate_value_size(&bool_val), 1);
        
        let u32_val = ScVal::U32(42);
        assert_eq!(tracker.estimate_value_size(&u32_val), 4);
        
        let bytes_val = ScVal::Bytes(vec![1, 2, 3, 4, 5]);
        assert_eq!(tracker.estimate_value_size(&bytes_val), 5);
    }

    #[tokio::test]
    async fn test_risk_assessment() {
        let tracker = OrphanedStateTracker::new();
        
        let balance_val = ScVal::U64(1000);
        assert_eq!(tracker.assess_data_loss_risk("balance", &balance_val), DataLossRisk::High);
        
        let temp_val = ScVal::U32(42);
        assert_eq!(tracker.assess_data_loss_risk("temp_counter", &temp_val), DataLossRisk::Low);
    }

    #[tokio::test]
    async fn test_orphaned_summary() {
        let tracker = OrphanedStateTracker::new();
        let contract_id = "test_contract";
        
        let summary = tracker.get_orphaned_summary(contract_id);
        assert_eq!(summary.total_entries, 0);
        assert_eq!(summary.risk_level, OverallRisk::Low);
    }
}
