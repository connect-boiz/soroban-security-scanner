//! Emergency Stop Mechanism for Security Scanner
//!
//! Provides graceful shutdown capabilities when critical vulnerabilities are detected
//! or when user initiates an emergency stop via signals.
//! Also includes a ScanWatchdog that detects and halts stuck scans automatically.

use anyhow::Result;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Emergency stop state and control
#[derive(Debug, Clone)]
pub struct EmergencyStop {
    /// Flag indicating if emergency stop has been triggered
    is_stopped: Arc<AtomicBool>,
    /// Channel for sending stop commands
    stop_sender: Sender<StopCommand>,
}

/// Commands that can trigger emergency stop
#[derive(Debug, Clone)]
pub enum StopCommand {
    /// User initiated stop (Ctrl+C, SIGTERM)
    UserInitiated { reason: String },
    /// Critical vulnerability detected
    CriticalVulnerability {
        file_path: String,
        vulnerability: String,
    },
    /// Scanner timeout exceeded
    Timeout { duration: Duration },
    /// Resource exhaustion
    ResourceExhaustion { resource: String },
}

/// Stop reason details
#[derive(Debug, Clone)]
pub struct StopReason {
    pub command: StopCommand,
    pub timestamp: Instant,
    pub partial_results: Option<String>,
}

impl EmergencyStop {
    /// Create a new emergency stop controller
    pub fn new() -> Result<Self> {
        let (stop_sender, stop_receiver) = mpsc::channel();
        let is_stopped = Arc::new(AtomicBool::new(false));
        let is_stopped_clone = is_stopped.clone();

        // Start the emergency stop listener thread
        thread::spawn(move || {
            Self::emergency_stop_listener(stop_receiver, is_stopped_clone);
        });

        // Setup signal handlers for graceful shutdown
        Self::setup_signal_handlers(stop_sender.clone())?;

        Ok(Self {
            is_stopped,
            stop_sender,
        })
    }

    /// Check if emergency stop has been triggered
    pub fn is_stopped(&self) -> bool {
        self.is_stopped.load(Ordering::Relaxed)
    }

    /// Trigger emergency stop with specific reason
    pub fn trigger_stop(&self, command: StopCommand) -> Result<()> {
        if self.is_stopped() {
            warn!("Emergency stop already triggered");
            return Ok(());
        }

        info!("Triggering emergency stop: {:?}", command);
        self.stop_sender.send(command)?;
        Ok(())
    }

    /// Trigger stop due to critical vulnerability
    pub fn stop_on_critical_vulnerability(
        &self,
        file_path: &str,
        vulnerability: &str,
    ) -> Result<()> {
        self.trigger_stop(StopCommand::CriticalVulnerability {
            file_path: file_path.to_string(),
            vulnerability: vulnerability.to_string(),
        })
    }

    /// Trigger stop due to timeout
    pub fn stop_on_timeout(&self, duration: Duration) -> Result<()> {
        self.trigger_stop(StopCommand::Timeout { duration })
    }

    /// Trigger stop due to resource exhaustion
    pub fn stop_on_resource_exhaustion(&self, resource: &str) -> Result<()> {
        self.trigger_stop(StopCommand::ResourceExhaustion {
            resource: resource.to_string(),
        })
    }

    /// Allow cancellation of ongoing operations
    pub fn allow_cancellation<F, R>(&self, operation: F) -> Option<R>
    where
        F: FnOnce() -> R,
    {
        if self.is_stopped() {
            info!("Operation cancelled due to emergency stop");
            return None;
        }

        // Execute the operation with periodic stop checks
        // For long-running operations, this should check periodically
        Some(operation())
    }

