//! Input validation and sandboxing for the smart-contract analysis engine
//! (issue #340).
//!
//! A self-contained safety layer that sits in front of the analyzer: it
//! validates and sanitizes contract input, enforces an AST schema, runs the
//! analysis in a crash-contained, resource- and timeout-bounded sandbox,
//! validates result consistency, schedules jobs with priority/resource limits,
//! provides a parser-fuzzing harness, and monitors failures and resource
//! exhaustion. A hostile contract can be rejected — never crash or hang the
//! engine.
//!
//! # Acceptance-criteria mapping
//!
//! | Requirement | Where |
//! |-------------|-------|
//! | Input validation (syntax, structure, size) | [`input::validate`] |
//! | AST schema validation | [`ast::validate`] |
//! | Resource limits (CPU/memory/time) | [`limits::ResourceBudget`], [`sandbox::Sandbox`] |
//! | Timeout (5-minute maximum) | [`limits::ResourceBudget::max_wall_ms`] |
//! | Sandboxed execution | [`sandbox::Sandbox`] (crash-contained) |
//! | Parser fuzzing | [`fuzz::fuzz_parser`] |
//! | Input sanitization | [`input::validate`] (BOM / newline normalization) |
//! | Analysis job queue (priority + resources) | [`queue::AnalysisQueue`] |
//! | Result validation & consistency | [`result_check`] |
//! | Failure / resource-exhaustion monitoring | [`monitoring::GuardMonitor`] |
//! | Zero parser crashes | [`sandbox`] containment + [`monitoring::GuardMonitor::is_crash_free`] |
//! | Comprehensive testing | per-module tests + [`tests`] |
//!
//! # Example
//!
//! ```
//! use soroban_security_scanner::analysis_guard::*;
//!
//! let guard = AnalysisGuard::new(GuardLimits::default());
//! // A panicking analyzer is contained, not fatal.
//! let err = guard.analyze(b"pub fn main() {}", |_| panic!("parser bug")).unwrap_err();
//! assert!(matches!(err, GuardError::Sandbox(SandboxError::Crashed)));
//! assert!(!guard.monitor().is_crash_free());
//! ```

pub mod ast;
pub mod engine;
pub mod fuzz;
pub mod input;
pub mod limits;
pub mod monitoring;
pub mod queue;
pub mod result_check;
pub mod sandbox;

#[cfg(test)]
mod tests;

pub use ast::{validate as validate_ast, AstError, AstNode, AstSummary};
pub use engine::{AnalysisGuard, GuardError};
pub use fuzz::{fuzz_parser, FuzzReport};
pub use input::{validate as validate_input, InputError, ValidatedInput};
pub use limits::{AstLimits, GuardLimits, InputLimits, ResourceBudget};
pub use monitoring::{GuardMonitor, GuardStats};
pub use queue::{AdmissionError, AnalysisJob, AnalysisQueue, Priority};
pub use result_check::{
    fingerprint, is_consistent, AnalysisResult, Finding, ResultError, ALLOWED_SEVERITIES,
};
pub use sandbox::{ResourceUsage, Sandbox, SandboxError};
