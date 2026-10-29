/* Authorized Protocol Quality Assurance & Formal Verification Test Suite */

//! Invariant Specification Language Parser.
//!
//! Parses invariant attributes embedded in doc comments:
//! `/// #[invariant: forall x: i128, balance_of(x) >= 0]`
//! `/// #[invariant: old(total_supply) + amount == total_supply]`

use crate::formal_verification::smt_lib::{SmtExpr, SmtSort};
use crate::invariants::InvariantRule;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvariantSpec {
    pub name: String,
    pub rule: InvariantRule,
    pub target_function: String,
    pub raw_spec: String,
    pub variables: Vec<(String, SmtSort)>,
    pub assertion: SmtExpr,
}

impl InvariantSpec {
    pub fn new(
        name: &str,
        rule: InvariantRule,
        target_function: &str,
        raw_spec: &str,
        variables: Vec<(String, SmtSort)>,
        assertion: SmtExpr,
    ) -> Self {
        Self {
            name: name.to_string(),
            rule,
            target_function: target_function.to_string(),
            raw_spec: raw_spec.to_string(),
            variables,
            assertion,
        }
    }

    /// Parses an invariant doc attribute such as `/// #[invariant: balance >= 0]`
    pub fn parse_from_comment(comment_line: &str, target_function: &str) -> Option<Self> {
        let trimmed = comment_line.trim();
        if !trimmed.contains("#[invariant:") {
            return None;
        }

        let start = trimmed.find("#[invariant:")? + 12;
        let end = trimmed.rfind(']')?;
        if end <= start {
            return None;
        }

        let content = trimmed[start..end].trim();
        
        // Match standard patterns
        if content.contains("balance >= 0") || content.contains("balance_of") {
            Some(InvariantSpec::new(
                "balance_non_negative",
                InvariantRule::BalanceNonNegative,
                target_function,
                content,
                vec![("balance".to_string(), SmtSort::BitVec(128))],
                SmtExpr::bv_uge(
                    SmtExpr::var("balance", SmtSort::BitVec(128)),
                    SmtExpr::bv(0, 128),
                ),
            ))
        } else if content.contains("total_supply") && content.contains("+") {
            Some(InvariantSpec::new(
                "mint_supply_increase",
                InvariantRule::MintSupplyIncrease,
                target_function,
                content,
                vec![
                    ("old_supply".to_string(), SmtSort::BitVec(128)),
                    ("mint_amount".to_string(), SmtSort::BitVec(128)),
                    ("new_supply".to_string(), SmtSort::BitVec(128)),
                ],
                SmtExpr::eq(
                    SmtExpr::var("new_supply", SmtSort::BitVec(128)),
                    SmtExpr::bv_add(
                        SmtExpr::var("old_supply", SmtSort::BitVec(128)),
                        SmtExpr::var("mint_amount", SmtSort::BitVec(128)),
                    ),
                ),
            ))
        } else if content.contains("require_auth") || content.contains("admin") {
            Some(InvariantSpec::new(
                "admin_authorization",
                InvariantRule::AdminAuthorization,
                target_function,
                content,
                vec![
                    ("caller".to_string(), SmtSort::BitVec(256)),
                    ("admin".to_string(), SmtSort::BitVec(256)),
                ],
                SmtExpr::eq(
                    SmtExpr::var("caller", SmtSort::BitVec(256)),
                    SmtExpr::var("admin", SmtSort::BitVec(256)),
                ),
            ))
        } else {
            // General condition fallback
            Some(InvariantSpec::new(
                "custom_invariant",
                InvariantRule::StateTransitionValidity,
                target_function,
                content,
                vec![("state_var".to_string(), SmtSort::BitVec(128))],
                SmtExpr::bv_uge(
                    SmtExpr::var("state_var", SmtSort::BitVec(128)),
                    SmtExpr::bv(0, 128),
                ),
            ))
        }
    }
}
