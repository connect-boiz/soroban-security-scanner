#![cfg(test)]
extern crate std;

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{symbol_short, token, vec, Address, BytesN, Env, Map, String, Symbol, Vec};

struct TestContext<'a> {
    env: Env,
    admin: Address,
    contract_id: Address,
    client: SecurityScannerContractClient<'a>,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
}

fn setup() -> TestContext<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(issuer.clone());
    let token_id = token_contract.address();
    let token = token::Client::new(&env, &token_id);
    let token_admin = token::StellarAssetClient::new(&env, &token_id);

    let contract_id = env.register(SecurityScannerContract, ());
    let client = SecurityScannerContractClient::new(&env, &contract_id);
    client.initialize(&admin, &token_id);

    TestContext {
        env,
        admin,
        contract_id,
        client,
        token,
        token_admin,
    }
}

/// Grant a role directly through instance storage (test helper). The real
/// multi-sig role-grant flow is exercised separately.
fn grant_roles_direct(ctx: &TestContext, user: &Address, roles: Vec<Role>) {
    let env = &ctx.env;
    env.as_contract(&ctx.contract_id, || {
        let mut admin_roles: Map<Address, Vec<Role>> = env
            .storage()
            .instance()
            .get(&ADMIN_ROLES)
            .unwrap_or(Map::new(env));
        admin_roles.set(user.clone(), roles);
        env.storage().instance().set(&ADMIN_ROLES, &admin_roles);
    });
}

/// Create a vulnerability report with minimal unique parameters.
fn report(ctx: &TestContext, reporter: &Address, seed: u8, severity: Symbol) -> u64 {
    ctx.client.report_vulnerability(
        reporter,
        &BytesN::from_array(&ctx.env, &[seed; 32]),
        &String::from_str(&ctx.env, "reentrancy"),
        &severity,
        &String::from_str(&ctx.env, "description"),
        &String::from_str(&ctx.env, "lib.rs"),
    )
}

// ---------------------------------------------------------------------------
// Initialization & error system
// ---------------------------------------------------------------------------

#[test]
fn initialize_rejects_default_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let issuer = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(issuer.clone());
    let token_id = token_contract.address();
    let contract_id = env.register(SecurityScannerContract, ());
    let client = SecurityScannerContractClient::new(&env, &contract_id);

    let null_admin = Address::from_string(&String::from_str(
        &env,
        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
    ));

    let result = client.try_initialize(&null_admin, &token_id);
    assert_eq!(result, Err(Ok(ContractError::DefaultAddress)));
}

#[test]
fn initialize_rejects_default_token() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(SecurityScannerContract, ());
    let client = SecurityScannerContractClient::new(&env, &contract_id);

    let null_token = Address::from_string(&String::from_str(
        &env,
        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
    ));

    let result = client.try_initialize(&admin, &null_token);
    assert_eq!(result, Err(Ok(ContractError::DefaultAddress)));
}

