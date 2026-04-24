#![cfg(test)]

use soroban_sdk::{Env, Address, BytesN};
use crate::SecurityScannerContract;

#[test]
fn test_gas_limit_configuration() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token_id, _token_admin) = setup_test(&env);
    
    // Test default gas configuration
    let gas_config = client.get_gas_config();
    assert_eq!(gas_config.single_transfer_limit, 100_000);
    assert_eq!(gas_config.batch_transfer_limit, 500_000);
    assert_eq!(gas_config.emergency_limit, 200_000);
    assert_eq!(gas_config.max_batch_size, 50);
    assert_eq!(gas_config.gas_price_multiplier, 100);
    
    // Test updating gas configuration
    client.update_gas_config(
        &admin,
        Some(150_000),
        Some(750_000),
        Some(300_000),
        Some(100),
        Some(150)
    ).unwrap();
    
    let updated_config = client.get_gas_config();
    assert_eq!(updated_config.single_transfer_limit, 150_000);
    assert_eq!(updated_config.batch_transfer_limit, 750_000);
    assert_eq!(updated_config.emergency_limit, 300_000);
    assert_eq!(updated_config.max_batch_size, 100);
    assert_eq!(updated_config.gas_price_multiplier, 150);
}

#[test]
fn test_transfer_bounty_gas_limits() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, token_id, token_admin) = setup_test(&env);
    
    let researcher = Address::generate(&env);
    let funder = Address::generate(&env);
    
    // Mint tokens to contract
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_client.mint(&funder, &10000);
    
    // Fund bounty pool
    client.fund_bounty_pool(&funder, &token_id, &5000);
    
    // Test successful transfer with sufficient gas
    env.as_contract(&client.address, || {
        client.verify_vulnerability(
            &funder,
            1, // report_id
            &String::from_str(&env, "critical"),
            &5000i128,
            &token_id
        ).unwrap();
    });
    
    // Test transfer with insufficient gas (mock low gas scenario)
    env.set_budget(50_000); // Less than required
    let result = env.as_contract(&client.address, || {
        client.verify_vulnerability(
            &funder,
            2, // report_id
            &String::from_str(&env, "high"),
            &3000i128,
            &token_id
        )
    });
    
    assert!(result.is_err());
    // Note: In real environment, this would fail with InsufficientGas error
}

#[test]
fn test_batch_escrow_release_gas_management() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token_id, token_admin) = setup_test(&env);
    
    let recipients = vec![
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];
    let amounts = vec![1000i128, 1500i128, 2000i128];
    let tokens = vec![token_id.clone(), token_id.clone(), token_id.clone()];
    
    // Test successful batch release with sufficient gas
    let batch_id = client.batch_escrow_release(
        &admin,
        recipients.clone(),
        amounts.clone(),
        tokens.clone(),
    ).unwrap();
    
    // Verify batch status
    let batch_status = client.get_batch_status(batch_id).unwrap();
    assert_eq!(batch_status.status, String::from_str(&env, "completed"));
    assert_eq!(batch_status.recipients.len(), 3);
    assert_eq!(batch_status.total_amount, 4500i128);
    
    // Test batch with too many recipients
    let large_recipients: Vec<Address> = (0..60).map(|_| Address::generate(&env)).collect();
    let large_amounts: Vec<i128> = (0..60).map(|_| 100i128).collect();
    let large_tokens: Vec<Address> = (0..60).map(|_| token_id.clone()).collect();
    
    let result = client.batch_escrow_release(
        &admin,
        large_recipients,
        large_amounts,
        large_tokens,
    );
    
    assert!(result.is_err());
    // Should fail with BatchTooLarge error
}

