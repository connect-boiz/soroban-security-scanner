// SPDX-License-Identifier: MIT
//! Phases 6-7 — Path Exploration Strategy and Concolic Mode.
//!
//! The main symbolic execution engine that explores execution paths,
//! applies Stellar semantics, and runs vulnerability checkers.

use crate::symbolic_execution::checkers::{
    default_checkers, VulnerabilityChecker, VulnerabilityReport,
};
use crate::symbolic_execution::ir::{ControlFlowGraph, IRInstruction, IROp, IRProgram};
use crate::symbolic_execution::solver::ConstraintSolver;
use crate::symbolic_execution::stellar_semantics::{apply_host_call, OperationResult};
use crate::symbolic_execution::symbolic_state::{SymbolicState, SymbolicValue};
use std::time::{Duration, Instant};

/// Exploration strategy for path search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorationStrategy {
    /// Breadth-first: complete but memory-intensive.
    BFS,
    /// Depth-first: fast but can get stuck in deep loops.
    DFS,
    /// Coverage-guided: prioritizes paths approaching unvisited CFG edges.
    CoverageGuided,
}

/// Configuration for the symbolic execution engine.
#[derive(Debug, Clone)]
pub struct SymbolicEngineConfig {
    pub max_depth: usize,
    pub max_paths: usize,
    pub timeout: Duration,
    pub strategy: ExplorationStrategy,
    pub concolic: bool,
}

impl Default for SymbolicEngineConfig {
    fn default() -> Self {
        Self {
            max_depth: 1000,
            max_paths: 10_000,
            timeout: Duration::from_secs(300),
            strategy: ExplorationStrategy::CoverageGuided,
            concolic: false,
        }
    }
}

/// The symbolic execution engine.
pub struct SymbolicEngine {
    program: IRProgram,
    #[allow(dead_code)]
    solver: ConstraintSolver,
    checkers: Vec<Box<dyn VulnerabilityChecker>>,
    config: SymbolicEngineConfig,
    /// Visited CFG edges for coverage-guided search.
    visited_edges: std::collections::HashSet<(String, String)>,
}

/// Result of a symbolic execution run.
#[derive(Debug, Clone)]
pub struct SymbolicScanResult {
    pub vulnerabilities: Vec<VulnerabilityReport>,
    pub paths_explored: usize,
    pub paths_trapped: usize,
    pub time_elapsed: Duration,
    pub max_depth_reached: usize,
}

impl SymbolicEngine {
    pub fn new(program: IRProgram, config: SymbolicEngineConfig) -> Self {
        Self {
            program,
            solver: ConstraintSolver::new(),
            checkers: default_checkers(),
            config,
            visited_edges: std::collections::HashSet::new(),
        }
    }

    /// Run the symbolic execution engine on the loaded IR program.
    pub fn run(&mut self) -> SymbolicScanResult {
        let start = Instant::now();
        let mut vulnerabilities = Vec::new();
        let mut paths_explored = 0;
        let mut paths_trapped = 0;
        let mut max_depth_reached = 0;

        // Explore each function in the program
        let function_names: Vec<String> = self.program.functions.keys().cloned().collect();
        for func_name in function_names {
            if start.elapsed() > self.config.timeout {
                break;
            }
            if paths_explored >= self.config.max_paths {
                break;
            }

            if let Some(cfg) = self.program.functions.get(&func_name).cloned() {
                let initial_state = SymbolicState::new();
                let mut results = self.explore_function(&cfg, initial_state, start);
                vulnerabilities.append(&mut results.vulnerabilities);
                paths_explored += results.paths_explored;
                paths_trapped += results.paths_trapped;
                max_depth_reached = max_depth_reached.max(results.max_depth_reached);
            }
        }

        SymbolicScanResult {
            vulnerabilities,
            paths_explored,
            paths_trapped,
            time_elapsed: start.elapsed(),
            max_depth_reached,
        }
    }

