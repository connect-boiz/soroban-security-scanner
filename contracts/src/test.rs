#![cfg(test)]

use soroban_sdk::{symbol, Address, BytesN, Env, String, Map};
use crate::{SecurityScannerContract, ContractError, VulnerabilityReport, TokenInfo};

#[test]
fn test_initialize() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SecurityScannerContract);
    let client = SecurityScannerContract::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    
    // Initialize contract
    client.initialize(&admin);
    
    // Verify admin is set
    assert_eq!(client.get_bounty_pool(Address::generate(&env)).unwrap(), 0);
    
    // Check emergency config is initialized
    let emergency_config = client.get_emergency_config();
    assert_eq!(emergency_config.base_amount, 1000000);
    assert!(!emergency_config.oracle_enabled);
}

#[test]
fn test_report_vulnerability() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SecurityScannerContract);
    let client = SecurityScannerContract::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let reporter = Address::generate(&env);
    
    // Initialize contract
    client.initialize(&admin);
    
    // Add a supported token for testing
    let token_address = Address::generate(&env);
    client.add_supported_token(&admin, &token_address, 7, &1000000i128);
    
    // Report vulnerability
    let contract_address = BytesN::from_array(&env, &[1u8; 32]);
    let vuln_type = String::from_str(&env, "Access Control");
    let severity = String::from_str(&env, "critical");
    let description = String::from_str(&env, "Missing access control");
    let location = String::from_str(&env, "src/lib.rs:45");
    
    let report_id = client.report_vulnerability(
        &reporter,
        &contract_address,
        &vuln_type,
        &severity,
        &description,
        &location,
        &token_address,
    );
    
    // Verify report was created
    let report = client.get_vulnerability(report_id).unwrap();
    assert_eq!(report.reporter, reporter);
    assert_eq!(report.contract_id, contract_address);
    assert_eq!(report.vulnerability_type, vuln_type);
    assert_eq!(report.status, String::from_str(&env, "pending"));
    assert_eq!(report.token_address, token_address);
}

#[test]
fn test_verify_vulnerability() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SecurityScannerContract);
    let client = SecurityScannerContract::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let reporter = Address::generate(&env);
    
    // Initialize contract
    client.initialize(&admin);
    
    // Add a supported token for testing
    let token_address = Address::generate(&env);
    client.add_supported_token(&admin, &token_address, 7, &1000000i128);
    
    // Fund the bounty pool
    client.fund_bounty_pool(&admin, &token_address, &10000000i128);
    
    // Report vulnerability
    let contract_address = BytesN::from_array(&env, &[1u8; 32]);
    let vuln_type = String::from_str(&env, "Access Control");
    let severity = String::from_str(&env, "critical");
    let description = String::from_str(&env, "Missing access control");
    let location = String::from_str(&env, "src/lib.rs:45");
    
    let report_id = client.report_vulnerability(
        &reporter,
        &contract_address,
        &vuln_type,
        &severity,
        &description,
        &location,
        &token_address,
    );
    
    // Verify vulnerability and award bounty (using custom amount)
    client.verify_vulnerability(&admin, report_id, Some(1000000i128));
    
    // Check updated report
    let report = client.get_vulnerability(report_id).unwrap();
    assert_eq!(report.status, String::from_str(&env, "verified"));
    assert_eq!(report.bounty_amount, 1000000);
    assert_eq!(report.token_address, token_address);
    
    // Check reputation
    let reputation = client.get_reputation(reporter).unwrap();
    assert_eq!(reputation.successful_reports, 1);
    assert_eq!(reputation.total_earnings, 1000000);
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
