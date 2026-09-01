//! Stellar Ledger State "Time Travel" Debugger
//! 
//! This module provides functionality to "fork" the current Stellar Mainnet state at a specific 
//! ledger sequence to test contracts against live data while maintaining read-only access.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use lru::LruCache;
use reqwest::Client;
use soroban_sdk::xdr::{ScVal, LedgerKey, ContractDataEntry, ContractCodeLedgerKey};

use access_control::{AccessController, UserContext, UserRole, UserTier, Permission, ApprovalStatus};
use audit_log::{AuditLogger, AuditOperation, AuditLogQuery, AuditLogSummary};
use rate_limiter::{RateLimiter, RateLimitConfig, RateLimitStatus};
use data_retention::{DataRetentionManager, RetentionPolicy, StoredDataType, RetentionCleanupReport, StorageUsage};
use encryption::{DataEncryptor, EncryptionConfig, EncryptedData};
use quota::{QuotaManager, QuotaConfig, QuotaOperation, QuotaStatus};
use monitoring::{MonitoringEngine, MonitoringConfig, SuspiciousPattern, SuspiciousPatternType, UserPatternSummary};

mod state_injection;
mod contract_upgrade;
mod orphaned_state;
mod cache;
mod access_control;
mod audit_log;
mod rate_limiter;
mod data_retention;
mod encryption;
mod quota;
mod monitoring;

#[cfg(test)]
mod tests;

pub use state_injection::StateInjector;
pub use contract_upgrade::ContractUpgradeSimulator;
pub use orphaned_state::OrphanedStateTracker;
pub use cache::StateCache;
pub use access_control::{
    AccessController, UserRole, UserContext, UserTier, Permission, PermissionCheck,
    ApprovalRequest, ApprovalStatus,
};
pub use audit_log::{AuditLogger, AuditEntry, AuditOperation, AuditLogQuery, AuditLogSummary};
pub use rate_limiter::{RateLimiter, RateLimitConfig, RateLimitStatus};
pub use data_retention::{
    DataRetentionManager, RetentionPolicy, StoredDataType, StorageUsage,
    RetentionCleanupReport,
};
pub use encryption::{DataEncryptor, EncryptionConfig, EncryptionAlgorithm, EncryptedData};
pub use quota::{QuotaManager, QuotaConfig, QuotaOperation, QuotaStatus, UserQuotaState, UserTier as QuotaUserTier};
pub use monitoring::{
    MonitoringEngine, MonitoringConfig, SuspiciousPattern, SuspiciousPatternType,
    SuspiciousSeverity, AccessPattern, UserPatternSummary,
};

/// Configuration for the Time Travel Debugger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeTravelConfig {
    /// Stellar RPC endpoint URL
    pub rpc_url: String,
    /// Network passphrase (mainnet/testnet)
    pub network_passphrase: String,
    /// Maximum number of ledger entries to cache
    pub cache_size: usize,
    /// Request timeout in seconds
    pub request_timeout: u64,
    /// Maximum retry attempts for failed requests
    pub max_retries: u32,
    /// Rate limit configuration
    pub rate_limit_config: RateLimitConfig,
    /// Retention policy for historical data
    pub retention_policy: RetentionPolicy,
    /// Encryption configuration
    pub encryption_config: EncryptionConfig,
    /// Monitoring configuration
    pub monitoring_config: MonitoringConfig,
    /// Maximum audit log entries
    pub max_audit_log_entries: usize,
    /// Enable access control
    pub access_control_enabled: bool,
    /// Enable audit logging
    pub audit_logging_enabled: bool,
    /// Enable rate limiting
    pub rate_limiting_enabled: bool,
    /// Enable data encryption
    pub encryption_enabled: bool,
    /// Enable quota management
    pub quotas_enabled: bool,
    /// Enable suspicious access monitoring
    pub monitoring_enabled: bool,
}