#[test]
fn initialize_cannot_be_called_twice() {
    let ctx = setup();
    let other_admin = Address::generate(&ctx.env);
    let result = ctx
        .client
        .try_initialize(&other_admin, &Address::generate(&ctx.env));
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn error_codes_are_structured_and_roundtrip() {
    // The error space scales to 200 distinct codes.
    assert_eq!(ContractError::MAX_ERROR_CODE, 200);
    assert_eq!(ContractError::COUNT, 30);

    // Every variant maps to a stable domain-grouped code...
    assert_eq!(ContractError::Unauthorized.code(), 1);
    assert_eq!(ContractError::InsufficientPermissions.code(), 2);
    assert_eq!(ContractError::InvalidInput.code(), 20);
    assert_eq!(ContractError::DefaultAddress.code(), 21);
    assert_eq!(ContractError::TextTooLong.code(), 22);
    assert_eq!(ContractError::InvalidSeverity.code(), 23);
    assert_eq!(ContractError::InvalidPurpose.code(), 24);
    assert_eq!(ContractError::ZeroAmount.code(), 25);
    assert_eq!(ContractError::NotFound.code(), 40);
    assert_eq!(ContractError::InsufficientFunds.code(), 60);
    assert_eq!(ContractError::EscrowLocked.code(), 80);
    assert_eq!(ContractError::EscrowAlreadyReleased.code(), 82);
    assert_eq!(ContractError::ProposalNotFound.code(), 100);
    assert_eq!(ContractError::ProposalNotReady.code(), 102);
    assert_eq!(ContractError::MultiSigRequired.code(), 105);
    assert_eq!(ContractError::SignatureInvalid.code(), 107);
    assert_eq!(ContractError::InvalidRole.code(), 120);
    assert_eq!(ContractError::RoleAlreadyGranted.code(), 121);
    assert_eq!(ContractError::EmergencyModeActive.code(), 140);
    assert_eq!(ContractError::AlreadyVerified.code(), 141);
    assert_eq!(ContractError::Overflow.code(), 160);

    // ...and the reverse mapping round-trips every assigned code.
    let mut seen = 0u32;
    for code in 1..=ContractError::MAX_ERROR_CODE {
        if let Some(err) = ContractError::from_code(code) {
            assert_eq!(err.code(), code);
            seen += 1;
        }
    }
    assert_eq!(seen, ContractError::COUNT);
    assert_eq!(
        ContractError::from_code(ContractError::MAX_ERROR_CODE + 1),
        None
    );
    assert_eq!(ContractError::from_code(0), None);
}

// ---------------------------------------------------------------------------
// Vulnerability reporting & verification
// ---------------------------------------------------------------------------

#[test]
fn report_vulnerability_creates_pending_report() {
    let ctx = setup();
    let reporter = Address::generate(&ctx.env);

    let report_id = report(&ctx, &reporter, 1, symbol_short!("high"));

    let v = ctx.client.get_vulnerability(&report_id);
    assert_eq!(v.reporter, reporter);
    assert_eq!(v.status, STATUS_PENDING);
    assert_eq!(v.severity, symbol_short!("high"));
    assert_eq!(v.bounty_amount, 0);
}

#[test]
fn report_vulnerability_rejects_long_text() {
    let ctx = setup();
    let long = String::from_str(&ctx.env, &"x".repeat(281));
    let result = ctx.client.try_report_vulnerability(
        &Address::generate(&ctx.env),
        &BytesN::from_array(&ctx.env, &[1u8; 32]),
        &String::from_str(&ctx.env, "reentrancy"),
        &symbol_short!("high"),
        &long,
        &String::from_str(&ctx.env, "lib.rs"),
    );
    assert_eq!(result, Err(Ok(ContractError::TextTooLong)));
}

#[test]
fn report_vulnerability_rejects_default_reporter() {
    let ctx = setup();
    let null_reporter = Address::from_string(&String::from_str(
        &ctx.env,
        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
    ));
    let result = ctx.client.try_report_vulnerability(
        &null_reporter,
        &BytesN::from_array(&ctx.env, &[1u8; 32]),
        &String::from_str(&ctx.env, "reentrancy"),
        &symbol_short!("high"),
        &String::from_str(&ctx.env, "description"),
        &String::from_str(&ctx.env, "lib.rs"),
    );
    assert_eq!(result, Err(Ok(ContractError::DefaultAddress)));
}

#[test]
fn get_vulnerability_not_found() {
    let ctx = setup();
    assert_eq!(
        ctx.client.try_get_vulnerability(&999),
        Err(Ok(ContractError::NotFound))
    );
}

#[test]
fn verify_vulnerability_rejects_double_payout() {
    let ctx = setup();
    let reporter = Address::generate(&ctx.env);

    ctx.token_admin.mint(&ctx.admin, &1_000_000);
    ctx.client.fund_bounty_pool(&ctx.admin, &500_000);

    let report_id = report(&ctx, &reporter, 1, symbol_short!("high"));

    ctx.client
        .verify_vulnerability(&ctx.admin, &report_id, &100_000);
    assert_eq!(ctx.client.get_bounty_pool(), 400_000);

    let second = ctx
        .client
        .try_verify_vulnerability(&ctx.admin, &report_id, &100_000);
    assert_eq!(second, Err(Ok(ContractError::AlreadyVerified)));
}

#[test]
fn verify_vulnerability_deducts_bounty_pool_and_transfers_tokens() {
    let ctx = setup();
    let reporter = Address::generate(&ctx.env);

    ctx.token_admin.mint(&ctx.admin, &1_000_000);
    ctx.client.fund_bounty_pool(&ctx.admin, &500_000);

    let report_id = report(&ctx, &reporter, 2, symbol_short!("critical"));

    ctx.client
        .verify_vulnerability(&ctx.admin, &report_id, &150_000);

    assert_eq!(ctx.client.get_bounty_pool(), 350_000);
    assert_eq!(ctx.token.balance(&reporter), 150_000);

    let reputation = ctx.client.get_reputation(&reporter);
    assert_eq!(reputation.successful_reports, 1);
    assert_eq!(reputation.total_earnings, 150_000);
}

#[test]
fn verify_vulnerability_requires_permission() {
    let ctx = setup();
    let reporter = Address::generate(&ctx.env);
    let stranger = Address::generate(&ctx.env);

    ctx.token_admin.mint(&ctx.admin, &1_000_000);
    ctx.client.fund_bounty_pool(&ctx.admin, &500_000);

    let report_id = report(&ctx, &reporter, 3, symbol_short!("high"));

    let result = ctx
        .client
        .try_verify_vulnerability(&stranger, &report_id, &100_000);
    assert_eq!(result, Err(Ok(ContractError::InsufficientPermissions)));
}

#[test]
fn verify_vulnerability_rejects_zero_bounty() {
    let ctx = setup();
    let report_id = report(&ctx, &Address::generate(&ctx.env), 4, symbol_short!("high"));

    let result = ctx
        .client
        .try_verify_vulnerability(&ctx.admin, &report_id, &0);
    assert_eq!(result, Err(Ok(ContractError::ZeroAmount)));
}

#[test]
fn verify_vulnerability_rejects_insufficient_pool() {
    let ctx = setup();
    let report_id = report(&ctx, &Address::generate(&ctx.env), 5, symbol_short!("high"));

    let result = ctx
        .client
        .try_verify_vulnerability(&ctx.admin, &report_id, &100_000);
    assert_eq!(result, Err(Ok(ContractError::InsufficientFunds)));
}

#[test]
fn verify_vulnerability_rejects_high_bounty_without_multisig() {
    let ctx = setup();
    ctx.token_admin.mint(&ctx.admin, &5_000_000);
    ctx.client.fund_bounty_pool(&ctx.admin, &5_000_000);

    let report_id = report(&ctx, &Address::generate(&ctx.env), 6, symbol_short!("high"));

    let result = ctx
        .client
        .try_verify_vulnerability(&ctx.admin, &report_id, &2_000_000);
    assert_eq!(result, Err(Ok(ContractError::MultiSigRequired)));
}

#[test]
fn verify_vulnerability_marks_report_verified() {
    let ctx = setup();
    let reporter = Address::generate(&ctx.env);
    ctx.token_admin.mint(&ctx.admin, &1_000_000);
    ctx.client.fund_bounty_pool(&ctx.admin, &500_000);

    let report_id = report(&ctx, &reporter, 7, symbol_short!("high"));
    ctx.client
        .verify_vulnerability(&ctx.admin, &report_id, &100_000);

    let v = ctx.client.get_vulnerability(&report_id);
    assert_eq!(v.status, STATUS_VERIFIED);
    assert_eq!(v.bounty_amount, 100_000);
}

#[test]
fn reputation_uses_address_keyed_storage() {
    let ctx = setup();
    let researcher_a = Address::generate(&ctx.env);
    let researcher_b = Address::generate(&ctx.env);

    ctx.token_admin.mint(&ctx.admin, &2_000_000);
    ctx.client.fund_bounty_pool(&ctx.admin, &1_000_000);

    let report_a = report(&ctx, &researcher_a, 8, symbol_short!("high"));
    let report_b = report(&ctx, &researcher_b, 9, symbol_short!("high"));

    ctx.client
        .verify_vulnerability(&ctx.admin, &report_a, &100_000);
    ctx.client
        .verify_vulnerability(&ctx.admin, &report_b, &200_000);

    let rep_a = ctx.client.get_reputation(&researcher_a);
    let rep_b = ctx.client.get_reputation(&researcher_b);

    assert_eq!(rep_a.successful_reports, 1);
    assert_eq!(rep_b.successful_reports, 1);
    assert_eq!(rep_a.total_earnings, 100_000);
    assert_eq!(rep_b.total_earnings, 200_000);
}

#[test]
fn get_reputation_not_found() {
    let ctx = setup();
    assert_eq!(
        ctx.client.try_get_reputation(&Address::generate(&ctx.env)),
        Err(Ok(ContractError::NotFound))
    );
}

// ---------------------------------------------------------------------------
// Bounty pool
// ---------------------------------------------------------------------------

#[test]
fn fund_bounty_pool_increases_pool() {
    let ctx = setup();
    ctx.token_admin.mint(&ctx.admin, &1_000_000);
    ctx.client.fund_bounty_pool(&ctx.admin, &250_000);
    assert_eq!(ctx.client.get_bounty_pool(), 250_000);
}

#[test]
fn fund_bounty_pool_rejects_zero_amount() {
    let ctx = setup();
    let result = ctx.client.try_fund_bounty_pool(&ctx.admin, &0);
    assert_eq!(result, Err(Ok(ContractError::ZeroAmount)));
}

// ---------------------------------------------------------------------------
// Multi-signature high-bounty flow
// ---------------------------------------------------------------------------

#[test]
fn high_bounty_proposal_enforces_minimum_approvals() {
    let ctx = setup();

    let report_id = report(
        &ctx,
        &Address::generate(&ctx.env),
        10,
        symbol_short!("high"),
    );

    let proposal_id = ctx
        .client
        .propose_high_bounty_verification(&ctx.admin, &report_id, &2_000_000, &1, &0);

    let proposal = ctx.client.get_proposal(&proposal_id);
    assert_eq!(proposal.required_approvals, MIN_HIGH_BOUNTY_APPROVALS);
    assert!(!ctx.client.can_execute_proposal_check(&proposal_id));
}

#[test]
fn high_bounty_execution_requires_approvals_and_delay() {
    let ctx = setup();
    let verifier = Address::generate(&ctx.env);
    grant_roles_direct(&ctx, &verifier, vec![&ctx.env, Role::Verifier]);

    ctx.token_admin.mint(&ctx.admin, &5_000_000);
    ctx.client.fund_bounty_pool(&ctx.admin, &5_000_000);

    let reporter = Address::generate(&ctx.env);
    let report_id = report(&ctx, &reporter, 11, symbol_short!("high"));

    let proposal_id = ctx
        .client
        .propose_high_bounty_verification(&ctx.admin, &report_id, &2_000_000, &2, &0);

    // Not enough approvals yet -> not ready.
    let result = ctx
        .client
        .try_execute_high_bounty_verification(&ctx.admin, &proposal_id);
    assert_eq!(result, Err(Ok(ContractError::ProposalNotReady)));

    // First approval succeeds; a duplicate approval is rejected.
    ctx.client
        .approve_bounty_verification(&ctx.admin, &proposal_id);
    let duplicate = ctx
        .client
        .try_approve_bounty_verification(&ctx.admin, &proposal_id);
    assert_eq!(duplicate, Err(Ok(ContractError::AlreadyApproved)));

    // Still one short.
    let result = ctx
        .client
        .try_execute_high_bounty_verification(&ctx.admin, &proposal_id);
    assert_eq!(result, Err(Ok(ContractError::ProposalNotReady)));

    ctx.client
        .approve_bounty_verification(&verifier, &proposal_id);

    // Approvals met -> executes and pays out.
    ctx.client
        .execute_high_bounty_verification(&ctx.admin, &proposal_id);

    assert_eq!(ctx.token.balance(&reporter), 2_000_000);
    assert_eq!(ctx.client.get_bounty_pool(), 3_000_000);
    let v = ctx.client.get_vulnerability(&report_id);
    assert_eq!(v.status, STATUS_VERIFIED);
    assert_eq!(v.bounty_amount, 2_000_000);

    // Executing again is rejected.
    let second = ctx
        .client
        .try_execute_high_bounty_verification(&ctx.admin, &proposal_id);
    assert_eq!(second, Err(Ok(ContractError::ProposalAlreadyExecuted)));
}

#[test]
fn high_bounty_execution_respects_delay() {
    let ctx = setup();
    let verifier = Address::generate(&ctx.env);
    grant_roles_direct(&ctx, &verifier, vec![&ctx.env, Role::Verifier]);

    ctx.token_admin.mint(&ctx.admin, &5_000_000);
    ctx.client.fund_bounty_pool(&ctx.admin, &5_000_000);

    let report_id = report(
        &ctx,
        &Address::generate(&ctx.env),
        12,
        symbol_short!("high"),
    );
    let now = ctx.env.ledger().timestamp();
    ctx.env.ledger().set_timestamp(now + 1000);

    let proposal_id = ctx
        .client
        .propose_high_bounty_verification(&ctx.admin, &report_id, &2_000_000, &2, &500);
    ctx.client
        .approve_bounty_verification(&ctx.admin, &proposal_id);
    ctx.client
        .approve_bounty_verification(&verifier, &proposal_id);

    // Before the execution delay elapses.
    let result = ctx
        .client
        .try_execute_high_bounty_verification(&ctx.admin, &proposal_id);
    assert_eq!(result, Err(Ok(ContractError::ProposalNotReady)));

    // After the delay.
    ctx.env.ledger().set_timestamp(now + 1000 + 501);
    ctx.client
        .execute_high_bounty_verification(&ctx.admin, &proposal_id);
}

// ---------------------------------------------------------------------------
// Escrow lifecycle
// ---------------------------------------------------------------------------

#[test]
fn release_escrow_succeeds_without_signer_when_depositor_auth() {
    let ctx = setup();
    let depositor = ctx.admin.clone();
    let beneficiary = Address::generate(&ctx.env);

    ctx.token_admin.mint(&depositor, &500_000);

    let escrow_id = ctx.client.create_escrow(
        &depositor,
        &beneficiary,
        &100_000,
        &symbol_short!("bounty"),
        &0,
        &None,
    );

    ctx.client
        .mark_escrow_conditions_met(&escrow_id, &ctx.admin);

    ctx.client.release_escrow(&escrow_id, &depositor, &None);

    let escrow = ctx.client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, STATUS_RELEASED);
    assert_eq!(ctx.token.balance(&beneficiary), 100_000);
    assert_eq!(ctx.client.get_escrow_pool_balance(), 0);
}