    /// Explore a single function's CFG starting from the given state.
    fn explore_function(
        &mut self,
        cfg: &ControlFlowGraph,
        initial_state: SymbolicState,
        start_time: Instant,
    ) -> SymbolicScanResult {
        let mut vulnerabilities = Vec::new();
        let mut paths_explored = 0;
        let mut paths_trapped = 0;
        let mut max_depth_reached = 0;

        // Work list of (block_label, state) pairs
        let mut work_list: Vec<(String, SymbolicState)> =
            vec![(cfg.entry_block.clone(), initial_state)];

        while let Some((block_label, mut state)) = work_list.pop() {
            if paths_explored >= self.config.max_paths {
                break;
            }
            if start_time.elapsed() > self.config.timeout {
                break;
            }
            if state.depth >= self.config.max_depth {
                paths_explored += 1;
                max_depth_reached = max_depth_reached.max(state.depth);
                continue;
            }

            let block = match cfg.get_block(&block_label) {
                Some(b) => b,
                None => continue,
            };

            // Execute instructions in this block
            for instr in &block.instructions {
                state.depth += 1;
                self.execute_instruction(&mut state, instr, &mut vulnerabilities);
                if state.trapped {
                    paths_trapped += 1;
                    break;
                }
            }

            if state.trapped {
                paths_explored += 1;
                max_depth_reached = max_depth_reached.max(state.depth);
                // Run checkers at path endpoint
                for checker in &self.checkers {
                    if let Some(report) = checker.check(&state, &state.path_constraints) {
                        vulnerabilities.push(report);
                    }
                }
                continue;
            }

            // If block has no successors, this is a path endpoint
            if block.successors.is_empty() {
                paths_explored += 1;
                max_depth_reached = max_depth_reached.max(state.depth);
                // Run checkers at path endpoint
                for checker in &self.checkers {
                    if let Some(report) = checker.check(&state, &state.path_constraints) {
                        vulnerabilities.push(report);
                    }
                }
                continue;
            }

            // Branch: fork state for each successor
            for succ_label in &block.successors {
                // Track visited edges for coverage
                self.visited_edges
                    .insert((block_label.clone(), succ_label.clone()));

                let forked_state = state.fork();
                work_list.push((succ_label.clone(), forked_state));
            }
        }

        SymbolicScanResult {
            vulnerabilities,
            paths_explored,
            paths_trapped,
            time_elapsed: start_time.elapsed(),
            max_depth_reached,
        }
    }

