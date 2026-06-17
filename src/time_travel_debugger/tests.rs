#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // Access Control Tests
    // ============================================================

    #[tokio::test]
    async fn test_admin_has_all_permissions() {
        let controller = AccessController::new();
        let admin = UserContext {
            user_id: "admin_1".to_string(),
            roles: {
                let mut r = std::collections::HashSet::new();
                r.insert(UserRole::Admin);
                r
            },
            tier: UserTier::Enterprise,
            contract_ownerships: std::collections::HashSet::new(),
            issued_at: std::time::Instant::now(),
        };

        let result = controller.check_permission(&admin, &Permission::ForkAtLedger, None, None).await.unwrap();
        assert!(result.allowed, "Admin should be able to fork at ledger");

        let result = controller.check_permission(&admin, &Permission::GetContractState, None, None).await.unwrap();
        assert!(result.allowed, "Admin should be able to get contract state");

        let result = controller.check_permission(&admin, &Permission::ViewAuditLogs, None, None).await.unwrap();
        assert!(result.allowed, "Admin should be able to view audit logs");

        let result = controller.check_permission(&admin, &Permission::ManageAccessControl, None, None).await.unwrap();
        assert!(result.allowed, "Admin should be able to manage access control");
    }

    #[tokio::test]
    async fn test_free_user_cannot_simulate_upgrade() {
        let controller = AccessController::new();
        let free_user = UserContext {
            user_id: "user_free".to_string(),
            roles: {
                let mut r = std::collections::HashSet::new();
                r.insert(UserRole::User);
                r
            },
            tier: UserTier::Free,
            contract_ownerships: std::collections::HashSet::new(),
            issued_at: std::time::Instant::now(),
        };

        let result = controller.check_permission(&free_user, &Permission::SimulateUpgrade, None, None).await.unwrap();
        assert!(!result.allowed, "Free user should not be able to simulate upgrade");
    }

    #[tokio::test]
    async fn test_contract_owner_can_access_owned_contract() {
        let controller = AccessController::new();
        let owner = UserContext {
            user_id: "owner_1".to_string(),
            roles: {
                let mut r = std::collections::HashSet::new();
                r.insert(UserRole::ContractOwner);
                r
            },
            tier: UserTier::Pro,
            contract_ownerships: {
                let mut c = std::collections::HashSet::new();
                c.insert("contract_123".to_string());
                c
            },
            issued_at: std::time::Instant::now(),
        };

        controller.register_contract_owner("contract_123", "owner_1").await.unwrap();
        let sensitive = controller.check_sensitive_contract_access(&owner, "contract_123", 100000).await.unwrap();
        assert!(sensitive, "Owner should have sensitive access to own contract");
    }

    #[tokio::test]
    async fn test_contract_owner_cannot_access_unowned_contract() {
        let controller = AccessController::new();
        let owner = UserContext {
            user_id: "owner_1".to_string(),
            roles: {
                let mut r = std::collections::HashSet::new();
                r.insert(UserRole::ContractOwner);
                r
            },
            tier: UserTier::Pro,
            contract_ownerships: {
                let mut c = std::collections::HashSet::new();
                c.insert("contract_456".to_string());
                c
            },
            issued_at: std::time::Instant::now(),
        };

        let result = controller.check_permission(
            &owner, &Permission::InjectState, Some("contract_123"), None
        ).await.unwrap();
        assert!(!result.allowed, "Owner should not be able to inject state for unowned contract");
    }

    #[tokio::test]
    async fn test_auditor_can_view_audit_logs() {
        let controller = AccessController::new();
        let auditor = UserContext {
            user_id: "auditor_1".to_string(),
            roles: {
                let mut r = std::collections::HashSet::new();
                r.insert(UserRole::Auditor);
                r
            },
            tier: UserTier::Enterprise,
            contract_ownerships: std::collections::HashSet::new(),
            issued_at: std::time::Instant::now(),
        };

        let result = controller.check_permission(&auditor, &Permission::ViewAuditLogs, None, None).await.unwrap();
        assert!(result.allowed, "Auditor should be able to view audit logs");

        let result = controller.check_permission(&auditor, &Permission::ExportData, None, None).await.unwrap();
        assert!(result.allowed, "Auditor should be able to export data");
    }

    #[tokio::test]
    async fn test_free_user_cannot_access_old_ledgers() {
        let controller = AccessController::new();
        let free_user = UserContext {
            user_id: "user_free".to_string(),
            roles: {
                let mut r = std::collections::HashSet::new();
                r.insert(UserRole::User);
                r
            },
            tier: UserTier::Free,
            contract_ownerships: std::collections::HashSet::new(),
            issued_at: std::time::Instant::now(),
        };

        let result = controller.check_permission(
            &free_user, &Permission::GetContractState, None, Some(50)
        ).await.unwrap();
        assert!(!result.allowed, "Free user should not access ledger before 100000");
    }

    #[tokio::test]
    async fn test_approval_workflow() {
        let controller = AccessController::new();

        let request = controller.request_approval("user_1", "sensitive_op", Some("contract_123"), Some(100000), "Need access for audit").await.unwrap();
        assert_eq!(request.status, ApprovalStatus::Pending);

        let status = controller.check_approval_status(&request.id).await;
        assert_eq!(status, Some(ApprovalStatus::Pending));

        controller.resolve_approval(&request.id, "admin_1", true).await.unwrap();
        let status = controller.check_approval_status(&request.id).await;
        assert_eq!(status, Some(ApprovalStatus::Approved));
    }

    #[tokio::test]
    async fn test_approval_rejection() {
        let controller = AccessController::new();

        let request = controller.request_approval("user_1", "sensitive_op", None, None, "Request").await.unwrap();
        controller.resolve_approval(&request.id, "admin_1", false).await.unwrap();

        let status = controller.check_approval_status(&request.id).await;
        assert_eq!(status, Some(ApprovalStatus::Rejected));
    }

    #[tokio::test]
    async fn test_enterprise_user_all_ledgers() {
        let controller = AccessController::new();
        let enterprise = UserContext {
            user_id: "enterprise_1".to_string(),
            roles: {
                let mut r = std::collections::HashSet::new();
                r.insert(UserRole::Admin);
                r
            },
            tier: UserTier::Enterprise,
            contract_ownerships: std::collections::HashSet::new(),
            issued_at: std::time::Instant::now(),
        };

        let result = controller.check_permission(
            &enterprise, &Permission::GetContractState, None, Some(1)
        ).await.unwrap();
        assert!(result.allowed, "Enterprise users should access any ledger");
    }

    #[tokio::test]
    async fn test_role_permission_customization() {
        let controller = AccessController::new();
        controller.add_role_permission(UserRole::User, Permission::ViewAuditLogs).await.unwrap();

        let user = UserContext {
            user_id: "user_1".to_string(),
            roles: {
                let mut r = std::collections::HashSet::new();
                r.insert(UserRole::User);
                r
            },
            tier: UserTier::Free,
            contract_ownerships: std::collections::HashSet::new(),
            issued_at: std::time::Instant::now(),
        };

        let result = controller.check_permission(&user, &Permission::ViewAuditLogs, None, None).await.unwrap();
        assert!(result.allowed, "User should have custom ViewAuditLogs permission");
    }

    // ============================================================
    // Audit Log Tests
    // ============================================================

    #[tokio::test]
    async fn test_audit_log_creation() {
        let logger = AuditLogger::new(100);

        let entry = logger.log("user_1", vec!["User".to_string()], AuditOperation::ForkAtLedger, "ledger:100000", Some(100000), None, true, "Forked successfully").await;

        assert_eq!(entry.user_id, "user_1");
        assert!(matches!(entry.operation, AuditOperation::ForkAtLedger));
        assert!(entry.success);
    }

    #[tokio::test]
    async fn test_audit_log_query() {
        let logger = AuditLogger::new(100);

        logger.log("user_1", vec![], AuditOperation::ForkAtLedger, "ledger:100000", Some(100000), None, true, "").await;
        logger.log("user_2", vec![], AuditOperation::GetContractState, "contract:abc", None, Some("contract_abc"), true, "").await;
        logger.log("user_1", vec![], AuditOperation::PermissionDenied, "upgrade", Some(100001), None, false, "Access denied").await;

        let query = AuditLogQuery {
            user_id: Some("user_1".to_string()),
            operation: None,
            contract_id: None,
            ledger_sequence: None,
            success: None,
            limit: 10,
        };

        let results = logger.query(&query).await;
        assert_eq!(results.len(), 2, "Should find 2 logs for user_1");
    }

    #[tokio::test]
    async fn test_audit_log_summary() {
        let logger = AuditLogger::new(100);

        logger.log("user_1", vec![], AuditOperation::ForkAtLedger, "", None, None, true, "").await;
        logger.log("user_1", vec![], AuditOperation::PermissionDenied, "", None, None, false, "Denied").await;
        logger.log("user_2", vec![], AuditOperation::ForkAtLedger, "", None, None, true, "").await;

        let summary = logger.get_summary().await;
        assert_eq!(summary.total_entries, 3);
        assert_eq!(summary.unique_users, 2);
        assert!(summary.success_rate > 0.5);
        assert_eq!(summary.recent_denials, 1);
    }

    #[tokio::test]
    async fn test_audit_log_max_entries() {
        let logger = AuditLogger::new(5);

        for i in 0..10u32 {
            logger.log(&format!("user_{}", i), vec![], AuditOperation::ForkAtLedger, "", None, None, true, "").await;
        }

        assert_eq!(logger.entry_count().await, 5);
    }

    #[tokio::test]
    async fn test_audit_log_clear() {
        let logger = AuditLogger::new(100);
        logger.log("user_1", vec![], AuditOperation::ForkAtLedger, "", None, None, true, "").await;
        logger.clear().await;
        assert_eq!(logger.entry_count().await, 0);
    }

    #[tokio::test]
    async fn test_audit_log_permission_denied() {
        let logger = AuditLogger::new(100);
        let entry = logger.log_permission_denied("user_1", vec!["User".to_string()], "contract:abc", "No permission").await;
        assert!(matches!(entry.operation, AuditOperation::PermissionDenied));
        assert!(!entry.success);
    }

    // ============================================================
    // Rate Limiter Tests
    // ============================================================

    #[tokio::test]
    async fn test_rate_limiter_allows_request() {
        let config = RateLimitConfig {
            max_requests_per_window: 100,
            window_duration_seconds: 60,
            max_concurrent_operations: 10,
            burst_size: 50,
        };
        let limiter = RateLimiter::new(config);
        let status = limiter.check_rate_limit("user_1").await;
        assert!(status.allowed);
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_excess() {
        let config = RateLimitConfig {
            max_requests_per_window: 3,
            window_duration_seconds: 60,
            max_concurrent_operations: 10,
            burst_size: 3,
        };
        let limiter = RateLimiter::new(config);

        for _ in 0..3 {
            let status = limiter.check_rate_limit("user_1").await;
            assert!(status.allowed);
        }

        let status = limiter.check_rate_limit("user_1").await;
        assert!(!status.allowed, "Should block 4th request");
    }

    #[tokio::test]
    async fn test_rate_limiter_burst_refill() {
        let config = RateLimitConfig {
            max_requests_per_window: 10,
            window_duration_seconds: 60,
            max_concurrent_operations: 10,
            burst_size: 2,
        };
        let limiter = RateLimiter::new(config);

        assert!(limiter.check_rate_limit("user_1").await.allowed);
        assert!(limiter.check_rate_limit("user_1").await.allowed);
        assert!(!limiter.check_rate_limit("user_1").await.allowed, "Should block after burst exhausted");
    }

    #[tokio::test]
    async fn test_rate_limiter_release_concurrent() {
        let config = RateLimitConfig {
            max_requests_per_window: 100,
            window_duration_seconds: 60,
            max_concurrent_operations: 2,
            burst_size: 10,
        };
        let limiter = RateLimiter::new(config);

        assert!(limiter.check_rate_limit("user_1").await.allowed);
        assert!(limiter.check_rate_limit("user_1").await.allowed);
        assert!(!limiter.check_rate_limit("user_1").await.allowed, "Should block 3rd concurrent");

        limiter.release_concurrent("user_1").await;
        assert!(limiter.check_rate_limit("user_1").await.allowed, "Should allow after release");
    }

    #[tokio::test]
    async fn test_rate_limiter_user_independence() {
        let config = RateLimitConfig {
            max_requests_per_window: 2,
            window_duration_seconds: 60,
            max_concurrent_operations: 10,
            burst_size: 2,
        };
        let limiter = RateLimiter::new(config);

        assert!(limiter.check_rate_limit("user_1").await.allowed);
        assert!(limiter.check_rate_limit("user_1").await.allowed);
        assert!(!limiter.check_rate_limit("user_1").await.allowed);

        assert!(limiter.check_rate_limit("user_2").await.allowed, "User 2 should not be affected by user 1's limit");
    }

    #[tokio::test]
    async fn test_rate_limiter_stale_cleanup() {
        let config = RateLimitConfig {
            max_requests_per_window: 10,
            window_duration_seconds: 1,
            max_concurrent_operations: 10,
            burst_size: 10,
        };
        let limiter = RateLimiter::new(config);

        limiter.check_rate_limit("user_1").await;
        limiter.check_rate_limit("user_2").await;

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        limiter.cleanup_stale_entries().await;

        let status = limiter.get_user_status("user_1").await;
        assert!(status.is_some());
    }

    // ============================================================
    // Data Retention Tests
    // ============================================================

    #[tokio::test]
    async fn test_retention_policy_default() {
        let policy = RetentionPolicy::default();
        assert_eq!(policy.contract_state_retention_days, 30);
        assert_eq!(policy.audit_log_retention_days, 90);
        assert_eq!(policy.max_stored_states_per_contract, 100);
    }

    #[tokio::test]
    async fn test_strict_retention_policy() {
        let policy = RetentionPolicy::strict();
        assert_eq!(policy.contract_state_retention_days, 7);
        assert_eq!(policy.ledger_snapshot_retention_days, 3);
    }

    #[tokio::test]
    async fn test_data_retention_recording() {
        let manager = DataRetentionManager::new(RetentionPolicy::default());
        manager.record_storage("contract_123", 100000, StoredDataType::ContractState, 1024, false).await.unwrap();
        manager.record_storage("contract_123", 100001, StoredDataType::LedgerSnapshot, 512, false).await.unwrap();

        let usage = manager.get_storage_usage().await;
        assert_eq!(usage.contract_state_count, 1);
        assert_eq!(usage.ledger_snapshot_count, 1);
    }

    #[tokio::test]
    async fn test_storage_exceeds_max_per_contract() {
        let policy = RetentionPolicy {
            max_stored_states_per_contract: 2,
            ..Default::default()
        };
        let manager = DataRetentionManager::new(policy);

        manager.record_storage("contract_123", 1, StoredDataType::ContractState, 10, false).await.unwrap();
        manager.record_storage("contract_123", 2, StoredDataType::ContractState, 10, false).await.unwrap();
        let result = manager.record_storage("contract_123", 3, StoredDataType::ContractState, 10, false).await;
        assert!(result.is_err(), "Should reject 3rd state for same contract");
    }

    #[tokio::test]
    async fn test_retention_cleanup() {
        let policy = RetentionPolicy {
            contract_state_retention_days: 0,
            cleanup_interval_hours: 0,
            auto_cleanup_enabled: true,
            ..Default::default()
        };
        let manager = DataRetentionManager::new(policy);

        manager.record_storage("contract_123", 100000, StoredDataType::ContractState, 100, false).await.unwrap();
        manager.record_storage("contract_456", 100001, StoredDataType::LedgerSnapshot, 50, false).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let report = manager.perform_cleanup().await;
        assert!(report.entries_removed >= 2);
    }

    // ============================================================
    // Encryption Tests
    // ============================================================

    #[tokio::test]
    async fn test_encryption_roundtrip() {
        let config = EncryptionConfig::default();
        let encryptor = DataEncryptor::new(config);
        encryptor.initialize(b"this_is_a_32_byte_test_key!!!!!!").await.unwrap();

        let plaintext = b"Hello, Time Travel Debugger!";
        let encrypted = encryptor.encrypt(plaintext).await.unwrap();
        let decrypted = encryptor.decrypt(&encrypted).await.unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[tokio::test]
    async fn test_encryption_corruption_detection() {
        let encryptor = DataEncryptor::new(EncryptionConfig::default());
        encryptor.initialize(b"this_is_a_32_byte_test_key!!!!!!").await.unwrap();

        let mut encrypted = encryptor.encrypt(b"test").await.unwrap();
        encrypted.ciphertext[0] ^= 0xFF;

        let result = encryptor.decrypt(&encrypted).await;
        assert!(result.is_err(), "Should detect corrupted data");
    }

    #[tokio::test]
    async fn test_encryption_key_rotation() {
        let encryptor = DataEncryptor::new(EncryptionConfig::default());
        encryptor.initialize(b"first_32_byte_key_for_testing!!!!!").await.unwrap();

        let encrypted = encryptor.encrypt(b"sensitive").await.unwrap();
        encryptor.rotate_key(b"second_32_byte_key_for_testing!!!!").await.unwrap();

        let result = encryptor.decrypt(&encrypted).await;
        assert!(result.is_err(), "Should fail to decrypt with new key");
    }

    #[tokio::test]
    async fn test_encryption_disabled_state() {
        let config = EncryptionConfig {
            enabled: false,
            ..Default::default()
        };
        let encryptor = DataEncryptor::new(config);
        encryptor.initialize(b"this_is_a_32_byte_test_key!!!!!!").await.unwrap();

        let result = encryptor.encrypt(b"test").await;
        assert!(result.is_err(), "Should fail when encryption is disabled");
    }

    // ============================================================
    // Quota Management Tests
    // ============================================================

    #[tokio::test]
    async fn test_free_user_quota() {
        let manager = QuotaManager::new();
        let status = manager.check_quota("user_free", &UserTier::Free, &QuotaOperation::Fork).await;
        assert_eq!(status.limit, 10);
        assert!(status.allowed);
    }

    #[tokio::test]
    async fn test_pro_user_quota() {
        let manager = QuotaManager::new();
        let status = manager.check_quota("user_pro", &UserTier::Pro, &QuotaOperation::StateRetrieval).await;
        assert_eq!(status.limit, 1000);
    }

    #[tokio::test]
    async fn test_enterprise_user_quota() {
        let manager = QuotaManager::new();
        let status = manager.check_quota("user_ent", &UserTier::Enterprise, &QuotaOperation::UpgradeSimulation).await;
        assert_eq!(status.limit, 5000);
    }

    #[tokio::test]
    async fn test_quota_exceeded() {
        let manager = QuotaManager::new();
        let tier = UserTier::Free;

        for _ in 0..10 {
            let status = manager.record_operation("user_test", &tier, &QuotaOperation::Fork).await;
            assert!(status.allowed, "Should allow up to limit");
        }

        let status = manager.record_operation("user_test", &tier, &QuotaOperation::Fork).await;
        assert!(!status.allowed, "Should exceed fork quota for free tier");
    }

    #[tokio::test]
    async fn test_concurrent_fork_quota() {
        let manager = QuotaManager::new();
        let tier = UserTier::Free;

        assert!(manager.check_concurrent_forks("user_test", &tier).await);
        manager.track_active_fork("user_test", true).await;
        manager.track_active_fork("user_test", true).await;
        assert!(!manager.check_concurrent_forks("user_test", &tier).await, "Free tier max 2 concurrent forks");
    }

    #[tokio::test]
    async fn test_storage_quota() {
        let manager = QuotaManager::new();

        assert!(manager.check_storage_quota("user_free", &UserTier::Free, 0).await);
        assert!(!manager.check_storage_quota("user_free", &UserTier::Free, 200 * 1024 * 1024).await, "Free tier 100MB max");
    }

    // ============================================================
    // Monitoring Tests
    // ============================================================

    #[tokio::test]
    async fn test_monitoring_rapid_fire_detection() {
        let config = MonitoringConfig {
            rapid_fire_threshold_ms: 1000,
            ..Default::default()
        };
        let engine = MonitoringEngine::new(config);

        for _ in 0..10 {
            let pattern = engine.record_access("user_1", "fork", Some("contract_123"), Some(100000), None).await;
            if let Some(p) = pattern {
                assert_eq!(p.pattern_type, SuspiciousPatternType::RapidFireAccess);
                return;
            }
        }
        panic!("Should have detected rapid fire access");
    }

    #[tokio::test]
    async fn test_monitoring_sequential_scan_detection() {
        let config = MonitoringConfig {
            sequential_scan_threshold: 5,
            ..Default::default()
        };
        let engine = MonitoringEngine::new(config);

        for i in 0..10u32 {
            let pattern = engine.record_access("user_scan", "get_state", Some("contract_123"), Some(100000 - i), None).await;
            if let Some(p) = pattern {
                assert_eq!(p.pattern_type, SuspiciousPatternType::SequentialLedgerScan);
                return;
            }
        }
        panic!("Should have detected sequential ledger scan");
    }

    #[tokio::test]
    async fn test_monitoring_failed_attempts() {
        let config = MonitoringConfig {
            max_failed_attempts: 3,
            failed_attempts_window_seconds: 60,
            ..Default::default()
        };
        let engine = MonitoringEngine::new(config);

        for _ in 0..4 {
            let pattern = engine.record_failed_attempt("user_bad").await;
            if let Some(p) = pattern {
                assert_eq!(p.pattern_type, SuspiciousPatternType::ExcessiveFailedAttempts);
                return;
            }
        }
        panic!("Should have detected excessive failed attempts");
    }

    #[tokio::test]
    async fn test_monitoring_user_pattern_summary() {
        let engine = MonitoringEngine::new(MonitoringConfig::default());

        engine.record_access("user_1", "fork", Some("contract_123"), Some(100000), None).await;
        engine.record_access("user_1", "fork", Some("contract_456"), Some(100001), None).await;

        let summary = engine.get_user_pattern_summary("user_1").await;
        assert!(summary.is_some());
        assert_eq!(summary.unwrap().total_operations, 2);
    }

    #[tokio::test]
    async fn test_monitoring_alert_threshold() {
        let config = MonitoringConfig {
            alert_threshold: 3,
            max_failed_attempts: 1,
            failed_attempts_window_seconds: 60,
            rapid_fire_threshold_ms: 10000,
            ..Default::default()
        };
        let engine = MonitoringEngine::new(config);

        for _ in 0..4 {
            engine.record_failed_attempt("user_1").await;
        }

        assert!(engine.should_alert().await, "Should trigger alert after threshold");
    }

    #[tokio::test]
    async fn test_monitoring_suspicious_event_logging() {
        let engine = MonitoringEngine::new(MonitoringConfig::default());

        let event = SuspiciousPattern {
            pattern_type: SuspiciousPatternType::GeographicAnomaly,
            user_id: "user_1".to_string(),
            severity: SuspiciousSeverity::High,
            description: "Access from unusual location".to_string(),
            detected_at: std::time::Instant::now(),
            details: std::collections::HashMap::new(),
        };
        engine.log_suspicious_event(event).await;

        let events = engine.get_suspicious_events(Some(SuspiciousSeverity::Medium), 10).await;
        assert_eq!(events.len(), 1);
    }

    // ============================================================
    // Integration Tests
    // ============================================================

    #[tokio::test]
    async fn test_full_security_pipeline() {
        let config = TimeTravelConfig {
            access_control_enabled: true,
            audit_logging_enabled: true,
            rate_limiting_enabled: true,
            quotas_enabled: true,
            monitoring_enabled: true,
            encryption_enabled: false,
            ..Default::default()
        };

        let debugger = TimeTravelDebugger::new(config).await.unwrap();

        let user = UserContext {
            user_id: "test_admin".to_string(),
            roles: {
                let mut r = std::collections::HashSet::new();
                r.insert(UserRole::Admin);
                r
            },
            tier: UserTier::Enterprise,
            contract_ownerships: {
                let mut c = std::collections::HashSet::new();
                c.insert("contract_test".to_string());
                c
            },
            issued_at: std::time::Instant::now(),
        };

        assert!(debugger.authorize_operation(&user, &Permission::ForkAtLedger, None, None).await.is_ok());
        assert!(debugger.authorize_operation(&user, &Permission::GetContractState, Some("contract_test"), None).await.is_ok());

        let rate_status = debugger.check_rate_limit("test_admin").await.unwrap();
        assert!(rate_status.allowed);

        let quota_status = debugger.check_quota("test_admin", &UserTier::Enterprise, &QuotaOperation::Fork).await.unwrap();
        assert!(quota_status.allowed);

        debugger.audit_log("test_admin", vec!["Admin".to_string()], AuditOperation::ForkAtLedger, "ledger:100000", Some(100000), None, true, "Integration test").await;

        let summary = debugger.get_audit_log_summary().await;
        assert!(summary.total_entries >= 1);

        let suspicious = debugger.monitor_access("test_admin", "fork", Some("contract_test"), Some(100000)).await;
        assert!(suspicious.is_none(), "Admin access should not trigger suspicious pattern");

        assert!(!debugger.should_alert().await);
    }

    #[tokio::test]
    async fn test_unauthorized_user_blocked() {
        let debugger = TimeTravelDebugger::new(TimeTravelConfig::default()).await.unwrap();

        let user = UserContext {
            user_id: "malicious_user".to_string(),
            roles: {
                let mut r = std::collections::HashSet::new();
                r.insert(UserRole::User);
                r
            },
            tier: UserTier::Free,
            contract_ownerships: std::collections::HashSet::new(),
            issued_at: std::time::Instant::now(),
        };

        let result = debugger.authorize_operation(
            &user, &Permission::SimulateUpgrade, Some("contract_other"), None
        ).await;
        assert!(result.is_err(), "Unauthorized user should be blocked from upgrade simulation");
    }

    #[tokio::test]
    async fn test_rate_limit_blocks_excessive_requests() {
        let config = TimeTravelConfig {
            rate_limiting_enabled: true,
            rate_limit_config: RateLimitConfig {
                max_requests_per_window: 2,
                window_duration_seconds: 60,
                max_concurrent_operations: 10,
                burst_size: 2,
            },
            ..Default::default()
        };
        let debugger = TimeTravelDebugger::new(config).await.unwrap();

        assert!(debugger.check_rate_limit("heavy_user").await.unwrap().allowed);
        assert!(debugger.check_rate_limit("heavy_user").await.unwrap().allowed);
        assert!(!debugger.check_rate_limit("heavy_user").await.unwrap().allowed);
    }

    #[tokio::test]
    async fn test_quota_tracking_across_operations() {
        let config = TimeTravelConfig {
            quotas_enabled: true,
            ..Default::default()
        };
        let debugger = TimeTravelDebugger::new(config).await.unwrap();

        let status = debugger.check_quota("quota_user", &UserTier::Free, &QuotaOperation::StateInjection).await.unwrap();
        assert!(status.allowed);
        assert_eq!(status.remaining, 20);

        debugger.record_quota_usage("quota_user", &UserTier::Free, &QuotaOperation::StateInjection).await;

        let status = debugger.check_quota("quota_user", &UserTier::Free, &QuotaOperation::StateInjection).await.unwrap();
        assert_eq!(status.remaining, 19);
    }

    #[tokio::test]
    async fn test_approval_workflow_integration() {
        let debugger = TimeTravelDebugger::new(TimeTravelConfig::default()).await.unwrap();

        let request = debugger.request_approval(
            "user_1", "sensitive_operation", Some("contract_high_value"), Some(100000), "Need access for security audit"
        ).await.unwrap();
        assert_eq!(request.status, ApprovalStatus::Pending);

        let pending = debugger.get_pending_approvals().await;
        assert!(!pending.is_empty());

        debugger.resolve_approval(&request.id, "admin_approver", true).await.unwrap();
        let status = debugger.check_approval_status(&request.id).await;
        assert_eq!(status, Some(ApprovalStatus::Approved));
    }

    #[tokio::test]
    async fn test_encryption_key_management_integration() {
        let config = TimeTravelConfig {
            encryption_enabled: true,
            ..Default::default()
        };
        let debugger = TimeTravelDebugger::new(config).await.unwrap();

        debugger.initialize_encryption(b"this_is_a_32_byte_test_key!!!!!!").await.unwrap();

        let plaintext = b"sensitive_contract_state_data";
        let encrypted = debugger.encrypt_state_data(plaintext).await.unwrap();
        let decrypted = debugger.decrypt_state_data(&encrypted).await.unwrap();
        assert_eq!(plaintext.to_vec(), decrypted);

        debugger.rotate_encryption_key(b"new_32_byte_encryption_key_for_test!").await.unwrap();
        let result = debugger.decrypt_state_data(&encrypted).await;
        assert!(result.is_err(), "Old encrypted data should be undecryptable after rotation");
    }

    #[tokio::test]
    async fn test_sensitive_contract_access_control() {
        let controller = AccessController::new();
        controller.register_contract_owner("contract_sensitive", "owner_1").await.unwrap();

        let owner = UserContext {
            user_id: "owner_1".to_string(),
            roles: {
                let mut r = std::collections::HashSet::new();
                r.insert(UserRole::ContractOwner);
                r
            },
            tier: UserTier::Pro,
            contract_ownerships: {
                let mut c = std::collections::HashSet::new();
                c.insert("contract_sensitive".to_string());
                c
            },
            issued_at: std::time::Instant::now(),
        };

        let has_access = controller.check_sensitive_contract_access(&owner, "contract_sensitive", 100000).await.unwrap();
        assert!(has_access, "Owner should access sensitive contract data");

        let stranger = UserContext {
            user_id: "stranger".to_string(),
            roles: {
                let mut r = std::collections::HashSet::new();
                r.insert(UserRole::User);
                r
            },
            tier: UserTier::Free,
            contract_ownerships: std::collections::HashSet::new(),
            issued_at: std::time::Instant::now(),
        };

        let no_access = controller.check_sensitive_contract_access(&stranger, "contract_sensitive", 100000).await.unwrap();
        assert!(!no_access, "Stranger should not access sensitive contract data");
    }

    #[tokio::test]
    async fn test_role_permission_management() {
        let controller = AccessController::new();

        let user_perms = controller.get_role_permissions(&UserRole::User).await;
        assert!(!user_perms.contains(&Permission::ManageAccessControl));

        controller.add_role_permission(UserRole::User, Permission::ManageAccessControl).await.unwrap();
        let updated_perms = controller.get_role_permissions(&UserRole::User).await;
        assert!(updated_perms.contains(&Permission::ManageAccessControl));

        controller.remove_role_permission(UserRole::User, Permission::ManageAccessControl).await.unwrap();
        let final_perms = controller.get_role_permissions(&UserRole::User).await;
        assert!(!final_perms.contains(&Permission::ManageAccessControl));
    }
}