#[test]
fn create_escrow_rejects_invalid_purpose() {
    let ctx = setup();
    let result = ctx.client.try_create_escrow(
        &ctx.admin,
        &Address::generate(&ctx.env),
        &1_000,
        &symbol_short!("gift"),
        &0,
        &None,
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidPurpose)));
}

#[test]
fn create_escrow_rejects_zero_amount() {
    let ctx = setup();
    let result = ctx.client.try_create_escrow(
        &ctx.admin,
        &Address::generate(&ctx.env),
        &0,
        &symbol_short!("bounty"),
        &0,
        &None,
    );
    assert_eq!(result, Err(Ok(ContractError::ZeroAmount)));
}

#[test]
fn release_escrow_locked_until_expiry() {
    let ctx = setup();
    let depositor = ctx.admin.clone();
    let beneficiary = Address::generate(&ctx.env);
    ctx.token_admin.mint(&depositor, &500_000);

    let now = ctx.env.ledger().timestamp();
    let escrow_id = ctx.client.create_escrow(
        &depositor,
        &beneficiary,
        &100_000,
        &symbol_short!("bounty"),
        &1000,
        &None,
    );

    // Before the lock expires and without conditions met -> locked.
    let result = ctx.client.try_release_escrow(&escrow_id, &depositor, &None);
    assert_eq!(result, Err(Ok(ContractError::EscrowLocked)));

    // After the lock expires -> releasable.
    ctx.env.ledger().set_timestamp(now + 1001);
    ctx.client.release_escrow(&escrow_id, &depositor, &None);
    assert_eq!(ctx.token.balance(&beneficiary), 100_000);
}

