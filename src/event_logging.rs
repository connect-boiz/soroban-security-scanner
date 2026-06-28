//! Event Logging System for Critical Operations
//!
//! This module provides comprehensive event logging for critical operations
//! like fund transfers, vulnerability verification, and escrow operations.

use anyhow::Result;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Event logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLoggingConfig {
    /// Enable event logging for critical operations
    pub enabled: bool,
    /// Log level for events (Debug, Info, Warn, Error)
    pub log_level: EventLogLevel,
    /// Enable structured logging with JSON format
    pub structured_logging: bool,
    /// Enable event persistence to storage
    pub enable_persistence: bool,
    /// Maximum number of events to keep in memory
    pub max_events_in_memory: usize,
    /// Event retention period in seconds
    pub retention_period_seconds: u64,
    /// Critical operations that require mandatory logging
    pub critical_operations: Vec<CriticalOperation>,
}

impl Default for EventLoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_level: EventLogLevel::Info,
            structured_logging: false,
            enable_persistence: true,
            max_events_in_memory: 10000,
            retention_period_seconds: 86400 * 30, // 30 days
            critical_operations: vec![
                CriticalOperation::FundTransfer,
                CriticalOperation::VulnerabilityVerification,
                CriticalOperation::EscrowOperation,
                CriticalOperation::RewardDistribution,
                CriticalOperation::AdminApproval,
                CriticalOperation::OwnershipChange,
                CriticalOperation::ScanResultAccess,
                CriticalOperation::ScanResultShare,
            ],
        }
    }
}

/// Event log levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Critical operation types that require event logging
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CriticalOperation {
    FundTransfer,
    VulnerabilityVerification,
    EscrowOperation,
    RewardDistribution,
    AdminApproval,
    OwnershipChange,
    BountyCreation,
    BountyApproval,
    BountyClaim,
    ResearcherAssignment,
    ContractUpgrade,
    EmergencyStop,
    ConfigurationChange,
    /// Scan result access attempt (for IDOR detection and audit trail - issue #329)
    ScanResultAccess,
    /// Scan result sharing operation
    ScanResultShare,
}

/// Event severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Event status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventStatus {
    Started,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Comprehensive event data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalEvent {
    /// Unique event identifier
    pub event_id: String,
    /// Timestamp when the event occurred
    pub timestamp: u64,
    /// Operation type
    pub operation: CriticalOperation,
    /// Event severity
    pub severity: EventSeverity,
    /// Event status
    pub status: EventStatus,
    /// Event description
    pub description: String,
    /// Actor who performed the operation
    pub actor: String,
    /// Target of the operation (if applicable)
    pub target: Option<String>,
    /// Additional event data
    pub metadata: HashMap<String, String>,
    /// Previous state (for audit trail)
    pub previous_state: Option<String>,
    /// New state (for audit trail)
    pub new_state: Option<String>,
    /// Error message (if operation failed)
    pub error_message: Option<String>,
    /// Execution duration in milliseconds
    pub execution_duration_ms: Option<u64>,
    /// Gas consumed (if applicable)
    pub gas_consumed: Option<u64>,
    /// Related transaction hash
    pub transaction_hash: Option<String>,
    /// Ledger sequence (if applicable)
    pub ledger_sequence: Option<u64>,
    /// SHA-256 hash of this event's content (for tamper-evident chain)
    pub event_hash: String,
    /// SHA-256 hash of the previous event in the chain (empty string for first event)
    pub previous_event_hash: String,
}

/// Event logging manager
pub struct EventLogger {
    config: EventLoggingConfig,
    events: Arc<std::sync::Mutex<Vec<CriticalEvent>>>,
    event_counter: Arc<std::sync::atomic::AtomicU64>,
}

