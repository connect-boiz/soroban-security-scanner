#![cfg(test)]


    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, token_id, token_admin) = setup_test(&env);
    let funder = Address::generate(&env);
    
    // Mint tokens to funder
    let token_client = token::StellarAssetClient::new(&env, &token_id);
    token_client.mint(&funder, &1000);
    
    // Fund bounty pool
    client.fund_bounty_pool(&funder, &token_id, &500);
    

}

#[test]
fn test_oracle_emergency_reward() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token_id, _token_admin) = setup_test(&env);
    
    // Register mock oracle
    let oracle_id = env.register_contract(None, MockOracle);
    client.set_oracle(&admin, &oracle_id);
    

    let contract_address = BytesN::from_array(&env, &[1u8; 32]);
    
    // Report emergency vulnerability
    let alert_id = client.report_emergency_vulnerability(
        &reporter,
        &contract_address,

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

    
    // Verification with 600 >= 500 should succeed
    client.verify_vulnerability(&admin, &report_id, &600);
}

#[test]
fn test_oracle_integration() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SecurityScannerContract);
    let client = SecurityScannerContract::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let oracle_address = Address::generate(&env);
    
    // Initialize contract
    client.initialize(&admin);
    
    // Configure oracle
    client.configure_oracle(&admin, &oracle_address, true);
    
    // Check oracle is enabled
    let emergency_config = client.get_emergency_config();
    assert!(emergency_config.oracle_enabled);
    assert_eq!(emergency_config.price_feed_address, oracle_address);
}

#[test]
fn test_slippage_protection() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SecurityScannerContract);
    let client = SecurityScannerContract::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    
    // Initialize contract
    client.initialize(&admin);
    
    // Set slippage tolerance to 3% (300 basis points)
    client.set_slippage_tolerance(&admin, 300i128);
    
    // Test with invalid tolerance (should fail)
    let result = client.try_set_slippage_tolerance(&admin, 15000i128);
    assert!(result.is_err());
}

#[test]
fn test_multi_token_support() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SecurityScannerContract);
    let client = SecurityScannerContract::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    
    // Initialize contract
    client.initialize(&admin);
    
    // Add multiple supported tokens
    let token1 = Address::generate(&env);
    let token2 = Address::generate(&env);
    
    client.add_supported_token(&admin, &token1, 7, &1000000i128);
    client.add_supported_token(&admin, &token2, 18, &500000i128);
    
    // Get supported tokens
    let supported_tokens = client.get_supported_tokens();
    assert!(supported_tokens.contains_key(token1));
    assert!(supported_tokens.contains_key(token2));
    
    // Test unsupported token (should fail)
    let unsupported_token = Address::generate(&env);
    let result = client.try_report_vulnerability(
        &Address::generate(&env),
        &BytesN::from_array(&env, &[1u8; 32]),
        &String::from_str(&env, "Test"),
        &String::from_str(&env, "low"),
        &String::from_str(&env, "Test description"),
        &String::from_str(&env, "test.rs:1"),
        &unsupported_token,
    );
    assert!(result.is_err());
}

#[test]
fn test_liquidity_management() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SecurityScannerContract);
    let client = SecurityScannerContract::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    
    // Initialize contract
    client.initialize(&admin);
    
    // Add supported token
    let token_address = Address::generate(&env);
    client.add_supported_token(&admin, &token_address, 7, &1000000i128);
    
    // Fund bounty pool
    client.fund_bounty_pool(&admin, &token_address, &10000000i128);
    
    // Check liquidity
    let balance = client.get_bounty_pool(token_address).unwrap();
    assert_eq!(balance, 10000000);
    
    // Get liquidity info
    let liquidity_info = client.get_liquidity_info(token_address).unwrap();
    assert_eq!(liquidity_info.balance, 10000000);
}
