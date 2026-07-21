//! Symbolic Execution Engine for Soroban Smart Contract Vulnerability Detection
//!
//! This module implements path-aware vulnerability detection using symbolic
//! execution — the same technique used by Mythril (Ethereum) and KEVM,
//! adapted for Soroban's WASM-based execution model and Stellar's
//! account/trustline architecture.
//!
//! ## Architecture
//!
//! The engine works in phases:
//! 1. IR Lifting — WASM bytecode → three-address-code IR with CFG
//! 2. Symbolic State — symbolic ledger, call stack, path constraints
//! 3. Stellar Semantics — model Stellar-specific operations as constraints
//! 4. Constraint Solver — Z3 SMT solver for 128-bit arithmetic
//! 5. Vulnerability Checkers — path-condition-based bug detection
//! 6. Path Exploration — BFS/DFS/coverage-guided search strategies
//! 7. Concolic Mode — hybrid concrete + symbolic execution
//! 8. Benchmark Suite — known vulnerabilities for validation
//! 9. CLI Integration — `stellar-scanner scan --symbolic`

pub mod ir;
pub mod state;
pub mod checkers;
pub mod explorer;

pub use ir::{IrInstruction, ControlFlowGraph, BasicBlock};
pub use state::{SymbolicState, PathConstraint, SymbolicValue};
pub use checkers::{VulnerabilityChecker, VulnerabilityReport, VulnerabilityType};
pub use explorer::{PathExplorer, SearchStrategy, ExplorationConfig};
