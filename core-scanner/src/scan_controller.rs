//! Emergency Stop and Scan Control Module
//!
//! This module provides emergency stop, pause, and resume functionality for security scans.
//! It allows terminating scans when critical vulnerabilities are detected or when manual
//! intervention is required.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex, broadcast};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tracing::{info, warn, error, debug};

/// Scan control commands that can be issued
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanCommand {
    /// Stop the scan immediately
    Stop,
    /// Pause the scan (can be resumed)
    Pause,
    /// Resume a paused scan
    Resume,
    /// Get current scan status
    Status,
}

/// Current status of a scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanStatus {
    /// Scan is running normally
    Running,
    /// Scan is paused and can be resumed
    Paused,
    /// Scan was stopped and cannot be resumed
    Stopped,
    /// Scan completed successfully
    Completed,
    /// Scan failed with an error
    Failed,
}

/// Scan control information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanControl {
    /// Unique scan identifier
    pub scan_id: String,
    /// Current status of the scan
    pub status: ScanStatus,
    /// Reason for stop/pause if applicable
    pub reason: Option<String>,
    /// Timestamp when scan was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Timestamp when scan was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Number of critical vulnerabilities found (triggers auto-stop)
    pub critical_count: u32,
    /// Whether auto-stop is enabled
    pub auto_stop_enabled: bool,
    /// Threshold for auto-stop (number of critical vulnerabilities)
    pub auto_stop_threshold: u32,
}

impl ScanControl {
    pub fn new(scan_id: String, auto_stop_enabled: bool, auto_stop_threshold: u32) -> Self {
        let now = chrono::Utc::now();
        Self {
            scan_id,
            status: ScanStatus::Running,
            reason: None,
            created_at: now,
            updated_at: now,
            critical_count: 0,
            auto_stop_enabled,
            auto_stop_threshold,
        }
    }

    pub fn update_status(&mut self, status: ScanStatus, reason: Option<String>) {
        self.status = status;
        self.reason = reason;
        self.updated_at = chrono::Utc::now();
    }

    pub fn increment_critical(&mut self) -> bool {
        self.critical_count += 1;
        self.updated_at = chrono::Utc::now();
        
        // Check if auto-stop should be triggered
        if self.auto_stop_enabled && self.critical_count >= self.auto_stop_threshold {
            self.status = ScanStatus::Stopped;
            self.reason = Some(format!(
                "Auto-stop triggered: {} critical vulnerabilities found (threshold: {})",
                self.critical_count, self.auto_stop_threshold
            ));
            return true;
        }
        false
    }
}

/// Global scan controller for managing multiple scans
#[derive(Clone)]
pub struct ScanController {
    /// Map of scan_id to scan control information
    scans: Arc<RwLock<HashMap<String, ScanControl>>>,
    /// Command sender for issuing commands to scans
    command_sender: broadcast::Sender<(String, ScanCommand)>,
}

impl ScanController {
    /// Create a new scan controller
    pub fn new() -> Self {
        let (command_sender, _) = broadcast::channel(1000);
        
        Self {
            scans: Arc::new(RwLock::new(HashMap::new())),
            command_sender,
        }
    }

    /// Register a new scan for control
    pub async fn register_scan(
        &self,
        scan_id: String,
        auto_stop_enabled: bool,
        auto_stop_threshold: u32,
    ) -> Result<broadcast::Receiver<(String, ScanCommand)>> {
        let control = ScanControl::new(scan_id.clone(), auto_stop_enabled, auto_stop_threshold);
        
        {
            let mut scans = self.scans.write().await;
            scans.insert(scan_id.clone(), control);
        }
        
        info!("Registered scan {} for control", scan_id);
        
        // Return a receiver for this scan to listen for commands
        Ok(self.command_sender.subscribe())
    }

