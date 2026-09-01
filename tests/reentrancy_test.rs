use soroban_sdk::{Address, Env, Symbol};
use soroban_security_scanner::escrow::{Escrow, EscrowData};
use soroban_security_scanner::escrow::EscrowClient;

#[test]
fn test_reentrancy_security() {
    let env = Env::default();
    let contract_id = env.register_contract(None, Escrow);
    let client = EscrowClient::new(&env, &contract_id);
    
    let beneficiary = Address::generate(&env);
    let amount = 1000;
    
    // Create escrow
    client.create_escrow(&beneficiary, &amount);
    
    // Test secure version - this should be safe from reentrancy
    let escrow_info_before = client.get_escrow_info();
    assert!(!escrow_info_before.released);
    
    // This call is now secure against reentrancy
    client.release();
    
    let escrow_info_after = client.get_escrow_info();
    assert!(escrow_info_after.released);
    
    // Verify that calling release again fails
    let result = std::panic::catch_unwind(|| {
        client.release();
    });
    assert!(result.is_err());
}


#[test]
fn test_security_analyzer() {
    let env = Env::default();
    let analyzer = soroban_security_scanner::security_analyzer::SecurityAnalyzer;
    
    // Analyze for reentrancy vulnerabilities
    let report = analyzer.analyze_reentrancy(&env);
    
    // The report should now be secure since vulnerability is fixed
    assert!(report.is_secure());
    assert!(!report.has_high_severity());
}
