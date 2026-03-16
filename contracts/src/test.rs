#![cfg(test)]

use soroban_sdk::{symbol, Address, BytesN, Env, String};
use crate::{SecurityScannerContract, ContractError, VulnerabilityReport};

#[test]
fn test_initialize() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SecurityScannerContract);
    let client = SecurityScannerContract::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    
    // Initialize contract
    client.initialize(&admin);
    
    // Verify admin is set
    assert_eq!(client.get_bounty_pool(), 0);
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
    );
    
    // Verify report was created
    let report = client.get_vulnerability(report_id).unwrap();
    assert_eq!(report.reporter, reporter);
    assert_eq!(report.contract_id, contract_address);
    assert_eq!(report.vulnerability_type, vuln_type);
    assert_eq!(report.status, String::from_str(&env, "pending"));
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
    );
    
    // Verify vulnerability and award bounty
    client.verify_vulnerability(&admin, report_id, &1000000);
    
    // Check updated report
    let report = client.get_vulnerability(report_id).unwrap();
    assert_eq!(report.status, String::from_str(&env, "verified"));
    assert_eq!(report.bounty_amount, 1000000);
    
    // Check reputation
    let reputation = client.get_reputation(reporter).unwrap();
    assert_eq!(reputation.successful_reports, 1);
    assert_eq!(reputation.total_earnings, 1000000);
}
