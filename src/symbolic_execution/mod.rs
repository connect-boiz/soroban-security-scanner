// Copyright 2026 soroban-security-scanner Contributors
// SPDX-License-Identifier: MIT

//! # Symbolic Execution Engine
//!
//! Issue #442: Build a Symbolic Execution Engine for Path-Aware Vulnerability Detection
//!
//! This module implements a symbolic execution engine that represents contract
//! state and function inputs as symbolic variables, explores all feasible
//! execution paths by solving path constraints with an SMT solver, and checks
//! vulnerability conditions at each path endpoint.
//!
//! ## Architecture
//!
//! - [`ir`] — Three-address-code IR with explicit control flow graphs (Phase 1)
//! - [`symbolic_state`] — Symbolic state model (Phase 2)
//! - [`stellar_semantics`] — Stellar-specific operation modeling (Phase 3)
//! - [`solver`] — Constraint solver integration (Phase 4)
//! - [`checkers`] — Vulnerability checkers as path conditions (Phase 5)
//! - [`engine`] — Path exploration strategy and concolic mode (Phases 6-7)

pub mod ir;
pub mod symbolic_state;
pub mod stellar_semantics;
pub mod solver;
pub mod checkers;
pub mod engine;

pub use engine::{SymbolicEngine, SymbolicEngineConfig, ExplorationStrategy};
pub use ir::{IRInstruction, IRProgram, BasicBlock, ControlFlowGraph};
pub use symbolic_state::{SymbolicState, PathConstraint, SymbolicValue};
pub use checkers::{VulnerabilityChecker, VulnerabilityReport, ReentrancyChecker, IntegerOverflowChecker, AccessControlChecker};
pub use solver::{ConstraintSolver, SolverResult};

/// Configuration for the symbolic execution engine.
#[derive(Debug, Clone)]
pub struct SymbolicScanOptions {
    /// Maximum exploration depth in IR instructions (default 1000).
    pub max_depth: usize,
    /// Maximum number of paths to explore (default 10,000).
    pub max_paths: usize,
    /// Timeout in seconds (default 300).
    pub timeout_secs: u64,
    /// Exploration strategy.
    pub strategy: ExplorationStrategy,
    /// Enable concolic mode (hybrid concrete + symbolic).
    pub concolic: bool,
}

impl Default for SymbolicScanOptions {
    fn default() -> Self {
        Self {
            max_depth: 1000,
            max_paths: 10_000,
            timeout_secs: 300,
            strategy: ExplorationStrategy::CoverageGuided,
            concolic: false,
        }
    }
}
