//! Backup integrity verification and recovery testing.
//!
//! Provides typed assertions for backup manifests, checksum
//! verification helpers, and structured recovery test results.

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub backup_id:    String,
    pub created_at:   u64, // Unix timestamp
    pub size_bytes:   u64,
    pub checksum_sha256: String,
    pub components:  Vec<BackupComponent>,
    pub retention_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupComponent {
    pub name:    String,
    pub kind:    ComponentKind,
    pub size_bytes: u64,
    pub checksum:   String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentKind { PostgresDump, RedisAof, ContractCode, AuditLogs }

/// Recovery test result.
#[derive(Debug, Serialize, Deserialize)]
pub struct RecoveryTestResult {
    pub backup_id:       String,
    pub test_passed:     bool,
    pub restore_time:    Duration,
    pub data_integrity:  bool,
    pub service_healthy: bool,
    pub failures:        Vec<String>,
}

/// Validate a backup manifest without performing the restore.
pub fn validate_manifest(m: &BackupManifest) -> Result<(), String> {
    if m.backup_id.is_empty() { return Err("backup_id is empty".into()); }
    if m.size_bytes == 0 { return Err("backup is empty (size_bytes == 0)".into()); }
    if m.checksum_sha256.len() != 64 {
        return Err(format!("checksum_sha256 must be 64 hex chars, got {}", m.checksum_sha256.len()));
    }
    if m.components.is_empty() { return Err("backup has no components".into()); }
    let age_secs = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs();
    let age_days = (age_secs.saturating_sub(m.created_at)) / 86400;
    if age_days > m.retention_days as u64 {
        return Err(format!("backup is {} days old, exceeds retention of {} days", age_days, m.retention_days));
    }
    Ok(())
}

/// Check that all required component kinds are present.
pub fn assert_complete_backup(m: &BackupManifest) -> Result<(), String> {
    let required = [ComponentKind::PostgresDump, ComponentKind::RedisAof, ComponentKind::AuditLogs];
    for req in &required {
        if !m.components.iter().any(|c| &c.kind == req) {
            return Err(format!("missing component: {:?}", req));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn valid_manifest() -> BackupManifest {
        BackupManifest {
            backup_id: "bkp-001".into(),
            created_at: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
            size_bytes: 1024,
            checksum_sha256: "a".repeat(64),
            components: vec![
                BackupComponent { name: "db".into(), kind: ComponentKind::PostgresDump, size_bytes: 512, checksum: "b".repeat(64) },
                BackupComponent { name: "redis".into(), kind: ComponentKind::RedisAof, size_bytes: 256, checksum: "c".repeat(64) },
                BackupComponent { name: "audit".into(), kind: ComponentKind::AuditLogs, size_bytes: 256, checksum: "d".repeat(64) },
            ],
            retention_days: 30,
        }
    }
    #[test] fn valid_manifest_passes()        { assert!(validate_manifest(&valid_manifest()).is_ok()); }
    #[test] fn empty_id_fails()               { let mut m = valid_manifest(); m.backup_id = "".into(); assert!(validate_manifest(&m).is_err()); }
    #[test] fn zero_size_fails()              { let mut m = valid_manifest(); m.size_bytes = 0; assert!(validate_manifest(&m).is_err()); }
    #[test] fn complete_backup_passes()       { assert!(assert_complete_backup(&valid_manifest()).is_ok()); }
    #[test] fn missing_component_fails()      {
        let mut m = valid_manifest();
        m.components.retain(|c| c.kind != ComponentKind::AuditLogs);
        assert!(assert_complete_backup(&m).is_err());
    }
}