impl Default for TimeTravelConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://mainnet.stellar.rpc".to_string(),
            network_passphrase: "Public Global Stellar Network ; September 2015".to_string(),
            cache_size: 10000,
            request_timeout: 30,
            max_retries: 3,
            rate_limit_config: RateLimitConfig::moderate(),
            retention_policy: RetentionPolicy::default(),
            encryption_config: EncryptionConfig::default(),
            monitoring_config: MonitoringConfig::default(),
            max_audit_log_entries: 10000,
            access_control_enabled: true,
            audit_logging_enabled: true,
            rate_limiting_enabled: true,
            encryption_enabled: true,
            quotas_enabled: true,
            monitoring_enabled: true,
        }
    }
}

/// Represents a specific point in time on the Stellar network
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LedgerSnapshot {
    /// Ledger sequence number
    pub ledger_sequence: u32,
    /// Ledger hash
    pub ledger_hash: String,
    /// Timestamp when the ledger was closed
    pub close_time: u64,
    /// Protocol version
    pub protocol_version: u32,
    /// Total number of operations in this ledger
    pub operation_count: u32,
    /// Base fee in stroops
    pub base_fee: u32,
    /// Base reserve in stroops
    pub base_reserve: u32,
}

/// Contract state at a specific ledger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractState {
    /// Contract ID
    pub contract_id: String,
    /// WASM hash of the contract code
    pub wasm_hash: String,
    /// Contract storage entries
    pub storage: HashMap<String, ScVal>,
    /// Ledger sequence when this state was captured
    pub ledger_sequence: u32,
}

/// Main Time Travel Debugger interface
pub struct TimeTravelDebugger {
    config: TimeTravelConfig,
    http_client: Client,
    state_cache: Arc<RwLock<LruCache<String, ContractState>>>,
    ledger_cache: Arc<RwLock<LruCache<u32, LedgerSnapshot>>>,
    state_injector: StateInjector,
    upgrade_simulator: ContractUpgradeSimulator,
    orphaned_tracker: OrphanedStateTracker,
    access_controller: AccessController,
    audit_logger: AuditLogger,
    rate_limiter: RateLimiter,
    data_retention: DataRetentionManager,
    encryptor: DataEncryptor,
    quota_manager: QuotaManager,
    monitoring_engine: MonitoringEngine,
    admin_key: Arc<RwLock<Option<Vec<u8>>>>,
}