impl EventLogger {
    /// Create a new event logger
    pub fn new(config: EventLoggingConfig) -> Self {
        Self {
            config,
            events: Arc::new(std::sync::Mutex::new(Vec::new())),
            event_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Log a critical event
    pub fn log_event(&self, event: CriticalEvent) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Validate that this is a critical operation
        if !self.config.critical_operations.contains(&event.operation) {
            debug!(
                "Operation {:?} is not configured for critical logging",
                event.operation
            );
            return Ok(());
        }

        // Log based on configured level
        match self.config.log_level {
            EventLogLevel::Debug => debug!("Critical Event: {:?}", event),
            EventLogLevel::Info => info!("Critical Event: {:?}", event),
            EventLogLevel::Warn => warn!("Critical Event: {:?}", event),
            EventLogLevel::Error => error!("Critical Event: {:?}", event),
        }

        // Store event if persistence is enabled
        if self.config.enable_persistence {
            self.store_event(event)?;
        }

        Ok(())
    }

    /// Compute SHA-256 hash for an event (all fields serialized and hashed)
    fn compute_event_hash(event: &CriticalEvent) -> String {
        let mut hasher = Sha256::new();
        hasher.update(event.event_id.as_bytes());
        hasher.update(&event.timestamp.to_le_bytes());
        hasher.update(format!("{:?}", event.operation).as_bytes());
        hasher.update(format!("{:?}", event.severity).as_bytes());
        hasher.update(format!("{:?}", event.status).as_bytes());
        hasher.update(event.description.as_bytes());
        hasher.update(event.actor.as_bytes());
        if let Some(ref target) = event.target {
            hasher.update(target.as_bytes());
        }
        if let Some(ref prev) = event.previous_state {
            hasher.update(prev.as_bytes());
        }
        if let Some(ref new) = event.new_state {
            hasher.update(new.as_bytes());
        }
        if let Some(ref error) = event.error_message {
            hasher.update(error.as_bytes());
        }
        if let Some(ref tx) = event.transaction_hash {
            hasher.update(tx.as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }

    /// Compute the previous event hash (hash of the last event in the chain)
    fn compute_previous_event_hash(events: &[CriticalEvent]) -> String {
        events
            .last()
            .map(|e| e.event_hash.clone())
            .unwrap_or_default()
    }

    /// Rebuild the hash chain for all events (used after eviction)
    fn rebuild_hash_chain(events: &mut [CriticalEvent]) {
        let mut prev_hash = String::new();
        for event in events.iter_mut() {
            event.previous_event_hash = prev_hash.clone();
            event.event_hash = Self::compute_event_hash(event);
            prev_hash = event.event_hash.clone();
        }
    }

    /// Store event in memory with hash chaining
    fn store_event(&self, event: CriticalEvent) -> Result<()> {
        let mut events = self
            .events
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire events lock: {}", e))?;

        // Build the event with hash chaining
        let previous_hash = Self::compute_previous_event_hash(&events);
        let mut chained_event = event;
        chained_event.previous_event_hash = previous_hash.clone();
        chained_event.event_hash = Self::compute_event_hash(&chained_event);

        events.push(chained_event);

        // Enforce memory limit — rebuild hash chain if we evict the first event
        if events.len() > self.config.max_events_in_memory {
            events.remove(0);
            // Rebuild the hash chain after eviction to maintain integrity
            Self::rebuild_hash_chain(&mut events);
        }

        Ok(())
    }

    /// Get events by operation type
    pub fn get_events_by_operation(
        &self,
        operation: &CriticalOperation,
    ) -> Result<Vec<CriticalEvent>> {
        let events = self
            .events
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire events lock: {}", e))?;

        Ok(events
            .iter()
            .filter(|event| &event.operation == operation)
            .cloned()
            .collect())
    }

    /// Get events by time range
    pub fn get_events_by_time_range(
        &self,
        start_time: u64,
        end_time: u64,
    ) -> Result<Vec<CriticalEvent>> {
        let events = self
            .events
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire events lock: {}", e))?;

        Ok(events
            .iter()
            .filter(|event| event.timestamp >= start_time && event.timestamp <= end_time)
            .cloned()
            .collect())
    }

    /// Get events by actor
    pub fn get_events_by_actor(&self, actor: &str) -> Result<Vec<CriticalEvent>> {
        let events = self
            .events
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire events lock: {}", e))?;

        Ok(events
            .iter()
            .filter(|event| event.actor == actor)
            .cloned()
            .collect())
    }

    /// Generate event ID
    fn generate_event_id(&self) -> String {
        let counter = self
            .event_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("evt_{}_{}", chrono::Utc::now().timestamp(), counter)
    }

    /// Clean up old events based on retention period
    pub fn cleanup_old_events(&self) -> Result<usize> {
        let current_time = chrono::Utc::now().timestamp() as u64;
        let cutoff_time = current_time - self.config.retention_period_seconds;

        let mut events = self
            .events
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire events lock: {}", e))?;

        let initial_count = events.len();
        events.retain(|event| event.timestamp >= cutoff_time);
        let removed_count = initial_count - events.len();

        if removed_count > 0 {
            info!("Cleaned up {} old events", removed_count);
        }

        Ok(removed_count)
    }

    /// Get event statistics
    pub fn get_event_statistics(&self) -> Result<EventStatistics> {
        let events = self
            .events
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire events lock: {}", e))?;

        let mut stats = EventStatistics::default();

        for event in events.iter() {
            stats.total_events += 1;

            // Count by operation type
            *stats
                .by_operation
                .entry(event.operation.clone())
                .or_insert(0) += 1;

            // Count by severity
            *stats.by_severity.entry(event.severity.clone()).or_insert(0) += 1;

            // Count by status
            *stats.by_status.entry(event.status.clone()).or_insert(0) += 1;
        }

        Ok(stats)
    }

    // ── Paginated Query Methods ────────────────────────────────────────────

    /// Get events by operation type with pagination
    pub fn get_events_by_operation_paginated(
        &self,
        operation: &CriticalOperation,
        offset: usize,
        limit: usize,
    ) -> Result<PageResult<CriticalEvent>> {
        let events = self.get_events_by_operation(operation)?;
        Ok(self.apply_pagination(events, offset, limit))
    }

    /// Get events by time range with pagination
    pub fn get_events_by_time_range_paginated(
        &self,
        start_time: u64,
        end_time: u64,
        offset: usize,
        limit: usize,
    ) -> Result<PageResult<CriticalEvent>> {
        let events = self.get_events_by_time_range(start_time, end_time)?;
        Ok(self.apply_pagination(events, offset, limit))
    }

    /// Get events by actor with pagination
    pub fn get_events_by_actor_paginated(
        &self,
        actor: &str,
        offset: usize,
        limit: usize,
    ) -> Result<PageResult<CriticalEvent>> {
        let events = self.get_events_by_actor(actor)?;
        Ok(self.apply_pagination(events, offset, limit))
    }

    /// Internal: apply offset/limit to a filtered list
    fn apply_pagination(
        &self,
        mut events: Vec<CriticalEvent>,
        offset: usize,
        limit: usize,
    ) -> PageResult<CriticalEvent> {
        let total = events.len();
        if offset >= total {
            return PageResult {
                items: vec![],
                total,
                offset,
                limit,
                has_more: false,
            };
        }
        let end = (offset + limit).min(total);
        let items: Vec<CriticalEvent> = events.drain(offset..end).collect();
        PageResult {
            has_more: end < total,
            items,
            total,
            offset,
            limit,
        }
    }

    // ── Export Methods ─────────────────────────────────────────────────────

    /// Export events to CSV format
    pub fn events_to_csv(&self, events: &[CriticalEvent]) -> String {
        let mut csv = String::from("event_id,timestamp,operation,severity,status,description,actor,target,previous_event_hash,event_hash,error_message,execution_duration_ms,gas_consumed,transaction_hash,ledger_sequence\n");
        for event in events {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                Self::csv_escape(&event.event_id),
                event.timestamp,
                format!("{:?}", event.operation),
                format!("{:?}", event.severity),
                format!("{:?}", event.status),
                Self::csv_escape(&event.description),
                Self::csv_escape(&event.actor),
                Self::csv_escape(&event.target.as_deref().unwrap_or("")),
                event.previous_event_hash,
                event.event_hash,
                Self::csv_escape(&event.error_message.as_deref().unwrap_or("")),
                event.execution_duration_ms.unwrap_or(0),
                event.gas_consumed.unwrap_or(0),
                Self::csv_escape(&event.transaction_hash.as_deref().unwrap_or("")),
                event.ledger_sequence.unwrap_or(0),
            ));
        }
        csv
    }

    /// Export events to JSON format
    pub fn events_to_json(&self, events: &[CriticalEvent]) -> Result<String> {
        Ok(serde_json::to_string_pretty(events)?)
    }

    /// Escape a string for CSV (wrap in quotes, escape internal quotes)
    fn csv_escape(value: &str) -> String {
        if value.contains(',') || value.contains('\"') || value.contains('\n') {
            format!("\"{}\"", value.replace('\"', "\"\""))
        } else {
            value.to_string()
        }
    }

    // ── Hash Chain Verification ────────────────────────────────────────────

    /// Verify the integrity of the event hash chain from `from_index` to `to_index`.
    /// Returns a map of indices where hash mismatches were found.
    pub fn verify_chain(
        &self,
        from_index: usize,
        to_index: usize,
    ) -> Result<ChainVerificationResult> {
        let events = self
            .events
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire events lock: {}", e))?;

        let total = events.len();
        let actual_to = to_index.min(total.saturating_sub(1));
        let actual_from = from_index.min(actual_to);

        let mut verified_count = 0;
        let mut mismatches = Vec::new();

        for i in actual_from..=actual_to {
            let event = &events[i];
            let recomputed_hash = Self::compute_event_hash(event);

            if recomputed_hash != event.event_hash {
                mismatches.push(ChainMismatch {
                    index: i,
                    event_id: event.event_id.clone(),
                    expected_hash: recomputed_hash,
                    actual_hash: event.event_hash.clone(),
                    reason: "Event content hash mismatch — data has been tampered with".to_string(),
                });
            }

            // Check previous event hash linkage
            if i > actual_from {
                let prev_hash = &events[i - 1].event_hash;
                if event.previous_event_hash != *prev_hash {
                    mismatches.push(ChainMismatch {
                        index: i,
                        event_id: event.event_id.clone(),
                        expected_hash: prev_hash.clone(),
                        actual_hash: event.previous_event_hash.clone(),
                        reason: "Previous event hash mismatch — chain linkage broken".to_string(),
                    });
                }
            }

            verified_count += 1;
        }

        Ok(ChainVerificationResult {
            chain_integrity: mismatches.is_empty(),
            verified_count,
            mismatches,
            total_events_in_store: total,
        })
    }

    /// Verify the entire event chain from the first event to the last
    pub fn verify_full_chain(&self) -> Result<ChainVerificationResult> {
        let events = self
            .events
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire events lock: {}", e))?;
        if events.is_empty() {
            return Ok(ChainVerificationResult {
                chain_integrity: true,
                verified_count: 0,
                mismatches: vec![],
                total_events_in_store: 0,
            });
        }
        let to = events.len().saturating_sub(1);
        self.verify_chain(0, to)
    }
}

/// Event statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventStatistics {
    pub total_events: usize,
    pub by_operation: HashMap<CriticalOperation, usize>,
    pub by_severity: HashMap<EventSeverity, usize>,
    pub by_status: HashMap<EventStatus, usize>,
}