    /// Setup signal handlers for graceful shutdown
    #[cfg(unix)]
    fn setup_signal_handlers(sender: Sender<StopCommand>) -> Result<()> {
        use signal_hook::{consts::SIGTERM, iterator::Signals};

        let mut signals = Signals::new(&[SIGTERM, signal_hook::consts::SIGINT])?;

        thread::spawn(move || {
            for sig in &mut signals {
                match sig {
                    SIGTERM | signal_hook::consts::SIGINT => {
                        let _ = sender.send(StopCommand::UserInitiated {
                            reason: format!("Signal {} received", sig),
                        });
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// Setup signal handlers for Windows
    #[cfg(windows)]
    fn setup_signal_handlers(sender: Sender<StopCommand>) -> Result<()> {
        use std::sync::mpsc::channel;
        use windows::Win32::System::Console::{
            SetConsoleCtrlHandler, CTRL_BREAK_EVENT, CTRL_CLOSE_EVENT, CTRL_C_EVENT,
            PHANDLER_ROUTINE,
        };

        let (tx, rx) = channel();
        let sender_clone = sender.clone();

        thread::spawn(move || {
            if let Ok(reason) = rx.recv() {
                let _ = sender_clone.send(StopCommand::UserInitiated { reason });
            }
        });

        unsafe {
            let handler: PHANDLER_ROUTINE = std::mem::transmute(Box::new(move |ctrl_type| {
                let reason = match ctrl_type {
                    CTRL_C_EVENT => "Ctrl+C pressed".to_string(),
                    CTRL_BREAK_EVENT => "Ctrl+Break pressed".to_string(),
                    CTRL_CLOSE_EVENT => "Console window closed".to_string(),
                    _ => format!("Control signal {}", ctrl_type),
                };
                let _ = tx.send(reason);
                true // Return true to indicate we handled the signal
            }));

            SetConsoleCtrlHandler(handler, true)?;
        }

        Ok(())
    }

    /// Emergency stop listener thread
    fn emergency_stop_listener(receiver: Receiver<StopCommand>, is_stopped: Arc<AtomicBool>) {
        while let Ok(command) = receiver.recv() {
            match &command {
                StopCommand::CriticalVulnerability {
                    file_path,
                    vulnerability,
                } => {
                    error!("🚨 CRITICAL VULNERABILITY DETECTED - EMERGENCY STOP TRIGGERED");
                    error!("File: {}", file_path);
                    error!("Vulnerability: {}", vulnerability);
                }
                StopCommand::UserInitiated { reason } => {
                    warn!("🛑 User initiated emergency stop: {}", reason);
                }
                StopCommand::Timeout { duration } => {
                    error!("⏰ Scanner timeout exceeded: {:?}", duration);
                }
                StopCommand::ResourceExhaustion { resource } => {
                    error!("💾 Resource exhaustion detected: {}", resource);
                }
            }

            // Set the stop flag
            is_stopped.store(true, Ordering::Relaxed);

            // Give some time for graceful shutdown
            thread::sleep(Duration::from_millis(100));

            info!("Emergency stop completed");
            break;
        }
    }
}

impl Default for EmergencyStop {
    fn default() -> Self {
        Self::new().expect("Failed to create emergency stop controller")
    }
}

/// Macro for checking emergency stop during operations
#[macro_export]
macro_rules! check_emergency_stop {
    ($emergency_stop:expr) => {
        if $emergency_stop.is_stopped() {
            log::info!("Operation cancelled due to emergency stop");
            return Ok(Vec::new()); // or appropriate default return
        }
    };
}

/// Utility trait for emergency stop functionality
pub trait EmergencyStopExt {
    /// Execute operation with emergency stop checks
    fn with_emergency_stop<F, R>(&self, emergency_stop: &EmergencyStop, operation: F) -> Option<R>
    where
        F: FnOnce() -> R;
}

impl<T> EmergencyStopExt for T {
    fn with_emergency_stop<F, R>(&self, emergency_stop: &EmergencyStop, operation: F) -> Option<R>
    where
        F: FnOnce() -> R,
    {
        emergency_stop.allow_cancellation(operation)
    }
}

/// ScanWatchdog monitors scan progress via heartbeat and detects stuck scans.
/// If no heartbeat update is received within the timeout window, the watchdog
/// automatically triggers an emergency stop.
#[derive(Debug, Clone)]
pub struct ScanWatchdog {
    /// Timestamp (in millis since epoch) of the last heartbeat
    last_heartbeat: Arc<AtomicU64>,
    /// Timeout in seconds before automatic stop
    timeout_seconds: u64,
    /// Total number of files processed so far
    files_processed: Arc<AtomicU64>,
    /// Current file being processed (name/path)
    current_file: Arc<Mutex<Option<String>>>,
    /// Whether the watchdog has timed out
    timed_out: Arc<AtomicBool>,
    /// Reference to emergency stop to trigger on timeout
    emergency_stop: Option<EmergencyStop>,
    /// Whether the monitor thread is running
    monitor_active: Arc<AtomicBool>,
}

impl ScanWatchdog {
    /// Create a new ScanWatchdog with default timeout (120s)
    pub fn new() -> Self {
        Self::with_timeout(120)
    }

    /// Create a new ScanWatchdog with custom timeout in seconds
    pub fn with_timeout(timeout_seconds: u64) -> Self {
        let now = Self::now_millis();
        Self {
            last_heartbeat: Arc::new(AtomicU64::new(now)),
            timeout_seconds,
            files_processed: Arc::new(AtomicU64::new(0)),
            current_file: Arc::new(Mutex::new(None)),
            timed_out: Arc::new(AtomicBool::new(false)),
            emergency_stop: None,
            monitor_active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Attach an EmergencyStop to this watchdog so it can trigger auto-stop on timeout
    pub fn with_emergency_stop(mut self, emergency_stop: EmergencyStop) -> Self {
        self.emergency_stop = Some(emergency_stop);
        self
    }

    /// Record a heartbeat — called after each file is processed.
    /// Updates the last heartbeat timestamp.
    pub fn heartbeat(&self) {
        self.last_heartbeat
            .store(Self::now_millis(), Ordering::Relaxed);
        self.files_processed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a heartbeat with the current file name being processed.
    pub fn heartbeat_with_file(&self, file_name: &str) {
        self.heartbeat();
        if let Ok(mut current) = self.current_file.lock() {
            *current = Some(file_name.to_string());
        }
    }

    /// Get the time (in millis) since the last heartbeat
    pub fn millis_since_last_heartbeat(&self) -> u64 {
        let last = self.last_heartbeat.load(Ordering::Relaxed);
        Self::now_millis().saturating_sub(last)
    }

    /// Check if the watchdog has timed out
    pub fn has_timed_out(&self) -> bool {
        self.timed_out.load(Ordering::Relaxed)
    }

    /// Check if the monitored scan has stalled (no heartbeat within timeout window)
    pub fn is_stalled(&self) -> bool {
        self.millis_since_last_heartbeat() > self.timeout_seconds * 1000
    }

    /// Get the number of files processed so far
    pub fn files_processed(&self) -> u64 {
        self.files_processed.load(Ordering::Relaxed)
    }

    /// Get the current file being processed
    pub fn current_file(&self) -> Option<String> {
        if let Ok(guard) = self.current_file.lock() {
            guard.clone()
        } else {
            None
        }
    }

    /// Get the watchdog timeout in seconds
    pub fn timeout_seconds(&self) -> u64 {
        self.timeout_seconds
    }

    /// Start the background monitor thread that periodically checks for stalls.
    /// If a stall is detected, it triggers the emergency stop and logs the event.
    pub fn start_monitoring(&self) {
        if self.emergency_stop.is_none() {
            warn!(
                "ScanWatchdog: No EmergencyStop attached — monitoring will log but not auto-stop"
            );
        }
        if self.monitor_active.load(Ordering::Relaxed) {
            return; // Already running
        }
        self.monitor_active.store(true, Ordering::Relaxed);

        let last_heartbeat = self.last_heartbeat.clone();
        let timed_out = self.timed_out.clone();
        let timeout_s = self.timeout_seconds;
        let monitor_active = self.monitor_active.clone();
        let emergency_stop = self.emergency_stop.clone();
        let current_file = self.current_file.clone();

        thread::spawn(move || {
            let check_interval = Duration::from_secs(5); // Check every 5 seconds
            loop {
                thread::sleep(check_interval);
                if !monitor_active.load(Ordering::Relaxed) {
                    break;
                }

                let last = last_heartbeat.load(Ordering::Relaxed);
                let elapsed_secs = (Self::now_millis().saturating_sub(last)) / 1000;

                if elapsed_secs > timeout_s {
                    warn!(
                        "🛑 ScanWatchdog TIMEOUT — no heartbeat for {}s (threshold: {}s)",
                        elapsed_secs, timeout_s
                    );

                    if let Ok(guard) = current_file.lock() {
                        if let Some(ref file) = *guard {
                            error!("💀 Stuck on file: {}", file);
                        }
                    }

                    timed_out.store(true, Ordering::Relaxed);

                    if let Some(ref es) = emergency_stop {
                        let _ = es.stop_on_timeout(Duration::from_secs(timeout_s));
                        info!("✅ ScanWatchdog triggered emergency stop due to timeout");
                    }

                    break;
                }
            }
        });
    }

    /// Stop the monitor thread
    pub fn stop_monitoring(&self) {
        self.monitor_active.store(false, Ordering::Relaxed);
    }

    /// Get a human-readable status report
    pub fn get_status(&self) -> WatchdogStatus {
        let elapsed = self.millis_since_last_heartbeat();
        WatchdogStatus {
            is_monitoring: self.monitor_active.load(Ordering::Relaxed),
            has_timed_out: self.has_timed_out(),
            is_stalled: self.is_stalled(),
            files_processed: self.files_processed(),
            current_file: self.current_file(),
            timeout_seconds: self.timeout_seconds,
            millis_since_last_heartbeat: elapsed,
            heartbeat_age_seconds: elapsed / 1000,
        }
    }

    fn now_millis() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Reset the watchdog state for a new scan
    pub fn reset(&self) {
        self.last_heartbeat
            .store(Self::now_millis(), Ordering::Relaxed);
        self.files_processed.store(0, Ordering::Relaxed);
        self.timed_out.store(false, Ordering::Relaxed);
        if let Ok(mut current) = self.current_file.lock() {
            *current = None;
        }
    }
}

impl Default for ScanWatchdog {
    fn default() -> Self {
        Self::new()
    }
}

/// Status report from the ScanWatchdog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchdogStatus {
    pub is_monitoring: bool,
    pub has_timed_out: bool,
    pub is_stalled: bool,
    pub files_processed: u64,
    pub current_file: Option<String>,
    pub timeout_seconds: u64,
    pub millis_since_last_heartbeat: u64,
    pub heartbeat_age_seconds: u64,
}
