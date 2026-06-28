//! Scan Access Control & Ownership Verification Module
//!
//! This module implements strict ownership verification for scan result access,
//! role-based access control (RBAC), scan result sharing mechanisms, and
//! access audit logging to prevent Insecure Direct Object Reference (IDOR)
//! vulnerabilities (Issue #329).
//!
//! Key features:
//! - Ownership verification: every scan result access is checked against the
//!   requesting user's identity
//! - RBAC: Admin, Auditor, SecurityResearcher, and Developer roles with
//!   different access levels
//! - UUID-based scan IDs to prevent enumeration attacks
//! - Scan result sharing with explicit authorization and expiration
//! - Access audit logging for compliance and forensic analysis

use chrono::{DateTime, Duration, Utc};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

// ── Role-Based Access Control ────────────────────────────────────────────────

/// User roles for scan result access control.
/// Ordered from most privileged to least privileged.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ScanAccessRole {
    /// Full access: can view, modify, delete, and share all scan results
    Admin,
    /// Can view and audit all scan results but cannot modify or share
    Auditor,
    /// Can view their own scan results and results shared with them
    SecurityResearcher,
    /// Can view their own scan results only
    Developer,
    /// Minimal access: can only view explicitly shared scan results
    User,
}

impl ScanAccessRole {
    /// Check if this role can access scan results owned by another user
    pub fn can_access_others(&self) -> bool {
        matches!(self, ScanAccessRole::Admin | ScanAccessRole::Auditor)
    }

    /// Check if this role can share scan results
    pub fn can_share(&self) -> bool {
        matches!(self, ScanAccessRole::Admin | ScanAccessRole::SecurityResearcher)
    }

    /// Check if this role can delete scan results
    pub fn can_delete(&self) -> bool {
        matches!(self, ScanAccessRole::Admin)
    }

    /// Convert from string (used when deserializing from JWT claims)
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "admin" => ScanAccessRole::Admin,
            "auditor" => ScanAccessRole::Auditor,
            "security_researcher" | "researcher" => ScanAccessRole::SecurityResearcher,
            "developer" => ScanAccessRole::Developer,
            _ => ScanAccessRole::User,
        }
    }
}

// ── Scan Record with Ownership ──────────────────────────────────────────────

/// A scan result record with ownership information.
/// Uses UUIDs instead of sequential IDs to prevent enumeration attacks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRecord {
    /// Unique scan identifier (UUID v4)
    pub scan_id: Uuid,
    /// The user who initiated and owns this scan
    pub owner_id: String,
    /// The contract or file that was scanned
    pub target: String,
    /// Scan status
    pub status: ScanStatus,
    /// Severity summary
    pub severity_summary: ScanSeveritySummary,
    /// When the scan was created
    pub created_at: DateTime<Utc>,
    /// When the scan was last updated
    pub updated_at: DateTime<Utc>,
    /// When the scan was completed (if finished)
    pub completed_at: Option<DateTime<Utc>>,
    /// Number of vulnerabilities found
    pub vulnerability_count: u32,
    /// Number of invariant violations found
    pub invariant_violation_count: u32,
    /// Whether this scan is publicly accessible (requires explicit opt-in)
    pub is_public: bool,
    /// Access control metadata
    pub access_metadata: ScanAccessMetadata,
}

/// Severity summary for a scan result
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanSeveritySummary {
    pub critical: u32,
    pub high: u32,
    pub medium: u32,
    pub low: u32,
    pub informational: u32,
}

/// Access control metadata attached to each scan record
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanAccessMetadata {
    /// List of user IDs this scan has been shared with
    pub shared_with: Vec<String>,
    /// When the sharing expires (if set)
    pub sharing_expires_at: Option<DateTime<Utc>>,
    /// Users who have viewed this scan (for audit trail)
    pub access_log: Vec<ScanAccessLogEntry>,
    /// Whether the scan result has been verified by an auditor
    pub auditor_verified: bool,
    /// The auditor who verified the scan (if applicable)
    pub verified_by: Option<String>,
}

