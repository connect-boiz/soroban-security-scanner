/* Authorized Protocol Quality Assurance & Formal Verification Test Suite */

//! Soroban Host Environment SMT Theory.
//!
//! Axiomatizes the Soroban execution environment:
//! - Persistent & Instance Storage access
//! - Authorization guards (`require_auth`)
//! - Ledger sequence & timestamp monotonicity
//! - Cross-contract call summaries

use crate::formal_verification::smt_lib::{SmtExpr, SmtSort};

#[derive(Debug, Clone)]
pub struct HostEnvironmentTheory {
    pub contract_id: String,
    pub deployment_timestamp: u64,
}

impl Default for HostEnvironmentTheory {
    fn default() -> Self {
        Self::new("C_TARGET_CONTRACT", 1700000000)
    }
}

impl HostEnvironmentTheory {
    pub fn new(contract_id: &str, deployment_timestamp: u64) -> Self {
        Self {
            contract_id: contract_id.to_string(),
            deployment_timestamp,
        }
    }

    /// Axiom for storage access: `storage_get(key)`
    pub fn storage_lookup(&self, key_var: &str) -> SmtExpr {
        SmtExpr::Apply(
            "storage_get".to_string(),
            vec![SmtExpr::var(key_var, SmtSort::BitVec(256))],
            SmtSort::BitVec(1024),
        )
    }

    /// Path constraint enforced by `caller.require_auth()`
    pub fn auth_constraint(&self, caller_var: &str, target_addr_var: &str) -> SmtExpr {
        SmtExpr::eq(
            SmtExpr::var(caller_var, SmtSort::BitVec(256)),
            SmtExpr::var(target_addr_var, SmtSort::BitVec(256)),
        )
    }

    /// Ledger timestamp monotonicity axiom: `timestamp >= deployment_timestamp`
    pub fn timestamp_monotonicity_axiom(&self, timestamp_var: &str) -> SmtExpr {
        SmtExpr::bv_uge(
            SmtExpr::var(timestamp_var, SmtSort::BitVec(64)),
            SmtExpr::bv(self.deployment_timestamp as u128, 64),
        )
    }
}