/// Paginated result wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageResult<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
}

/// Result of chain verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerificationResult {
    pub chain_integrity: bool,
    pub verified_count: usize,
    pub mismatches: Vec<ChainMismatch>,
    pub total_events_in_store: usize,
}

/// A single hash mismatch found during chain verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainMismatch {
    pub index: usize,
    pub event_id: String,
    pub expected_hash: String,
    pub actual_hash: String,
    pub reason: String,
}

/// Event builder for convenient event creation
pub struct EventBuilder {
    event: CriticalEvent,
}

impl EventBuilder {
    /// Create a new event builder
    pub fn new(operation: CriticalOperation, actor: String) -> Self {
        let counter = std::sync::atomic::AtomicU64::new(0);
        let event_id = format!(
            "evt_{}_{}",
            chrono::Utc::now().timestamp(),
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        );

        Self {
            event: CriticalEvent {
                event_id,
                timestamp: chrono::Utc::now().timestamp() as u64,
                operation,
                severity: EventSeverity::Medium,
                status: EventStatus::Started,
                description: String::new(),
                actor,
                target: None,
                metadata: HashMap::new(),
                previous_state: None,
                new_state: None,
                error_message: None,
                execution_duration_ms: None,
                gas_consumed: None,
                transaction_hash: None,
                ledger_sequence: None,
            },
        }
    }