/// A single entry in the scan access log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanAccessLogEntry {
    pub user_id: String,
    pub role: String,
    pub accessed_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub action: ScanAccessAction,
    pub success: bool,
    pub failure_reason: Option<String>,
}

/// Actions that can be performed on scan results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanAccessAction {
    View,
    Download,
    Share,
    Delete,
    Verify,
    Export,
}

/// Scan status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for ScanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanStatus::Pending => write!(f, "pending"),
            ScanStatus::InProgress => write!(f, "in_progress"),
            ScanStatus::Completed => write!(f, "completed"),
            ScanStatus::Failed => write!(f, "failed"),
            ScanStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

// ── Scan Sharing ────────────────────────────────────────────────────────────

/// Request to share a scan result with another user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareScanRequest {
    /// The scan to share
    pub scan_id: Uuid,
    /// User to share with
    pub share_with_user_id: String,
    /// Optional expiration for the share
    pub expires_in_hours: Option<u32>,
    /// Optional note about why the scan is being shared
    pub note: Option<String>,
}

/// Response after sharing a scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareScanResponse {
    pub scan_id: Uuid,
    pub shared_with: String,
    pub shared_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

// ── Access Control Engine ───────────────────────────────────────────────────

/// Configuration for the scan access control engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanAccessControlConfig {
    /// Whether access control is enforced (should always be true in production)
    pub enforce_access_control: bool,
    /// Maximum number of users a scan can be shared with
    pub max_share_recipients: usize,
    /// Default sharing expiration in hours (None = no expiration)
    pub default_share_expiry_hours: Option<u32>,
    /// Maximum sharing expiration in hours
    pub max_share_expiry_hours: u32,
    /// Whether to log all access attempts
    pub log_all_access: bool,
    /// Maximum access log entries per scan before rotation
    pub max_access_log_entries: usize,
}

impl Default for ScanAccessControlConfig {
    fn default() -> Self {
        Self {
            enforce_access_control: true,
            max_share_recipients: 50,
            default_share_expiry_hours: Some(720), // 30 days
            max_share_expiry_hours: 8760,          // 1 year
            log_all_access: true,
            max_access_log_entries: 1000,
        }
    }
}

/// Core access control engine for scan results.
/// Must be used before any scan result retrieval endpoint.
pub struct ScanAccessControl {
    config: ScanAccessControlConfig,
    /// In-memory store of scan records (production would use database)
    pub(crate) scans: Arc<std::sync::RwLock<HashMap<Uuid, ScanRecord>>>,
}

