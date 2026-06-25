//! Scan result ownership enforcement.
//!
//! Fixes IDOR vulnerability: scan result endpoints previously checked
//! authentication but not ownership. Any authenticated user could read
//! another user's scan results by guessing the scan ID.
//!
//! This module provides `ScanOwnership` — a guard that enforces the
//! authenticated caller owns (or is permitted to view) the requested scan.

use crate::app_error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// The minimal scan metadata needed for ownership checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanMeta {
    pub scan_id:    String,
    pub owner_id:   String,
    /// Additional principals allowed to read this scan (e.g. team members).
    pub shared_with: HashSet<String>,
    /// True if the scan result is publicly visible.
    pub public:     bool,
}

// ---------------------------------------------------------------------------
// Ownership guard
// ---------------------------------------------------------------------------

/// Enforces scan-result ownership before any read operation.
///
/// Allowed principals:
/// 1. The owner of the scan.
/// 2. A principal explicitly listed in `shared_with`.
/// 3. Admins (any principal whose ID is in `admin_ids`).
/// 4. The scan is public (`ScanMeta::public == true`).
#[derive(Debug, Clone)]
pub struct ScanOwnership<'a> {
    meta:      &'a ScanMeta,
    admin_ids: &'a HashSet<String>,
}

impl<'a> ScanOwnership<'a> {
    pub fn new(meta: &'a ScanMeta, admin_ids: &'a HashSet<String>) -> Self {
        Self { meta, admin_ids }
    }

    /// Returns `Ok(())` if `caller_id` may access this scan result.
    /// Returns `Err(AppError::Forbidden)` otherwise — **not** NotFound,
    /// to avoid leaking the existence of scans owned by other users.
    pub fn require_read_access(&self, caller_id: &str) -> Result<(), AppError> {
        if self.meta.public {
            return Ok(());
        }
        if self.meta.owner_id == caller_id {
            return Ok(());
        }
        if self.meta.shared_with.contains(caller_id) {
            return Ok(());
        }
        if self.admin_ids.contains(caller_id) {
            return Ok(());
        }
        tracing::warn!(
            scan_id  = %self.meta.scan_id,
            caller   = %caller_id,
            owner    = %self.meta.owner_id,
            "IDOR attempt: unauthorized scan result access"
        );
        // Return Forbidden (not NotFound) to avoid confirming the scan exists.
        Err(AppError::Forbidden)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn meta(owner: &str, public: bool) -> ScanMeta {
        ScanMeta {
            scan_id:     "scan-1".into(),
            owner_id:    owner.into(),
            shared_with: HashSet::new(),
            public,
        }
    }

    fn admins(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn owner_can_access() {
        let m = meta("alice", false);
        let g = ScanOwnership::new(&m, &admins(&[]));
        assert!(g.require_read_access("alice").is_ok());
    }

    #[test]
    fn other_user_denied() {
        let m = meta("alice", false);
        let g = ScanOwnership::new(&m, &admins(&[]));
        assert!(g.require_read_access("bob").is_err());
    }

    #[test]
    fn admin_can_access() {
        let m = meta("alice", false);
        let g = ScanOwnership::new(&m, &admins(&["admin", "superadmin"]));
        assert!(g.require_read_access("admin").is_ok());
    }

    #[test]
    fn shared_user_can_access() {
        let mut m = meta("alice", false);
        m.shared_with.insert("charlie".into());
        let g = ScanOwnership::new(&m, &admins(&[]));
        assert!(g.require_read_access("charlie").is_ok());
    }

    #[test]
    fn public_scan_anyone_can_access() {
        let m = meta("alice", true);
        let g = ScanOwnership::new(&m, &admins(&[]));
        assert!(g.require_read_access("anonymous").is_ok());
    }

    #[test]
    fn returns_forbidden_not_not_found() {
        let m = meta("alice", false);
        let g = ScanOwnership::new(&m, &admins(&[]));
        let err = g.require_read_access("eve").unwrap_err();
        assert!(matches!(err, crate::app_error::AppError::Forbidden));
    }
}