impl TimeTravelDebugger {
    /// Create a new Time Travel Debugger instance
    pub async fn new(config: TimeTravelConfig) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.request_timeout))
            .build()?;

        let state_cache = Arc::new(RwLock::new(LruCache::new(
            std::num::NonZeroUsize::new(config.cache_size)
                .ok_or_else(|| anyhow!("Invalid cache size"))?,
        )));

        let ledger_cache = Arc::new(RwLock::new(LruCache::new(
            std::num::NonZeroUsize::new(1000)
                .ok_or_else(|| anyhow!("Invalid ledger cache size"))?,
        )));

        let state_injector = StateInjector::new(config.clone());
        let upgrade_simulator = ContractUpgradeSimulator::new(config.clone());
        let orphaned_tracker = OrphanedStateTracker::new();

        let access_controller = AccessController::new();
        let audit_logger = AuditLogger::new(config.max_audit_log_entries);
        let rate_limiter = RateLimiter::new(config.rate_limit_config.clone());
        let data_retention = DataRetentionManager::new(config.retention_policy.clone());
        let encryptor = DataEncryptor::new(config.encryption_config.clone());
        let quota_manager = QuotaManager::new();
        let monitoring_engine = MonitoringEngine::new(config.monitoring_config.clone());

        Ok(Self {
            config,
            http_client,
            state_cache,
            ledger_cache,
            state_injector,
            upgrade_simulator,
            orphaned_tracker,
            access_controller,
            audit_logger,
            rate_limiter,
            data_retention,
            encryptor,
            quota_manager,
            monitoring_engine,
            admin_key: Arc::new(RwLock::new(None)),
        })
    }

    /// Get ledger information at a specific sequence
    pub async fn get_ledger_info(&self, sequence: u32) -> Result<LedgerSnapshot> {
        // Check cache first
        {
            let cache = self.ledger_cache.read().await;
            if let Some(snapshot) = cache.get(&sequence) {
                return Ok(snapshot.clone());
            }
        }

        // Fetch from Stellar RPC via HTTP
        let url = format!("{}/ledgers/{}", self.config.rpc_url, sequence);
        let response = self.http_client.get(&url).send().await?;
        let ledger_data: serde_json::Value = response.json().await?;
        
        // Parse the response (this will need to be adapted based on actual RPC response format)
        let snapshot = LedgerSnapshot {
            ledger_sequence: sequence,
            ledger_hash: ledger_data["hash"].as_str().unwrap_or("").to_string(),
            close_time: ledger_data["closed_at"].as_u64().unwrap_or(0),
            protocol_version: ledger_data["protocol_version"].as_u64().unwrap_or(0) as u32,
            operation_count: ledger_data["operation_count"].as_u64().unwrap_or(0) as u32,
            base_fee: ledger_data["base_fee"].as_u64().unwrap_or(0) as u32,
            base_reserve: ledger_data["base_reserve"].as_u64().unwrap_or(0) as u32,
        };

        // Cache the result
        {
            let mut cache = self.ledger_cache.write().await;
            cache.put(sequence, snapshot.clone());
        }

        Ok(snapshot)
    }

    /// Get contract state at a specific ledger sequence
    pub async fn get_contract_state(&self, contract_id: &str, ledger_sequence: u32) -> Result<ContractState> {
        let cache_key = format!("{}:{}", contract_id, ledger_sequence);
        
        // Check cache first
        {
            let cache = self.state_cache.read().await;
            if let Some(state) = cache.get(&cache_key) {
                return Ok(state.clone());
            }
        }

        // Fetch contract data from Stellar RPC via HTTP
        let url = format!("{}/contracts/{}/data?ledger={}", self.config.rpc_url, contract_id, ledger_sequence);
        let response = self.http_client.get(&url).send().await?;
        let contract_data: serde_json::Value = response.json().await?;

        let mut storage = HashMap::new();
        let mut wasm_hash = String::new();

        // Parse contract data (this will need to be adapted based on actual RPC response format)
        if let Some(data_array) = contract_data["data"].as_array() {
            for entry in data_array {
                if let Some(key) = entry["key"].as_str() {
                    if let Some(value_str) = entry["value"].as_str() {
                        // Parse the value based on its type
                        let value = self.parse_scval_from_string(value_str)?;
                        storage.insert(key.to_string(), value);
                    }
                }
            }
        }

        if let Some(hash) = contract_data["wasm_hash"].as_str() {
            wasm_hash = hash.to_string();
        }

        let state = ContractState {
            contract_id: contract_id.to_string(),
            wasm_hash,
            storage,
            ledger_sequence,
        };

        // Cache the result
        {
            let mut cache = self.state_cache.write().await;
            cache.put(cache_key, state.clone());
        }

        Ok(state)
    }

    /// Parse ScVal from string representation
    fn parse_scval_from_string(&self, value_str: &str) -> Result<ScVal> {
        // This is a simplified parser - in reality, this would need to handle
        // the full XDR serialization format
        if value_str.starts_with("u64:") {
            let num_str = &value_str[4..];
            let value: u64 = num_str.parse()?;
            Ok(ScVal::U64(value))
        } else if value_str.starts_with("u32:") {
            let num_str = &value_str[4..];
            let value: u32 = num_str.parse()?;
            Ok(ScVal::U32(value))
        } else if value_str.starts_with("bytes:") {
            let hex_str = &value_str[6..];
            let bytes = hex::decode(hex_str)?;
            Ok(ScVal::Bytes(bytes))
        } else if value_str == "true" {
            Ok(ScVal::Bool(true))
        } else if value_str == "false" {
            Ok(ScVal::Bool(false))
        } else {
            // Default to string
            Ok(ScVal::String(value_str.to_string()))
        }
    }

    /// Fork the network state at a specific ledger and prepare for testing
    pub async fn fork_at_ledger(&self, ledger_sequence: u32) -> Result<ForkedState> {
        let ledger_info = self.get_ledger_info(ledger_sequence).await?;
        
        Ok(ForkedState {
            ledger_snapshot: ledger_info,
            debugger: self,
            created_at: Instant::now(),
        })
    }

    /// Simulate a contract upgrade and check for compatibility issues
    pub async fn simulate_contract_upgrade(
        &self,
        contract_id: &str,
        new_wasm: &[u8],
        ledger_sequence: u32,
    ) -> Result<UpgradeSimulationResult> {
        let current_state = self.get_contract_state(contract_id, ledger_sequence).await?;
        
        self.upgrade_simulator
            .simulate_upgrade(&current_state, new_wasm)
            .await
    }

    /// Inject contract state into local WASM runner for testing
    pub async fn inject_state_for_testing(
        &self,
        contract_id: &str,
        ledger_sequence: u32,
    ) -> Result<()> {
        let state = self.get_contract_state(contract_id, ledger_sequence).await?;
        self.state_injector.inject_state(&state).await
    }

    /// Get orphaned state entries for a contract after upgrade
    pub async fn get_orphaned_state(
        &self,
        contract_id: &str,
        old_ledger_sequence: u32,
        new_wasm: &[u8],
    ) -> Result<Vec<String>> {
        let old_state = self.get_contract_state(contract_id, old_ledger_sequence).await?;
        self.orphaned_tracker
            .find_orphaned_entries(&old_state, new_wasm)
            .await
    }

    /// Clear all caches
    pub async fn clear_caches(&self) {
        {
            let mut cache = self.state_cache.write().await;
            cache.clear();
        }
        {
            let mut cache = self.ledger_cache.write().await;
            cache.clear();
        }
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> CacheStats {
        let state_len = self.state_cache.read().await.len();
        let ledger_len = self.ledger_cache.read().await.len();
        
        CacheStats {
            contract_states_cached: state_len,
            ledgers_cached: ledger_len,
            max_contract_states: self.config.cache_size,
            max_ledgers: 1000,
        }
    }

    // ============================================================
    // Security & Access Control Methods
    // ============================================================

    pub async fn set_admin_key(&self, key: &[u8]) -> Result<()> {
        let mut admin_key = self.admin_key.write().await;
        *admin_key = Some(key.to_vec());
        Ok(())
    }

    pub async fn initialize_encryption(&self, key: &[u8]) -> Result<()> {
        if !self.config.encryption_enabled {
            return Ok(());
        }
        self.encryptor.initialize(key).await
    }

    pub async fn authorize_operation(
        &self,
        user: &UserContext,
        permission: &Permission,
        contract_id: Option<&str>,
        ledger_sequence: Option<u32>,
    ) -> Result<()> {
        if !self.config.access_control_enabled {
            return Ok(());
        }

        let check = self.access_controller
            .check_permission(user, permission, contract_id, ledger_sequence)
            .await?;

        if !check.allowed {
            let reason = check.reason.clone().unwrap_or_default();
            self.audit_logger
                .log_permission_denied(
                    &user.user_id,
                    user.roles.iter().map(|r| format!("{:?}", r)).collect(),
                    &format!("{:?}", permission),
                    &reason,
                )
                .await;
            return Err(anyhow!("Access denied: {}", reason));
        }

        Ok(())
    }

    pub async fn check_rate_limit(&self, user_id: &str) -> Result<RateLimitStatus> {
        if !self.config.rate_limiting_enabled {
            return Ok(RateLimitStatus {
                allowed: true,
                remaining_requests: u32::MAX,
                retry_after_seconds: 0,
                current_concurrent: 0,
                max_concurrent: u32::MAX,
            });
        }

        let status = self.rate_limiter.check_rate_limit(user_id).await;
        if !status.allowed {
            self.audit_logger
                .log(
                    user_id,
                    Vec::new(),
                    AuditOperation::RateLimitExceeded,
                    "rate_limit",
                    None,
                    None,
                    false,
                    &format!("Rate limit exceeded. Retry after {}s", status.retry_after_seconds),
                )
                .await;
        }
        Ok(status)
    }

    pub async fn check_quota(
        &self,
        user_id: &str,
        tier: &UserTier,
        operation: &QuotaOperation,
    ) -> Result<QuotaStatus> {
        if !self.config.quotas_enabled {
            return Ok(QuotaStatus {
                operation: operation.clone(),
                used: 0,
                limit: u32::MAX,
                remaining: u32::MAX,
                resets_in_seconds: 0,
                allowed: true,
            });
        }

        let status = self.quota_manager.check_quota(user_id, tier, operation).await;
        if !status.allowed {
            self.audit_logger
                .log(
                    user_id,
                    Vec::new(),
                    AuditOperation::QuotaExceeded,
                    &format!("{:?}", operation),
                    None,
                    None,
                    false,
                    &format!("Quota exceeded for {:?}: {}/{}", operation, status.used, status.limit),
                )
                .await;
        }
        Ok(status)
    }

    pub async fn record_quota_usage(
        &self,
        user_id: &str,
        tier: &UserTier,
        operation: &QuotaOperation,
    ) -> QuotaStatus {
        self.quota_manager.record_operation(user_id, tier, operation).await
    }

    pub async fn audit_log(
        &self,
        user_id: &str,
        user_roles: Vec<String>,
        operation: AuditOperation,
        resource: &str,
        ledger_sequence: Option<u32>,
        contract_id: Option<&str>,
        success: bool,
        details: &str,
    ) {
        if self.config.audit_logging_enabled {
            self.audit_logger
                .log(
                    user_id,
                    user_roles,
                    operation,
                    resource,
                    ledger_sequence,
                    contract_id,
                    success,
                    details,
                )
                .await;
        }
    }

    pub async fn monitor_access(
        &self,
        user_id: &str,
        operation: &str,
        contract_id: Option<&str>,
        ledger_sequence: Option<u32>,
    ) -> Option<SuspiciousPattern> {
        if !self.config.monitoring_enabled {
            return None;
        }
        self.monitoring_engine
            .record_access(user_id, operation, contract_id, ledger_sequence, None)
            .await
    }

    pub async fn check_concurrent_forks(&self, user_id: &str, tier: &UserTier) -> bool {
        if !self.config.quotas_enabled {
            return true;
        }
        self.quota_manager.check_concurrent_forks(user_id, tier).await
    }

    pub async fn track_active_fork(&self, user_id: &str, start: bool) {
        if self.config.quotas_enabled {
            self.quota_manager.track_active_fork(user_id, start).await;
        }
    }

    // ============================================================
    // Audit Log Methods
    // ============================================================

    pub async fn query_audit_logs(&self, query: &AuditLogQuery) -> Vec<AuditEntry> {
        self.audit_logger.query(query).await
    }

    pub async fn get_audit_log_summary(&self) -> AuditLogSummary {
        self.audit_logger.get_summary().await
    }

    // ============================================================
    // Access Control Management Methods
    // ============================================================

    pub async fn register_contract_owner(
        &self,
        contract_id: &str,
        owner_id: &str,
    ) -> Result<()> {
        self.access_controller
            .register_contract_owner(contract_id, owner_id)
            .await
    }

    pub async fn request_approval(
        &self,
        requester_id: &str,
        operation: &str,
        contract_id: Option<&str>,
        ledger_sequence: Option<u32>,
        reason: &str,
    ) -> Result<ApprovalRequest> {
        self.access_controller
            .request_approval(requester_id, operation, contract_id, ledger_sequence, reason)
            .await
    }

    pub async fn resolve_approval(
        &self,
        approval_id: &str,
        resolver_id: &str,
        approved: bool,
    ) -> Result<()> {
        self.access_controller
            .resolve_approval(approval_id, resolver_id, approved)
            .await
    }

    pub async fn check_approval_status(&self, approval_id: &str) -> Option<ApprovalStatus> {
        self.access_controller.check_approval_status(approval_id).await
    }

    pub async fn get_pending_approvals(&self) -> Vec<ApprovalRequest> {
        self.access_controller.get_pending_approvals().await
    }

    // ============================================================
    // Data Retention Methods
    // ============================================================

    pub async fn perform_retention_cleanup(&self) -> RetentionCleanupReport {
        self.data_retention.perform_cleanup().await
    }

    pub async fn get_storage_usage(&self) -> StorageUsage {
        self.data_retention.get_storage_usage().await
    }

    pub async fn update_retention_policy(&self, policy: RetentionPolicy) -> Result<()> {
        self.data_retention.update_retention_policy(policy).await
    }

    pub async fn get_retention_policy(&self) -> RetentionPolicy {
        self.data_retention.get_retention_policy().await
    }

    // ============================================================
    // Encryption Methods
    // ============================================================

    pub async fn encrypt_state_data(&self, data: &[u8]) -> Result<EncryptedData> {
        if !self.config.encryption_enabled {
            return Err(anyhow!("Encryption is disabled"));
        }
        self.encryptor.encrypt(data).await
    }

    pub async fn decrypt_state_data(&self, encrypted: &EncryptedData) -> Result<Vec<u8>> {
        if !self.config.encryption_enabled {
            return Err(anyhow!("Encryption is disabled"));
        }
        self.encryptor.decrypt(encrypted).await
    }

    pub async fn rotate_encryption_key(&self, new_key: &[u8]) -> Result<()> {
        self.encryptor.rotate_key(new_key).await
    }

    // ============================================================
    // Monitoring Methods
    // ============================================================

    pub async fn get_suspicious_events(
        &self,
        min_severity: Option<SuspiciousSeverity>,
        limit: usize,
    ) -> Vec<SuspiciousPattern> {
        self.monitoring_engine
            .get_suspicious_events(min_severity, limit)
            .await
    }

    pub async fn get_user_pattern_summary(
        &self,
        user_id: &str,
    ) -> Option<UserPatternSummary> {
        self.monitoring_engine.get_user_pattern_summary(user_id).await
    }

    pub async fn should_alert(&self) -> bool {
        self.monitoring_engine.should_alert().await
    }

    // ============================================================
    // Quota Methods
    // ============================================================

    pub async fn get_user_quota_usage(&self, user_id: &str) -> Option<quota::UserQuotaState> {
        self.quota_manager.get_usage(user_id).await
    }
}