impl ScanAccessControl {
    /// Create a new scan access control engine
    pub fn new(config: ScanAccessControlConfig) -> Self {
        Self {
            config,
            scans: Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }

    // ── Ownership Verification ──────────────────────────────────────────

    /// Verify that a user owns or is authorized to access a scan result.
    /// This is the PRIMARY defense against IDOR attacks.
    ///
    /// # Arguments
    /// * `scan_id` - The scan being accessed
    /// * `user_id` - The requesting user
    /// * `user_role` - The requesting user's role
    ///
    /// # Returns
    /// * `Ok(())` if access is authorized
    /// * `Err(...)` with a reason if access is denied
    pub fn verify_scan_access(
        &self,
        scan_id: &Uuid,
        user_id: &str,
        user_role: &ScanAccessRole,
    ) -> Result<(), ScanAccessError> {
        if !self.config.enforce_access_control {
            warn!("Scan access control is disabled — this should never happen in production");
            return Ok(());
        }

        let scans = self
            .scans
            .read()
            .map_err(|e| ScanAccessError::InternalError(e.to_string()))?;

        let scan = scans
            .get(scan_id)
            .ok_or(ScanAccessError::ScanNotFound(*scan_id))?;

        // Check 1: Is the user the owner?
        if scan.owner_id == user_id {
            return Ok(());
        }

        // Check 2: Does the user have a role that allows accessing others' scans?
        if user_role.can_access_others() {
            return Ok(());
        }

        // Check 3: Has the scan been explicitly shared with this user?
        if self.is_scan_shared_with(&scan, user_id) {
            return Ok(());
        }

        // Check 4: Is the scan public?
        if scan.is_public {
            return Ok(());
        }

        // Access denied — log the attempt
        warn!(
            "IDOR prevention: Access denied for user {} to scan {} (owner: {})",
            user_id, scan_id, scan.owner_id
        );

        Err(ScanAccessError::AccessDenied {
            scan_id: *scan_id,
            user_id: user_id.to_string(),
            reason: "User does not own this scan, does not have a privileged role, "
                .to_string()
                + "and the scan has not been shared with them",
        })
    }

    /// Verify that a user can modify (delete, update) a scan result.
    /// Only the owner and admins can modify scan results.
    pub fn verify_scan_modification(
        &self,
        scan_id: &Uuid,
        user_id: &str,
        user_role: &ScanAccessRole,
    ) -> Result<(), ScanAccessError> {
        let scans = self
            .scans
            .read()
            .map_err(|e| ScanAccessError::InternalError(e.to_string()))?;

        let scan = scans
            .get(scan_id)
            .ok_or(ScanAccessError::ScanNotFound(*scan_id))?;

        // Only owner or admin can modify
        if scan.owner_id == user_id || *user_role == ScanAccessRole::Admin {
            Ok(())
        } else {
            Err(ScanAccessError::AccessDenied {
                scan_id: *scan_id,
                user_id: user_id.to_string(),
                reason: "Only the scan owner or an admin can modify scan results".to_string(),
            })
        }
    }

    // ── Scan Record Management ──────────────────────────────────────────

    /// Register a new scan record with the given owner.
    /// Generates a cryptographically random UUID to prevent enumeration.
    pub fn register_scan(
        &self,
        owner_id: &str,
        target: &str,
    ) -> Result<Uuid, ScanAccessError> {
        let scan_id = Uuid::new_v4(); // Cryptographically random — not enumerable
        let now = Utc::now();

        let scan = ScanRecord {
            scan_id,
            owner_id: owner_id.to_string(),
            target: target.to_string(),
            status: ScanStatus::Pending,
            severity_summary: ScanSeveritySummary::default(),
            created_at: now,
            updated_at: now,
            completed_at: None,
            vulnerability_count: 0,
            invariant_violation_count: 0,
            is_public: false,
            access_metadata: ScanAccessMetadata::default(),
        };

        let mut scans = self
            .scans
            .write()
            .map_err(|e| ScanAccessError::InternalError(e.to_string()))?;

        scans.insert(scan_id, scan);

        info!(
            "Scan registered: scan_id={} owner={} target={}",
            scan_id, owner_id, target
        );

        Ok(scan_id)
    }

    /// Get a scan record (no access check — use verify_scan_access first)
    pub fn get_scan_record(&self, scan_id: &Uuid) -> Result<ScanRecord, ScanAccessError> {
        let scans = self
            .scans
            .read()
            .map_err(|e| ScanAccessError::InternalError(e.to_string()))?;

        scans
            .get(scan_id)
            .cloned()
            .ok_or(ScanAccessError::ScanNotFound(*scan_id))
    }

    /// Update scan status after completion
    pub fn update_scan_status(
        &self,
        scan_id: &Uuid,
        status: ScanStatus,
        vulnerability_count: u32,
        invariant_violation_count: u32,
    ) -> Result<(), ScanAccessError> {
        let mut scans = self
            .scans
            .write()
            .map_err(|e| ScanAccessError::InternalError(e.to_string()))?;

        let scan = scans
            .get_mut(scan_id)
            .ok_or(ScanAccessError::ScanNotFound(*scan_id))?;

        scan.status = status;
        scan.vulnerability_count = vulnerability_count;
        scan.invariant_violation_count = invariant_violation_count;
        scan.updated_at = Utc::now();

        if matches!(scan.status, ScanStatus::Completed | ScanStatus::Failed) {
            scan.completed_at = Some(Utc::now());
        }

        Ok(())
    }

    /// List scans owned by a specific user
    pub fn list_user_scans(&self, user_id: &str) -> Result<Vec<ScanRecord>, ScanAccessError> {
        let scans = self
            .scans
            .read()
            .map_err(|e| ScanAccessError::InternalError(e.to_string()))?;

        Ok(scans
            .values()
            .filter(|s| s.owner_id == user_id)
            .cloned()
            .collect())
    }

    // ── Scan Sharing ────────────────────────────────────────────────────

    /// Share a scan result with another user
    pub fn share_scan(
        &self,
        scan_id: &Uuid,
        owner_id: &str,
        share_with_user_id: &str,
        expires_in_hours: Option<u32>,
    ) -> Result<ShareScanResponse, ScanAccessError> {
        let mut scans = self
            .scans
            .write()
            .map_err(|e| ScanAccessError::InternalError(e.to_string()))?;

        let scan = scans
            .get_mut(scan_id)
            .ok_or(ScanAccessError::ScanNotFound(*scan_id))?;

        // Only the owner or admin can share
        if scan.owner_id != owner_id {
            return Err(ScanAccessError::AccessDenied {
                scan_id: *scan_id,
                user_id: owner_id.to_string(),
                reason: "Only the scan owner can share scan results".to_string(),
            });
        }

        // Don't share with self
        if share_with_user_id == owner_id {
            return Err(ScanAccessError::InvalidOperation(
                "Cannot share a scan with yourself".to_string(),
            ));
        }

        // Check max share recipients
        if scan.access_metadata.shared_with.len() >= self.config.max_share_recipients {
            return Err(ScanAccessError::LimitExceeded(
                "Maximum number of share recipients reached".to_string(),
            ));
        }

        let hours = expires_in_hours
            .or(self.config.default_share_expiry_hours)
            .unwrap_or(720)
            .min(self.config.max_share_expiry_hours);

        let expires_at = Utc::now() + Duration::hours(hours as i64);

        // Add user to shared list if not already present
        if !scan.access_metadata.shared_with.contains(&share_with_user_id.to_string()) {
            scan.access_metadata.shared_with.push(share_with_user_id.to_string());
        }

        scan.access_metadata.sharing_expires_at = Some(expires_at);
        scan.updated_at = Utc::now();

        info!(
            "Scan {} shared with user {} by {} (expires: {})",
            scan_id, share_with_user_id, owner_id, expires_at
        );

        Ok(ShareScanResponse {
            scan_id: *scan_id,
            shared_with: share_with_user_id.to_string(),
            shared_at: Utc::now(),
            expires_at: Some(expires_at),
        })
    }

    /// Revoke sharing for a specific user
    pub fn revoke_share(
        &self,
        scan_id: &Uuid,
        owner_id: &str,
        revoke_user_id: &str,
    ) -> Result<(), ScanAccessError> {
        let mut scans = self
            .scans
            .write()
            .map_err(|e| ScanAccessError::InternalError(e.to_string()))?;

        let scan = scans
            .get_mut(scan_id)
            .ok_or(ScanAccessError::ScanNotFound(*scan_id))?;

        if scan.owner_id != owner_id {
            return Err(ScanAccessError::AccessDenied {
                scan_id: *scan_id,
                user_id: owner_id.to_string(),
                reason: "Only the scan owner can revoke sharing".to_string(),
            });
        }

        scan.access_metadata
            .shared_with
            .retain(|id| id != revoke_user_id);
        scan.updated_at = Utc::now();

        Ok(())
    }

    // ── Access Audit Logging ────────────────────────────────────────────

    /// Log an access attempt to a scan result
    pub fn log_access(
        &self,
        scan_id: &Uuid,
        user_id: &str,
        role: &ScanAccessRole,
        ip_address: Option<&str>,
        action: ScanAccessAction,
        success: bool,
        failure_reason: Option<&str>,
    ) -> Result<(), ScanAccessError> {
        if !self.config.log_all_access {
            return Ok(());
        }

        let entry = ScanAccessLogEntry {
            user_id: user_id.to_string(),
            role: format!("{:?}", role),
            accessed_at: Utc::now(),
            ip_address: ip_address.map(|s| s.to_string()),
            action,
            success,
            failure_reason: failure_reason.map(|s| s.to_string()),
        };

        let mut scans = self
            .scans
            .write()
            .map_err(|e| ScanAccessError::InternalError(e.to_string()))?;

        if let Some(scan) = scans.get_mut(scan_id) {
            scan.access_metadata.access_log.push(entry);

            // Rotate old log entries
            if scan.access_metadata.access_log.len() > self.config.max_access_log_entries {
                let excess =
                    scan.access_metadata.access_log.len() - self.config.max_access_log_entries;
                scan.access_metadata.access_log.drain(0..excess);
            }
        }

        Ok(())
    }

    /// Get the access log for a scan (admin/auditor only)
    pub fn get_access_log(
        &self,
        scan_id: &Uuid,
        user_id: &str,
        user_role: &ScanAccessRole,
    ) -> Result<Vec<ScanAccessLogEntry>, ScanAccessError> {
        // First verify access
        self.verify_scan_access(scan_id, user_id, user_role)?;

        // Only admin/auditor/owner can see access logs
        let scans = self
            .scans
            .read()
            .map_err(|e| ScanAccessError::InternalError(e.to_string()))?;

        let scan = scans
            .get(scan_id)
            .ok_or(ScanAccessError::ScanNotFound(*scan_id))?;

        if scan.owner_id != user_id && !user_role.can_access_others() {
            return Err(ScanAccessError::AccessDenied {
                scan_id: *scan_id,
                user_id: user_id.to_string(),
                reason: "Only the scan owner, admin, or auditor can view access logs".to_string(),
            });
        }

        Ok(scan.access_metadata.access_log.clone())
    }

    // ── Private Helpers ─────────────────────────────────────────────────

    /// Check if a scan has been shared with a user and the share hasn't expired
    fn is_scan_shared_with(&self, scan: &ScanRecord, user_id: &str) -> bool {
        // Check if sharing has expired
        if let Some(expiry) = scan.access_metadata.sharing_expires_at {
            if Utc::now() > expiry {
                return false;
            }
        }

        scan.access_metadata.shared_with.contains(&user_id.to_string())
    }
}

// ── Error Types ─────────────────────────────────────────────────────────────

/// Errors that can occur during scan access control operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum ScanAccessError {
    #[error("Scan not found: {0}")]
    ScanNotFound(Uuid),

