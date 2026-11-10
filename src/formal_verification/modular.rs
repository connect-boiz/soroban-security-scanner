/* Authorized Protocol Quality Assurance & Formal Verification Test Suite */

//! Modular Verification and Function Summaries.
//!
//! Decomposes verification into independent function-level proofs using
//! pre- and post-condition summaries composed bottom-up on the call graph.

use crate::formal_verification::smt_lib::SmtExpr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSummary {
    pub function_name: String,
    pub preconditions: Vec<SmtExpr>,
    pub postconditions: Vec<SmtExpr>,
    pub modified_storage_keys: Vec<String>,
}

impl FunctionSummary {
    pub fn new(function_name: &str) -> Self {
        Self {
            function_name: function_name.to_string(),
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            modified_storage_keys: Vec::new(),
        }
    }

    pub fn with_precondition(mut self, pre: SmtExpr) -> Self {
        self.preconditions.push(pre);
        self
    }

    pub fn with_postcondition(mut self, post: SmtExpr) -> Self {
        self.postconditions.push(post);
        self
    }

    pub fn with_modified_key(mut self, key: &str) -> Self {
        self.modified_storage_keys.push(key.to_string());
        self
    }

    /// Composes the function summary into an SMT implication:
    /// `preconditions => postconditions`
    pub fn as_smt_implication(&self) -> SmtExpr {
        let pre_conj = SmtExpr::and(self.preconditions.clone());
        let post_conj = SmtExpr::and(self.postconditions.clone());
        SmtExpr::Or(vec![SmtExpr::not(pre_conj), post_conj])
    }
}
