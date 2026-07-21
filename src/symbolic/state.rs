//! Phase 2 — Symbolic State Model
//!
//! Defines the symbolic representation of Soroban ledger state,
//! call stack, and path constraints.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A symbolic value (can be concrete or symbolic).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolicValue {
    /// A concrete value.
    Concrete(i128),
    /// A symbolic variable (e.g., function input, storage read).
    Symbolic(String),
    /// A computed expression (e.g., a + b).
    Expression {
        op: String,
        operands: Vec<SymbolicValue>,
    },
}

/// A path constraint (condition that must be true for this path).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConstraint {
    /// The constraint expression.
    pub condition: SymbolicValue,
    /// Source location where this branch was taken.
    pub source_loc: Option<String>,
}

/// Symbolic representation of a Soroban ledger entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolicLedgerEntry {
    /// The key (can be symbolic).
    pub key: SymbolicValue,
    /// The value (symbolic variable for fresh reads).
    pub value: SymbolicValue,
}

/// The symbolic state during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolicState {
    /// Symbolic ledger entries (contract storage).
    pub ledger: HashMap<String, SymbolicLedgerEntry>,
    /// Call stack with per-frame local variables.
    pub call_stack: Vec<CallFrame>,
    /// Path constraints accumulated along the current path.
    pub path_constraints: Vec<PathConstraint>,
    /// Whether require_auth has been called.
    pub auth_required: bool,
    /// The authorized caller (if auth_required is true).
    pub authorized_caller: Option<SymbolicValue>,
}

/// A single frame in the symbolic call stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallFrame {
    /// Function name.
    pub function: String,
    /// Local variables (register ID → symbolic value).
    pub locals: HashMap<u32, SymbolicValue>,
    /// Current instruction pointer.
    pub pc: u32,
}

impl SymbolicState {
    /// Create a fresh symbolic state.
    pub fn new() -> Self {
        Self {
            ledger: HashMap::new(),
            call_stack: Vec::new(),
            path_constraints: Vec::new(),
            auth_required: false,
            authorized_caller: None,
        }
    }

    /// Read from contract storage (produces a fresh symbolic variable).
    pub fn read_storage(&mut self, key: &str) -> SymbolicValue {
        if let Some(entry) = self.ledger.get(key) {
            entry.value.clone()
        } else {
            // Fresh symbolic variable for unknown storage
            let var = SymbolicValue::Symbolic(format!("storage_{}", key));
            self.ledger.insert(
                key.to_string(),
                SymbolicLedgerEntry {
                    key: SymbolicValue::Concrete(0),
                    value: var.clone(),
                },
            );
            var
        }
    }

    /// Write to contract storage.
    pub fn write_storage(&mut self, key: &str, value: SymbolicValue) {
        self.ledger.insert(
            key.to_string(),
            SymbolicLedgerEntry {
                key: SymbolicValue::Concrete(0),
                value,
            },
        );
    }

    /// Add a path constraint.
    pub fn add_constraint(&mut self, constraint: PathConstraint) {
        self.path_constraints.push(constraint);
    }

    /// Check if require_auth has been called.
    pub fn is_authenticated(&self) -> bool {
        self.auth_required
    }

    /// Mark that require_auth was called.
    pub fn require_auth(&mut self, caller: SymbolicValue) {
        self.auth_required = true;
        self.authorized_caller = Some(caller);
    }
}

impl Default for SymbolicState {
    fn default() -> Self {
        Self::new()
    }
}
