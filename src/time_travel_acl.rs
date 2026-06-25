//! Access control for the Time Travel Debugger.
//!
//! Enforces that only authorised principals can access historical ledger
//! sequences and contract states. Prevents information leakage of
//! vulnerability data stored in past ledger snapshots.

use crate::app_error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Roles and permissions
// ---------------------------------------------------------------------------

/// Principal roles in the system.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Admin,
    Auditor,
    Researcher,
    ReadOnly,
}

/// Permissions that can be checked against a role.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TimeTravelPermission {
    /// View ledger state at any sequence.
    ReadLedgerState,
    /// Read contract storage at a given sequence.
    ReadContractState,
    /// Replay a transaction trace.
    ReplayTransaction,
    /// Access vulnerability data embedded in historical state.
    ReadVulnerabilityHistory,
}

impl Role {
    /// Returns the set of time-travel permissions granted to this role.
    pub fn time_travel_permissions(&self) -> HashSet<TimeTravelPermission> {
        use TimeTravelPermission::*;
        match self {
            Role::Admin => [
                ReadLedgerState, ReadContractState, ReplayTransaction, ReadVulnerabilityHistory,
            ].into(),
            Role::Auditor => [
                ReadLedgerState, ReadContractState, ReadVulnerabilityHistory,
            ].into(),
            Role::Researcher => [
                ReadLedgerState, ReadContractState,
            ].into(),
            Role::ReadOnly => [
                ReadLedgerState,
            ].into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Guard
// ---------------------------------------------------------------------------

/// Access control guard for time-travel debugger operations.
#[derive(Debug, Clone)]
pub struct TimeTravelGuard {
    pub principal_id: String,
    pub roles:        Vec<Role>,
    /// Maximum ledger sequence this principal may access.
    /// `None` means unrestricted (Admin only).
    pub max_ledger_sequence: Option<u32>,
}

impl TimeTravelGuard {
    pub fn new(principal_id: String, roles: Vec<Role>, max_ledger_sequence: Option<u32>) -> Self {
        Self { principal_id, roles, max_ledger_sequence }
    }

    /// Returns `Ok(())` if this principal holds `permission`, else `Err(AppError::Forbidden)`.
    pub fn require_permission(&self, permission: &TimeTravelPermission) -> Result<(), AppError> {
        let has = self.roles
            .iter()
            .any(|r| r.time_travel_permissions().contains(permission));
        if has { Ok(()) } else { Err(AppError::Forbidden) }
    }

    /// Returns `Ok(())` if `ledger_seq` is within the allowed range.
    pub fn require_ledger_access(&self, ledger_seq: u32) -> Result<(), AppError> {
        match self.max_ledger_sequence {
            None        => Ok(()), // unrestricted
            Some(max) if ledger_seq <= max => Ok(()),
            Some(max) => {
                tracing::warn!(
                    principal = %self.principal_id,
                    requested = ledger_seq,
                    max_allowed = max,
                    "ledger sequence access denied"
                );
                Err(AppError::Forbidden)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn guard(role: Role) -> TimeTravelGuard {
        TimeTravelGuard::new("test".into(), vec![role], None)
    }

    #[test]
    fn admin_can_read_vulnerability_history() {
        assert!(guard(Role::Admin)
            .require_permission(&TimeTravelPermission::ReadVulnerabilityHistory)
            .is_ok());
    }

    #[test]
    fn readonly_cannot_read_vulnerability_history() {
        assert!(guard(Role::ReadOnly)
            .require_permission(&TimeTravelPermission::ReadVulnerabilityHistory)
            .is_err());
    }

    #[test]
    fn readonly_can_read_ledger_state() {
        assert!(guard(Role::ReadOnly)
            .require_permission(&TimeTravelPermission::ReadLedgerState)
            .is_ok());
    }

    #[test]
    fn ledger_sequence_cap_enforced() {
        let g = TimeTravelGuard::new("user".into(), vec![Role::Researcher], Some(1000));
        assert!(g.require_ledger_access(999).is_ok());
        assert!(g.require_ledger_access(1001).is_err());
    }

    #[test]
    fn unrestricted_admin_any_sequence() {
        let g = TimeTravelGuard::new("admin".into(), vec![Role::Admin], None);
        assert!(g.require_ledger_access(u32::MAX).is_ok());
    }
}
