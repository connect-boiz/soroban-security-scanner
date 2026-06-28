//! Integration tests for scan access control (IDOR prevention — Issue #329)
//!
//! These tests verify that:
//! 1. Users can only access their own scan results
//! 2. Enumeration of sequential IDs is not possible (UUIDs are used)
//! 3. RBAC roles properly control access
//! 4. Shared scans can be accessed by authorized recipients
//! 5. Revoked shares prevent further access
//! 6. Access audit logs are properly maintained
//! 7. Admin and auditor roles can access all scans
//! 8. Non-owners cannot modify or delete others' scans

#[cfg(test)]
mod idor_tests {
use crate::scan_access_control::{
    ScanAccessAction, ScanAccessControl, ScanAccessControlConfig, ScanAccessError,
    ScanAccessRole, ScanStatus,
};
    use uuid::Uuid;

    fn setup() -> ScanAccessControl {
        ScanAccessControl::new(ScanAccessControlConfig::default())
    }

    fn create_scan(ac: &ScanAccessControl, owner: &str) -> Uuid {
        ac.register_scan(owner, "test_contract.wasm")
            .expect("Failed to create test scan")
    }

    // ── Ownership Verification Tests ────────────────────────────────────

    #[test]
    fn test_owner_access_own_scan() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        // Alice (owner, Developer role) can access
        assert!(ac
            .verify_scan_access(&scan_id, "alice", &ScanAccessRole::Developer)
            .is_ok());
    }

    #[test]
    fn test_non_owner_blocked() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        // Bob (non-owner, Developer role) cannot access
        let result = ac.verify_scan_access(&scan_id, "bob", &ScanAccessRole::Developer);
        assert!(result.is_err());

        if let Err(ScanAccessError::AccessDenied { .. }) = result {
            // Expected — IDOR prevented
        } else {
            panic!("Expected AccessDenied error for non-owner access");
        }
    }

    #[test]
    fn test_attacker_cannot_enumerate_scan_ids() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        // Attacker tries to enumerate by guessing UUIDs
        let random_id = Uuid::new_v4();
        assert_ne!(random_id, scan_id, "Random UUID should differ from real scan");

        let result = ac.verify_scan_access(&random_id, "attacker", &ScanAccessRole::Developer);
        assert!(matches!(result, Err(ScanAccessError::ScanNotFound(_))));
    }

    #[test]
    fn test_uuids_are_not_sequential() {
        let ac = setup();
        let id1 = create_scan(&ac, "user1");
        let id2 = create_scan(&ac, "user1");
        let id3 = create_scan(&ac, "user1");

        // UUIDs are random — there should be no sequential pattern
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);

        // UUID v4 has specific bit pattern (version 4, variant 1)
        let bytes = id1.as_bytes();
        assert_eq!(bytes[6] >> 4, 4, "UUID must be version 4");
        assert_eq!(bytes[8] >> 6, 2, "UUID must be variant 1 (RFC 4122)");
    }

    // ── Role-Based Access Control Tests ─────────────────────────────────

    #[test]
    fn test_admin_can_access_all_scans() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        assert!(ac
            .verify_scan_access(&scan_id, "admin_user", &ScanAccessRole::Admin)
            .is_ok());
    }

    #[test]
    fn test_auditor_can_access_all_scans() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        assert!(ac
            .verify_scan_access(&scan_id, "auditor_user", &ScanAccessRole::Auditor)
            .is_ok());
    }

    #[test]
    fn test_researcher_can_only_access_own_scans() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        // Researcher who owns it
        assert!(ac
            .verify_scan_access(&scan_id, "alice", &ScanAccessRole::SecurityResearcher)
            .is_ok());

        // Different researcher
        let result =
            ac.verify_scan_access(&scan_id, "bob", &ScanAccessRole::SecurityResearcher);
        assert!(result.is_err());
    }

    #[test]
    fn test_developer_cannot_access_others_scans() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        let result = ac.verify_scan_access(&scan_id, "bob", &ScanAccessRole::Developer);
        assert!(result.is_err());
    }

    #[test]
    fn test_user_role_most_restricted() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        let result = ac.verify_scan_access(&scan_id, "charlie", &ScanAccessRole::User);
        assert!(result.is_err());
    }

    // ── Scan Sharing Tests ──────────────────────────────────────────────

    #[test]
    fn test_shared_scan_accessible() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        // Alice shares with Bob
        ac.share_scan(&scan_id, "alice", "bob", None)
            .expect("Share should succeed");

        // Bob can now access
        assert!(ac
            .verify_scan_access(&scan_id, "bob", &ScanAccessRole::Developer)
            .is_ok());
    }

    #[test]
    fn test_revoked_share_blocks_access() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        ac.share_scan(&scan_id, "alice", "bob", None).unwrap();
        ac.revoke_share(&scan_id, "alice", "bob").unwrap();

        let result = ac.verify_scan_access(&scan_id, "bob", &ScanAccessRole::Developer);
        assert!(
            result.is_err(),
            "Revoked share must prevent access"
        );
    }

    #[test]
    fn test_cannot_share_with_self() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        let result = ac.share_scan(&scan_id, "alice", "alice", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_non_owner_cannot_share() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        let result = ac.share_scan(&scan_id, "bob", "charlie", None);
        assert!(result.is_err(), "Only owner can share");
    }

    #[test]
    fn test_shared_scan_not_accessible_by_unrelated_user() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        // Alice shares with Bob only
        ac.share_scan(&scan_id, "alice", "bob", None).unwrap();

        // Charlie (not shared with) cannot access
        let result = ac.verify_scan_access(&scan_id, "charlie", &ScanAccessRole::Developer);
        assert!(result.is_err());
    }

    // ── Modification Tests ──────────────────────────────────────────────

    #[test]
    fn test_owner_can_modify_own_scan() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        assert!(ac
            .verify_scan_modification(&scan_id, "alice", &ScanAccessRole::Developer)
            .is_ok());
    }

    #[test]
    fn test_non_owner_cannot_modify_scan() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        let result =
            ac.verify_scan_modification(&scan_id, "bob", &ScanAccessRole::Developer);
        assert!(result.is_err());
    }

    #[test]
    fn test_admin_can_modify_any_scan() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        assert!(ac
            .verify_scan_modification(&scan_id, "admin_user", &ScanAccessRole::Admin)
            .is_ok());
    }

    #[test]
    fn test_auditor_cannot_modify_scans() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        let result =
            ac.verify_scan_modification(&scan_id, "auditor_user", &ScanAccessRole::Auditor);
        assert!(
            result.is_err(),
            "Auditor should not be able to modify scans"
        );
    }

    // ── Access Audit Log Tests ──────────────────────────────────────────

    #[test]
    fn test_access_attempts_are_logged() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        ac.log_access(
            &scan_id,
            "alice",
            &ScanAccessRole::Developer,
            Some("192.168.1.1"),
            ScanAccessAction::View,
            true,
            None,
        )
        .unwrap();

        ac.log_access(
            &scan_id,
            "attacker",
            &ScanAccessRole::User,
            Some("10.0.0.99"),
            ScanAccessAction::View,
            false,
            Some("Access denied"),
        )
        .unwrap();

        let log = ac
            .get_access_log(&scan_id, "alice", &ScanAccessRole::Developer)
            .unwrap();
        assert_eq!(log.len(), 2);
        assert!(log[0].success);
        assert!(!log[1].success);
    }

    #[test]
    fn test_non_owner_cannot_view_access_logs() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        let result = ac.get_access_log(&scan_id, "bob", &ScanAccessRole::Developer);
        assert!(result.is_err());
    }

    #[test]
    fn test_admin_can_view_access_logs() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        ac.log_access(
            &scan_id,
            "alice",
            &ScanAccessRole::Developer,
            None,
            ScanAccessAction::View,
            true,
            None,
        )
        .unwrap();

        let log = ac
            .get_access_log(&scan_id, "admin_user", &ScanAccessRole::Admin)
            .unwrap();
        assert_eq!(log.len(), 1);
    }

    // ── Edge Case Tests ─────────────────────────────────────────────────

    #[test]
    fn test_nonexistent_scan_id_returns_not_found() {
        let ac = setup();
        let fake_id = Uuid::new_v4();

        let result = ac.verify_scan_access(&fake_id, "alice", &ScanAccessRole::Admin);
        assert!(matches!(result, Err(ScanAccessError::ScanNotFound(_))));
    }

    #[test]
    fn test_scan_status_update_preserves_ownership() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        ac.update_scan_status(&scan_id, ScanStatus::Completed, 5, 2)
            .unwrap();

        let scan = ac.get_scan_record(&scan_id).unwrap();
        assert_eq!(scan.owner_id, "alice");
        assert_eq!(scan.vulnerability_count, 5);
        assert_eq!(scan.invariant_violation_count, 2);
    }

    #[test]
    fn test_list_scans_only_returns_owned_scans() {
        let ac = setup();
        create_scan(&ac, "alice");
        create_scan(&ac, "alice");
        create_scan(&ac, "bob");
        create_scan(&ac, "charlie");

        assert_eq!(ac.list_user_scans("alice").unwrap().len(), 2);
        assert_eq!(ac.list_user_scans("bob").unwrap().len(), 1);
        assert_eq!(ac.list_user_scans("charlie").unwrap().len(), 1);
        assert!(ac.list_user_scans("nobody").unwrap().is_empty());
    }

    #[test]
    fn test_concurrent_access_to_different_scans() {
        use std::sync::Arc;
        use std::thread;

        let ac = Arc::new(setup());
        let scan_id_a = create_scan(&ac, "alice");
        let scan_id_b = create_scan(&ac, "bob");

        let ac_clone1 = ac.clone();
        let ac_clone2 = ac.clone();

        let handle1 = thread::spawn(move || {
            ac_clone1.verify_scan_access(&scan_id_a, "alice", &ScanAccessRole::Developer)
        });

        let handle2 = thread::spawn(move || {
            ac_clone2.verify_scan_access(&scan_id_b, "bob", &ScanAccessRole::Developer)
        });

        assert!(handle1.join().unwrap().is_ok());
        assert!(handle2.join().unwrap().is_ok());
    }

    #[test]
    fn test_public_scan_accessible_to_anyone() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        // Make scan public
        {
            let mut scans = ac.scans.write().unwrap();
            scans.get_mut(&scan_id).unwrap().is_public = true;
        }

        // Any user can access public scan
        assert!(ac
            .verify_scan_access(&scan_id, "random_user", &ScanAccessRole::User)
            .is_ok());
    }

    // ── Ownership Guard Tests ───────────────────────────────────────────

    #[test]
    fn test_ownership_guard_verifies_correctly() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        let guard = crate::scan_access_control::ScanOwnershipGuard::new(
            scan_id,
            "alice".to_string(),
            ScanAccessRole::Developer,
        );

        assert!(guard.verify(&ac).is_ok());
    }

    #[test]
    fn test_ownership_guard_blocks_non_owner() {
        let ac = setup();
        let scan_id = create_scan(&ac, "alice");

        let guard = crate::scan_access_control::ScanOwnershipGuard::new(
            scan_id,
            "attacker".to_string(),
            ScanAccessRole::Developer,
        );

        assert!(guard.verify(&ac).is_err());
    }
}
