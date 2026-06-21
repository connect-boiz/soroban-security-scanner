//! Pre-built backup/recovery test scenarios.

use crate::backup_testing::{
    integrity::BackupIntegrityVerifier,
    metrics::{RecoveryMetrics, RpoRtoPolicy},
    notifications::BackupNotificationService,
    replication::{ReplicationConfig, ReplicationManager},
    retention::RetentionPolicy,
    suite::BackupRecoveryTestSuite,
    types::{BackupArtifact, BackupFormat},
};

/// Build sample artifacts covering all backup formats.
pub fn sample_artifacts() -> Vec<BackupArtifact> {
    let verifier = BackupIntegrityVerifier::new();

    let state_data = br#"{"transactions":[],"queue_stats":{"pending":0}}"#.to_vec();
    let state_checksum = verifier.compute_checksum(&state_data);

    let wallet_data = br#"{"encrypted_seed":"abc","integrity_hmac":"def"}"#.to_vec();
    let wallet_checksum = verifier.compute_checksum(&wallet_data);

    let db_data = b"-- PostgreSQL dump\nCREATE TABLE users (id SERIAL);".to_vec();
    let db_checksum = verifier.compute_checksum(&db_data);

    let archive_data = b"\x1f\x8b\x08\x00backup_archive_data".to_vec();
    let archive_checksum = verifier.compute_checksum(&archive_data);

    vec![
        BackupArtifact::new(
            "state-001",
            BackupFormat::JsonState,
            state_data,
            state_checksum,
        ),
        BackupArtifact::new(
            "wallet-001",
            BackupFormat::WalletExport,
            wallet_data,
            wallet_checksum,
        )
        .with_encryption(),
        BackupArtifact::new("db-001", BackupFormat::DatabaseDump, db_data, db_checksum),
        BackupArtifact::new(
            "archive-001",
            BackupFormat::CompressedArchive,
            archive_data,
            archive_checksum,
        ),
    ]
}

/// Default metrics with successful backup/recovery within RTO/RPO.
pub fn default_metrics() -> RecoveryMetrics {
    let policy = RpoRtoPolicy::default();
    let mut metrics = RecoveryMetrics::new(policy);
    metrics.record_backup(500, 4096);
    metrics.record_recovery_test(120_000, true);
    metrics
}

/// Default baseline scenario.
pub fn default_baseline() -> (
    BackupIntegrityVerifier,
    RecoveryMetrics,
    BackupNotificationService,
    ReplicationManager,
    RetentionPolicy,
    Vec<BackupArtifact>,
) {
    (
        BackupIntegrityVerifier::new(),
        default_metrics(),
        BackupNotificationService::new(),
        ReplicationManager::new(ReplicationConfig::default()),
        RetentionPolicy::default(),
        sample_artifacts(),
    )
}

/// Build the default test suite.
pub fn default_suite() -> BackupRecoveryTestSuite {
    let (verifier, metrics, notifications, replication, retention, artifacts) = default_baseline();
    BackupRecoveryTestSuite::new(
        verifier,
        metrics,
        notifications,
        replication,
        retention,
        artifacts,
    )
}

/// Monthly full recovery drill schedule metadata.
pub fn monthly_recovery_drill_schedule() -> &'static str {
    "monthly"
}
