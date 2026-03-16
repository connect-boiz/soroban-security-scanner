//! Example contract with security vulnerabilities for testing

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

#[contract]
pub struct VulnerableToken;

#[contractimpl]
impl VulnerableToken {
    // VULNERABILITY: Missing access control
    pub fn mint(env: Env, to: Address, amount: i128) {
        let mut balance = env.storage().persistent().get::<Address, i128>(&to).unwrap_or(0);
        balance += amount; // VULNERABILITY: Integer overflow possible
        env.storage().persistent().set(&to, &balance);
        
        // VULNERABILITY: No total supply tracking
        // VULNERABILITY: Missing event emission
    }

    // VULNERABILITY: Missing access control
    pub fn burn(env: Env, from: Address, amount: i128) {
        let mut balance = env.storage().persistent().get::<Address, i128>(&from).unwrap_or(0);
        balance -= amount; // VULNERABILITY: Integer underflow possible
        env.storage().persistent().set(&from, &balance);
    }

    // VULNERABILITY: No access control on transfer
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        let mut from_balance = env.storage().persistent().get::<Address, i128>(&from).unwrap_or(0);
        let mut to_balance = env.storage().persistent().get::<Address, i128>(&to).unwrap_or(0);
        
        // VULNERABILITY: No balance check before transfer
        from_balance -= amount;
        to_balance += amount;
        
        env.storage().persistent().set(&from, &from_balance);
        env.storage().persistent().set(&to, &to_balance);
        
        // VULNERABILITY: Missing event emission
    }

    // VULNERABILITY: Hardcoded admin address
    pub fn admin_only_function(env: Env, data: Symbol) {
        let hardcoded_admin = Address::from_string(&soroban_sdk::String::from_str(&env, "GDUK..."));
        if env.current_contract_address() != hardcoded_admin {
            panic!("Not authorized"); // VULNERABILITY: Poor error handling
        }
        
        // VULNERABILITY: No input validation
        env.storage().persistent().set(&data, &"admin_data");
    }

    // VULNERABILITY: Potential reentrancy
    pub fn withdraw(env: Env, amount: i128) {
        let caller = env.current_contract_address();
        let mut balance = env.storage().persistent().get::<Address, i128>(&caller).unwrap_or(0);
        
        if balance >= amount {
            balance -= amount;
            env.storage().persistent().set(&caller, &balance);
            
            // VULNERABILITY: External call after state change (reentrancy risk)
            // This would be an external contract call in a real scenario
            env.storage().temporary().set(&"withdraw_event", &amount);
        }
    }

    // VULNERABILITY: No access control
    pub fn set_total_supply(env: Env, new_supply: i128) {
        env.storage().persistent().set(&"total_supply", &new_supply);
    }

    pub fn get_balance(env: Env, account: Address) -> i128 {
        env.storage().persistent().get::<Address, i128>(&account).unwrap_or(0)
    }
}
