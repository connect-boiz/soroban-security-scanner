// SPDX-License-Identifier: MIT
//! Phase 5 — Vulnerability Checkers as Path Conditions.
//!
//! Each checker defines a `check(state, path) -> Option<VulnerabilityReport>`.
//! The engine calls all checkers at each path endpoint and at registered
//! interest points (e.g., after every external call for reentrancy).

use crate::symbolic_execution::symbolic_state::{SymbolicState, SymbolicValue, PathConstraint};
use crate::symbolic_execution::ir::IROp;
use crate::vulnerabilities::VulnerabilityType;

/// A vulnerability report from a symbolic execution checker.
#[derive(Debug, Clone)]
pub struct VulnerabilityReport {
    pub vulnerability_type: VulnerabilityType,
    pub description: String,
    pub function: String,
    pub source_file: Option<String>,
    pub source_line: Option<u32>,
    pub path_constraints: Vec<String>,
    pub detection_method: String, // "symbolic"
    pub severity: String,
}

/// Trait for vulnerability checkers.
pub trait VulnerabilityChecker: Send + Sync {
    /// Check for a vulnerability at the current state and path.
    fn check(&self, state: &SymbolicState, path: &[PathConstraint]) -> Option<VulnerabilityReport>;

    /// Whether this checker should be called after every external/host call.
    fn check_on_host_call(&self) -> bool { false }

    /// Name of the checker.
    fn name(&self) -> &str;
}

/// Reentrancy checker: detects violations of the checks-effects-interactions pattern.
pub struct ReentrancyChecker;

impl VulnerabilityChecker for ReentrancyChecker {
    fn name(&self) -> &str { "reentrancy" }

    fn check_on_host_call(&self) -> bool { true }

    fn check(&self, state: &SymbolicState, _path: &[PathConstraint]) -> Option<VulnerabilityReport> {
        // Check: was a storage write performed BEFORE an external call
        // in the current call frame? If so, a reentrancy attack could
        // see stale state.
        let has_write_before_call = state.storage.values()
            .any(|e| e.written);

        let has_external_call = state.call_stack.len() > 1;

        if has_write_before_call && has_external_call {
            // Check if the external call happens after the write
            // (simplified: if both are present in the same path, flag it)
            return Some(VulnerabilityReport {
                vulnerability_type: VulnerabilityType::Reentrancy,
                description: "State modification detected before external contract call. \
                    A malicious external contract could re-enter and observe stale state."
                    .to_string(),
                function: state.current_function().to_string(),
                source_file: None,
                source_line: None,
                path_constraints: state.path_constraints.iter()
                    .map(|c| format!("{:?}", c.condition))
                    .collect(),
                detection_method: "symbolic".to_string(),
                severity: "high".to_string(),
            });
        }
        None
    }
}

/// Integer overflow checker: detects paths where i128 arithmetic can overflow.
pub struct IntegerOverflowChecker;

impl VulnerabilityChecker for IntegerOverflowChecker {
    fn name(&self) -> &str { "integer_overflow" }

    fn check(&self, state: &SymbolicState, _path: &[PathConstraint]) -> Option<VulnerabilityReport> {
        // Check: are there any BinOp(add/sub/mul) operations on symbolic values
        // where the result could overflow i128?
        for entry in state.storage.values() {
            if let SymbolicValue::BinOp(op, lhs, rhs) = &entry.value {
                if matches!(op.as_str(), "add" | "sub" | "mul") {
                    // If either operand is symbolic, overflow is possible
                    let lhs_sym = matches!(**lhs, SymbolicValue::Symbol(_));
                    let rhs_sym = matches!(**rhs, SymbolicValue::Symbol(_));
                    if lhs_sym || rhs_sym {
                        return Some(VulnerabilityReport {
                            vulnerability_type: VulnerabilityType::IntegerOverflow,
                            description: format!(
                                "Potential i128 overflow in {} operation with symbolic operand. \
                                The operation may overflow for certain input values.",
                                op
                            ),
                            function: state.current_function().to_string(),
                            source_file: None,
                            source_line: None,
                            path_constraints: state.path_constraints.iter()
                                .map(|c| format!("{:?}", c.condition))
                                .collect(),
                            detection_method: "symbolic".to_string(),
                            severity: "medium".to_string(),
                        });
                    }
                }
            }
        }
        None
    }
}

/// Access control checker: detects paths where require_auth is missing.
pub struct AccessControlChecker;

impl VulnerabilityChecker for AccessControlChecker {
    fn name(&self) -> &str { "access_control" }

    fn check(&self, state: &SymbolicState, path: &[PathConstraint]) -> Option<VulnerabilityReport> {
        // Check: are there paths that reach a storage write without
        // passing through a require_auth constraint?
        let has_write = state.storage.values().any(|e| e.written);
        let has_auth = path.iter().any(|c| {
            // Look for auth-related constraints
            matches!(&c.condition, SymbolicValue::Compare(op, _, _)
                if op == "eq")
        });

        if has_write && !has_auth {
            return Some(VulnerabilityReport {
                vulnerability_type: VulnerabilityType::MissingAuth,
                description: "Storage modification detected on a path with no require_auth \
                    constraint. An unauthorized caller could modify contract state."
                    .to_string(),
                function: state.current_function().to_string(),
                source_file: None,
                source_line: None,
                path_constraints: path.iter()
                    .map(|c| format!("{:?}", c.condition))
                    .collect(),
                detection_method: "symbolic".to_string(),
                severity: "critical".to_string(),
            });
        }
        None
    }
}

/// Get all built-in vulnerability checkers.
pub fn default_checkers() -> Vec<Box<dyn VulnerabilityChecker>> {
    vec![
        Box::new(ReentrancyChecker),
        Box::new(IntegerOverflowChecker),
        Box::new(AccessControlChecker),
    ]
}
