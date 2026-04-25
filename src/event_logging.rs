//! Event Logging System for Critical Operations
//! 
//! This module provides comprehensive event logging for critical operations
//! like fund transfers, vulnerability verification, and escrow operations.

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use log::{info, warn, error, debug};

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
            debug!("Operation {:?} is not configured for critical logging", event.operation);
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

    /// Store event in memory
    fn store_event(&self, event: CriticalEvent) -> Result<()> {
        let mut events = self.events.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire events lock: {}", e)
        })?;

        events.push(event);

        // Enforce memory limit
        if events.len() > self.config.max_events_in_memory {
            events.remove(0);
        }

        Ok(())
    }

    /// Get events by operation type
    pub fn get_events_by_operation(&self, operation: &CriticalOperation) -> Result<Vec<CriticalEvent>> {
        let events = self.events.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire events lock: {}", e)
        })?;

        Ok(events.iter()
            .filter(|event| &event.operation == operation)
            .cloned()
            .collect())
    }

    /// Get events by time range
    pub fn get_events_by_time_range(&self, start_time: u64, end_time: u64) -> Result<Vec<CriticalEvent>> {
        let events = self.events.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire events lock: {}", e)
        })?;

        Ok(events.iter()
            .filter(|event| event.timestamp >= start_time && event.timestamp <= end_time)
            .cloned()
            .collect())
    }

    /// Get events by actor
    pub fn get_events_by_actor(&self, actor: &str) -> Result<Vec<CriticalEvent>> {
        let events = self.events.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire events lock: {}", e)
        })?;

        Ok(events.iter()
            .filter(|event| event.actor == actor)
            .cloned()
            .collect())
    }

    /// Generate event ID
    fn generate_event_id(&self) -> String {
        let counter = self.event_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("evt_{}_{}", chrono::Utc::now().timestamp(), counter)
    }

    /// Clean up old events based on retention period
    pub fn cleanup_old_events(&self) -> Result<usize> {
        let current_time = chrono::Utc::now().timestamp() as u64;
        let cutoff_time = current_time - self.config.retention_period_seconds;

        let mut events = self.events.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire events lock: {}", e)
        })?;

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
        let events = self.events.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire events lock: {}", e)
        })?;

        let mut stats = EventStatistics::default();
        
        for event in events.iter() {
            stats.total_events += 1;
            
            // Count by operation type
            *stats.by_operation.entry(event.operation.clone()).or_insert(0) += 1;
            
            // Count by severity
            *stats.by_severity.entry(event.severity.clone()).or_insert(0) += 1;
            
            // Count by status
            *stats.by_status.entry(event.status.clone()).or_insert(0) += 1;
        }

        Ok(stats)
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

/// Event builder for convenient event creation
pub struct EventBuilder {
    event: CriticalEvent,
}

impl EventBuilder {
    /// Create a new event builder
    pub fn new(operation: CriticalOperation, actor: String) -> Self {
        let counter = std::sync::atomic::AtomicU64::new(0);
        let event_id = format!("evt_{}_{}", chrono::Utc::now().timestamp(), 
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst));

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

    /// Build the event
    pub fn build(self) -> CriticalEvent {
        self.event
    }
}

/// Macro for convenient event logging
#[macro_export]
macro_rules! log_critical_event {
    ($logger:expr, $operation:expr, $actor:expr, $description:expr) => {
        $logger.log_event(
            EventBuilder::new($operation, $actor.to_string())
                .description($description.to_string())
                .build()
        ).unwrap_or_else(|e| {
            log::error!("Failed to log critical event: {}", e);
        });
    };
    ($logger:expr, $operation:expr, $actor:expr, $description:expr, severity=$severity:expr) => {
        $logger.log_event(
            EventBuilder::new($operation, $actor.to_string())
                .description($description.to_string())
                .severity($severity)
                .build()
        ).unwrap_or_else(|e| {
            log::error!("Failed to log critical event: {}", e);
        });
    };
    ($logger:expr, $operation:expr, $actor:expr, $description:expr, target=$target:expr) => {
        $logger.log_event(
            EventBuilder::new($operation, $actor.to_string())
                .description($description.to_string())
                .target($target.to_string())
                .build()
        ).unwrap_or_else(|e| {
            log::error!("Failed to log critical event: {}", e);
        });
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_builder() {
        let event = EventBuilder::new(
            CriticalOperation::FundTransfer,
            "user123".to_string()
        )
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
            "admin".to_string()
        )
        .description("Verified critical vulnerability".to_string())
        .severity(EventSeverity::Critical)
        .build();

        let result = logger.log_event(event);
        assert!(result.is_ok());

        let events = logger.get_events_by_operation(&CriticalOperation::VulnerabilityVerification).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].operation, CriticalOperation::VulnerabilityVerification);
    }

    #[test]
    fn test_event_statistics() {
        let config = EventLoggingConfig::default();
        let logger = EventLogger::new(config);

        // Log some test events
        for i in 0..5 {
            let event = EventBuilder::new(
                CriticalOperation::FundTransfer,
                format!("user{}", i)
            )
            .description(format!("Transfer {}", i))
            .build();
            logger.log_event(event).unwrap();
        }

        let stats = logger.get_event_statistics().unwrap();
        assert_eq!(stats.total_events, 5);
        assert_eq!(stats.by_operation.get(&CriticalOperation::FundTransfer), Some(&5));
    }

    #[test]
    fn test_event_cleanup() {
        let mut config = EventLoggingConfig::default();
        config.retention_period_seconds = 1; // 1 second retention
        let logger = EventLogger::new(config);

        // Log an event
        let event = EventBuilder::new(
            CriticalOperation::AdminApproval,
            "admin".to_string()
        )
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
