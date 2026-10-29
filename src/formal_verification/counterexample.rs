/* Authorized Protocol Quality Assurance & Formal Verification Test Suite */

//! Counterexample Synthesis and Explanatory Trace Generation.
//!
//! When Z3 finds an invariant violation (SAT), maps symbolic assignments into
//! concrete contract transaction parameters for reproducer tests.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counterexample {
    pub target_function: String,
    pub violated_property: String,
    pub variable_assignments: HashMap<String, String>,
    pub execution_trace: Vec<String>,
}

impl Counterexample {
    pub fn new(
        target_function: &str,
        violated_property: &str,
        variable_assignments: HashMap<String, String>,
        execution_trace: Vec<String>,
    ) -> Self {
        Self {
            target_function: target_function.to_string(),
            violated_property: violated_property.to_string(),
            variable_assignments,
            execution_trace,
        }
    }

    /// Formats a concrete reproduction transaction description for developers.
    pub fn format_reproduction_diagnostic(&self) -> String {
        let mut s = format!(
            "Invariant Violation Witness for '{}' in function '{}':\n",
            self.violated_property, self.target_function
        );
        s.push_str("Concrete Input Assignments:\n");
        for (var, val) in &self.variable_assignments {
            s.push_str(&format!("  * {} = {}\n", var, val));
        }
        s.push_str("Trace Steps:\n");
        for step in &self.execution_trace {
            s.push_str(&format!("  -> {}\n", step));
        }
        s
    }
}
