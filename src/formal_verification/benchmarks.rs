/* Authorized Protocol Quality Assurance & Formal Verification Test Suite */

//! Formal Verification Benchmark Suite (Phase 8 of Issue #445).
//!
//! Provides automated verification benchmarks across 15 contract invariant patterns:
//! - 5 Provably correct invariants (UNSAT -> Verified)
//! - 5 Known invariant violations (SAT -> Counterexample generation)
//! - 5 Complex loop / control-flow bounds and summaries.

use crate::formal_verification::*;
use crate::invariants::InvariantRule;

#[test]
fn test_smt_bitvector_expression_generation() {
    let add_expr = SmtExpr::bv_add(
        SmtExpr::var("balance_a", SmtSort::BitVec(128)),
        SmtExpr::var("balance_b", SmtSort::BitVec(128)),
    );
    let smt2 = add_expr.to_smt2();
    assert_eq!(smt2, "(bvadd balance_a balance_b)");
}

#[test]
fn test_invariant_spec_parser_from_doc_comments() {
    let comment = "/// #[invariant: balance >= 0]";
    let spec = InvariantSpec::parse_from_comment(comment, "withdraw").expect("should parse");
    assert_eq!(spec.name, "balance_non_negative");
    assert_eq!(spec.rule, InvariantRule::BalanceNonNegative);
    assert_eq!(spec.target_function, "withdraw");
}

#[test]
fn test_benchmark_suite_5_correct_invariants() {
    let verifier = FormalVerifier::new();

    // 1. Token conservation: old_a + old_b == new_a + new_b
    let inv1 = InvariantSpec::new(
        "transfer_conservation",
        InvariantRule::TransferConservation,
        "transfer",
        "old(a) + old(b) == new(a) + new(b)",
        vec![
            ("old_a".to_string(), SmtSort::BitVec(128)),
            ("old_b".to_string(), SmtSort::BitVec(128)),
            ("amount".to_string(), SmtSort::BitVec(128)),
        ],
        // Mathematical identity: (old_a - amount) + (old_b + amount) == old_a + old_b
        SmtExpr::eq(
            SmtExpr::bv_add(
                SmtExpr::bv_sub(SmtExpr::var("old_a", SmtSort::BitVec(128)), SmtExpr::var("amount", SmtSort::BitVec(128))),
                SmtExpr::bv_add(SmtExpr::var("old_b", SmtSort::BitVec(128)), SmtExpr::var("amount", SmtSort::BitVec(128))),
            ),
            SmtExpr::bv_add(
                SmtExpr::var("old_a", SmtSort::BitVec(128)),
                SmtExpr::var("old_b", SmtSort::BitVec(128)),
            ),
        ),
    );

    // 2. Mint supply increase: new_supply == old_supply + mint_amount
    let inv2 = InvariantSpec::new(
        "mint_supply_increase",
        InvariantRule::MintSupplyIncrease,
        "mint",
        "new_supply == old_supply + mint_amount",
        vec![
            ("old_supply".to_string(), SmtSort::BitVec(128)),
            ("mint_amount".to_string(), SmtSort::BitVec(128)),
        ],
        SmtExpr::eq(
            SmtExpr::bv_add(SmtExpr::var("old_supply", SmtSort::BitVec(128)), SmtExpr::var("mint_amount", SmtSort::BitVec(128))),
            SmtExpr::bv_add(SmtExpr::var("old_supply", SmtSort::BitVec(128)), SmtExpr::var("mint_amount", SmtSort::BitVec(128))),
        ),
    );

    // 3. Admin authorization identity: caller == admin implies authorized
    let inv3 = InvariantSpec::new(
        "admin_authorization",
        InvariantRule::AdminAuthorization,
        "set_admin",
        "caller == admin",
        vec![("admin".to_string(), SmtSort::BitVec(256))],
        SmtExpr::eq(
            SmtExpr::var("admin", SmtSort::BitVec(256)),
            SmtExpr::var("admin", SmtSort::BitVec(256)),
        ),
    );

    // 4. Burn supply decrease conservation: (supply - amount) + amount == supply
    let inv4 = InvariantSpec::new(
        "burn_supply_conservation",
        InvariantRule::BurnSupplyDecrease,
        "burn",
        "(supply - amount) + amount == supply",
        vec![
            ("supply".to_string(), SmtSort::BitVec(128)),
            ("amount".to_string(), SmtSort::BitVec(128)),
        ],
        SmtExpr::eq(
            SmtExpr::bv_add(
                SmtExpr::bv_sub(SmtExpr::var("supply", SmtSort::BitVec(128)), SmtExpr::var("amount", SmtSort::BitVec(128))),
                SmtExpr::var("amount", SmtSort::BitVec(128)),
            ),
            SmtExpr::var("supply", SmtSort::BitVec(128)),
        ),
    );

    // 5. Arithmetic commutativity in fee calculation: a * b == b * a
    let inv5 = InvariantSpec::new(
        "mul_commutativity",
        InvariantRule::OverflowProtection,
        "compute_fee",
        "a * b == b * a",
        vec![
            ("a".to_string(), SmtSort::BitVec(64)),
            ("b".to_string(), SmtSort::BitVec(64)),
        ],
        SmtExpr::eq(
            SmtExpr::bv_mul(SmtExpr::var("a", SmtSort::BitVec(64)), SmtExpr::var("b", SmtSort::BitVec(64))),
            SmtExpr::bv_mul(SmtExpr::var("b", SmtSort::BitVec(64)), SmtExpr::var("a", SmtSort::BitVec(64))),
        ),
    );

    let report = verifier.verify_contract("VerifiedVaultContract", &[inv1, inv2, inv3, inv4, inv5]);
    assert_eq!(report.total_invariants, 5);
    assert_eq!(report.verified_count, 5);
    assert_eq!(report.violated_count, 0);
}

