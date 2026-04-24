//! Tests for the emergency stop mechanism and scan controller

use std::time::Duration;
use tokio::time::sleep;

use soroban_security_scanner_core::scan_controller::{ScanController, ScanCommand, ScanStatus};

#[tokio::test]
async fn test_scan_registration_and_status() {
    let controller = ScanController::new();
    let scan_id = "test-scan-1".to_string();
    
    // Register a scan
    let mut receiver = controller
        .register_scan(scan_id.clone(), true, 3)
        .await
        .expect("Failed to register scan");
    
    // Check initial status
    let status = controller
        .get_scan_status(&scan_id)
        .await
        .expect("Failed to get scan status");
    
    assert!(status.is_some());
    let control = status.unwrap();
    assert_eq!(control.scan_id, scan_id);
    assert!(matches!(control.status, ScanStatus::Running));
    assert_eq!(control.critical_count, 0);
    assert!(control.auto_stop_enabled);
    assert_eq!(control.auto_stop_threshold, 3);
}

#[tokio::test]
async fn test_manual_stop_command() {
    let controller = ScanController::new();
    let scan_id = "test-scan-2".to_string();
    
    // Register a scan
    let mut receiver = controller
        .register_scan(scan_id.clone(), false, 0)
        .await
        .expect("Failed to register scan");
    
    // Issue stop command
    controller
        .issue_command(&scan_id, ScanCommand::Stop)
        .await
        .expect("Failed to issue stop command");
    
    // Check status
    let status = controller
        .get_scan_status(&scan_id)
        .await
        .expect("Failed to get scan status");
    
    assert!(status.is_some());
    let control = status.unwrap();
    assert!(matches!(control.status, ScanStatus::Stopped));
    assert_eq!(control.reason, Some("Manual stop requested".to_string()));
}

#[tokio::test]
async fn test_pause_and_resume_commands() {
    let controller = ScanController::new();
    let scan_id = "test-scan-3".to_string();
    
    // Register a scan
    let mut receiver = controller
        .register_scan(scan_id.clone(), false, 0)
        .await
        .expect("Failed to register scan");
    
    // Issue pause command
    controller
        .issue_command(&scan_id, ScanCommand::Pause)
        .await
        .expect("Failed to issue pause command");
    
    // Check status
    let status = controller
        .get_scan_status(&scan_id)
        .await
        .expect("Failed to get scan status");
    
    assert!(status.is_some());
    let control = status.unwrap();
    assert!(matches!(control.status, ScanStatus::Paused));
    assert_eq!(control.reason, Some("Manual pause requested".to_string()));
    
    // Issue resume command
    controller
        .issue_command(&scan_id, ScanCommand::Resume)
        .await
        .expect("Failed to issue resume command");
    
    // Check status
    let status = controller
        .get_scan_status(&scan_id)
        .await
        .expect("Failed to get scan status");
    
    assert!(status.is_some());
    let control = status.unwrap();
    assert!(matches!(control.status, ScanStatus::Running));
    assert_eq!(control.reason, Some("Scan resumed".to_string()));
}

#[tokio::test]
async fn test_auto_stop_on_critical_threshold() {
    let controller = ScanController::new();
    let scan_id = "test-scan-4".to_string();
    
    // Register a scan with auto-stop enabled
    let mut receiver = controller
        .register_scan(scan_id.clone(), true, 2) // Auto-stop after 2 critical
        .await
        .expect("Failed to register scan");
    
    // Increment critical count (first time)
    let should_stop = controller
        .increment_critical_count(&scan_id)
        .await
        .expect("Failed to increment critical count");
    
    assert!(!should_stop); // Should not stop yet
    
    let status = controller
        .get_scan_status(&scan_id)
        .await
        .expect("Failed to get scan status");
    let control = status.unwrap();
    assert_eq!(control.critical_count, 1);
    assert!(matches!(control.status, ScanStatus::Running));
    
    // Increment critical count (second time - should trigger auto-stop)
    let should_stop = controller
        .increment_critical_count(&scan_id)
        .await
        .expect("Failed to increment critical count");
    
    assert!(should_stop); // Should stop now
    
    let status = controller
        .get_scan_status(&scan_id)
        .await
        .expect("Failed to get scan status");
    let control = status.unwrap();
    assert_eq!(control.critical_count, 2);
    assert!(matches!(control.status, ScanStatus::Stopped));
    assert!(control.reason.is_some());
    assert!(control.reason.unwrap().contains("Auto-stop triggered"));
}