#[test]
fn release_escrow_requires_depositor() {
    let ctx = setup();
    let depositor = ctx.admin.clone();
    let beneficiary = Address::generate(&ctx.env);
    let attacker = Address::generate(&ctx.env);
    ctx.token_admin.mint(&depositor, &500_000);

    let escrow_id = ctx.client.create_escrow(
        &depositor,
        &beneficiary,
        &100_000,
        &symbol_short!("bounty"),
        &0,
        &None,
    );
    ctx.client
        .mark_escrow_conditions_met(&escrow_id, &ctx.admin);

    let result = ctx.client.try_release_escrow(&escrow_id, &attacker, &None);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn release_escrow_cannot_be_released_twice() {
    let ctx = setup();
    let depositor = ctx.admin.clone();
    let beneficiary = Address::generate(&ctx.env);
    ctx.token_admin.mint(&depositor, &500_000);

    let escrow_id = ctx.client.create_escrow(
        &depositor,
        &beneficiary,
        &100_000,
        &symbol_short!("bounty"),
        &0,
        &None,
    );
    ctx.client
        .mark_escrow_conditions_met(&escrow_id, &ctx.admin);
    ctx.client.release_escrow(&escrow_id, &depositor, &None);

    let second = ctx.client.try_release_escrow(&escrow_id, &depositor, &None);
    assert_eq!(second, Err(Ok(ContractError::EscrowAlreadyReleased)));
}

#[test]
fn refund_escrow_after_lock_expiry() {
    let ctx = setup();
    let depositor = ctx.admin.clone();
    let beneficiary = Address::generate(&ctx.env);
    ctx.token_admin.mint(&depositor, &500_000);

    let now = ctx.env.ledger().timestamp();
    let escrow_id = ctx.client.create_escrow(
        &depositor,
        &beneficiary,
        &100_000,
        &symbol_short!("bounty"),
        &1000,
        &None,
    );

    // Lock still active -> no refund.
    let result = ctx.client.try_refund_escrow(&escrow_id, &depositor);
    assert_eq!(result, Err(Ok(ContractError::EscrowLocked)));

    // After expiry, with conditions not met -> refund succeeds.
    ctx.env.ledger().set_timestamp(now + 1001);
    ctx.client.refund_escrow(&escrow_id, &depositor);
    let escrow = ctx.client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, STATUS_REFUNDED);
    // Deposit went out (100_000) and came back on refund.
    assert_eq!(ctx.token.balance(&depositor), 500_000);

    // Already refunded -> rejected.
    let second = ctx.client.try_refund_escrow(&escrow_id, &depositor);
    assert_eq!(second, Err(Ok(ContractError::EscrowAlreadyRefunded)));
}

