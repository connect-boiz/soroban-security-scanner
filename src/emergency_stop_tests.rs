//! Tests for emergency stop and scan watchdog functionality

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_scan_watchdog_creation() {
        let watchdog = ScanWatchdog::new();
        assert!(!watchdog.has_timed_out(), "Watchdog should not have timed out initially");
        assert!(!watchdog.is_stalled(), "Watchdog should not be stalled initially");
        assert_eq!(watchdog.files_processed(), 0);
        assert_eq!(watchdog.timeout_seconds(), 120);
        assert!(watchdog.current_file().is_none());
    }

    #[test]
    fn test_scan_watchdog_custom_timeout() {
        let watchdog = ScanWatchdog::with_timeout(30);
        assert_eq!(watchdog.timeout_seconds(), 30);
    }

    #[test]
    fn test_scan_watchdog_heartbeat() {
        let watchdog = ScanWatchdog::new();
        
        // Initial state
        assert_eq!(watchdog.files_processed(), 0);
        
        // After heartbeat
        watchdog.heartbeat();
        assert_eq!(watchdog.files_processed(), 1);
        
        // Multiple heartbeats
        watchdog.heartbeat();
        watchdog.heartbeat();
        assert_eq!(watchdog.files_processed(), 3);
    }

    #[test]
    fn test_scan_watchdog_heartbeat_with_file() {
        let watchdog = ScanWatchdog::new();
        watchdog.heartbeat_with_file("test.rs");
        
        assert_eq!(watchdog.files_processed(), 1);
        assert_eq!(watchdog.current_file(), Some("test.rs".to_string()));
    }

    #[test]
    fn test_scan_watchdog_reset() {
        let watchdog = ScanWatchdog::new();
        
        watchdog.heartbeat_with_file("test.rs");
        watchdog.heartbeat();
        assert_eq!(watchdog.files_processed(), 2);
        assert!(watchdog.current_file().is_some());
        
        watchdog.reset();
        assert_eq!(watchdog.files_processed(), 0);
        assert!(!watchdog.has_timed_out());
        assert!(watchdog.current_file().is_none());
    }

    #[test]
    fn test_scan_watchdog_get_status() {
        let watchdog = ScanWatchdog::new();
        watchdog.heartbeat_with_file("main.rs");
        
        let status = watchdog.get_status();
        assert!(!status.has_timed_out);
        assert!(!status.is_monitoring); // Monitor not started yet
        assert_eq!(status.files_processed, 1);
        assert_eq!(status.current_file, Some("main.rs".to_string()));
        assert_eq!(status.timeout_seconds, 120);
        assert!(status.heartbeat_age_seconds < 1); // Just did heartbeat
    }

    #[test]
    fn test_scan_watchdog_with_emergency_stop() {
        let es = EmergencyStop::new().unwrap();
        let watchdog = ScanWatchdog::with_timeout(1)
            .with_emergency_stop(es.clone()); // 1 second timeout for quick test
        
        // Should not be stopped initially
        assert!(!es.is_stopped());
        assert!(!watchdog.has_timed_out());
    }

    #[test]
    fn test_scan_watchdog_stall_detection() {
        let watchdog = ScanWatchdog::with_timeout(1); // 1 second timeout
        
        // Immediately after creation, should not be stalled
        assert!(!watchdog.is_stalled());
        
        // Wait longer than timeout
        thread::sleep(Duration::from_millis(1100));
        
        // Should now be stalled
        assert!(watchdog.is_stalled());
    }

    #[test]
    fn test_scan_watchdog_heartbeat_prevents_stall() {
        let watchdog = ScanWatchdog::with_timeout(2); // 2 second timeout
        
        // Heartbeat periodically to prevent stall
        for _ in 0..5 {
            thread::sleep(Duration::from_millis(200));
            watchdog.heartbeat();
        }
        
        // Should not be stalled because we kept sending heartbeats
        assert!(!watchdog.is_stalled());
        
        // After stopping heartbeats, give it time to exceed timeout
        thread::sleep(Duration::from_millis(2500));
        assert!(watchdog.is_stalled());
    }

    #[test]
    fn test_scan_watchdog_monitor_auto_stop() {
        let es = EmergencyStop::new().unwrap();
        let watchdog = ScanWatchdog::with_timeout(1) // 1 second timeout
            .with_emergency_stop(es.clone());
        
        // Start monitoring
        watchdog.start_monitoring();
        assert!(!es.is_stopped());
        
        // Wait for timeout
        thread::sleep(Duration::from_millis(1500));
        
        // Emergency stop should have been triggered by watchdog
        assert!(watchdog.has_timed_out());
        
        watchdog.stop_monitoring();
    }

    #[test]
    fn test_scan_watchdog_files_processed_tracking() {
        let watchdog = ScanWatchdog::new();
        
        // Simulate processing 100 files
        for i in 1..=100 {
            watchdog.heartbeat_with_file(&format!("file_{}.rs", i));
        }
        
        assert_eq!(watchdog.files_processed(), 100);
        assert_eq!(watchdog.current_file(), Some("file_100.rs".to_string()));
    }

    #[test]
    fn test_scan_watchdog_reset_between_scans() {
        let watchdog = ScanWatchdog::new();
        
        // First scan
        for i in 1..=5 {
            watchdog.heartbeat_with_file(&format!("scan1_file_{}.rs", i));
        }
        assert_eq!(watchdog.files_processed(), 5);
        
        // Reset for second scan
        watchdog.reset();
        assert_eq!(watchdog.files_processed(), 0);
        assert!(watchdog.current_file().is_none());
        assert!(!watchdog.has_timed_out());
        
        // Second scan
        for i in 1..=3 {
            watchdog.heartbeat_with_file(&format!("scan2_file_{}.rs", i));
        }
        assert_eq!(watchdog.files_processed(), 3);
    }

    #[test]
    fn test_emergency_stop_creation() {
        let stop = EmergencyStop::new().unwrap();
        assert!(!stop.is_stopped());
    }

    #[test]
    fn test_emergency_stop_trigger() {
        let stop = EmergencyStop::new().unwrap();
        stop.trigger_stop(StopCommand::UserInitiated {
            reason: "Test stop".to_string(),
        }).unwrap();
        
        // Give some time for the async processing
        thread::sleep(Duration::from_millis(200));
        assert!(stop.is_stopped());
    }

    #[test]
    fn test_critical_vulnerability_stop() {
        let stop = EmergencyStop::new().unwrap();
        stop.stop_on_critical_vulnerability("test.rs", "Infinite Mint").unwrap();
        
        thread::sleep(Duration::from_millis(200));
        assert!(stop.is_stopped());
    }

    #[test]
    fn test_timeout_stop() {
        let stop = EmergencyStop::new().unwrap();
        stop.stop_on_timeout(Duration::from_secs(300)).unwrap();
        
        thread::sleep(Duration::from_millis(200));
        assert!(stop.is_stopped());
    }

    #[test]
    fn test_resource_exhaustion_stop() {
        let stop = EmergencyStop::new().unwrap();
        stop.stop_on_resource_exhaustion("Memory").unwrap();
        
        thread::sleep(Duration::from_millis(200));
        assert!(stop.is_stopped());
    }

    #[test]
    fn test_cancellation() {
        let stop = EmergencyStop::new().unwrap();
        
        // Test normal operation
        let result = stop.allow_cancellation(|| 42);
        assert_eq!(result, Some(42));
        
        // Test cancelled operation
        stop.trigger_stop(StopCommand::UserInitiated {
            reason: "Test".to_string(),
        }).unwrap();
        
        thread::sleep(Duration::from_millis(200));
        let result = stop.allow_cancellation(|| 42);
        assert_eq!(result, None);
    }

    #[test]
    fn test_multiple_triggers() {
        let stop = EmergencyStop::new().unwrap();
        
        // First trigger should work
        stop.trigger_stop(StopCommand::UserInitiated {
            reason: "First trigger".to_string(),
        }).unwrap();
        
        thread::sleep(Duration::from_millis(200));
        
        // Second trigger should be ignored (no panic)
        let result = stop.trigger_stop(StopCommand::UserInitiated {
            reason: "Second trigger".to_string(),
        });
        assert!(result.is_ok());
        
        assert!(stop.is_stopped());
    }

    #[test]
    fn test_emergency_stop_ext_trait() {
        let stop = EmergencyStop::new().unwrap();
        let data = "test data";
        
        // Test normal operation
        let result = data.with_emergency_stop(&stop, || data.len());
        assert_eq!(result, Some(9));
        
        // Test cancelled operation
        stop.trigger_stop(StopCommand::UserInitiated {
            reason: "Test".to_string(),
        }).unwrap();
        
        thread::sleep(Duration::from_millis(200));
        let result = data.with_emergency_stop(&stop, || data.len());
        assert_eq!(result, None);
    }

    #[test]
    fn test_scanner_integration() {
        let emergency_stop = EmergencyStop::new().unwrap();
        let scanner = SecurityScanner::new_with_emergency_stop(emergency_stop.clone()).unwrap();
        
        // Scanner should have the emergency stop
        assert!(!scanner.emergency_stop.is_stopped());
        
        // Trigger emergency stop
        scanner.emergency_stop.trigger_stop(StopCommand::UserInitiated {
            reason: "Test".to_string(),
        }).unwrap();
        
        thread::sleep(Duration::from_millis(200));
        assert!(scanner.emergency_stop.is_stopped());
    }

    #[test]
    fn test_invariant_scanner_integration() {
        let emergency_stop = EmergencyStop::new().unwrap();
        let scanner = InvariantScanner::new_with_emergency_stop(emergency_stop.clone()).unwrap();
        
        // Scanner should have the emergency stop
        assert!(!scanner.emergency_stop.is_stopped());
        
        // Trigger emergency stop
        scanner.emergency_stop.trigger_stop(StopCommand::UserInitiated {
            reason: "Test".to_string(),
        }).unwrap();
        
        thread::sleep(Duration::from_millis(200));
        assert!(scanner.emergency_stop.is_stopped());
    }

    #[test]
    fn test_stop_command_debug() {
        let command = StopCommand::CriticalVulnerability {
            file_path: "test.rs".to_string(),
            vulnerability: "Infinite Mint".to_string(),
        };
        
        let debug_str = format!("{:?}", command);
        assert!(debug_str.contains("CriticalVulnerability"));
        assert!(debug_str.contains("test.rs"));
        assert!(debug_str.contains("Infinite Mint"));
    }

    #[test]
    fn test_default_emergency_stop() {
        let stop = EmergencyStop::default();
        assert!(!stop.is_stopped());
    }

    #[test]
    fn test_concurrent_access() {
        let stop = EmergencyStop::new().unwrap();
        let stop_clone = stop.clone();
        
        // Test concurrent access from multiple threads
        let handle = thread::spawn(move || {
            stop_clone.is_stopped()
        });
        
        let main_result = stop.is_stopped();
        let thread_result = handle.join().unwrap();
        
        assert_eq!(main_result, thread_result);
        assert!(!main_result);
    }
}