#[tokio::test]
async fn test_get_active_scans() {
    let controller = ScanController::new();
    let scan_id1 = "test-scan-5a".to_string();
    let scan_id2 = "test-scan-5b".to_string();
    let scan_id3 = "test-scan-5c".to_string();
    
    // Register three scans
    controller
        .register_scan(scan_id1.clone(), false, 0)
        .await
        .expect("Failed to register scan 1");
    
    controller
        .register_scan(scan_id2.clone(), false, 0)
        .await
        .expect("Failed to register scan 2");
    
    controller
        .register_scan(scan_id3.clone(), false, 0)
        .await
        .expect("Failed to register scan 3");
    
    // Stop one scan
    controller
        .issue_command(&scan_id2, ScanCommand::Stop)
        .await
        .expect("Failed to issue stop command");
    
    // Get active scans
    let active_scans = controller
        .get_active_scans()
        .await
        .expect("Failed to get active scans");
    
    assert_eq!(active_scans.len(), 2); // Only 2 should be active
    
    let active_ids: Vec<String> = active_scans.iter().map(|s| s.scan_id.clone()).collect();
    assert!(active_ids.contains(&scan_id1));
    assert!(active_ids.contains(&scan_id3));
    assert!(!active_ids.contains(&scan_id2));
}

#[tokio::test]
async fn test_mark_completed_and_failed() {
    let controller = ScanController::new();
    let scan_id1 = "test-scan-6a".to_string();
    let scan_id2 = "test-scan-6b".to_string();
    
    // Register two scans
    controller
        .register_scan(scan_id1.clone(), false, 0)
        .await
        .expect("Failed to register scan 1");
    
    controller
        .register_scan(scan_id2.clone(), false, 0)
        .await
        .expect("Failed to register scan 2");
    
    // Mark one as completed
    controller
        .mark_completed(&scan_id1)
        .await
        .expect("Failed to mark scan as completed");
    
    // Mark one as failed
    controller
        .mark_failed(&scan_id2, "Test error")
        .await
        .expect("Failed to mark scan as failed");
    
    // Check statuses
    let status1 = controller
        .get_scan_status(&scan_id1)
        .await
        .expect("Failed to get scan status");
    assert!(matches!(status1.unwrap().status, ScanStatus::Completed));
    
    let status2 = controller
        .get_scan_status(&scan_id2)
        .await
        .expect("Failed to get scan status");
    assert!(matches!(status2.unwrap().status, ScanStatus::Failed));
    assert_eq!(status2.unwrap().reason, Some("Test error".to_string()));
}

#[tokio::test]
async fn test_cleanup_old_scans() {
    let controller = ScanController::new();
    let scan_id1 = "test-scan-7a".to_string();
    let scan_id2 = "test-scan-7b".to_string();
    
    // Register two scans
    controller
        .register_scan(scan_id1.clone(), false, 0)
        .await
        .expect("Failed to register scan 1");
    
    controller
        .register_scan(scan_id2.clone(), false, 0)
        .await
        .expect("Failed to register scan 2");
    
    // Mark one as completed (makes it eligible for cleanup)
    controller
        .mark_completed(&scan_id1)
        .await
        .expect("Failed to mark scan as completed");
    
    // Wait a bit to ensure time difference
    sleep(Duration::from_millis(10)).await;
    
    // Cleanup with very short duration (should remove the completed scan)
    let cleaned_count = controller
        .cleanup_old_scans(chrono::Duration::milliseconds(1))
        .await
        .expect("Failed to cleanup old scans");
    
    assert_eq!(cleaned_count, 1);
    
    // Check that the completed scan is gone but the running one remains
    let status1 = controller
        .get_scan_status(&scan_id1)
        .await
        .expect("Failed to get scan status");
    assert!(status1.is_none()); // Should be cleaned up
    
    let status2 = controller
        .get_scan_status(&scan_id2)
        .await
        .expect("Failed to get scan status");
    assert!(status2.is_some()); // Should still exist
}

#[tokio::test]
async fn test_command_broadcast() {
    let controller = ScanController::new();
    let scan_id = "test-scan-8".to_string();
    
    // Register a scan and get receiver
    let mut receiver = controller
        .register_scan(scan_id.clone(), false, 0)
        .await
        .expect("Failed to register scan");
    
    // Issue a command
    controller
        .issue_command(&scan_id, ScanCommand::Pause)
        .await
        .expect("Failed to issue pause command");
    
    // Check that the command was received
    tokio::time::timeout(Duration::from_millis(100), async {
        loop {
            match receiver.try_recv() {
                Ok((target_id, command)) => {
                    if target_id == scan_id {
                        assert!(matches!(command, ScanCommand::Pause));
                        break;
                    }
                }
                Err(_) => {
                    // No message yet, continue waiting
                    sleep(Duration::from_millis(1)).await;
                }
            }
        }
    })
    .await
    .expect("Timeout waiting for command");
}
