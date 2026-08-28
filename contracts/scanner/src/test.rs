#![cfg(test)]
extern crate std;

use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, BytesN, Env, String};

struct TestContext<'a> {
    env: Env,
    admin: Address,
    client: SecurityScannerContractClient<'a>,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
}

fn setup() -> TestContext<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(issuer.clone());
    let token_id = token_contract.address();
    let token = token::Client::new(&env, &token_id);
    let token_admin = token::StellarAssetClient::new(&env, &token_id);

    let contract_id = env.register(SecurityScannerContract, ());
    let client = SecurityScannerContractClient::new(&env, &contract_id);
    client.initialize(&admin, &token_id);

    TestContext {
        env,
        admin,
        client,
        token,
        token_admin,
    }
}

#[test]
fn initialize_rejects_default_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let issuer = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(issuer.clone());
    let token_id = token_contract.address();
    let contract_id = env.register(SecurityScannerContract, ());
    let client = SecurityScannerContractClient::new(&env, &contract_id);

    let null_admin = Address::from_string(&String::from_str(
        &env,
        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
    ));

    let result = client.try_initialize(&null_admin, &token_id);
    assert!(result.is_err());
}

#[test]
fn verify_vulnerability_rejects_double_payout() {
    let ctx = setup();
    let reporter = Address::generate(&ctx.env);

    ctx.token_admin.mint(&ctx.admin, &1_000_000);
    ctx.client.fund_bounty_pool(&ctx.admin, &500_000);

    let report_id = ctx.client.report_vulnerability(
        &reporter,
        &BytesN::from_array(&ctx.env, &[1u8; 32]),
        &String::from_str(&ctx.env, "reentrancy"),
        &String::from_str(&ctx.env, "high"),
        &String::from_str(&ctx.env, "critical bug"),
        &String::from_str(&ctx.env, "lib.rs"),
    );

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

    let report_id = ctx.client.report_vulnerability(
        &reporter,
        &BytesN::from_array(&ctx.env, &[2u8; 32]),
        &String::from_str(&ctx.env, "overflow"),
        &String::from_str(&ctx.env, "critical"),
        &String::from_str(&ctx.env, "unchecked math"),
        &String::from_str(&ctx.env, "token.rs"),
    );

    ctx.client
        .verify_vulnerability(&ctx.admin, &report_id, &150_000);

    assert_eq!(ctx.client.get_bounty_pool(), 350_000);
    assert_eq!(ctx.token.balance(&reporter), 150_000);

    let reputation = ctx.client.get_reputation(&reporter);
    assert_eq!(reputation.successful_reports, 1);
    assert_eq!(reputation.total_earnings, 150_000);
}

#[test]
fn high_bounty_proposal_enforces_minimum_approvals() {
    let ctx = setup();

    let report_id = ctx.client.report_vulnerability(
        &Address::generate(&ctx.env),
        &BytesN::from_array(&ctx.env, &[3u8; 32]),
        &String::from_str(&ctx.env, "auth"),
        &String::from_str(&ctx.env, "high"),
        &String::from_str(&ctx.env, "missing auth"),
        &String::from_str(&ctx.env, "admin.rs"),
    );

    let proposal_id = ctx
        .client
        .propose_high_bounty_verification(&ctx.admin, &report_id, &2_000_000, &1, &0);

    let proposal = ctx.client.get_proposal(&proposal_id);
    assert_eq!(proposal.required_approvals, MIN_HIGH_BOUNTY_APPROVALS);
    assert!(!ctx.client.can_execute_proposal_check(&proposal_id));
}

#[test]
fn reputation_uses_address_keyed_storage() {
    let ctx = setup();
    let researcher_a = Address::generate(&ctx.env);
    let researcher_b = Address::generate(&ctx.env);

    ctx.token_admin.mint(&ctx.admin, &2_000_000);
    ctx.client.fund_bounty_pool(&ctx.admin, &1_000_000);

    let report_a = ctx.client.report_vulnerability(
        &researcher_a,
        &BytesN::from_array(&ctx.env, &[4u8; 32]),
        &String::from_str(&ctx.env, "x"),
        &String::from_str(&ctx.env, "high"),
        &String::from_str(&ctx.env, "a"),
        &String::from_str(&ctx.env, "a.rs"),
    );
    let report_b = ctx.client.report_vulnerability(
        &researcher_b,
        &BytesN::from_array(&ctx.env, &[5u8; 32]),
        &String::from_str(&ctx.env, "y"),
        &String::from_str(&ctx.env, "high"),
        &String::from_str(&ctx.env, "b"),
        &String::from_str(&ctx.env, "b.rs"),
    );

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
fn release_escrow_succeeds_without_signer_when_depositor_auth() {
    let ctx = setup();
    let depositor = ctx.admin.clone();
    let beneficiary = Address::generate(&ctx.env);

    ctx.token_admin.mint(&depositor, &500_000);

    let escrow_id = ctx.client.create_escrow(
        &depositor,
        &beneficiary,
        &100_000,
        &String::from_str(&ctx.env, "bounty"),
        &0,
        &None,
    );

    ctx.client
        .mark_escrow_conditions_met(&escrow_id, &ctx.admin);

    ctx.client.release_escrow(&escrow_id, &depositor, &None);

    let escrow = ctx.client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, String::from_str(&ctx.env, "released"));
    assert_eq!(ctx.token.balance(&beneficiary), 100_000);
}
