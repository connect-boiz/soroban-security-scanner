#![cfg(test)]

use soroban_sdk::{symbol_short, Address, BytesN, Env, String, testutils::{Address as _}, token};
use crate::{SecurityScannerContract, ContractError, VulnerabilityReport, SecurityScannerContractClient};

// Mock Oracle Contract
pub struct MockOracle;

#[soroban_sdk::contractimpl]
impl MockOracle {
    pub fn get_reward(env: Env, severity: String) -> i128 {
        if severity == String::from_str(&env, "emergency") {
            20000000i128 // 20 XLM
        } else {
            10000000i128 // 10 XLM
        }
    }
}

fn setup_test(env: &Env) -> (SecurityScannerContractClient<'_>, Address, Address, Address) {
    let contract_id = env.register_contract(None, SecurityScannerContract);
    let client = SecurityScannerContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone()).address();
    
    client.initialize(&admin);
    client.add_token(&admin, &token_id); // Whitelist the default token (#125)
    (client, admin, token_id, token_admin)
}

#[test]
fn test_multi_token_bounty_pool() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, token_id, token_admin) = setup_test(&env);
    let funder = Address::generate(&env);
    
    // Mint tokens to funder
    let token_client = token::StellarAssetClient::new(&env, &token_id);
    token_client.mint(&funder, &1000);
    
    // Fund bounty pool
    client.fund_bounty_pool(&funder, &token_id, &500);
    
    assert_eq!(client.get_bounty_pool(&token_id), 500);
    assert_eq!(token::Client::new(&env, &token_id).balance(&client.address), 500);
}

#[test]
fn test_oracle_emergency_reward() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token_id, _token_admin) = setup_test(&env);
    
    // Register mock oracle
    let oracle_id = env.register_contract(None, MockOracle);
    client.set_oracle(&admin, &oracle_id);
    
    let reporter = Address::generate(&env);
    let contract_address = BytesN::from_array(&env, &[1u8; 32]);
    
    // Report emergency vulnerability
    let alert_id = client.report_emergency_vulnerability(
        &reporter,
        &contract_address,
        &String::from_str(&env, "Critical Bug"),
        &String::from_str(&env, "emergency"),
        &String::from_str(&env, "Exploit found"),
        &String::from_str(&env, "lib.rs"),
        &token_id,
        &15000000i128, // min_reward
    );
    
    let alert = client.get_emergency_alert(&alert_id).unwrap();
    assert_eq!(alert.emergency_reward, 20000000i128); // From Oracle
}

#[test]
fn test_slippage_protection() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, token_id, _token_admin) = setup_test(&env);
    
    let reporter = Address::generate(&env);
    let contract_address = BytesN::from_array(&env, &[1u8; 32]);
    
    // This should fail because 10,000,000 (hardcoded fallback) < 15,000,000 (min_reward)
    let result = env.as_contract(&client.address, || {
        client.try_report_emergency_vulnerability(
            &reporter,
            &contract_address,
            &String::from_str(&env, "Critical Bug"),
            &String::from_str(&env, "emergency"),
            &String::from_str(&env, "Exploit found"),
            &String::from_str(&env, "lib.rs"),
            &token_id,
            &15000000i128,
        )
    });
    
    assert!(result.is_err());
}

#[test]
fn test_admin_withdraw_liquidity() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token_id, token_admin) = setup_test(&env);
    
    // Fund contract
    let funder = Address::generate(&env);
    let token_client = token::StellarAssetClient::new(&env, &token_id);
    token_client.mint(&funder, &1000);
    client.fund_bounty_pool(&funder, &token_id, &1000);
    
    let treasury = Address::generate(&env);
    client.admin_withdraw(&admin, &token_id, &400, &treasury);
    
    assert_eq!(token::Client::new(&env, &token_id).balance(&treasury), 400);
    assert_eq!(token::Client::new(&env, &token_id).balance(&client.address), 600);
}

