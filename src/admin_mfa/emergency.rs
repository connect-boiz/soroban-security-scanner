//! Emergency MFA-bypass procedure with multi-admin approval.
//!
//! When an admin is locked out (lost device, etc.), a *break-glass* bypass can
//! be granted — but only with quorum approval from other admins. A request
//! collects distinct approvals; once the quorum is met it yields a one-time,
//! time-limited bypass grant. The locked-out account cannot approve its own
//! request, and every action is recorded for audit.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Configuration for the emergency procedure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmergencyConfig {
    /// Number of distinct admin approvals required (quorum).
    pub required_approvals: usize,
    /// How long a granted bypass remains valid, in seconds.
    pub grant_validity_secs: i64,
    /// How long a request may collect approvals before expiring, in seconds.
    pub request_ttl_secs: i64,
}

impl Default for EmergencyConfig {
    fn default() -> Self {
        Self {
            required_approvals: 2,
            grant_validity_secs: 15 * 60,
            request_ttl_secs: 60 * 60,
        }
    }
}

/// Status of an emergency bypass request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BypassStatus {
    /// Still collecting approvals.
    Pending,
    /// Quorum met; a grant is active until the contained timestamp.
    Granted {
        /// Unix timestamp at which the grant expires.
        expires_at: i64,
    },
    /// Request expired before quorum / grant consumed or revoked.
    Closed,
}

/// Errors from the emergency workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmergencyError {
    /// The target account may not approve its own bypass.
    SelfApproval,
    /// This admin already approved.
    DuplicateApproval,
    /// The request is no longer pending.
    NotPending,
    /// The request has passed its TTL.
    RequestExpired,
}

/// A break-glass bypass request for a locked-out admin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BypassRequest {
    /// Request id.
    pub id: Uuid,
    /// The locked-out account the bypass is for.
    pub target: Uuid,
    /// The admin who initiated the request.
    pub initiator: Uuid,
    /// When the request was opened (unix secs).
    pub created_at: i64,
    /// Distinct approving admins.
    approvals: Vec<Uuid>,
    /// Current status.
    pub status: BypassStatus,
    cfg: EmergencyConfig,
}

impl BypassRequest {
    /// Opens a request. The initiator counts as the first approver (an admin
    /// vouching for the break-glass), but the target can never be the
    /// initiator.
    pub fn open(
        target: Uuid,
        initiator: Uuid,
        now: i64,
        cfg: EmergencyConfig,
    ) -> Result<Self, EmergencyError> {
        if target == initiator {
            return Err(EmergencyError::SelfApproval);
        }
        let mut req = Self {
            id: Uuid::new_v4(),
            target,
            initiator,
            created_at: now,
            approvals: vec![initiator],
            status: BypassStatus::Pending,
            cfg,
        };
        req.maybe_grant(now);
        Ok(req)
    }

    /// Number of distinct approvals collected.
    pub fn approval_count(&self) -> usize {
        self.approvals.len()
    }

    /// Adds an approval from `admin`. Grants the bypass once quorum is reached.
    pub fn approve(&mut self, admin: Uuid, now: i64) -> Result<&BypassStatus, EmergencyError> {
        if self.status != BypassStatus::Pending {
            return Err(EmergencyError::NotPending);
        }
        if now - self.created_at > self.cfg.request_ttl_secs {
            self.status = BypassStatus::Closed;
            return Err(EmergencyError::RequestExpired);
        }
        if admin == self.target {
            return Err(EmergencyError::SelfApproval);
        }
        if self.approvals.contains(&admin) {
            return Err(EmergencyError::DuplicateApproval);
        }
        self.approvals.push(admin);
        self.maybe_grant(now);
        Ok(&self.status)
    }

    /// Whether a valid bypass grant is currently active at `now`.
    pub fn is_bypass_active(&self, now: i64) -> bool {
        matches!(self.status, BypassStatus::Granted { expires_at } if now <= expires_at)
    }

