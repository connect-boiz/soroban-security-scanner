/* Authorized Protocol Quality Assurance & Formal Verification Test Suite */

//! SMT-LIB2 Encoding Engine for Soroban Smart Contract Invariants.
//!
//! Maps Soroban types and arithmetic expressions to SMT-LIB2 standard bit-vector
//! (`(_ BitVec 128)`, `(_ BitVec 64)`, `(_ BitVec 32)`) and array logic (QF_ABV).

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SmtSort {
    BitVec(u32),
    Bool,
    Array(Box<SmtSort>, Box<SmtSort>),
}

impl fmt::Display for SmtSort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SmtSort::BitVec(w) => write!(f, "(_ BitVec {})", w),
            SmtSort::Bool => write!(f, "Bool"),
            SmtSort::Array(d, r) => write!(f, "(Array {} {})", d, r),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmtExpr {
    Var(String, SmtSort),
    BvConst(u128, u32),
    BoolConst(bool),
    BvAdd(Box<SmtExpr>, Box<SmtExpr>),
    BvSub(Box<SmtExpr>, Box<SmtExpr>),
    BvMul(Box<SmtExpr>, Box<SmtExpr>),
    BvUgt(Box<SmtExpr>, Box<SmtExpr>),
    BvUge(Box<SmtExpr>, Box<SmtExpr>),
    BvUlt(Box<SmtExpr>, Box<SmtExpr>),
    BvUle(Box<SmtExpr>, Box<SmtExpr>),
    Eq(Box<SmtExpr>, Box<SmtExpr>),
    Not(Box<SmtExpr>),
    And(Vec<SmtExpr>),
    Or(Vec<SmtExpr>),
    Ite(Box<SmtExpr>, Box<SmtExpr>, Box<SmtExpr>),
    Apply(String, Vec<SmtExpr>, SmtSort),
    Select(Box<SmtExpr>, Box<SmtExpr>),
    Store(Box<SmtExpr>, Box<SmtExpr>, Box<SmtExpr>),
}

impl SmtExpr {
    pub fn var(name: &str, sort: SmtSort) -> Self {
        SmtExpr::Var(name.to_string(), sort)
    }

    pub fn bv(val: u128, width: u32) -> Self {
        SmtExpr::BvConst(val, width)
    }

    pub fn bool_val(val: bool) -> Self {
        SmtExpr::BoolConst(val)
    }

    pub fn bv_add(lhs: SmtExpr, rhs: SmtExpr) -> Self {
        SmtExpr::BvAdd(Box::new(lhs), Box::new(rhs))
    }

    pub fn bv_sub(lhs: SmtExpr, rhs: SmtExpr) -> Self {
        SmtExpr::BvSub(Box::new(lhs), Box::new(rhs))
    }

    pub fn bv_mul(lhs: SmtExpr, rhs: SmtExpr) -> Self {
        SmtExpr::BvMul(Box::new(lhs), Box::new(rhs))
    }

    pub fn bv_uge(lhs: SmtExpr, rhs: SmtExpr) -> Self {
        SmtExpr::BvUge(Box::new(lhs), Box::new(rhs))
    }

    pub fn bv_ugt(lhs: SmtExpr, rhs: SmtExpr) -> Self {
        SmtExpr::BvUgt(Box::new(lhs), Box::new(rhs))
    }

    pub fn eq(lhs: SmtExpr, rhs: SmtExpr) -> Self {
        SmtExpr::Eq(Box::new(lhs), Box::new(rhs))
    }

    pub fn not(expr: SmtExpr) -> Self {
        SmtExpr::Not(Box::new(expr))
    }

    pub fn and(exprs: Vec<SmtExpr>) -> Self {
        SmtExpr::And(exprs)
    }

    pub fn ite(cond: SmtExpr, then_branch: SmtExpr, else_branch: SmtExpr) -> Self {
        SmtExpr::Ite(Box::new(cond), Box::new(then_branch), Box::new(else_branch))
    }