#[test]
fn refund_escrow_blocked_when_conditions_met() {
    let ctx = setup();
    let depositor = ctx.admin.clone();
    let beneficiary = Address::generate(&ctx.env);
    ctx.token_admin.mint(&depositor, &500_000);

    let now = ctx.env.ledger().timestamp();
    let escrow_id = ctx.client.create_escrow(
        &depositor,
        &beneficiary,
        &100_000,
        &symbol_short!("bounty"),
        &1000,
        &None,
    );
    ctx.client
        .mark_escrow_conditions_met(&escrow_id, &ctx.admin);
    ctx.env.ledger().set_timestamp(now + 1001);

    let result = ctx.client.try_refund_escrow(&escrow_id, &depositor);
    assert_eq!(result, Err(Ok(ContractError::EscrowLocked)));
}

#[test]
fn mark_escrow_conditions_requires_escrow_manager() {
    let ctx = setup();
    let manager = Address::generate(&ctx.env);
    grant_roles_direct(&ctx, &manager, vec![&ctx.env, Role::EscrowManager]);
    let stranger = Address::generate(&ctx.env);

    let depositor = ctx.admin.clone();
    let beneficiary = Address::generate(&ctx.env);
    ctx.token_admin.mint(&depositor, &500_000);
    let escrow_id = ctx.client.create_escrow(
        &depositor,
        &beneficiary,
        &100_000,
        &symbol_short!("bounty"),
        &1000,
        &None,
    );

    // Stranger without ManageEscrow is denied.
    let denied = ctx
        .client
        .try_mark_escrow_conditions_met(&escrow_id, &stranger);
    assert_eq!(denied, Err(Ok(ContractError::InsufficientPermissions)));

    // A granted EscrowManager succeeds.
    ctx.client.mark_escrow_conditions_met(&escrow_id, &manager);
    let escrow = ctx.client.get_escrow(&escrow_id);
    assert!(escrow.conditions_met);
}

