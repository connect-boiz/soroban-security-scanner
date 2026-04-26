//! State Cache Module
//! 
//! This module provides LRU caching for contract states and ledger data
//! to optimize performance when frequently accessing the same data.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::{Result, anyhow};
use tokio::sync::RwLock;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use crate::time_travel_debugger::{ContractState, LedgerSnapshot, TimeTravelConfig};

/// LRU cache for contract states and ledger data
pub struct StateCache {
    config: CacheConfig,
    contract_cache: Arc<RwLock<LruCache<String, CachedContractState>>>,
    ledger_cache: Arc<RwLock<LruCache<u32, CachedLedgerSnapshot>>>,
    stats: Arc<RwLock<CacheStatistics>>,
}

/// Configuration for the cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Maximum number of contract states to cache
    pub max_contract_states: usize,
    /// Maximum number of ledger snapshots to cache
    pub max_ledger_snapshots: usize,
    /// TTL for cached entries (in seconds)
    pub ttl_seconds: u64,
    /// Enable cache compression
    pub enable_compression: bool,
    /// Background cleanup interval (in seconds)
    pub cleanup_interval_seconds: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_contract_states: 10000,
            max_ledger_snapshots: 1000,
            ttl_seconds: 3600, // 1 hour
            enable_compression: true,
            cleanup_interval_seconds: 300, // 5 minutes
        }
    }
}

impl From<TimeTravelConfig> for CacheConfig {
    fn from(config: TimeTravelConfig) -> Self {
        Self {
            max_contract_states: config.cache_size,
            max_ledger_snapshots: 1000,
            ttl_seconds: 3600,
            enable_compression: true,
            cleanup_interval_seconds: 300,
        }
    }
}

/// Cached contract state with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedContractState {
    state: ContractState,
    cached_at: Instant,
    access_count: u64,
    size_bytes: usize,
    compressed: bool,
}

/// Cached ledger snapshot with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedLedgerSnapshot {
    snapshot: LedgerSnapshot,
    cached_at: Instant,
    access_count: u64,
}

/// Cache statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStatistics {
    pub contract_hits: u64,
    pub contract_misses: u64,
    pub ledger_hits: u64,
    pub ledger_misses: u64,
    pub evictions: u64,
    pub total_size_bytes: usize,
    pub compression_ratio: f64,
}

impl StateCache {
    /// Create a new state cache
    pub fn new(config: CacheConfig) -> Result<Self> {
        let contract_cache = Arc::new(RwLock::new(LruCache::new(
            std::num::NonZeroUsize::new(config.max_contract_states)
                .ok_or_else(|| anyhow!("Invalid contract cache size"))?,
        )));

        let ledger_cache = Arc::new(RwLock::new(LruCache::new(
            std::num::NonZeroUsize::new(config.max_ledger_snapshots)
                .ok_or_else(|| anyhow!("Invalid ledger cache size"))?,
        )));

        Ok(Self {
            config,
            contract_cache,
            ledger_cache,
            stats: Arc::new(RwLock::new(CacheStatistics::default())),
        })
    }

    /// Get a cached contract state
    pub async fn get_contract_state(&self, contract_id: &str, ledger_sequence: u32) -> Option<ContractState> {
        let cache_key = format!("{}:{}", contract_id, ledger_sequence);
        
        // Check cache
        {
            let mut cache = self.contract_cache.write().await;
            if let Some(cached_state) = cache.get_mut(&cache_key) {
                // Check if entry is still valid
                if cached_state.cached_at.elapsed() < Duration::from_secs(self.config.ttl_seconds) {
                    cached_state.access_count += 1;
                    
                    // Update statistics
                    {
                        let mut stats = self.stats.write().await;
                        stats.contract_hits += 1;
                    }
                    
                    // Decompress if needed
                    let state = if cached_state.compressed {
                        match self.decompress_state(&cached_state.state) {
                            Ok(decompressed) => decompressed,
                            Err(_) => {
                                // Remove corrupted entry
                                cache.pop(&cache_key);
                                {
                                    let mut stats = self.stats.write().await;
                                    stats.contract_misses += 1;
                                }
                                return None;
                            }
                        }
                    } else {
                        cached_state.state.clone()
                    };
                    
                    return Some(state);
                } else {
                    // Entry expired, remove it
                    cache.pop(&cache_key);
                }
            }
        }
        
        // Update statistics for miss
        {
            let mut stats = self.stats.write().await;
            stats.contract_misses += 1;
        }
        
        None
    }

