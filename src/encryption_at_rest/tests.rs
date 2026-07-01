//! End-to-end integration tests for encryption at rest: field encryption with
//! rotation, key compromise + re-encryption, backups, escrow recovery, and
//! compliance reporting.

use super::*;

const PERIOD: i64 = 30 * 24 * 3600;

fn service() -> EncryptionService {
    EncryptionService::new(KeyManager::new(1000, PERIOD).unwrap(), PERIOD)
}

#[test]
fn sensitive_fields_are_never_stored_in_plaintext() {
    let mut svc = service();
    let secrets = [
        ("users.api_key:1", "sk-live-abcdef"),
        ("wallets.private_key:1", "ed25519-private-key-material"),
        ("sessions.token:1", "bearer-token-xyz"),
    ];
    for (ctx, value) in secrets {
        let stored = svc
            .encrypt_field(value, ctx, 1000, "db", KeyRole::Service, 30)
            .unwrap();
        assert!(!stored.contains(value), "{ctx} leaked plaintext");
        let restored = svc
            .decrypt_field(&stored, ctx, 1000, "db", KeyRole::Service, 20)
            .unwrap();
        assert_eq!(restored, value);
    }
    // Perf monitoring captured every operation.
    assert_eq!(svc.perf().encrypt_ops, 3);
    assert_eq!(svc.perf().decrypt_ops, 3);
}

#[test]
fn key_rotation_preserves_old_data_and_advances_new() {
    let mut svc = service();
    let old = svc
        .encrypt_field("old-secret", "t:1", 1000, "db", KeyRole::Service, 0)
        .unwrap();

    let new_key_id = svc
        .keys_mut()
        .rotate(2000, "admin", KeyRole::KeyAdmin)
        .unwrap();
    let new = svc
        .encrypt_field("new-secret", "t:2", 2000, "db", KeyRole::Service, 0)
        .unwrap();

    // Old value decrypts via the retired key; new value uses the new key.
    assert_eq!(
        svc.decrypt_field(&old, "t:1", 2000, "db", KeyRole::Service, 0)
            .unwrap(),
        "old-secret"
    );
    assert_eq!(
        EncryptedField::from_storage_string(&new).unwrap().key_id,
        new_key_id
    );
}

#[test]
fn compromise_forces_rotation_and_reencryption_recovers_coverage() {
    let mut svc = service();
    let stored = svc
        .encrypt_field("secret", "row:1", 1000, "db", KeyRole::Service, 0)
        .unwrap();
    let original_key = EncryptedField::from_storage_string(&stored).unwrap().key_id;

    // The active key is compromised → automatic rotation to a clean key.
    let new_active = svc
        .keys_mut()
        .mark_compromised(original_key, 2000, "admin", KeyRole::KeyAdmin)
        .unwrap();
    assert_ne!(new_active, original_key);
    assert_eq!(
        svc.keys().status(original_key),
        Some(KeyStatus::Compromised)
    );

    // Data sealed with the compromised key can no longer be read...
    assert!(svc
        .decrypt_field(&stored, "row:1", 2000, "db", KeyRole::Service, 0)
        .is_err());

    // Compliance flags the compromised key until cleaned up.
    let report = svc.compliance_report(2000, 1, 1, true);
    assert!(!report.compliant);
    assert!(report.findings.iter().any(|f| f.contains("compromised")));
}

#[test]
fn backups_use_independent_keys() {
    let mut backup = BackupEncryptor::new(KeyManager::new(1000, 7 * 24 * 3600).unwrap());
    let dump = b"-- full SQL dump --\nINSERT INTO secrets ...";
    let sealed = backup
        .encrypt_backup(1000, "backup-job", KeyRole::Service, dump)
        .unwrap();
    assert_ne!(sealed.data, dump);
    let restored = backup
        .decrypt_backup(&sealed, 1000, "restore-job", KeyRole::Service)
        .unwrap();
    assert_eq!(restored, dump);
}

#[test]
fn escrow_enables_disaster_recovery() {
    let mut keys = KeyManager::new(1000, PERIOD).unwrap();
    let active = keys.active_id();
    // Escrow officer stores a recovery copy.
    keys.escrow(active, 1000, "officer", KeyRole::EscrowOfficer)
        .unwrap();
    // Later, recover it for DR.
    let recovered = keys
        .recover_escrow(active, 2000, "officer", KeyRole::EscrowOfficer)
        .unwrap();
    assert_eq!(recovered.len(), KEY_LEN);
    // A non-officer cannot recover.
    assert_eq!(
        keys.recover_escrow(active, 2000, "svc", KeyRole::Service)
            .unwrap_err(),
        KeyError::AccessDenied
    );
}

#[test]
fn key_access_is_fully_audited() {
    let mut svc = service();
    svc.encrypt_field("x", "c", 1000, "svc", KeyRole::Service, 0)
        .unwrap();
    let _ = svc.keys_mut().rotate(1000, "svc", KeyRole::Service); // denied
    svc.keys_mut()
        .rotate(1000, "admin", KeyRole::KeyAdmin)
        .unwrap();

    let log = svc.keys().audit_log();
    assert!(log.len() >= 3);
    assert!(log.iter().any(|r| !r.granted)); // the denied rotation
    assert!(log
        .iter()
        .any(|r| r.granted && matches!(r.operation, KeyOperation::Rotate)));
}

#[test]
fn full_coverage_with_encrypted_backups_is_compliant() {
    let svc = service();
    let report = svc.compliance_report(1000 + 5 * 86_400, 250, 250, true);
    assert!(report.compliant);
    assert_eq!(report.encrypted_percent, 100.0);
    assert_eq!(report.algorithm, "AES-256-GCM");
    assert!(report.full_field_coverage);
}