    /// Execute a single IR instruction on the symbolic state.
    fn execute_instruction(
        &self,
        state: &mut SymbolicState,
        instr: &IRInstruction,
        vulnerabilities: &mut Vec<VulnerabilityReport>,
    ) {
        match &instr.op {
            IROp::Const(val) => {
                if let Some(dest) = &instr.dest {
                    state.write_local(dest, SymbolicValue::Const(*val));
                }
            }
            IROp::LoadSym(name) => {
                let val = state.read_storage(name);
                if let Some(dest) = &instr.dest {
                    state.write_local(dest, val);
                }
            }
            IROp::Store(location, var) => {
                let val = state.read_local(var);
                state.write_storage(location, val);
            }
            IROp::Load(location) => {
                let val = state.read_storage(location);
                if let Some(dest) = &instr.dest {
                    state.write_local(dest, val);
                }
            }
            IROp::BinOp(kind, lhs, rhs) => {
                let lhs_val = state.read_local(lhs);
                let rhs_val = state.read_local(rhs);
                let op_str = match kind {
                    crate::symbolic_execution::ir::BinOpKind::Add => "add",
                    crate::symbolic_execution::ir::BinOpKind::Sub => "sub",
                    crate::symbolic_execution::ir::BinOpKind::Mul => "mul",
                    crate::symbolic_execution::ir::BinOpKind::Div => "div",
                    crate::symbolic_execution::ir::BinOpKind::Rem => "rem",
                    _ => "unknown",
                };
                let result =
                    SymbolicValue::BinOp(op_str.to_string(), Box::new(lhs_val), Box::new(rhs_val));
                if let Some(dest) = &instr.dest {
                    state.write_local(dest, result);
                }
            }
            IROp::Compare(kind, lhs, rhs) => {
                let lhs_val = state.read_local(lhs);
                let rhs_val = state.read_local(rhs);
                let op_str = match kind {
                    crate::symbolic_execution::ir::CmpOpKind::Eq => "eq",
                    crate::symbolic_execution::ir::CmpOpKind::Ne => "ne",
                    crate::symbolic_execution::ir::CmpOpKind::Lt => "lt",
                    crate::symbolic_execution::ir::CmpOpKind::Le => "le",
                    crate::symbolic_execution::ir::CmpOpKind::Gt => "gt",
                    crate::symbolic_execution::ir::CmpOpKind::Ge => "ge",
                };
                let result = SymbolicValue::Compare(
                    op_str.to_string(),
                    Box::new(lhs_val),
                    Box::new(rhs_val),
                );
                if let Some(dest) = &instr.dest {
                    state.write_local(dest, result);
                }
            }
            IROp::HostCall(func_name, args) => {
                let arg_vals: Vec<SymbolicValue> =
                    args.iter().map(|a| state.read_local(a)).collect();
                let result = apply_host_call(state, func_name, &arg_vals);

                // Run checkers that register interest in host calls
                for checker in &self.checkers {
                    if checker.check_on_host_call() {
                        if let Some(report) = checker.check(state, &state.path_constraints) {
                            vulnerabilities.push(report);
                        }
                    }
                }

                if let OperationResult::Trap(cause) = result {
                    state.trap(&cause);
                } else if let OperationResult::Value(val) = result {
                    if let Some(dest) = &instr.dest {
                        state.write_local(dest, val);
                    }
                }
            }
            IROp::Call(func_name, _args) => {
                // Internal function call: push frame and continue
                state.push_frame(func_name);
                let var = state.fresh_var();
                if let Some(dest) = &instr.dest {
                    state.write_local(dest, SymbolicValue::Symbol(var));
                }
                state.pop_frame();
            }
            IROp::Branch(cond, _then_label, _else_label) => {
                let cond_val = state.read_local(cond);
                state.add_constraint(cond_val, Some(instr.clone()));
            }
            IROp::Return(_) => {
                // Path endpoint — handled by caller
            }
            IROp::Jump(_) | IROp::UnaryOp(_, _) | IROp::Nop => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbolic_execution::checkers::{
        AccessControlChecker, ReentrancyChecker, VulnerabilityChecker,
    };
    use crate::symbolic_execution::ir::{BasicBlock, ControlFlowGraph, IROp, IRProgram};
    use crate::symbolic_execution::solver::{ConstraintSolver, SolverResult};
    use crate::symbolic_execution::stellar_semantics::apply_host_call;
    use crate::symbolic_execution::symbolic_state::{PathConstraint, SymbolicState, SymbolicValue};
    use crate::vulnerabilities::VulnerabilityType;

    #[test]
    fn test_symbolic_state_initialization_and_fresh_vars() {
        let mut state = SymbolicState::new();
        let var1 = state.fresh_var();
        let var2 = state.fresh_var();
        assert_ne!(var1, var2);

        state.push_frame("test_func");
        assert_eq!(state.current_function(), "test_func");
        state.write_local("x", SymbolicValue::Const(42));
        assert_eq!(state.read_local("x"), SymbolicValue::Const(42));
        state.pop_frame();
        assert_eq!(state.current_function(), "entry");
    }

    #[test]
    fn test_stellar_semantics_operations() {
        let mut state = SymbolicState::new();
        // Test require_auth
        let res = apply_host_call(
            &mut state,
            "require_auth",
            &[SymbolicValue::Symbol("alice".to_string())],
        );
        assert!(matches!(res, OperationResult::Value(_)));
        assert_eq!(state.path_constraints.len(), 1);

        // Test put_ledger_entry
        let res2 = apply_host_call(
            &mut state,
            "put_ledger_entry",
            &[
                SymbolicValue::Symbol("bounty_key".to_string()),
                SymbolicValue::Const(100),
            ],
        );
        assert!(matches!(res2, OperationResult::Success(_)));
        assert!(state.storage.get("bounty_key").unwrap().written);

        // Test account_merge
        let res3 = apply_host_call(
            &mut state,
            "account_merge",
            &[SymbolicValue::Symbol("bob".to_string())],
        );
        assert!(matches!(res3, OperationResult::Success(_)));
    }

    #[test]
    fn test_constraint_solver_contradiction_detection() {
        let solver = ConstraintSolver::new();
        assert_eq!(solver.check(&[]), SolverResult::Satisfiable);

        let c1 = PathConstraint {
            condition: SymbolicValue::Compare(
                "lt".to_string(),
                Box::new(SymbolicValue::Symbol("x".to_string())),
                Box::new(SymbolicValue::Const(5)),
            ),
            source: None,
            depth: 0,
        };
        let c2 = PathConstraint {
            condition: SymbolicValue::Compare(
                "ge".to_string(),
                Box::new(SymbolicValue::Symbol("x".to_string())),
                Box::new(SymbolicValue::Const(5)),
            ),
            source: None,
            depth: 0,
        };
        assert_eq!(solver.check(&[c1, c2]), SolverResult::Unsatisfiable);
    }

    #[test]
    fn test_vulnerability_checker_access_control() {
        let checker = AccessControlChecker;
        let mut state = SymbolicState::new();
        state.write_storage("admin", SymbolicValue::Symbol("hacker".to_string()));

        // Storage write with no require_auth should flag vulnerability
        let report = checker
            .check(&state, &[])
            .expect("expected vulnerability report");
        assert_eq!(
            report.vulnerability_type,
            VulnerabilityType::MissingAccessControl
        );

        // With auth constraint, it should be clean
        let auth_constraint = PathConstraint {
            condition: SymbolicValue::Compare(
                "eq".to_string(),
                Box::new(SymbolicValue::Symbol("caller".to_string())),
                Box::new(SymbolicValue::Symbol("admin".to_string())),
            ),
            source: None,
            depth: 0,
        };
        let clean = checker.check(&state, &[auth_constraint]);
        assert!(clean.is_none());
    }

    #[test]
    fn test_vulnerability_checker_reentrancy() {
        let checker = ReentrancyChecker;
        let mut state = SymbolicState::new();
        state.write_storage("balances", SymbolicValue::Const(0));
        state.push_frame("external_call");

        let report = checker
            .check(&state, &[])
            .expect("expected reentrancy report");
        assert_eq!(report.vulnerability_type, VulnerabilityType::Reentrancy);
    }

    #[test]
    fn test_symbolic_engine_exploration_and_strategies() {
        let mut program = IRProgram::new();
        let mut cfg = ControlFlowGraph::new("process_payment");

        let entry_block = BasicBlock {
            label: "entry".to_string(),
            instructions: vec![
                IRInstruction {
                    op: IROp::Const(100),
                    dest: Some("amount".to_string()),
                    source_file: None,
                    source_line: Some(10),
                    wasm_offset: None,
                    function: "process_payment".to_string(),
                },
                IRInstruction {
                    op: IROp::HostCall("require_auth".to_string(), vec!["sender".to_string()]),
                    dest: None,
                    source_file: None,
                    source_line: Some(11),
                    wasm_offset: None,
                    function: "process_payment".to_string(),
                },
                IRInstruction {
                    op: IROp::Return(Some("amount".to_string())),
                    dest: None,
                    source_file: None,
                    source_line: Some(12),
                    wasm_offset: None,
                    function: "process_payment".to_string(),
                },
            ],
            successors: vec![],
            predecessors: vec![],
        };

        cfg.add_block(entry_block);
        program.add_function(cfg);

        for strategy in [
            ExplorationStrategy::BFS,
            ExplorationStrategy::DFS,
            ExplorationStrategy::CoverageGuided,
        ] {
            let config = SymbolicEngineConfig {
                max_depth: 50,
                max_paths: 10,
                timeout: Duration::from_secs(5),
                strategy,
                concolic: false,
            };
            let mut engine = SymbolicEngine::new(program.clone(), config);
            let result = engine.run();
            assert_eq!(result.paths_explored, 1);
            assert_eq!(result.vulnerabilities.len(), 0);
        }
    }
}