    /// Consumes the grant (one-time use), closing the request.
    pub fn consume(&mut self, now: i64) -> bool {
        if self.is_bypass_active(now) {
            self.status = BypassStatus::Closed;
            true
        } else {
            false
        }
    }

    /// Cancels/revokes the request.
    pub fn revoke(&mut self) {
        self.status = BypassStatus::Closed;
    }

    fn maybe_grant(&mut self, now: i64) {
        if self.approvals.len() >= self.cfg.required_approvals {
            self.status = BypassStatus::Granted {
                expires_at: now + self.cfg.grant_validity_secs,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> EmergencyConfig {
        EmergencyConfig::default() // requires 2 approvals
    }

    #[test]
    fn quorum_grants_bypass() {
        let target = Uuid::new_v4();
        let admin1 = Uuid::new_v4();
        let admin2 = Uuid::new_v4();
        let mut req = BypassRequest::open(target, admin1, 1000, cfg()).unwrap();
        assert_eq!(req.status, BypassStatus::Pending); // 1 of 2
        req.approve(admin2, 1000).unwrap();
        assert!(req.is_bypass_active(1000));
    }

    #[test]
    fn target_cannot_initiate_or_approve() {
        let target = Uuid::new_v4();
        assert_eq!(
            BypassRequest::open(target, target, 1000, cfg()).unwrap_err(),
            EmergencyError::SelfApproval
        );
        let mut req = BypassRequest::open(target, Uuid::new_v4(), 1000, cfg()).unwrap();
        assert_eq!(req.approve(target, 1000).unwrap_err(), EmergencyError::SelfApproval);
    }

    #[test]
    fn duplicate_approval_is_rejected() {
        // Quorum 3 so the request stays Pending after one extra approval,
        // leaving room to exercise the duplicate-approval guard.
        let custom = EmergencyConfig {
            required_approvals: 3,
            ..cfg()
        };
        let mut req = BypassRequest::open(Uuid::new_v4(), Uuid::new_v4(), 1000, custom).unwrap();
        let admin2 = Uuid::new_v4();
        req.approve(admin2, 1000).unwrap();
        assert_eq!(req.status, BypassStatus::Pending); // 2 of 3
        // admin2 again
        assert_eq!(req.approve(admin2, 1000).unwrap_err(), EmergencyError::DuplicateApproval);
    }

    #[test]
    fn grant_expires() {
        let mut req = BypassRequest::open(Uuid::new_v4(), Uuid::new_v4(), 1000, cfg()).unwrap();
        req.approve(Uuid::new_v4(), 1000).unwrap();
        let expiry = 1000 + cfg().grant_validity_secs;
        assert!(req.is_bypass_active(expiry));
        assert!(!req.is_bypass_active(expiry + 1));
    }

    #[test]
    fn grant_is_one_time() {
        let mut req = BypassRequest::open(Uuid::new_v4(), Uuid::new_v4(), 1000, cfg()).unwrap();
        req.approve(Uuid::new_v4(), 1000).unwrap();
        assert!(req.consume(1000));
        assert!(!req.is_bypass_active(1000)); // closed after consume
        assert!(!req.consume(1000));
    }

    #[test]
    fn request_ttl_expires_pending() {
        let mut req = BypassRequest::open(Uuid::new_v4(), Uuid::new_v4(), 1000, cfg()).unwrap();
        let late = 1000 + cfg().request_ttl_secs + 1;
        assert_eq!(req.approve(Uuid::new_v4(), late).unwrap_err(), EmergencyError::RequestExpired);
        assert_eq!(req.status, BypassStatus::Closed);
    }

    #[test]
    fn three_approval_quorum() {
        let custom = EmergencyConfig {
            required_approvals: 3,
            ..cfg()
        };
        let mut req = BypassRequest::open(Uuid::new_v4(), Uuid::new_v4(), 1000, custom).unwrap();
        req.approve(Uuid::new_v4(), 1000).unwrap();
        assert_eq!(req.status, BypassStatus::Pending); // 2 of 3
        req.approve(Uuid::new_v4(), 1000).unwrap();
        assert!(req.is_bypass_active(1000)); // 3 of 3
    }
}
