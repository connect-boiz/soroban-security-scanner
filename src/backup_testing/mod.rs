//! Comprehensive Backup and Recovery Testing Framework
//!
//! Provides automated backup integrity verification, recovery testing,
//! retention policies, RTO/RPO tracking, encryption validation, and
//! cross-region replication checks. Implements issue #347 acceptance criteria.

pub mod acceptance;
pub mod integrity;
pub mod metrics;
pub mod notifications;
pub mod replication;
pub mod retention;
pub mod scenarios;
pub mod suite;
pub mod types;

pub use integrity::{BackupIntegrityVerifier, ChecksumAlgorithm};
pub use metrics::{RecoveryMetrics, RpoRtoPolicy};
pub use notifications::{BackupAlert, BackupNotificationService, NotificationChannel};
pub use replication::{ReplicationConfig, ReplicationStatus};
pub use retention::{RetentionPolicy, RetentionTier};
pub use suite::{BackupCheckResult, BackupRecoveryReport, BackupRecoveryTestSuite};
pub use types::{BackupArtifact, BackupFormat, BackupResult, RecoveryResult};
