// SPDX-License-Identifier: MIT
//! Phase 2 — Symbolic State Model.
//!
//! Defines the SymbolicState containing symbolic representations of the Soroban
//! ledger, a symbolic call stack, and a path constraint set.

use std::collections::HashMap;
use crate::symbolic_execution::ir::IRInstruction;

/// A symbolic value: either a concrete constant, a symbolic variable, or a
/// computed expression over other symbolic values.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolicValue {
    /// Concrete i128 constant.
    Const(i128),
    /// Fresh symbolic variable (e.g., from storage read or function input).
    Symbol(String),
    /// Binary operation on two symbolic values.
    BinOp(String, Box<SymbolicValue>, Box<SymbolicValue>),
    /// Comparison producing a boolean symbolic value.
    Compare(String, Box<SymbolicValue>, Box<SymbolicValue>),
    /// Result of a host function call (uninterpreted).
    HostCallResult(String, Vec<SymbolicValue>),
    /// Unknown / uninitialized.
    Unknown,
}

/// A single path constraint (SMT assertion).
#[derive(Debug, Clone)]
pub struct PathConstraint {
    /// The constraint expression.
    pub condition: SymbolicValue,
    /// The IR instruction that generated this constraint.
    pub source: Option<IRInstruction>,
    /// Depth at which this constraint was added.
    pub depth: usize,
}

/// Symbolic representation of a Soroban storage entry.
#[derive(Debug, Clone)]
pub struct SymbolicStorageEntry {
    pub key: String,
    pub value: SymbolicValue,
    /// Whether this entry was written (vs. read-only).
    pub written: bool,
}

/// A single frame in the symbolic call stack.
#[derive(Debug, Clone)]
pub struct CallFrame {
    pub function: String,
    pub locals: HashMap<String, SymbolicValue>,
    pub return_var: Option<String>,
}

/// The complete symbolic state at a point during execution.
#[derive(Debug, Clone)]
pub struct SymbolicState {
    /// Symbolic representation of the Soroban ledger.
    /// Maps storage keys to symbolic values.
    pub storage: HashMap<String, SymbolicStorageEntry>,
    /// Account balances (symbolic).
    pub balances: HashMap<String, SymbolicValue>,
    /// Trustline flags (symbolic).
    pub trustlines: HashMap<String, SymbolicValue>,
    /// Current call stack.
    pub call_stack: Vec<CallFrame>,
    /// Accumulated path constraints.
    pub path_constraints: Vec<PathConstraint>,
    /// Fresh variable counter for generating unique symbolic variable names.
    pub fresh_counter: u64,
    /// Current execution depth (instruction count).
    pub depth: usize,
    /// Whether the current path has hit an error/trap.
    pub trapped: bool,
    /// Trap cause if trapped.
    pub trap_cause: Option<String>,
}

impl SymbolicState {
    pub fn new() -> Self {
        Self {
            storage: HashMap::new(),
            balances: HashMap::new(),
            trustlines: HashMap::new(),
            call_stack: vec![CallFrame {
                function: "entry".to_string(),
                locals: HashMap::new(),
                return_var: None,
            }],
            path_constraints: Vec::new(),
            fresh_counter: 0,
            depth: 0,
            trapped: false,
            trap_cause: None,
        }
    }

    /// Generate a fresh symbolic variable name.
    pub fn fresh_var(&mut self) -> String {
        self.fresh_counter += 1;
        format!("sym_{}", self.fresh_counter)
    }

    /// Read from storage: produces a fresh symbolic variable if the key
    /// hasn't been written, or returns the existing symbolic value.
    pub fn read_storage(&mut self, key: &str) -> SymbolicValue {
        if let Some(entry) = self.storage.get(key) {
            if entry.written {
                return entry.value.clone();
            }
        }
        // Fresh symbolic variable for unread storage
        let var = self.fresh_var();
        let value = SymbolicValue::Symbol(var.clone());
        self.storage.insert(key.to_string(), SymbolicStorageEntry {
            key: key.to_string(),
            value: value.clone(),
            written: false,
        });
        value
    }

    /// Write to storage: updates the symbolic state map.
    pub fn write_storage(&mut self, key: &str, value: SymbolicValue) {
        self.storage.insert(key.to_string(), SymbolicStorageEntry {
            key: key.to_string(),
            value,
            written: true,
        });
    }

    /// Read a local variable from the current call frame.
    pub fn read_local(&self, name: &str) -> SymbolicValue {
        if let Some(frame) = self.call_stack.last() {
            if let Some(val) = frame.locals.get(name) {
                return val.clone();
            }
        }
        SymbolicValue::Unknown
    }

    /// Write a local variable in the current call frame.
    pub fn write_local(&mut self, name: &str, value: SymbolicValue) {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.locals.insert(name.to_string(), value);
        }
    }

    /// Add a path constraint.
    pub fn add_constraint(&mut self, condition: SymbolicValue, source: Option<IRInstruction>) {
        self.path_constraints.push(PathConstraint {
            condition,
            source,
            depth: self.depth,
        });
    }

    /// Push a new call frame.
    pub fn push_frame(&mut self, function: &str) {
        self.call_stack.push(CallFrame {
            function: function.to_string(),
            locals: HashMap::new(),
            return_var: None,
        });
    }

    /// Pop the current call frame.
    pub fn pop_frame(&mut self) -> Option<CallFrame> {
        if self.call_stack.len() > 1 {
            self.call_stack.pop()
        } else {
            None
        }
    }

    /// Mark this path as trapped.
    pub fn trap(&mut self, cause: &str) {
        self.trapped = true;
        self.trap_cause = Some(cause.to_string());
    }

    /// Get the current function name.
    pub fn current_function(&self) -> &str {
        &self.call_stack.last().unwrap().function
    }

    /// Clone the state for path branching.
    pub fn fork(&self) -> SymbolicState {
        self.clone()
    }
}

impl Default for SymbolicState {
    fn default() -> Self { Self::new() }
}