#[test]
fn escrow_release_message_is_stable_and_binds_escrow() {
    let ctx = setup();
    let depositor = ctx.admin.clone();
    let beneficiary = Address::generate(&ctx.env);
    ctx.token_admin.mint(&depositor, &500_000);

    let escrow_id = ctx.client.create_escrow(
        &depositor,
        &beneficiary,
        &100_000,
        &symbol_short!("bounty"),
        &0,
        &None,
    );
    let message = ctx.client.escrow_release_message(&escrow_id);
    assert!(!message.is_empty());

    assert_eq!(
        ctx.client.try_escrow_release_message(&999),
        Err(Ok(ContractError::NotFound))
    );
}

#[test]
fn get_escrow_not_found() {
    let ctx = setup();
    assert_eq!(
        ctx.client.try_get_escrow(&999),
        Err(Ok(ContractError::NotFound))
    );
}

#[test]
fn release_escrow_with_signer_requires_signature() {
    let ctx = setup();
    let depositor = ctx.admin.clone();
    let beneficiary = Address::generate(&ctx.env);
    ctx.token_admin.mint(&depositor, &500_000);

    let escrow_id = ctx.client.create_escrow(
        &depositor,
        &beneficiary,
        &100_000,
        &symbol_short!("bounty"),
        &0,
        &Some(BytesN::from_array(&ctx.env, &[1u8; 32])),
    );
    ctx.client
        .mark_escrow_conditions_met(&escrow_id, &ctx.admin);

    let result = ctx.client.try_release_escrow(&escrow_id, &depositor, &None);
    assert_eq!(result, Err(Ok(ContractError::SignatureMissing)));
}

// ---------------------------------------------------------------------------
// Emergency alerts
// ---------------------------------------------------------------------------

#[test]
fn report_emergency_vulnerability_requires_valid_severity() {
    let ctx = setup();
    let reporter = Address::generate(&ctx.env);

    let invalid = ctx.client.try_report_emergency_vulnerability(
        &reporter,
        &BytesN::from_array(&ctx.env, &[1u8; 32]),
        &String::from_str(&ctx.env, "critical"),
        &symbol_short!("medium"),
        &String::from_str(&ctx.env, "description"),
        &String::from_str(&ctx.env, "lib.rs"),
    );
    assert_eq!(invalid, Err(Ok(ContractError::InvalidSeverity)));

    let alert_id = ctx.client.report_emergency_vulnerability(
        &reporter,
        &BytesN::from_array(&ctx.env, &[1u8; 32]),
        &String::from_str(&ctx.env, "critical"),
        &symbol_short!("critical"),
        &String::from_str(&ctx.env, "description"),
        &String::from_str(&ctx.env, "lib.rs"),
    );
    let alert = ctx.client.get_emergency_alert(&alert_id);
    assert_eq!(alert.status, STATUS_PENDING);
    assert_eq!(alert.emergency_reward, 5_000_000);

    let emergency_id = ctx.client.report_emergency_vulnerability(
        &reporter,
        &BytesN::from_array(&ctx.env, &[2u8; 32]),
        &String::from_str(&ctx.env, "critical"),
        &symbol_short!("emergency"),
        &String::from_str(&ctx.env, "description"),
        &String::from_str(&ctx.env, "lib.rs"),
    );
    let alert = ctx.client.get_emergency_alert(&emergency_id);
    assert_eq!(alert.emergency_reward, 10_000_000);
}

#[test]
fn verify_emergency_vulnerability_always_requires_multisig() {
    let ctx = setup();
    let result = ctx
        .client
        .try_verify_emergency_vulnerability(&ctx.admin, &1, &true);
    assert_eq!(result, Err(Ok(ContractError::MultiSigRequired)));
}

