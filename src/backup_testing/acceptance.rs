//! Acceptance tests for issue #347 — run via `cargo test --lib backup_testing::acceptance`.

#[cfg(test)]
mod tests {
    use crate::backup_testing::{
        integrity::BackupIntegrityVerifier,
        metrics::RpoRtoPolicy,
        notifications::BackupNotificationService,
        replication::ReplicationConfig,
        retention::{RetentionPolicy, RetentionTier},
        scenarios,
        types::{BackupFormat, BackupResult, RecoveryResult},
    };
    use chrono::Utc;

    #[test]
    fn acceptance_integrity_verification_with_checksum() {
        let artifacts = scenarios::sample_artifacts();
        let verifier = BackupIntegrityVerifier::new();
        for artifact in &artifacts {
            assert!(
                verifier.verify(artifact),
                "artifact {} must pass checksum",
                artifact.id
            );
        }
    }

    #[test]
    fn acceptance_monthly_recovery_drill_schedule() {
        assert_eq!(scenarios::monthly_recovery_drill_schedule(), "monthly");
    }

    #[test]
    fn acceptance_rto_rpo_defined() {
        let policy = RpoRtoPolicy::default();
        assert!(policy.rpo_minutes > 0);
        assert!(policy.rto_minutes > 0);
        let metrics = scenarios::default_metrics();
        assert!(metrics.meets_rto());
        assert!(metrics.meets_rpo(Utc::now()));
    }

    #[test]
    fn acceptance_backup_encryption() {
        let artifacts = scenarios::sample_artifacts();
        let wallet = artifacts
            .iter()
            .find(|a| a.format == BackupFormat::WalletExport)
            .expect("wallet export artifact required");
        assert!(wallet.encrypted);
    }

    #[test]
    fn acceptance_cross_region_replication() {
        let config = ReplicationConfig::default();
        assert!(config.replica_regions.len() >= 2);
        assert!(config.encryption_in_transit);
    }

    #[test]
    fn acceptance_retention_policy_with_cleanup() {
        let policy = RetentionPolicy::default();
        assert!(policy.auto_cleanup_enabled);
        assert!(!policy.tiers.is_empty());
        assert!(policy.tiers.iter().any(|(t, _)| *t == RetentionTier::Daily));
    }

    #[test]
    fn acceptance_notification_system() {
        let mut svc = BackupNotificationService::new();
        let result = BackupResult {
            artifact_id: "bak-test".into(),
            success: true,
            duration_ms: 100,
            message: "ok".into(),
            timestamp: Utc::now(),
        };
        let alert = svc.on_backup_complete(&result);
        assert!(alert.delivered);

        let recovery = RecoveryResult {
            artifact_id: "bak-test".into(),
            success: true,
            duration_ms: 200,
            data_matches: true,
            message: "recovery ok".into(),
            timestamp: Utc::now(),
        };
        let recovery_alert = svc.on_recovery_complete(&recovery);
        assert!(recovery_alert.delivered);
    }

    #[test]
    fn acceptance_100_percent_backup_success_rate() {
        let metrics = scenarios::default_metrics();
        assert!(metrics.meets_success_target());
        assert_eq!(metrics.success_rate_pct, 100.0);
    }

    #[test]
    fn acceptance_all_backup_formats_tested() {
        let artifacts = scenarios::sample_artifacts();
        let formats: Vec<_> = artifacts.iter().map(|a| a.format).collect();
        assert!(formats.contains(&BackupFormat::JsonState));
        assert!(formats.contains(&BackupFormat::WalletExport));
        assert!(formats.contains(&BackupFormat::DatabaseDump));
        assert!(formats.contains(&BackupFormat::CompressedArchive));
    }

    #[test]
    fn full_backup_recovery_suite_passes() {
        let report = scenarios::default_suite().run();
        assert!(
            report.passed(),
            "failures: {:?}",
            report
                .results
                .iter()
                .filter(|r| !r.passed)
                .collect::<Vec<_>>()
        );
    }
}