    #[error("Access denied to scan {scan_id} for user {user_id}: {reason}")]
    AccessDenied {
        scan_id: Uuid,
        user_id: String,
        reason: String,
    },

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Limit exceeded: {0}")]
    LimitExceeded(String),

    #[error("Sharing has expired for scan {0}")]
    SharingExpired(Uuid),

    #[error("Internal error: {0}")]
    InternalError(String),
}

// ── Middleware Helpers ──────────────────────────────────────────────────────

/// Optional wrapper for use with Axum extractors.
/// Extracts user identity from request extensions and verifies scan ownership.
pub struct ScanOwnershipGuard {
    pub scan_id: Uuid,
    pub user_id: String,
    pub role: ScanAccessRole,
}

impl ScanOwnershipGuard {
    /// Create a new ownership guard. This should be called in route handlers
    /// after the auth middleware has populated request extensions.
    pub fn new(scan_id: Uuid, user_id: String, role: ScanAccessRole) -> Self {
        Self {
            scan_id,
            user_id,
            role,
        }
    }

    /// Verify ownership or authorized access. Call this at the start of
    /// every scan result endpoint handler.
    pub fn verify(
        &self,
        access_control: &ScanAccessControl,
    ) -> Result<(), ScanAccessError> {
        access_control.verify_scan_access(&self.scan_id, &self.user_id, &self.role)
    }