    /// Put a contract state into the cache
    pub async fn put_contract_state(&self, contract_id: &str, ledger_sequence: u32, state: ContractState) -> Result<()> {
        let cache_key = format!("{}:{}", contract_id, ledger_sequence);
        
        // Estimate size of the state
        let size_bytes = self.estimate_contract_state_size(&state);
        
        // Check if we should compress this entry
        let should_compress = self.config.enable_compression && size_bytes > 1024; // Compress entries > 1KB
        
        let (compressed_state, compressed, actual_size) = if should_compress {
            match self.compress_state(&state) {
                Ok(compressed) => (compressed, true, compressed.len()),
                Err(_) => (state.clone(), false, size_bytes) // Fallback to uncompressed
            }
        } else {
            (state.clone(), false, size_bytes)
        };
        
        let cached_state = CachedContractState {
            state: compressed_state,
            cached_at: Instant::now(),
            access_count: 1,
            size_bytes: actual_size,
            compressed,
        };
        
        {
            let mut cache = self.contract_cache.write().await;
            
            // Check if we're evicting an entry and update size tracking
            let was_full = cache.len() >= self.config.max_contract_states;
            let evicted_size = if was_full {
                cache.get(&cache_key).map(|cached| cached.size_bytes).unwrap_or(0)
            } else {
                0
            };
            
            cache.put(cache_key, cached_state);
            
            if was_full {
                let mut stats = self.stats.write().await;
                stats.evictions += 1;
                stats.total_size_bytes = stats.total_size_bytes.saturating_sub(evicted_size);
            }
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_size_bytes += actual_size;
        }
        
        Ok(())
    }

    /// Get a cached ledger snapshot
    pub async fn get_ledger_snapshot(&self, ledger_sequence: u32) -> Option<LedgerSnapshot> {
        // Check cache
        {
            let mut cache = self.ledger_cache.write().await;
            if let Some(cached_snapshot) = cache.get_mut(&ledger_sequence) {
                // Check if entry is still valid
                if cached_snapshot.cached_at.elapsed() < Duration::from_secs(self.config.ttl_seconds) {
                    cached_snapshot.access_count += 1;
                    
                    // Update statistics
                    {
                        let mut stats = self.stats.write().await;
                        stats.ledger_hits += 1;
                    }
                    
                    return Some(cached_snapshot.snapshot.clone());
                } else {
                    // Entry expired, remove it
                    cache.pop(&ledger_sequence);
                }
            }
        }
        
        // Update statistics for miss
        {
            let mut stats = self.stats.write().await;
            stats.ledger_misses += 1;
        }
        
        None
    }

    /// Put a ledger snapshot into the cache
    pub async fn put_ledger_snapshot(&self, ledger_sequence: u32, snapshot: LedgerSnapshot) -> Result<()> {
        let cached_snapshot = CachedLedgerSnapshot {
            snapshot: snapshot.clone(),
            cached_at: Instant::now(),
            access_count: 1,
        };
        
        {
            let mut cache = self.ledger_cache.write().await;
            
            // Check if we're evicting an entry
            let was_full = cache.len() >= self.config.max_ledger_snapshots;
            cache.put(ledger_sequence, cached_snapshot);
            
            if was_full {
                let mut stats = self.stats.write().await;
                stats.evictions += 1;
            }
        }
        
        Ok(())
    }

    /// Clear all cached entries
    pub async fn clear(&self) {
        {
            let mut cache = self.contract_cache.write().await;
            cache.clear();
        }
        {
            let mut cache = self.ledger_cache.write().await;
            cache.clear();
        }
        {
            let mut stats = self.stats.write().await;
            *stats = CacheStatistics::default();
        }
    }