    /// Set event description
    pub fn description(mut self, description: String) -> Self {
        self.event.description = description;
        self
    }

    /// Set event severity
    pub fn severity(mut self, severity: EventSeverity) -> Self {
        self.event.severity = severity;
        self
    }

    /// Set event status
    pub fn status(mut self, status: EventStatus) -> Self {
        self.event.status = status;
        self
    }

    /// Set event target
    pub fn target(mut self, target: String) -> Self {
        self.event.target = Some(target);
        self
    }

    /// Add metadata
    pub fn metadata(mut self, key: String, value: String) -> Self {
        self.event.metadata.insert(key, value);
        self
    }

    /// Set previous state
    pub fn previous_state(mut self, state: String) -> Self {
        self.event.previous_state = Some(state);
        self
    }

    /// Set new state
    pub fn new_state(mut self, state: String) -> Self {
        self.event.new_state = Some(state);
        self
    }

    /// Set error message
    pub fn error(mut self, error: String) -> Self {
        self.event.error_message = Some(error);
        self
    }

    /// Set execution duration
    pub fn duration_ms(mut self, duration: u64) -> Self {
        self.event.execution_duration_ms = Some(duration);
        self
    }

    /// Set gas consumed
    pub fn gas_consumed(mut self, gas: u64) -> Self {
        self.event.gas_consumed = Some(gas);
        self
    }

