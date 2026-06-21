//! Core types for backup and recovery testing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Supported backup artifact formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupFormat {
    JsonState,
    WalletExport,
    DatabaseDump,
    CompressedArchive,
}

impl BackupFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::JsonState => "json_state",
            Self::WalletExport => "wallet_export",
            Self::DatabaseDump => "database_dump",
            Self::CompressedArchive => "compressed_archive",
        }
    }
}

/// A backup artifact with metadata for integrity verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupArtifact {
    pub id: String,
    pub format: BackupFormat,
    pub data: Vec<u8>,
    pub checksum_sha256: String,
    pub encrypted: bool,
    pub created_at: DateTime<Utc>,
    pub region: String,
    pub size_bytes: usize,
}

impl BackupArtifact {
    pub fn new(
        id: impl Into<String>,
        format: BackupFormat,
        data: Vec<u8>,
        checksum: String,
    ) -> Self {
        let size = data.len();
        Self {
            id: id.into(),
            format,
            data,
            checksum_sha256: checksum,
            encrypted: false,
            created_at: Utc::now(),
            region: "primary".into(),
            size_bytes: size,
        }
    }

    pub fn with_encryption(mut self) -> Self {
        self.encrypted = true;
        self
    }

    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = region.into();
        self
    }
}

/// Result of a backup operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupResult {
    pub artifact_id: String,
    pub success: bool,
    pub duration_ms: u64,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

/// Result of a recovery operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryResult {
    pub artifact_id: String,
    pub success: bool,
    pub duration_ms: u64,
    pub data_matches: bool,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}