/// Represents a forked network state for testing
pub struct ForkedState {
    pub ledger_snapshot: LedgerSnapshot,
    debugger: &'static TimeTravelDebugger,
    pub created_at: Instant,
}

impl ForkedState {
    /// Get the ledger sequence of this fork
    pub fn ledger_sequence(&self) -> u32 {
        self.ledger_snapshot.ledger_sequence
    }

    /// Get the timestamp when this fork was created
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Test a contract against this forked state
    pub async fn test_contract(&self, contract_id: &str) -> Result<TestResult> {
        let state = self.debugger
            .get_contract_state(contract_id, self.ledger_sequence())
            .await?;
        
        // Inject state and run tests
        self.debugger
            .state_injector
            .inject_state(&state)
            .await?;
        
        // TODO: Implement actual contract testing logic
        Ok(TestResult {
            contract_id: contract_id.to_string(),
            ledger_sequence: self.ledger_sequence(),
            passed: true,
            issues: Vec::new(),
            execution_time: Duration::from_millis(100),
        })
    }
}

/// Result of contract testing against forked state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub contract_id: String,
    pub ledger_sequence: u32,
    pub passed: bool,
    pub issues: Vec<String>,
    pub execution_time: Duration,
}

/// Result of contract upgrade simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeSimulationResult {
    pub is_compatible: bool,
    pub compatibility_issues: Vec<String>,
    pub orphaned_entries: Vec<String>,
    pub warnings: Vec<String>,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub contract_states_cached: usize,
    pub ledgers_cached: usize,
    pub max_contract_states: usize,
    pub max_ledgers: usize,
}