#[test]
fn emergency_verification_full_flow() {
    let ctx = setup();
    let verifier_a = Address::generate(&ctx.env);
    let verifier_b = Address::generate(&ctx.env);
    grant_roles_direct(&ctx, &verifier_a, vec![&ctx.env, Role::Verifier]);
    grant_roles_direct(&ctx, &verifier_b, vec![&ctx.env, Role::Verifier]);

    // Admin (the depositor for the emergency escrow) must hold the reward.
    ctx.token_admin.mint(&ctx.admin, &10_000_000);

    let reporter = Address::generate(&ctx.env);
    let alert_id = ctx.client.report_emergency_vulnerability(
        &reporter,
        &BytesN::from_array(&ctx.env, &[3u8; 32]),
        &String::from_str(&ctx.env, "critical"),
        &symbol_short!("critical"),
        &String::from_str(&ctx.env, "description"),
        &String::from_str(&ctx.env, "lib.rs"),
    );

    let now = ctx.env.ledger().timestamp();
    let proposal_id = ctx
        .client
        .propose_emergency_verification(&ctx.admin, &alert_id, &true, &1, &0);

    // Minimums are enforced: 3 approvals and a 1-hour delay.
    let proposal = ctx.client.get_proposal(&proposal_id);
    assert_eq!(proposal.required_approvals, 3);
    assert_eq!(proposal.execution_delay, 3600);

    // Cannot execute yet.
    let result = ctx
        .client
        .try_execute_emergency_verification(&ctx.admin, &proposal_id);
    assert_eq!(result, Err(Ok(ContractError::ProposalNotReady)));

    ctx.client
        .approve_emergency_verification(&ctx.admin, &proposal_id);
    ctx.client
        .approve_emergency_verification(&verifier_a, &proposal_id);
    ctx.client
        .approve_emergency_verification(&verifier_b, &proposal_id);

    // Still time-locked.
    let result = ctx
        .client
        .try_execute_emergency_verification(&ctx.admin, &proposal_id);
    assert_eq!(result, Err(Ok(ContractError::ProposalNotReady)));

    ctx.env.ledger().set_timestamp(now + 3601);
    ctx.client
        .execute_emergency_verification(&ctx.admin, &proposal_id);

    let alert = ctx.client.get_emergency_alert(&alert_id);
    assert_eq!(alert.status, STATUS_VERIFIED);
    assert_eq!(alert.verified_by, Some(ctx.admin.clone()));

    // The emergency reward was escrowed and released to the reporter.
    assert_eq!(ctx.token.balance(&reporter), 5_000_000);

    let reputation = ctx.client.get_reputation(&reporter);
    assert_eq!(reputation.successful_reports, 1);
    assert_eq!(reputation.total_earnings, 5_000_000);
}

#[test]
fn emergency_verification_false_positive_path() {
    let ctx = setup();
    let verifier_a = Address::generate(&ctx.env);
    let verifier_b = Address::generate(&ctx.env);
    grant_roles_direct(&ctx, &verifier_a, vec![&ctx.env, Role::Verifier]);
    grant_roles_direct(&ctx, &verifier_b, vec![&ctx.env, Role::Verifier]);

    let reporter = Address::generate(&ctx.env);
    let alert_id = ctx.client.report_emergency_vulnerability(
        &reporter,
        &BytesN::from_array(&ctx.env, &[4u8; 32]),
        &String::from_str(&ctx.env, "critical"),
        &symbol_short!("critical"),
        &String::from_str(&ctx.env, "description"),
        &String::from_str(&ctx.env, "lib.rs"),
    );

    let now = ctx.env.ledger().timestamp();
    let proposal_id = ctx
        .client
        .propose_emergency_verification(&ctx.admin, &alert_id, &false, &3, &3600);
    ctx.client
        .approve_emergency_verification(&ctx.admin, &proposal_id);
    ctx.client
        .approve_emergency_verification(&verifier_a, &proposal_id);
    ctx.client
        .approve_emergency_verification(&verifier_b, &proposal_id);
    ctx.env.ledger().set_timestamp(now + 3601);
    ctx.client
        .execute_emergency_verification(&ctx.admin, &proposal_id);

    let alert = ctx.client.get_emergency_alert(&alert_id);
    assert_eq!(alert.status, Symbol::new(&ctx.env, STATUS_FALSE_POSITIVE));
    // No payout on a false positive.
    assert_eq!(ctx.token.balance(&reporter), 0);
}

#[test]
fn get_emergency_alert_not_found() {
    let ctx = setup();
    assert_eq!(
        ctx.client.try_get_emergency_alert(&999),
        Err(Ok(ContractError::NotFound))
    );
}

// ---------------------------------------------------------------------------
// Roles & permissions
// ---------------------------------------------------------------------------

#[test]
fn get_user_roles_returns_empty_for_unknown_user() {
    let ctx = setup();
    let roles = ctx.client.get_user_roles(&Address::generate(&ctx.env));
    assert!(roles.is_empty());
}

#[test]
fn admin_starts_as_super_admin() {
    let ctx = setup();
    let roles = ctx.client.get_user_roles(&ctx.admin);
    assert!(roles.contains(&Role::SuperAdmin));
}

#[test]
fn grant_role_and_revoke_require_multisig() {
    let ctx = setup();
    let user = Address::generate(&ctx.env);

    let direct = ctx
        .client
        .try_grant_role(&ctx.admin, &user, &Role::Verifier);
    assert_eq!(direct, Err(Ok(ContractError::MultiSigRequired)));

    let revoke = ctx
        .client
        .try_revoke_role(&ctx.admin, &user, &Role::Verifier);
    assert_eq!(revoke, Err(Ok(ContractError::MultiSigRequired)));
}