    /// Recursively formats the expression to valid SMT-LIB2 syntax.
    pub fn to_smt2(&self) -> String {
        match self {
            SmtExpr::Var(name, _) => name.clone(),
            SmtExpr::BvConst(val, width) => format!("(_ bv{} {})", val, width),
            SmtExpr::BoolConst(val) => if *val { "true".to_string() } else { "false".to_string() },
            SmtExpr::BvAdd(l, r) => format!("(bvadd {} {})", l.to_smt2(), r.to_smt2()),
            SmtExpr::BvSub(l, r) => format!("(bvsub {} {})", l.to_smt2(), r.to_smt2()),
            SmtExpr::BvMul(l, r) => format!("(bvmul {} {})", l.to_smt2(), r.to_smt2()),
            SmtExpr::BvUgt(l, r) => format!("(bvugt {} {})", l.to_smt2(), r.to_smt2()),
            SmtExpr::BvUge(l, r) => format!("(bvuge {} {})", l.to_smt2(), r.to_smt2()),
            SmtExpr::BvUlt(l, r) => format!("(bvult {} {})", l.to_smt2(), r.to_smt2()),
            SmtExpr::BvUle(l, r) => format!("(bvule {} {})", l.to_smt2(), r.to_smt2()),
            SmtExpr::Eq(l, r) => format!("(= {} {})", l.to_smt2(), r.to_smt2()),
            SmtExpr::Not(e) => format!("(not {})", e.to_smt2()),
            SmtExpr::And(items) => {
                if items.is_empty() {
                    "true".to_string()
                } else if items.len() == 1 {
                    items[0].to_smt2()
                } else {
                    let formatted = items.iter().map(|i| i.to_smt2()).collect::<Vec<_>>().join(" ");
                    format!("(and {})", formatted)
                }
            }
            SmtExpr::Or(items) => {
                if items.is_empty() {
                    "false".to_string()
                } else if items.len() == 1 {
                    items[0].to_smt2()
                } else {
                    let formatted = items.iter().map(|i| i.to_smt2()).collect::<Vec<_>>().join(" ");
                    format!("(or {})", formatted)
                }
            }
            SmtExpr::Ite(c, t, e) => {
                format!("(ite {} {} {})", c.to_smt2(), t.to_smt2(), e.to_smt2())
            }
            SmtExpr::Apply(func, args, _) => {
                if args.is_empty() {
                    func.clone()
                } else {
                    let formatted = args.iter().map(|a| a.to_smt2()).collect::<Vec<_>>().join(" ");
                    format!("({} {})", func, formatted)
                }
            }
            SmtExpr::Select(arr, idx) => format!("(select {} {})", arr.to_smt2(), idx.to_smt2()),
            SmtExpr::Store(arr, idx, val) => {
                format!("(store {} {} {})", arr.to_smt2(), idx.to_smt2(), val.to_smt2())
            }
        }
    }
}

/// SMT-LIB2 script builder for verification problems.
pub struct SmtScriptBuilder {
    logic: String,
    declarations: Vec<(String, SmtSort)>,
    uninterpreted_funcs: Vec<(String, Vec<SmtSort>, SmtSort)>,
    assertions: Vec<SmtExpr>,
}

impl SmtScriptBuilder {
    pub fn new(logic: &str) -> Self {
        Self {
            logic: logic.to_string(),
            declarations: Vec::new(),
            uninterpreted_funcs: Vec::new(),
            assertions: Vec::new(),
        }
    }

    pub fn declare_const(&mut self, name: &str, sort: SmtSort) {
        self.declarations.push((name.to_string(), sort));
    }

    pub fn declare_fun(&mut self, name: &str, args: Vec<SmtSort>, ret: SmtSort) {
        self.uninterpreted_funcs.push((name.to_string(), args, ret));
    }

    pub fn assert(&mut self, expr: SmtExpr) {
        self.assertions.push(expr);
    }

    pub fn build(&self) -> String {
        let mut script = String::new();
        script.push_str(&format!("(set-logic {})\n", self.logic));
        script.push_str("(set-option :produce-models true)\n\n");

        for (name, args, ret) in &self.uninterpreted_funcs {
            let args_fmt = args.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(" ");
            script.push_str(&format!("(declare-fun {} ({}) {})\n", name, args_fmt, ret));
        }

        for (name, sort) in &self.declarations {
            script.push_str(&format!("(declare-const {} {})\n", name, sort));
        }

        script.push('\n');
        for assertion in &self.assertions {
            script.push_str(&format!("(assert {})\n", assertion.to_smt2()));
        }

        script.push_str("\n(check-sat)\n(get-model)\n");
        script
    }
}
