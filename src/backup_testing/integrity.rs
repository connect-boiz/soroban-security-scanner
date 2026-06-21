//! Backup integrity verification with checksum validation.

use crate::backup_testing::types::BackupArtifact;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Checksum algorithms supported for integrity verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChecksumAlgorithm {
    Sha256,
}

impl ChecksumAlgorithm {
    pub fn compute(&self, data: &[u8]) -> String {
        match self {
            Self::Sha256 => {
                let hash = Sha256::digest(data);
                hex::encode(hash)
            }
        }
    }
}

/// Verifies backup artifact integrity via checksum validation.
#[derive(Debug, Clone)]
pub struct BackupIntegrityVerifier {
    algorithm: ChecksumAlgorithm,
}

impl BackupIntegrityVerifier {
    pub fn new() -> Self {
        Self {
            algorithm: ChecksumAlgorithm::Sha256,
        }
    }

    pub fn with_algorithm(algorithm: ChecksumAlgorithm) -> Self {
        Self { algorithm }
    }

    /// Compute checksum for raw backup data.
    pub fn compute_checksum(&self, data: &[u8]) -> String {
        self.algorithm.compute(data)
    }

    /// Verify that an artifact's stored checksum matches its data.
    pub fn verify(&self, artifact: &BackupArtifact) -> bool {
        let computed = self.compute_checksum(&artifact.data);
        computed == artifact.checksum_sha256
    }

    /// Verify and return a detailed result.
    pub fn verify_with_details(&self, artifact: &BackupArtifact) -> IntegrityResult {
        let computed = self.compute_checksum(&artifact.data);
        let valid = computed == artifact.checksum_sha256;
        IntegrityResult {
            artifact_id: artifact.id.clone(),
            valid,
            expected_checksum: artifact.checksum_sha256.clone(),
            computed_checksum: computed,
            algorithm: self.algorithm,
        }
    }

    /// Detect tampering by comparing before/after checksums.
    pub fn detect_tampering(&self, original: &BackupArtifact, modified: &BackupArtifact) -> bool {
        self.verify(original) && !self.verify(modified)
    }
}

impl Default for BackupIntegrityVerifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Detailed integrity verification result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityResult {
    pub artifact_id: String,
    pub valid: bool,
    pub expected_checksum: String,
    pub computed_checksum: String,
    pub algorithm: ChecksumAlgorithm,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup_testing::types::BackupFormat;

    #[test]
    fn checksum_roundtrip() {
        let verifier = BackupIntegrityVerifier::new();
        let data = b"backup payload data".to_vec();
        let checksum = verifier.compute_checksum(&data);
        let artifact = BackupArtifact::new("bak-001", BackupFormat::JsonState, data, checksum);
        assert!(verifier.verify(&artifact));
    }

    #[test]
    fn tampered_backup_fails_verification() {
        let verifier = BackupIntegrityVerifier::new();
        let data = b"original data".to_vec();
        let checksum = verifier.compute_checksum(&data);
        let mut artifact = BackupArtifact::new("bak-002", BackupFormat::JsonState, data, checksum);
        artifact.data = b"tampered data".to_vec();
        assert!(!verifier.verify(&artifact));
    }

    #[test]
    fn detect_tampering_works() {
        let verifier = BackupIntegrityVerifier::new();
        let data = b"original".to_vec();
        let checksum = verifier.compute_checksum(&data);
        let original = BackupArtifact::new(
            "bak-003",
            BackupFormat::JsonState,
            data.clone(),
            checksum.clone(),
        );
        let mut modified = BackupArtifact::new("bak-003", BackupFormat::JsonState, data, checksum);
        modified.data = b"modified".to_vec();
        assert!(verifier.detect_tampering(&original, &modified));
    }
}
