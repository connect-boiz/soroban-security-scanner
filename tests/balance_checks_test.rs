//! Comprehensive tests for enhanced balance checking functionality
//! 
//! This test suite validates the improved balance check patterns and
//! ensures they catch all edge cases mentioned in the issue.

use stellar_security_scanner::{scanners::SecurityScanner, vulnerabilities::VulnerabilityType};
use std::path::Path;

#[test]
fn test_insufficient_balance_detection() {
    let scanner = SecurityScanner::new().unwrap();
    let test_code = r#"
        pub fn transfer(from: Address, to: Address, amount: i64) {
            // Missing balance check - should be detected
            let from_balance = get_balance(from);
            let to_balance = get_balance(to);
            
            // Direct balance subtraction without sufficient balance check
            balances.insert(from, from_balance - amount);
            balances.insert(to, to_balance + amount);
        }
    "#;
    
    let test_file = Path::new("test_transfer.rs");
    std::fs::write(test_file, test_code).unwrap();
    
    let result = scanner.scan_file(test_file).unwrap();
    assert!(result.vulnerabilities.contains(&VulnerabilityType::InsufficientBalance));
    assert!(result.vulnerabilities.contains(&VulnerabilityType::TransferWithoutBalanceCheck));
    
    std::fs::remove_file(test_file).unwrap();
}

#[test]
fn test_balance_underflow_detection() {
    let scanner = SecurityScanner::new().unwrap();
    let test_code = r#"
        pub fn withdraw(amount: i64) {
            let current_balance = get_balance(env::current_contract_address());
            
            // Unsafe subtraction without underflow protection
            let new_balance = current_balance - amount;
            balances.insert(env::current_contract_address(), new_balance);
        }
    "#;
    
    let test_file = Path::new("test_withdraw.rs");
    std::fs::write(test_file, test_code).unwrap();
    
    let result = scanner.scan_file(test_file).unwrap();
    assert!(result.vulnerabilities.contains(&VulnerabilityType::BalanceUnderflow));
    
    std::fs::remove_file(test_file).unwrap();
}

#[test]
fn test_balance_overflow_detection() {
    let scanner = SecurityScanner::new().unwrap();
    let test_code = r#"
        pub fn deposit(amount: i64) {
            let current_balance = get_balance(env::current_contract_address());
            
            // Unsafe addition without overflow protection
            let new_balance = current_balance + amount;
            balances.insert(env::current_contract_address(), new_balance);
        }
    "#;
    
    let test_file = Path::new("test_deposit.rs");
    std::fs::write(test_file, test_code).unwrap();
    
    let result = scanner.scan_file(test_file).unwrap();
    assert!(result.vulnerabilities.contains(&VulnerabilityType::BalanceOverflow));
    
    std::fs::remove_file(test_file).unwrap();
}

#[test]
fn test_proper_balance_check_passes() {
    let scanner = SecurityScanner::new().unwrap();
    let test_code = r#"
        pub fn safe_transfer(from: Address, to: Address, amount: i64) {
            let from_balance = get_balance(from);
            
            // Proper balance check - should not be detected
            require!(from_balance >= amount, "Insufficient balance");
            
            let to_balance = get_balance(to);
            
            // Safe arithmetic operations
            let new_from_balance = from_balance.checked_sub(amount).unwrap();
            let new_to_balance = to_balance.checked_add(amount).unwrap();
            
            balances.insert(from, new_from_balance);
            balances.insert(to, new_to_balance);
        }
    "#;
    
    let test_file = Path::new("test_safe_transfer.rs");
    std::fs::write(test_file, test_code).unwrap();
    
    let result = scanner.scan_file(test_file).unwrap();
    assert!(!result.vulnerabilities.contains(&VulnerabilityType::InsufficientBalance));
    assert!(!result.vulnerabilities.contains(&VulnerabilityType::BalanceUnderflow));
    assert!(!result.vulnerabilities.contains(&VulnerabilityType::BalanceOverflow));
    
    std::fs::remove_file(test_file).unwrap();
}

#[test]
fn test_multiple_balance_operations_edge_cases() {
    let scanner = SecurityScanner::new().unwrap();
    let test_code = r#"
        pub fn complex_transfer(from: Address, to: Address, amount: i64, fee: i64) {
            let from_balance = get_balance(from);
            
            // Missing comprehensive balance check for total amount
            require!(from_balance >= amount, "Insufficient balance for transfer");
            
            let to_balance = get_balance(to);
            
            // Multiple balance operations without proper validation
            balances.insert(from, from_balance - amount);  // Unsafe
            balances.insert(to, to_balance + amount);      // Unsafe
            
            // Fee deduction without checking if user has enough for both
            let final_balance = get_balance(from);
            balances.insert(from, final_balance - fee);     // Unsafe
        }
    "#;
    
    let test_file = Path::new("test_complex_transfer.rs");
    std::fs::write(test_file, test_code).unwrap();
    
    let result = scanner.scan_file(test_file).unwrap();
    assert!(result.vulnerabilities.contains(&VulnerabilityType::InsufficientBalance));
    assert!(result.vulnerabilities.contains(&VulnerabilityType::BalanceUnderflow));
    
    std::fs::remove_file(test_file).unwrap();
}

#[test]
fn test_bounds_validation_missing() {
    let scanner = SecurityScanner::new().unwrap();
    let test_code = r#"
        pub fn mint_unbounded(amount: i64) {
            let current_supply = total_supply();
            let new_supply = current_supply + amount;  // No max limit check
            
            total_supply.set(new_supply);
            
            let user_balance = get_balance(env::current_contract_address());
            let new_balance = user_balance + amount;   // No per-account limit
            balances.insert(env::current_contract_address(), new_balance);
        }
    "#;
    
    let test_file = Path::new("test_mint_unbounded.rs");
    std::fs::write(test_file, test_code).unwrap();
    
    let result = scanner.scan_file(test_file).unwrap();
    assert!(result.vulnerabilities.contains(&VulnerabilityType::BalanceOverflow));
    
    std::fs::remove_file(test_file).unwrap();
}
