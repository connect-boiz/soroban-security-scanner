//! End-to-end integration tests for backup and recovery (issue #347).
//! Note: run via `cargo test --lib backup_testing::acceptance` in CI
//! because the binary target does not compile without broken-modules.

use soroban_security_scanner::backup_testing::{
    integrity::BackupIntegrityVerifier, metrics::RpoRtoPolicy, scenarios, types::BackupFormat,
};

#[test]
fn integration_backup_integrity_roundtrip() {
    let artifacts = scenarios::sample_artifacts();
    let verifier = BackupIntegrityVerifier::new();
    for artifact in &artifacts {
        assert!(verifier.verify(artifact));
    }
}

#[test]
fn integration_full_suite_passes() {
    let report = scenarios::default_suite().run();
    assert!(report.passed());
}

#[test]
fn integration_all_formats_represented() {
    let artifacts = scenarios::sample_artifacts();
    assert_eq!(artifacts.len(), 4);
    let formats: Vec<_> = artifacts.iter().map(|a| a.format).collect();
    assert!(formats.contains(&BackupFormat::JsonState));
    assert!(formats.contains(&BackupFormat::WalletExport));
}

#[test]
fn integration_rto_rpo_policy_defaults() {
    let policy = RpoRtoPolicy::default();
    assert_eq!(policy.rpo_minutes, 60);
    assert_eq!(policy.rto_minutes, 240);
}
