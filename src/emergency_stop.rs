//! Emergency Stop Mechanism for Security Scanner
//! 
//! Provides graceful shutdown capabilities when critical vulnerabilities are detected
//! or when user initiates an emergency stop via signals.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};
use anyhow::Result;
use log::{info, warn, error};

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
    CriticalVulnerability { file_path: String, vulnerability: String },
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
    pub fn stop_on_critical_vulnerability(&self, file_path: &str, vulnerability: &str) -> Result<()> {
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
        use windows::Win32::System::Console::{SetConsoleCtrlHandler, PHANDLER_ROUTINE, CTRL_C_EVENT, CTRL_BREAK_EVENT, CTRL_CLOSE_EVENT};
        use std::sync::mpsc::channel;
        
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
                StopCommand::CriticalVulnerability { file_path, vulnerability } => {
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