    /// Clear expired entries
    pub async fn clear_expired(&self) {
        let now = Instant::now();
        let ttl = Duration::from_secs(self.config.ttl_seconds);
        
        // Clear expired contract states
        {
            let mut cache = self.contract_cache.write().await;
            let expired_keys: Vec<String> = cache
                .iter()
                .filter(|(_, cached)| now.duration_since(cached.cached_at) > ttl)
                .map(|(key, _)| key.clone())
                .collect();
            
            for key in expired_keys {
                cache.pop(&key);
            }
        }
        
        // Clear expired ledger snapshots
        {
            let mut cache = self.ledger_cache.write().await;
            let expired_sequences: Vec<u32> = cache
                .iter()
                .filter(|(_, cached)| now.duration_since(cached.cached_at) > ttl)
                .map(|(seq, _)| *seq)
                .collect();
            
            for seq in expired_sequences {
                cache.pop(&seq);
            }
        }
    }

    /// Get cache statistics
    pub async fn get_statistics(&self) -> CacheStatistics {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Get detailed cache information
    pub async fn get_cache_info(&self) -> CacheInfo {
        let contract_len = self.contract_cache.read().await.len();
        let ledger_len = self.ledger_cache.read().await.len();
        let stats = self.stats.read().await.clone();
        
        let contract_hit_rate = if stats.contract_hits + stats.contract_misses > 0 {
            stats.contract_hits as f64 / (stats.contract_hits + stats.contract_misses) as f64
        } else {
            0.0
        };
        
        let ledger_hit_rate = if stats.ledger_hits + stats.ledger_misses > 0 {
            stats.ledger_hits as f64 / (stats.ledger_hits + stats.ledger_misses) as f64
        } else {
            0.0
        };
        
        CacheInfo {
            contract_states_cached: contract_len,
            ledger_snapshots_cached: ledger_len,
            max_contract_states: self.config.max_contract_states,
            max_ledger_snapshots: self.config.max_ledger_snapshots,
            contract_hit_rate,
            ledger_hit_rate,
            total_size_bytes: stats.total_size_bytes,
            compression_ratio: stats.compression_ratio,
            ttl_seconds: self.config.ttl_seconds,
        }
    }

    /// Preload cache with commonly accessed data
    pub async fn preload_common_contracts(&self, contract_ids: &[String], ledger_sequence: u32) -> Result<usize> {
        let mut loaded_count = 0;
        
        for contract_id in contract_ids {
            // In a real implementation, this would fetch the data from Stellar RPC
            // For now, we'll simulate the preloading process
            
            if self.get_contract_state(contract_id, ledger_sequence).await.is_none() {
                // Simulate loading the state
                let mock_state = ContractState {
                    contract_id: contract_id.clone(),
                    wasm_hash: format!("mock_hash_{}", contract_id),
                    storage: HashMap::new(),
                    ledger_sequence,
                };
                
                self.put_contract_state(contract_id, ledger_sequence, mock_state).await?;
                loaded_count += 1;
            }
        }
        
        Ok(loaded_count)
    }

    /// Estimate the size of a contract state
    fn estimate_contract_state_size(&self, state: &ContractState) -> usize {
        let mut size = state.contract_id.len();
        size += state.wasm_hash.len();
        size += state.storage.len() * 16; // Rough estimate per storage entry
        
        for (key, value) in &state.storage {
            size += key.len();
            size += self.estimate_scval_size(value);
        }
        
        size
    }

    /// Estimate the size of an ScVal
    fn estimate_scval_size(&self, _value: &soroban_sdk::xdr::ScVal) -> usize {
        // Simplified size estimation
        32 // Average size
    }

    /// Compress a contract state
    fn compress_state(&self, state: &ContractState) -> Result<ContractState> {
        // Use bincode for serialization and compression
        let serialized = bincode::serialize(state)
            .map_err(|e| anyhow!("Failed to serialize state: {}", e))?;
        
        // Compress using zstd
        let compressed = zstd::encode_all(serialized.as_slice(), 3)
            .map_err(|e| anyhow!("Failed to compress state: {}", e))?;
        
        // Create a compressed state (we'll store compressed data in a special field)
        // For now, we'll return the original state as compression is complex
        // In a real implementation, you'd modify ContractState to support compressed storage
        Ok(state.clone())
    }
    
    /// Decompress a contract state
    fn decompress_state(&self, compressed_state: &ContractState) -> Result<ContractState> {
        // In a real implementation, this would decompress the stored data
        // For now, we'll return the state as-is
        Ok(compressed_state.clone())
    }

    /// Optimize cache by removing least frequently used items
    pub async fn optimize_cache(&self) -> Result<()> {
        // This is a placeholder for cache optimization logic
        // In a real implementation, this could:
        // 1. Remove items with low access counts
        // 2. Compress large entries
        // 3. Reorganize cache structure
        
        self.clear_expired().await;
        Ok(())
    }

    /// Export cache statistics for monitoring
    pub async fn export_metrics(&self) -> CacheMetrics {
        let info = self.get_cache_info().await;
        let stats = self.get_statistics().await;
        
        CacheMetrics {
            cache_info: info,
            statistics: stats,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Detailed cache information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheInfo {
    pub contract_states_cached: usize,
    pub ledger_snapshots_cached: usize,
    pub max_contract_states: usize,
    pub max_ledger_snapshots: usize,
    pub contract_hit_rate: f64,
    pub ledger_hit_rate: f64,
    pub total_size_bytes: usize,
    pub compression_ratio: f64,
    pub ttl_seconds: u64,
}

/// Cache metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetrics {
    pub cache_info: CacheInfo,
    pub statistics: CacheStatistics,
    pub timestamp: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_travel_debugger::ContractState;

    #[tokio::test]
    async fn test_cache_creation() {
        let config = CacheConfig::default();
        let cache = StateCache::new(config);
        assert!(cache.is_ok());
    }

    #[tokio::test]
    async fn test_contract_state_caching() {
        let config = CacheConfig::default();
        let cache = StateCache::new(config).unwrap();
        
        let contract_id = "test_contract";
        let ledger_sequence = 1000;
        let state = ContractState {
            contract_id: contract_id.to_string(),
            wasm_hash: "test_hash".to_string(),
            storage: HashMap::new(),
            ledger_sequence,
        };
        
        // Put state in cache
        cache.put_contract_state(contract_id, ledger_sequence, state.clone()).await.unwrap();
        
        // Get state from cache
        let cached_state = cache.get_contract_state(contract_id, ledger_sequence).await;
        assert!(cached_state.is_some());
        assert_eq!(cached_state.unwrap().contract_id, contract_id);
    }

    #[tokio::test]
    async fn test_cache_statistics() {
        let config = CacheConfig::default();
        let cache = StateCache::new(config).unwrap();
        
        // Initially no hits or misses
        let stats = cache.get_statistics().await;
        assert_eq!(stats.contract_hits, 0);
        assert_eq!(stats.contract_misses, 0);
        
        // Cache miss
        let _ = cache.get_contract_state("nonexistent", 1000).await;
        let stats = cache.get_statistics().await;
        assert_eq!(stats.contract_misses, 1);
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let config = CacheConfig::default();
        let cache = StateCache::new(config).unwrap();
        
        let contract_id = "test_contract";
        let ledger_sequence = 1000;
        let state = ContractState {
            contract_id: contract_id.to_string(),
            wasm_hash: "test_hash".to_string(),
            storage: HashMap::new(),
            ledger_sequence,
        };
        
        // Put state in cache
        cache.put_contract_state(contract_id, ledger_sequence, state).await.unwrap();
        
        // Verify it's cached
        let cached_state = cache.get_contract_state(contract_id, ledger_sequence).await;
        assert!(cached_state.is_some());
        
        // Clear cache
        cache.clear().await;
        
        // Verify it's gone
        let cached_state = cache.get_contract_state(contract_id, ledger_sequence).await;
        assert!(cached_state.is_none());
    }
}