    /// Verify modification rights (owner or admin only)
    pub fn verify_modification(
        &self,
        access_control: &ScanAccessControl,
    ) -> Result<(), ScanAccessError> {
        access_control.verify_scan_modification(&self.scan_id, &self.user_id, &self.role)
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn create_access_control() -> ScanAccessControl {
        ScanAccessControl::new(ScanAccessControlConfig::default())
    }

    fn register_test_scan(ac: &ScanAccessControl, owner_id: &str) -> Uuid {
        ac.register_scan(owner_id, "test_contract.wasm").unwrap()
    }

    #[test]
    fn test_owner_can_access_own_scan() {
        let ac = create_access_control();
        let scan_id = register_test_scan(&ac, "user123");

        let result = ac.verify_scan_access(&scan_id, "user123", &ScanAccessRole::Developer);
        assert!(result.is_ok(), "Owner should be able to access their own scan");
    }

    #[test]
    fn test_non_owner_cannot_access_scan() {
        let ac = create_access_control();
        let scan_id = register_test_scan(&ac, "user123");

        let result = ac.verify_scan_access(&scan_id, "attacker", &ScanAccessRole::Developer);
        assert!(result.is_err(), "Non-owner should not be able to access scan");
    }

    #[test]
    fn test_admin_can_access_any_scan() {
        let ac = create_access_control();
        let scan_id = register_test_scan(&ac, "user123");

        let result = ac.verify_scan_access(&scan_id, "admin_user", &ScanAccessRole::Admin);
        assert!(result.is_ok(), "Admin should be able to access any scan");
    }

    #[test]
    fn test_auditor_can_access_any_scan() {
        let ac = create_access_control();
        let scan_id = register_test_scan(&ac, "user123");

        let result = ac.verify_scan_access(&scan_id, "auditor_user", &ScanAccessRole::Auditor);
        assert!(result.is_ok(), "Auditor should be able to access any scan");
    }

    #[test]
    fn test_shared_scan_can_be_accessed() {
        let ac = create_access_control();
        let scan_id = register_test_scan(&ac, "owner");

        // Share with another user
        ac.share_scan(&scan_id, "owner", "shared_user", None)
            .unwrap();

        let result = ac.verify_scan_access(&scan_id, "shared_user", &ScanAccessRole::Developer);
        assert!(result.is_ok(), "Shared user should be able to access scan");
    }

    #[test]
    fn test_revoked_share_prevents_access() {
        let ac = create_access_control();
        let scan_id = register_test_scan(&ac, "owner");

        ac.share_scan(&scan_id, "owner", "shared_user", None)
            .unwrap();
        ac.revoke_share(&scan_id, "owner", "shared_user")
            .unwrap();

        let result = ac.verify_scan_access(&scan_id, "shared_user", &ScanAccessRole::Developer);
        assert!(
            result.is_err(),
            "Revoked share should prevent access"
        );
    }

    #[test]
    fn test_public_scan_accessible() {
        let ac = create_access_control();
        let scan_id = register_test_scan(&ac, "owner");

        // Make scan public
        {
            let mut scans = ac.scans.write().unwrap();
            scans.get_mut(&scan_id).unwrap().is_public = true;
        }

        let result = ac.verify_scan_access(&scan_id, "any_user", &ScanAccessRole::User);
        assert!(result.is_ok(), "Public scan should be accessible to anyone");
    }

    #[test]
    fn test_non_owner_cannot_modify_scan() {
        let ac = create_access_control();
        let scan_id = register_test_scan(&ac, "owner");

        let result =
            ac.verify_scan_modification(&scan_id, "non_owner", &ScanAccessRole::Developer);
        assert!(
            result.is_err(),
            "Non-owner should not be able to modify scan"
        );
    }

    #[test]
    fn test_admin_can_modify_scan() {
        let ac = create_access_control();
        let scan_id = register_test_scan(&ac, "owner");

        let result = ac.verify_scan_modification(&scan_id, "admin", &ScanAccessRole::Admin);
        assert!(result.is_ok(), "Admin should be able to modify any scan");
    }

    #[test]
    fn test_nonexistent_scan_returns_not_found() {
        let ac = create_access_control();
        let fake_id = Uuid::new_v4();

        let result = ac.verify_scan_access(&fake_id, "user123", &ScanAccessRole::Admin);
        assert!(matches!(result, Err(ScanAccessError::ScanNotFound(_))));
    }

    #[test]
    fn test_uuid_generation_is_unique() {
        let ac = create_access_control();
        let id1 = register_test_scan(&ac, "user1");
        let id2 = register_test_scan(&ac, "user1");

        assert_ne!(id1, id2, "UUID scan IDs must be unique");
    }

    #[test]
    fn test_cannot_share_with_self() {
        let ac = create_access_control();
        let scan_id = register_test_scan(&ac, "owner");

        let result = ac.share_scan(&scan_id, "owner", "owner", None);
        assert!(result.is_err(), "Cannot share a scan with yourself");
    }

    #[test]
    fn test_role_from_string() {
        assert_eq!(ScanAccessRole::from_str("admin"), ScanAccessRole::Admin);
        assert_eq!(ScanAccessRole::from_str("auditor"), ScanAccessRole::Auditor);
        assert_eq!(
            ScanAccessRole::from_str("security_researcher"),
            ScanAccessRole::SecurityResearcher
        );
        assert_eq!(
            ScanAccessRole::from_str("developer"),
            ScanAccessRole::Developer
        );
        assert_eq!(ScanAccessRole::from_str("unknown"), ScanAccessRole::User);
    }

    #[test]
    fn test_scan_record_list_filtered_by_owner() {
        let ac = create_access_control();
        register_test_scan(&ac, "userA");
        register_test_scan(&ac, "userA");
        register_test_scan(&ac, "userB");

        let user_a_scans = ac.list_user_scans("userA").unwrap();
        assert_eq!(user_a_scans.len(), 2);

        let user_b_scans = ac.list_user_scans("userB").unwrap();
        assert_eq!(user_b_scans.len(), 1);

        let user_c_scans = ac.list_user_scans("userC").unwrap();
        assert!(user_c_scans.is_empty());
    }

    #[test]
    fn test_access_log_is_recorded() {
        let ac = create_access_control();
        let scan_id = register_test_scan(&ac, "owner");

        // Log successful access
        ac.log_access(
            &scan_id,
            "owner",
            &ScanAccessRole::Developer,
            Some("127.0.0.1"),
            ScanAccessAction::View,
            true,
            None,
        )
        .unwrap();

        // Log denied access
        ac.log_access(
            &scan_id,
            "attacker",
            &ScanAccessRole::User,
            Some("10.0.0.1"),
            ScanAccessAction::View,
            false,
            Some("Access denied — not owner"),
        )
        .unwrap();

        let log = ac
            .get_access_log(&scan_id, "owner", &ScanAccessRole::Developer)
            .unwrap();
        assert_eq!(log.len(), 2);
        assert!(log[0].success);
        assert!(!log[1].success);
    }

    #[test]
    fn test_list_user_scans_empty() {
        let ac = create_access_control();
        let scans = ac.list_user_scans("nonexistent").unwrap();
        assert!(scans.is_empty());
    }
}