#[test]
fn role_grant_flow_grants_role_after_approval_and_delay() {
    let ctx = setup();
    // A second SuperAdmin is needed to reach the 2-approval threshold; grant it
    // directly so the multi-sig machinery itself is what we exercise.
    let second_admin = Address::generate(&ctx.env);
    grant_roles_direct(&ctx, &second_admin, vec![&ctx.env, Role::SuperAdmin]);
    let user = Address::generate(&ctx.env);

    let now = ctx.env.ledger().timestamp();
    let proposal_id = ctx
        .client
        .propose_role_grant(&ctx.admin, &user, &Role::Verifier, &1, &0);

    let proposal = ctx.client.get_proposal(&proposal_id);
    // Minimums enforced: 2 approvals and a 24-hour delay.
    assert_eq!(proposal.required_approvals, 2);
    assert_eq!(proposal.execution_delay, 86400);

    ctx.client.approve_role_grant(&ctx.admin, &proposal_id);
    let result = ctx.client.try_execute_role_grant(&ctx.admin, &proposal_id);
    assert_eq!(result, Err(Ok(ContractError::ProposalNotReady)));

    ctx.client.approve_role_grant(&second_admin, &proposal_id);
    ctx.env.ledger().set_timestamp(now + 86401);
    ctx.client.execute_role_grant(&ctx.admin, &proposal_id);

    let roles = ctx.client.get_user_roles(&user);
    assert!(roles.contains(&Role::Verifier));

    // Executing again is rejected.
    let second = ctx.client.try_execute_role_grant(&ctx.admin, &proposal_id);
    assert_eq!(second, Err(Ok(ContractError::ProposalAlreadyExecuted)));
}

#[test]
fn role_grant_rejects_duplicate_role() {
    let ctx = setup();
    let second_admin = Address::generate(&ctx.env);
    grant_roles_direct(&ctx, &second_admin, vec![&ctx.env, Role::SuperAdmin]);
    let user = Address::generate(&ctx.env);

    let now = ctx.env.ledger().timestamp();
    let proposal_id = ctx
        .client
        .propose_role_grant(&ctx.admin, &user, &Role::Verifier, &2, &0);
    ctx.client.approve_role_grant(&ctx.admin, &proposal_id);
    ctx.client.approve_role_grant(&second_admin, &proposal_id);
    ctx.env.ledger().set_timestamp(now + 86401);
    ctx.client.execute_role_grant(&ctx.admin, &proposal_id);

    // Granting the same role again fails.
    let proposal_id = ctx
        .client
        .propose_role_grant(&ctx.admin, &user, &Role::Verifier, &2, &0);
    ctx.client.approve_role_grant(&ctx.admin, &proposal_id);
    ctx.client.approve_role_grant(&second_admin, &proposal_id);
    ctx.env.ledger().set_timestamp(now + 86401 + 86401);
    let result = ctx.client.try_execute_role_grant(&ctx.admin, &proposal_id);
    assert_eq!(result, Err(Ok(ContractError::RoleAlreadyGranted)));
}

#[test]
fn role_grant_approval_requires_manage_roles() {
    let ctx = setup();
    let stranger = Address::generate(&ctx.env);
    let user = Address::generate(&ctx.env);

    let proposal_id = ctx
        .client
        .propose_role_grant(&ctx.admin, &user, &Role::Verifier, &2, &0);

    let denied = ctx.client.try_approve_role_grant(&stranger, &proposal_id);
    assert_eq!(denied, Err(Ok(ContractError::InsufficientPermissions)));
}

#[test]
fn verifier_role_can_verify_vulnerabilities() {
    let ctx = setup();
    let verifier = Address::generate(&ctx.env);
    grant_roles_direct(&ctx, &verifier, vec![&ctx.env, Role::Verifier]);

    ctx.token_admin.mint(&ctx.admin, &1_000_000);
    ctx.client.fund_bounty_pool(&ctx.admin, &500_000);

    let report_id = report(
        &ctx,
        &Address::generate(&ctx.env),
        13,
        symbol_short!("high"),
    );
    ctx.client
        .verify_vulnerability(&verifier, &report_id, &100_000);
    let v = ctx.client.get_vulnerability(&report_id);
    assert_eq!(v.status, STATUS_VERIFIED);
}

#[test]
fn treasury_manager_can_fund_emergency_pool() {
    let ctx = setup();
    let treasurer = Address::generate(&ctx.env);
    grant_roles_direct(&ctx, &treasurer, vec![&ctx.env, Role::TreasuryManager]);
    let stranger = Address::generate(&ctx.env);

    ctx.token_admin.mint(&treasurer, &1_000_000);

    let denied = ctx.client.try_fund_emergency_pool(&stranger, &100_000);
    assert_eq!(denied, Err(Ok(ContractError::InsufficientPermissions)));

    ctx.client.fund_emergency_pool(&treasurer, &100_000);
    assert_eq!(ctx.client.get_emergency_pool_balance(), 100_000);
}

// ---------------------------------------------------------------------------
// Proposals
// ---------------------------------------------------------------------------

#[test]
fn get_proposal_not_found() {
    let ctx = setup();
    assert_eq!(
        ctx.client.try_get_proposal(&999),
        Err(Ok(ContractError::ProposalNotFound))
    );
}
