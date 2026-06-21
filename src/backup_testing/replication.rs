//! Cross-region backup replication for disaster recovery.

use crate::backup_testing::types::BackupArtifact;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Cross-region replication configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    pub primary_region: String,
    pub replica_regions: Vec<String>,
    pub replication_lag_threshold_seconds: u64,
    pub encryption_in_transit: bool,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            primary_region: "us-east-1".into(),
            replica_regions: vec!["us-west-2".into(), "eu-west-1".into()],
            replication_lag_threshold_seconds: 300,
            encryption_in_transit: true,
        }
    }
}

/// Status of a replicated backup copy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationStatus {
    pub artifact_id: String,
    pub primary_region: String,
    pub replica_region: String,
    pub replicated: bool,
    pub lag_seconds: u64,
    pub checksum_matches: bool,
    pub last_synced_at: DateTime<Utc>,
}

/// Manages cross-region backup replication verification.
#[derive(Debug, Clone)]
pub struct ReplicationManager {
    config: ReplicationConfig,
}

impl ReplicationManager {
    pub fn new(config: ReplicationConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &ReplicationConfig {
        &self.config
    }

    /// Verify that a backup is replicated to all configured replica regions.
    pub fn verify_replication(
        &self,
        primary: &BackupArtifact,
        replicas: &[BackupArtifact],
    ) -> Vec<ReplicationStatus> {
        self.config
            .replica_regions
            .iter()
            .map(|region| {
                let replica = replicas.iter().find(|r| r.region == *region);
                ReplicationStatus {
                    artifact_id: primary.id.clone(),
                    primary_region: self.config.primary_region.clone(),
                    replica_region: region.clone(),
                    replicated: replica.is_some(),
                    lag_seconds: 0,
                    checksum_matches: replica
                        .map(|r| r.checksum_sha256 == primary.checksum_sha256)
                        .unwrap_or(false),
                    last_synced_at: Utc::now(),
                }
            })
            .collect()
    }

    pub fn all_replicas_healthy(&self, statuses: &[ReplicationStatus]) -> bool {
        statuses.iter().all(|s| s.replicated && s.checksum_matches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup_testing::integrity::BackupIntegrityVerifier;
    use crate::backup_testing::types::BackupFormat;

    #[test]
    fn replication_to_all_regions() {
        let config = ReplicationConfig::default();
        let manager = ReplicationManager::new(config);
        let verifier = BackupIntegrityVerifier::new();
        let data = b"backup data".to_vec();
        let checksum = verifier.compute_checksum(&data);

        let primary = BackupArtifact::new(
            "bak-001",
            BackupFormat::JsonState,
            data.clone(),
            checksum.clone(),
        )
        .with_region("us-east-1");

        let replicas: Vec<BackupArtifact> = manager
            .config()
            .replica_regions
            .iter()
            .map(|region| {
                BackupArtifact::new(
                    "bak-001",
                    BackupFormat::JsonState,
                    data.clone(),
                    checksum.clone(),
                )
                .with_region(region)
            })
            .collect();

        let statuses = manager.verify_replication(&primary, &replicas);
        assert_eq!(statuses.len(), 2);
        assert!(manager.all_replicas_healthy(&statuses));
    }

    #[test]
    fn missing_replica_detected() {
        let config = ReplicationConfig::default();
        let manager = ReplicationManager::new(config);
        let primary = BackupArtifact::new(
            "bak-002",
            BackupFormat::JsonState,
            b"data".to_vec(),
            "abc".into(),
        );
        let statuses = manager.verify_replication(&primary, &[]);
        assert!(!manager.all_replicas_healthy(&statuses));
    }
}