#[test]
fn test_emergency_reward_distribution_gas_limits() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token_id, token_admin) = setup_test(&env);
    
    let reporter = Address::generate(&env);
    let contract_address = BytesN::from_array(&env, &[1u8; 32]);
    
    // Report multiple emergency vulnerabilities
    let alert1_id = client.report_emergency_vulnerability(
        &reporter,
        &contract_address,
        &String::from_str(&env, "critical"),
        &String::from_str(&env, "Emergency vulnerability 1"),
        &String::from_str(&env, "contract.rs:100"),
        &token_id
    ).unwrap();
    
    let alert2_id = client.report_emergency_vulnerability(
        &reporter,
        &contract_address,
        &String::from_str(&env, "critical"),
        &String::from_str(&env, "Emergency vulnerability 2"),
        &String::from_str(&env, "contract.rs:200"),
        &token_id
    ).unwrap();
    
    // Test successful emergency reward distribution
    let batch_id = client.emergency_reward_distribution(
        &admin,
        vec![alert1_id, alert2_id],
        &token_id
    ).unwrap();
    
    // Verify emergency batch status
    let batch_status = client.get_emergency_batch_status(batch_id).unwrap();
    assert_eq!(batch_status.status, String::from_str(&env, "completed"));
    assert_eq!(batch_status.alert_ids.len(), 2);
    assert!(batch_status.total_reward > 0);
    
    // Test emergency distribution with insufficient gas
    env.set_budget(100_000); // Less than required for emergency
    let result = client.emergency_reward_distribution(
        &admin,
        vec![alert1_id],
        &token_id
    );
    
    assert!(result.is_err());
    // Should fail with InsufficientGas error
}

#[test]
fn test_gas_limit_vulnerability_detection() {
    use soroban_security_scanner_core::analyzer::SecurityAnalyzer;
    use soroban_security_scanner_core::scan_controller::ScanController;
    
    let env = Env::default();
    let scan_controller = ScanController::new();
    let analyzer = SecurityAnalyzer::new(true, true, Some(scan_controller));
    
    // Test code with unbounded loop (should trigger GAS-001)
    let vulnerable_code = r#"
    pub fn vulnerable_function(env: Env, data: Vec<u64>) {
        for i in 0..data.len() {
            env.storage().instance().set(&Symbol::short(&format!("DATA_{}", i)), &data[i]);
            // No gas limit check - can exhaust transaction gas
        }
    }
    "#;
    
    let report = analyzer.analyze(vulnerable_code, "vulnerable.rs").unwrap();
    let gas_vulnerabilities: Vec<_> = report.vulnerabilities
        .iter()
        .filter(|v| v.vulnerability_type.to_string().contains("StellarSpecific"))
        .filter(|v| v.id.starts_with("GAS-"))
        .collect();
    
    assert!(!gas_vulnerabilities.is_empty());
    
    // Test code with missing gas validation (should trigger GAS-002)
    let missing_validation_code = r#"
    pub fn emergency_release(env: Env, recipients: Vec<Address>) {
        for recipient in recipients.iter() {
            let token_client = token::Client::new(&env, &some_token_address);
            token_client.transfer(&env.current_contract_address(), recipient, &1000i128);
            // No gas limit validation before transfers
        }
    }
    "#;
    
    let report2 = analyzer.analyze(missing_validation_code, "missing_validation.rs").unwrap();
    let gas_vulnerabilities2: Vec<_> = report2.vulnerabilities
        .iter()
        .filter(|v| v.id.starts_with("GAS-"))
        .collect();
    
    assert!(!gas_vulnerabilities2.is_empty());
}

fn setup_test(env: &Env) -> (SecurityScannerContract, Address, BytesN<32>, Address) {
    let contract_id = env.register_contract(None, SecurityScannerContract);
    let client = SecurityScannerContract::new(env, &contract_id);
    
    let admin = Address::generate(env);
    client.initialize(&admin);
    
    let token_id = env.register_contract(None, soroban_sdk::token::StellarAssetContract);
    let token_admin = Address::generate(env);
    
    soroban_sdk::token::StellarAssetClient::new(env, &token_id)
        .initialize(&token_admin, &7, &"Test Token", &"TEST", &1000000000);
    
    client.add_supported_token(&admin, &token_id, 7, &1000000i128);
    
    (client, admin, token_id, token_admin)
}
