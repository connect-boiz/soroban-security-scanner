// VULNERABLE Soroban Contract Example
// This contract demonstrates time-based attack vulnerabilities

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

#[contract]
pub struct VulnerableTimeLock {
    // VULNERABILITY: Direct timestamp usage for lock periods
    lock_end_timestamp: u64,
    owner: Address,
}

#[contractimpl]
impl VulnerableTimeLock {
    // VULNERABILITY: No protection against timestamp manipulation
    pub fn create_lock(env: Env, owner: Address, lock_duration_seconds: u64) {
        let current_timestamp = env.ledger().timestamp();
        // VULNERABLE: Direct timestamp arithmetic without validation
        let lock_end = current_timestamp + lock_duration_seconds;
        
        // Store vulnerable timestamp
        env.storage().instance().set(&Symbol::new(&env, "lock_end"), &lock_end);
        env.storage().instance().set(&Symbol::new(&env, "owner"), &owner);
    }
    
    // VULNERABILITY: Direct timestamp comparison
    pub fn release_funds(env: Env) -> Result<(), &'static str> {
        let current_timestamp = env.ledger().timestamp();
        let lock_end: u64 = env.storage().instance()
            .get(&Symbol::new(&env, "lock_end"))
            .unwrap_or(0);
        
        // VULNERABLE: Simple timestamp comparison without protection
        if current_timestamp > lock_end {
            // Attacker can manipulate this by controlling timestamp
            env.storage().instance().remove(&Symbol::new(&env, "lock_end"));
            return Ok(());
        }
        
        Err("Lock not expired")
    }
    
    // VULNERABILITY: Time-based condition without validation
    pub fn emergency_release(env: Env, caller: Address) -> Result<(), &'static str> {
        let owner: Address = env.storage().instance()
            .get(&Symbol::new(&env, "owner"))
            .unwrap();
            
        let current_timestamp = env.ledger().timestamp();
        let lock_end: u64 = env.storage().instance()
            .get(&Symbol::new(&env, "lock_end"))
            .unwrap_or(0);
        
        // VULNERABLE: Complex time condition without bounds checking
        if caller == owner && (current_timestamp - lock_end) > 86400 {
            // Attacker can manipulate timestamp difference
            env.storage().instance().remove(&Symbol::new(&env, "lock_end"));
            return Ok(());
        }
        
        Err("Conditions not met")
    }
    
    // VULNERABILITY: Lock period calculation using timestamp
    pub fn extend_lock(env: Env, additional_seconds: u64) {
        let current_timestamp = env.ledger().timestamp();
        let mut lock_end: u64 = env.storage().instance()
            .get(&Symbol::new(&env, "lock_end"))
            .unwrap_or(current_timestamp);
            
        // VULNERABLE: Timestamp arithmetic for lock period
        lock_end = lock_end + additional_seconds;
        
        env.storage().instance().set(&Symbol::new(&env, "lock_end"), &lock_end);
    }
}
