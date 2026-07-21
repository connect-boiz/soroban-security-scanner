//! Phase 5 — Vulnerability Checkers as Path Conditions
//!
//! Each checker defines a check that examines the symbolic state
//! at path endpoints and reports vulnerabilities.

use serde::{Deserialize, Serialize};
use super::state::{SymbolicState, PathConstraint};

/// The type of vulnerability found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VulnerabilityType {
    /// Reentrancy vulnerability.
    Reentrancy,
    /// Integer overflow/underflow.
    IntegerOverflow,
    /// Access control bypass.
    AccessControlBypass,
    /// Invariant violation.
    InvariantViolation,
    /// Missing require_auth.
    MissingAuth,
}

/// A vulnerability report from the symbolic engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityReport {
    /// Type of vulnerability.
    pub vuln_type: VulnerabilityType,
    /// Function where the vulnerability was found.
    pub function: String,
    /// Description of the vulnerability.
    pub description: String,
    /// Path constraints that lead to the vulnerability.
    pub path_constraints: Vec<String>,
    /// Detection method (always "symbolic" for this engine).
    pub detection_method: String,
    /// Severity (low, medium, high, critical).
    pub severity: String,
}

/// Trait for vulnerability checkers.
pub trait VulnerabilityChecker: Send + Sync {
    /// Check the symbolic state for a vulnerability.
    fn check(&self, state: &SymbolicState, path: &[PathConstraint]) -> Option<VulnerabilityReport>;

    /// Name of this checker.
    fn name(&self) -> &str;
}

/// Checker for missing require_auth calls.
pub struct MissingAuthChecker;

impl VulnerabilityChecker for MissingAuthChecker {
    fn check(&self, state: &SymbolicState, path: &[PathConstraint]) -> Option<VulnerabilityReport> {
        // If the function modifies state but doesn't call require_auth
        if !state.is_authenticated() && !state.ledger.is_empty() {
            Some(VulnerabilityReport {
                vuln_type: VulnerabilityType::MissingAuth,
                function: "unknown".to_string(),
                description: "Function modifies contract state without calling require_auth()".to_string(),
                path_constraints: path.iter().map(|p| format!("{:?}", p.condition)).collect(),
                detection_method: "symbolic".to_string(),
                severity: "high".to_string(),
            })
        } else {
            None
        }
    }

    fn name(&self) -> &str {
        "MissingAuthChecker"
    }
}

/// Checker for integer overflow vulnerabilities.
pub struct IntegerOverflowChecker;

impl VulnerabilityChecker for IntegerOverflowChecker {
    fn check(&self, _state: &SymbolicState, _path: &[PathConstraint]) -> Option<VulnerabilityReport> {
        // TODO: Check for i128 arithmetic operations that could overflow
        // This requires integration with the Z3 SMT solver
        None
    }

    fn name(&self) -> &str {
        "IntegerOverflowChecker"
    }
}

/// Checker for reentrancy vulnerabilities.
pub struct ReentrancyChecker;

impl VulnerabilityChecker for ReentrancyChecker {
    fn check(&self, _state: &SymbolicState, _path: &[PathConstraint]) -> Option<VulnerabilityReport> {
        // TODO: Check for external calls followed by state modifications
        // (checks-effects-interactions pattern violation)
        None
    }

    fn name(&self) -> &str {
        "ReentrancyChecker"
    }
}
