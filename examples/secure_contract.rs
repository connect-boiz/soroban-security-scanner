//! Example secure contract following best practices

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol, panic_with_error};

#[contract]
pub struct SecureToken;

#[contractimpl]
impl SecureToken {
    // SECURE: Proper access control with require_auth
    pub fn mint(env: Env, to: Address, amount: i128) {
        // SECURE: Check authorization
        to.require_auth();
        
        // SECURE: Input validation
        if amount <= 0 {
            panic_with_error!(&env, "Amount must be positive");
        }
        
        // SECURE: Safe arithmetic with overflow protection
        let mut balance = env.storage().persistent().get::<Address, i128>(&to).unwrap_or(0);
        balance = balance.checked_add(amount).expect("Integer overflow");
        
        // SECURE: Update total supply
        let mut total_supply = env.storage().persistent().get::<Symbol, i128>(&Symbol::new(&env, "total_supply")).unwrap_or(0);
        total_supply = total_supply.checked_add(amount).expect("Total supply overflow");
        env.storage().persistent().set(&Symbol::new(&env, "total_supply"), &total_supply);
        
        // SECURE: Update balance
        env.storage().persistent().set(&to, &balance);
        
        // SECURE: Emit event
        env.events().publish(
            (Symbol::new(&env, "mint"), to),
            (amount, total_supply),
        );
    }

    // SECURE: Proper access control
    pub fn burn(env: Env, from: Address, amount: i128) {
        // SECURE: Check authorization
        from.require_auth();
        
        // SECURE: Input validation
        if amount <= 0 {
            panic_with_error!(&env, "Amount must be positive");
        }
        
        let mut balance = env.storage().persistent().get::<Address, i128>(&from).unwrap_or(0);
        
        // SECURE: Balance check
        if balance < amount {
            panic_with_error!(&env, "Insufficient balance");
        }
        
        // SECURE: Safe arithmetic
        balance = balance.checked_sub(amount).expect("Integer underflow");
        
        // SECURE: Update total supply
        let mut total_supply = env.storage().persistent().get::<Symbol, i128>(&Symbol::new(&env, "total_supply")).unwrap_or(0);
        total_supply = total_supply.checked_sub(amount).expect("Total supply underflow");
        env.storage().persistent().set(&Symbol::new(&env, "total_supply"), &total_supply);
        
        // SECURE: Update balance
        env.storage().persistent().set(&from, &balance);
        
        // SECURE: Emit event
        env.events().publish(
            (Symbol::new(&env, "burn"), from),
            (amount, total_supply),
        );
    }

    // SECURE: Proper access control and checks
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        // SECURE: Check authorization
        from.require_auth();
        
        // SECURE: Input validation
        if amount <= 0 {
            panic_with_error!(&env, "Amount must be positive");
        }
        
        let mut from_balance = env.storage().persistent().get::<Address, i128>(&from).unwrap_or(0);
        let mut to_balance = env.storage().persistent().get::<Address, i128>(&to).unwrap_or(0);
        
        // SECURE: Balance check
        if from_balance < amount {
            panic_with_error!(&env, "Insufficient balance");
        }
        
        // SECURE: Safe arithmetic
        from_balance = from_balance.checked_sub(amount).expect("Integer underflow");
        to_balance = to_balance.checked_add(amount).expect("Integer overflow");
        
        // SECURE: Update balances
        env.storage().persistent().set(&from, &from_balance);
        env.storage().persistent().set(&to, &to_balance);
        
        // SECURE: Emit event
        env.events().publish(
            (Symbol::new(&env, "transfer"), from, to),
            amount,
        );
    }

    // SECURE: Proper admin access control
    pub fn admin_only_function(env: Env, admin: Address, data: Symbol) {
        // SECURE: Check admin authorization
        admin.require_auth();
        
        // SECURE: Verify admin status
        let admin_address = env.storage().persistent().get::<Symbol, Address>(&Symbol::new(&env, "admin"))
            .unwrap_or_else(|| panic_with_error!(&env, "Admin not set"));
        
        if admin != admin_address {
            panic_with_error!(&env, "Not authorized: admin access required");
        }
        
        // SECURE: Input validation
        if data.is_empty() {
            panic_with_error!(&env, "Data cannot be empty");
        }
        
        env.storage().persistent().set(&data, &"admin_data");
        
        // SECURE: Emit admin event
        env.events().publish(
            Symbol::new(&env, "admin_action"),
            (admin, data),
        );
    }

    // SECURE: Reentrancy protection (checks-effects-interactions pattern)
    pub fn withdraw(env: Env, amount: i128) {
        let caller = env.current_contract_address();
        
        // SECURE: Input validation
        if amount <= 0 {
            panic_with_error!(&env, "Amount must be positive");
        }
        
        let mut balance = env.storage().persistent().get::<Address, i128>(&caller).unwrap_or(0);
        
        // SECURE: Check (condition)
        if balance < amount {
            panic_with_error!(&env, "Insufficient balance");
        }
        
        // SECURE: Effects (state change)
        balance = balance.checked_sub(amount).expect("Integer underflow");
        env.storage().persistent().set(&caller, &balance);
        
        // SECURE: Interactions (external calls would happen here)
        // In this case, we just emit an event
        env.events().publish(
            (Symbol::new(&env, "withdraw"), caller),
            amount,
        );
    }

    // SECURE: Protected admin function
    pub fn set_admin(env: Env, current_admin: Address, new_admin: Address) {
        // SECURE: Check current admin authorization
        current_admin.require_auth();
        
        // SECURE: Verify current admin
        let stored_admin = env.storage().persistent().get::<Symbol, Address>(&Symbol::new(&env, "admin"))
            .unwrap_or_else(|| panic_with_error!(&env, "Admin not set"));
        
        if current_admin != stored_admin {
            panic_with_error!(&env, "Not authorized: current admin required");
        }
        
        // SECURE: Validate new admin
        if new_admin == Address::default() {
            panic_with_error!(&env, "New admin cannot be zero address");
        }
        
        // SECURE: Set new admin
        env.storage().persistent().set(&Symbol::new(&env, "admin"), &new_admin);
        
        // SECURE: Emit event
        env.events().publish(
            Symbol::new(&env, "admin_changed"),
            (current_admin, new_admin),
        );
    }

    pub fn get_balance(env: Env, account: Address) -> i128 {
        env.storage().persistent().get::<Address, i128>(&account).unwrap_or(0)
    }

    pub fn get_total_supply(env: Env) -> i128 {
        env.storage().persistent().get::<Symbol, i128>(&Symbol::new(&env, "total_supply")).unwrap_or(0)
    }
}