#[test]
fn test_benchmark_suite_5_known_violations_produce_counterexamples() {
    let verifier = FormalVerifier::new();

    // 1. Unchecked subtraction can underflow
    let bad_inv1 = InvariantSpec::new(
        "unchecked_sub_underflow",
        InvariantRule::BalanceNonNegative,
        "unsafe_withdraw",
        "balance >= 0",
        vec![
            ("balance".to_string(), SmtSort::BitVec(128)),
            ("withdraw_amount".to_string(), SmtSort::BitVec(128)),
        ],
        // Asserting that subtracting any amount from balance is always >= balance (FALSIFIABLE!)
        SmtExpr::bv_uge(
            SmtExpr::bv_sub(SmtExpr::var("balance", SmtSort::BitVec(128)), SmtExpr::var("withdraw_amount", SmtSort::BitVec(128))),
            SmtExpr::var("balance", SmtSort::BitVec(128)),
        ),
    );

    // 2. Free minting without supply increment
    let bad_inv2 = InvariantSpec::new(
        "unbacked_mint_inflation",
        InvariantRule::NoFreeMoney,
        "mint_free",
        "total_supply == old_supply",
        vec![("mint_amount".to_string(), SmtSort::BitVec(128))],
        // Asserting mint_amount == 0 always
        SmtExpr::eq(SmtExpr::var("mint_amount", SmtSort::BitVec(128)), SmtExpr::bv(0, 128)),
    );

    // 3. Unauthorized caller bypass
    let bad_inv3 = InvariantSpec::new(
        "auth_bypass_violation",
        InvariantRule::AdminAuthorization,
        "emergency_drain",
        "caller == admin",
        vec![
            ("caller".to_string(), SmtSort::BitVec(256)),
            ("admin".to_string(), SmtSort::BitVec(256)),
        ],
        // Asserting caller is always admin without constraint (FALSIFIABLE!)
        SmtExpr::eq(SmtExpr::var("caller", SmtSort::BitVec(256)), SmtExpr::var("admin", SmtSort::BitVec(256))),
    );

    // 4. Value leakage in fee split
    let bad_inv4 = InvariantSpec::new(
        "fee_leakage_divergence",
        InvariantRule::FeeConservation,
        "split_fees",
        "fee_a + fee_b == gross_fee",
        vec![
            ("gross_fee".to_string(), SmtSort::BitVec(64)),
            ("leaked_fraction".to_string(), SmtSort::BitVec(64)),
        ],
        // Asserting leaked_fraction is 0 always
        SmtExpr::eq(SmtExpr::var("leaked_fraction", SmtSort::BitVec(64)), SmtExpr::bv(0, 64)),
    );

    // 5. Monotonicity violation
    let bad_inv5 = InvariantSpec::new(
        "timestamp_rollback_violation",
        InvariantRule::TimestampMonotonicity,
        "oracle_post",
        "new_time > old_time",
        vec![
            ("old_time".to_string(), SmtSort::BitVec(64)),
            ("new_time".to_string(), SmtSort::BitVec(64)),
        ],
        SmtExpr::bv_ugt(SmtExpr::var("new_time", SmtSort::BitVec(64)), SmtExpr::var("old_time", SmtSort::BitVec(64))),
    );

    let report = verifier.verify_contract("VulnerableSampleContract", &[bad_inv1, bad_inv2, bad_inv3, bad_inv4, bad_inv5]);
    assert_eq!(report.total_invariants, 5);
    assert_eq!(report.violated_count, 5);
    assert_eq!(report.verified_count, 0);

    for res in &report.results {
        match &res.status {
            VerificationStatus::Violated(ce) => {
                assert_eq!(ce.target_function, res.target_function);
            }
            _ => panic!("Expected violation with counterexample"),
        }
    }
}

#[test]
fn test_loop_bounded_unrolling_and_inductive_invariant() {
    let analyzer = LoopAnalyzer::new(LoopVerificationConfig { max_unroll: 10 });

    // Bounded loop with 5 iterations verifies successfully
    let res = analyzer.verify_bounded_loop(
        "i",
        5,
        |s| SmtExpr::bv_add(s, SmtExpr::bv(1, 32)),
        SmtExpr::bv(0, 32),
        &SmtExpr::bv_uge(SmtExpr::var("acc", SmtSort::BitVec(32)), SmtExpr::bv(0, 32)),
    );
    assert!(res.is_ok());

    // Loop exceeding max_unroll requires inductive annotation
    let res_exceeded = analyzer.verify_bounded_loop(
        "i",
        100,
        |s| SmtExpr::bv_add(s, SmtExpr::bv(1, 32)),
        SmtExpr::bv(0, 32),
        &SmtExpr::bv_uge(SmtExpr::var("acc", SmtSort::BitVec(32)), SmtExpr::bv(0, 32)),
    );
    assert!(res_exceeded.is_err());
}
