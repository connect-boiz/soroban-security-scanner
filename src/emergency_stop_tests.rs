//! Tests for emergency stop functionality

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

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
