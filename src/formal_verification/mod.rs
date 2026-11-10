/* Authorized Protocol Quality Assurance & Formal Verification Test Suite */

//! SMT-Based Formal Verification Engine for Soroban Smart Contract Invariants.
//!
//! Implements formal mathematical proofs of smart contract invariants using Z3 (Issue #445).
//! Transforms contract AST and specification constraints into SMT-LIB2 bitvector formulas,
//! proves validity (UNSAT), or generates concrete counterexample states (SAT).

pub mod counterexample;
pub mod host_environment;
pub mod loops;
pub mod modular;
pub mod smt_lib;
pub mod spec_parser;
pub mod z3_solver;

#[cfg(test)]
pub mod benchmarks;

pub use counterexample::Counterexample;
pub use host_environment::HostEnvironmentTheory;
pub use loops::{LoopAnalyzer, LoopVerificationConfig};
pub use modular::FunctionSummary;
pub use smt_lib::{SmtExpr, SmtScriptBuilder, SmtSort};
pub use spec_parser::InvariantSpec;
pub use z3_solver::{SolverResult, Z3Solver};

use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationStatus {
    Verified,
    Violated(Counterexample),
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantVerificationResult {
    pub invariant_name: String,
    pub target_function: String,
    pub status: VerificationStatus,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormalVerificationReport {
    pub contract_name: String,
    pub solver_version: String,
    pub total_invariants: usize,
    pub verified_count: usize,
    pub violated_count: usize,
    pub unknown_count: usize,
    pub total_duration_ms: u64,
    pub results: Vec<InvariantVerificationResult>,
}

pub struct FormalVerifier {
    solver: Z3Solver,
    host_theory: HostEnvironmentTheory,
}

impl Default for FormalVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl FormalVerifier {
    pub fn new() -> Self {
        Self {
            solver: Z3Solver::new(),
            host_theory: HostEnvironmentTheory::default(),
        }
    }

    pub fn with_solver(solver: Z3Solver) -> Self {
        Self {
            solver,
            host_theory: HostEnvironmentTheory::default(),
        }
    }

    pub fn host_theory(&self) -> &HostEnvironmentTheory {
        &self.host_theory
    }

    /// Verifies a single invariant specification against the contract logic.
    pub fn verify_invariant(&self, spec: &InvariantSpec) -> InvariantVerificationResult {
        let start = Instant::now();
        let mut builder = SmtScriptBuilder::new("QF_BV");

        for (var_name, sort) in &spec.variables {
            builder.declare_const(var_name, sort.clone());
        }

        // Negate assertion: Counterexample = Invariant Violation (SAT = Violated, UNSAT = Verified)
        let negated_property = SmtExpr::not(spec.assertion.clone());
        builder.assert(negated_property);

        let script = builder.build();
        let solver_res = self.solver.solve(&script, &spec.target_function, &spec.name);

        let status = match solver_res {
            SolverResult::Unsat => VerificationStatus::Verified,
            SolverResult::Sat(ce) => VerificationStatus::Violated(ce),
            SolverResult::Unknown(reason) => VerificationStatus::Unknown(reason),
        };

        InvariantVerificationResult {
            invariant_name: spec.name.clone(),
            target_function: spec.target_function.clone(),
            status,
            execution_time_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// Verifies all invariant specifications for a contract and builds the formal report.
    pub fn verify_contract(
        &self,
        contract_name: &str,
        invariants: &[InvariantSpec],
    ) -> FormalVerificationReport {
        let start = Instant::now();
        let mut results = Vec::new();
        let mut verified_count = 0;
        let mut violated_count = 0;
        let mut unknown_count = 0;

        for inv in invariants {
            let res = self.verify_invariant(inv);
            match &res.status {
                VerificationStatus::Verified => verified_count += 1,
                VerificationStatus::Violated(_) => violated_count += 1,
                VerificationStatus::Unknown(_) => unknown_count += 1,
            }
            results.push(res);
        }

        FormalVerificationReport {
            contract_name: contract_name.to_string(),
            solver_version: self.solver.solver_version(),
            total_invariants: invariants.len(),
            verified_count,
            violated_count,
            unknown_count,
            total_duration_ms: start.elapsed().as_millis() as u64,
            results,
        }
    }
}