    /// Issue a command to a specific scan
    pub async fn issue_command(&self, scan_id: &str, command: ScanCommand) -> Result<()> {
        debug!("Issuing command {:?} to scan {}", command, scan_id);
        
        // Update scan status based on command
        {
            let mut scans = self.scans.write().await;
            if let Some(control) = scans.get_mut(scan_id) {
                match command {
                    ScanCommand::Stop => {
                        control.update_status(ScanStatus::Stopped, Some("Manual stop requested".to_string()));
                    }
                    ScanCommand::Pause => {
                        if matches!(control.status, ScanStatus::Running) {
                            control.update_status(ScanStatus::Paused, Some("Manual pause requested".to_string()));
                        }
                    }
                    ScanCommand::Resume => {
                        if matches!(control.status, ScanStatus::Paused) {
                            control.update_status(ScanStatus::Running, Some("Scan resumed".to_string()));
                        }
                    }
                    ScanCommand::Status => {
                        // Status query, no change needed
                    }
                }
            } else {
                return Err(anyhow::anyhow!("Scan {} not found", scan_id));
            }
        }
        
        // Send command to the scan
        if let Err(e) = self.command_sender.send((scan_id.to_string(), command)) {
            warn!("Failed to send command to scan {}: {}", scan_id, e);
        }
        
        Ok(())
    }

    /// Get the current status of a scan
    pub async fn get_scan_status(&self, scan_id: &str) -> Result<Option<ScanControl>> {
        let scans = self.scans.read().await;
        Ok(scans.get(scan_id).cloned())
    }

    /// Get all active scans
    pub async fn get_active_scans(&self) -> Result<Vec<ScanControl>> {
        let scans = self.scans.read().await;
        Ok(scans
            .values()
            .filter(|control| matches!(control.status, ScanStatus::Running | ScanStatus::Paused))
            .cloned()
            .collect())
    }

    /// Update critical vulnerability count for a scan
    pub async fn increment_critical_count(&self, scan_id: &str) -> Result<bool> {
        let mut scans = self.scans.write().await;
        if let Some(control) = scans.get_mut(scan_id) {
            let should_stop = control.increment_critical();
            
            if should_stop {
                warn!(
                    "Auto-stop triggered for scan {}: {} critical vulnerabilities",
                    scan_id, control.critical_count
                );
                
                // Send stop command
                if let Err(e) = self.command_sender.send((
                    scan_id.to_string(),
                    ScanCommand::Stop
                )) {
                    error!("Failed to send auto-stop command: {}", e);
                }
            }
            
            Ok(should_stop)
        } else {
            Err(anyhow::anyhow!("Scan {} not found", scan_id))
        }
    }

    /// Mark a scan as completed
    pub async fn mark_completed(&self, scan_id: &str) -> Result<()> {
        let mut scans = self.scans.write().await;
        if let Some(control) = scans.get_mut(scan_id) {
            control.update_status(ScanStatus::Completed, None);
            info!("Scan {} marked as completed", scan_id);
        } else {
            return Err(anyhow::anyhow!("Scan {} not found", scan_id));
        }
        Ok(())
    }

    /// Mark a scan as failed
    pub async fn mark_failed(&self, scan_id: &str, error: &str) -> Result<()> {
        let mut scans = self.scans.write().await;
        if let Some(control) = scans.get_mut(scan_id) {
            control.update_status(ScanStatus::Failed, Some(error.to_string()));
            error!("Scan {} marked as failed: {}", scan_id, error);
        } else {
            return Err(anyhow::anyhow!("Scan {} not found", scan_id));
        }
        Ok(())
    }

    /// Clean up old completed scans (older than specified duration)
    pub async fn cleanup_old_scans(&self, older_than: chrono::Duration) -> Result<usize> {
        let mut scans = self.scans.write().await;
        let cutoff = chrono::Utc::now() - older_than;
        
        let initial_count = scans.len();
        scans.retain(|_, control| {
            // Keep scans that are still running or paused, or completed/failed recently
            match control.status {
                ScanStatus::Running | ScanStatus::Paused => true,
                ScanStatus::Completed | ScanStatus::Stopped | ScanStatus::Failed => {
                    control.updated_at > cutoff
                }
            }
        });
        
        let cleaned_count = initial_count - scans.len();
        if cleaned_count > 0 {
            info!("Cleaned up {} old scan records", cleaned_count);
        }
        
        Ok(cleaned_count)
    }
}

impl Default for ScanController {
    fn default() -> Self {
        Self::new()
    }
}