#[test]
fn test_escrow_multi_token() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token_id, token_admin) = setup_test(&env);
    
    let depositor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    
    token::StellarAssetClient::new(&env, &token_id).mint(&depositor, &1000);
    
    let escrow_id = client.create_escrow(
        &depositor,
        &beneficiary,
        &500,
        &token_id,
        &String::from_str(&env, "bounty"),
        &100,
    );
    
    assert_eq!(client.get_escrow_pool_balance(&token_id), 500);
    
    // Release escrow
    client.mark_escrow_conditions_met(&admin, &escrow_id);
    client.release_escrow(&escrow_id, &depositor, &None);
    
    assert_eq!(token::Client::new(&env, &token_id).balance(&beneficiary), 500);
    assert_eq!(client.get_escrow_pool_balance(&token_id), 0);
}

#[test]
fn test_token_whitelisting() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token_id, _token_admin) = setup_test(&env);
    
    let other_token = Address::generate(&env);
    let reporter = Address::generate(&env);
    
    // Reporting with non-whitelisted token should fail
    let result = env.as_contract(&client.address, || {
        client.try_report_vulnerability(
            &reporter,
            &BytesN::from_array(&env, &[0u8; 32]),
            &String::from_str(&env, "Bug"),
            &String::from_str(&env, "medium"),
            &String::from_str(&env, "desc"),
            &String::from_str(&env, "loc"),
            &other_token,
            &0,
        )
    });
    assert!(result.is_err());
    
    // Add token and it should succeed
    client.add_token(&admin, &other_token);
    let _report_id = client.report_vulnerability(
        &reporter,
        &BytesN::from_array(&env, &[0u8; 32]),
        &String::from_str(&env, "Bug"),
        &String::from_str(&env, "medium"),
        &String::from_str(&env, "desc"),
        &String::from_str(&env, "loc"),
        &other_token,
        &0,
    );
}

#[test]
fn test_liquidity_management_insufficient_pool() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token_id, _token_admin) = setup_test(&env);
    
    let reporter = Address::generate(&env);
    let report_id = client.report_vulnerability(
        &reporter,
        &BytesN::from_array(&env, &[0u8; 32]),
        &String::from_str(&env, "Bug"),
        &String::from_str(&env, "medium"),
        &String::from_str(&env, "desc"),
        &String::from_str(&env, "loc"),
        &token_id,
        &0,
    );
    
    // Verification should fail if pool is empty
    let result = env.as_contract(&client.address, || {
        client.try_verify_vulnerability(&admin, &report_id, &1000)
    });
    assert!(result.is_err());
    
    // Fund pool and it should succeed
    let funder = Address::generate(&env);
    token::StellarAssetClient::new(&env, &token_id).mint(&funder, &1000);
    client.fund_bounty_pool(&funder, &token_id, &1000);
    
    client.verify_vulnerability(&admin, &report_id, &1000);
    assert_eq!(client.get_bounty_pool(&token_id), 0);
}

#[test]
fn test_slippage_protection_standard_report() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token_id, _token_admin) = setup_test(&env);
    
    let reporter = Address::generate(&env);
    let report_id = client.report_vulnerability(
        &reporter,
        &BytesN::from_array(&env, &[0u8; 32]),
        &String::from_str(&env, "Bug"),
        &String::from_str(&env, "medium"),
        &String::from_str(&env, "desc"),
        &String::from_str(&env, "loc"),
        &token_id,
        &500, // min_bounty
    );
    
    // Fund pool
    let funder = Address::generate(&env);
    token::StellarAssetClient::new(&env, &token_id).mint(&funder, &1000);
    client.fund_bounty_pool(&funder, &token_id, &1000);
    
    // Verification with 400 < 500 should fail
    let result = env.as_contract(&client.address, || {
        client.try_verify_vulnerability(&admin, &report_id, &400)
    });
    assert!(result.is_err());
    
    // Verification with 600 >= 500 should succeed
    client.verify_vulnerability(&admin, &report_id, &600);
}