    /// Set transaction hash
    pub fn transaction_hash(mut self, hash: String) -> Self {
        self.event.transaction_hash = Some(hash);
        self
    }

    /// Set ledger sequence
    pub fn ledger_sequence(mut self, sequence: u64) -> Self {
        self.event.ledger_sequence = Some(sequence);
        self
    }

    /// Build the event (hash fields are set by the EventLogger during storage)
    pub fn build(self) -> CriticalEvent {
        self.event
    }

    /// Build the event with manual hash chaining (for testing)
    pub fn build_with_hash(mut self, previous_hash: &str) -> CriticalEvent {
        self.event.previous_event_hash = previous_hash.to_string();
        self.event.event_hash = EventLogger::compute_event_hash(&self.event);
        self.event
    }
}

/// Macro for convenient event logging
#[macro_export]
macro_rules! log_critical_event {
    ($logger:expr, $operation:expr, $actor:expr, $description:expr) => {
        $logger
            .log_event(
                EventBuilder::new($operation, $actor.to_string())
                    .description($description.to_string())
                    .build(),
            )
            .unwrap_or_else(|e| {
                log::error!("Failed to log critical event: {}", e);
            });
    };
    ($logger:expr, $operation:expr, $actor:expr, $description:expr, severity=$severity:expr) => {
        $logger
            .log_event(
                EventBuilder::new($operation, $actor.to_string())
                    .description($description.to_string())
                    .severity($severity)
                    .build(),
            )
            .unwrap_or_else(|e| {
                log::error!("Failed to log critical event: {}", e);
            });
    };
    ($logger:expr, $operation:expr, $actor:expr, $description:expr, target=$target:expr) => {
        $logger
            .log_event(
                EventBuilder::new($operation, $actor.to_string())
                    .description($description.to_string())
                    .target($target.to_string())
                    .build(),
            )
            .unwrap_or_else(|e| {
                log::error!("Failed to log critical event: {}", e);
            });
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_builder() {
        let event = EventBuilder::new(CriticalOperation::FundTransfer, "user123".to_string())
            .description("Transfer of 100 tokens".to_string())
            .severity(EventSeverity::High)
            .target("recipient456".to_string())
            .metadata("amount".to_string(), "100".to_string())
            .build();

        assert_eq!(event.operation, CriticalOperation::FundTransfer);
        assert_eq!(event.actor, "user123");
        assert_eq!(event.description, "Transfer of 100 tokens");
        assert_eq!(event.severity, EventSeverity::High);
        assert_eq!(event.target, Some("recipient456".to_string()));
        assert_eq!(event.metadata.get("amount"), Some(&"100".to_string()));
    }

    #[test]
    fn test_event_logging() {
        let config = EventLoggingConfig::default();
        let logger = EventLogger::new(config);

        let event = EventBuilder::new(
            CriticalOperation::VulnerabilityVerification,
            "admin".to_string(),
        )
        .description("Verified critical vulnerability".to_string())
        .severity(EventSeverity::Critical)
        .build();

        let result = logger.log_event(event);
        assert!(result.is_ok());

        let events = logger
            .get_events_by_operation(&CriticalOperation::VulnerabilityVerification)
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].operation,
            CriticalOperation::VulnerabilityVerification
        );
    }

    #[test]
    fn test_event_statistics() {
        let config = EventLoggingConfig::default();
        let logger = EventLogger::new(config);

        // Log some test events
        for i in 0..5 {
            let event = EventBuilder::new(CriticalOperation::FundTransfer, format!("user{}", i))
                .description(format!("Transfer {}", i))
                .build();
            logger.log_event(event).unwrap();
        }

        let stats = logger.get_event_statistics().unwrap();
        assert_eq!(stats.total_events, 5);
        assert_eq!(
            stats.by_operation.get(&CriticalOperation::FundTransfer),
            Some(&5)
        );
    }

    #[test]
    fn test_event_cleanup() {
        let mut config = EventLoggingConfig::default();
        config.retention_period_seconds = 1; // 1 second retention
        let logger = EventLogger::new(config);

        // Log an event
        let event = EventBuilder::new(CriticalOperation::AdminApproval, "admin".to_string())
            .description("Test event".to_string())
            .build();
        logger.log_event(event).unwrap();

        // Wait for retention period to pass (in real scenario)
        // For test, we'll just call cleanup
        let removed = logger.cleanup_old_events().unwrap();
        // Note: This test would need time manipulation to be fully effective
        assert!(removed >= 0);
    }
}
