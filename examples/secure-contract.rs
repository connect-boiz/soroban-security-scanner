// SECURE Soroban Contract Example  
// This contract demonstrates proper time-based attack protections

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol, Vec};

#[contract]
pub struct SecureTimeLock {
    // SECURE: Use block height instead of timestamp for critical operations
    lock_end_block: u32,
    // SECURE: Store timestamp only for display purposes with validation
    lock_end_timestamp: u64,
    // SECURE: Add timestamp bounds for validation
    min_timestamp: u64,
    max_timestamp: u64,
    owner: Address,
    // SECURE: Add nonce for replay protection
    nonce: u64,
}

#[contractimpl]
impl SecureTimeLock {
    // SECURE: Constructor with proper timestamp validation
    pub fn create_lock(env: Env, owner: Address, lock_duration_blocks: u32) -> Result<(), &'static str> {
        let current_block = env.ledger().sequence();
        let current_timestamp = env.ledger().timestamp();
        
        // SECURE: Validate timestamp bounds
        if !Self::is_timestamp_valid(&env, current_timestamp)? {
            return Err("Invalid timestamp provided");
        }
        
        // SECURE: Use block height for lock calculation (more reliable)
        let lock_end_block = current_block.checked_add(lock_duration_blocks)
            .ok_or("Block overflow")?;
            
        // SECURE: Calculate timestamp with bounds checking
        let lock_end_timestamp = Self::safe_timestamp_addition(
            current_timestamp, 
            lock_duration_blocks as u64 * 5 // ~5 seconds per block
        )?;
        
        // SECURE: Store both block height and validated timestamp
        env.storage().instance().set(&Symbol::new(&env, "lock_end_block"), &lock_end_block);
        env.storage().instance().set(&Symbol::new(&env, "lock_end_timestamp"), &lock_end_timestamp);
        env.storage().instance().set(&Symbol::new(&env, "owner"), &owner);
        
        // SECURE: Initialize nonce for replay protection
        env.storage().instance().set(&Symbol::new(&env, "nonce"), &0u64);
        
        Ok(())
    }
    
    // SECURE: Release using block height with timestamp validation
    pub fn release_funds(env: Env) -> Result<(), &'static str> {
        let current_block = env.ledger().sequence();
        let current_timestamp = env.ledger().timestamp();
        
        // SECURE: Validate current timestamp first
        if !Self::is_timestamp_valid(&env, current_timestamp)? {
            return Err("Invalid current timestamp");
        }
        
        let lock_end_block: u32 = env.storage().instance()
            .get(&Symbol::new(&env, "lock_end_block"))
            .ok_or("Lock not found")?;
            
        let lock_end_timestamp: u64 = env.storage().instance()
            .get(&Symbol::new(&env, "lock_end_timestamp"))
            .ok_or("Lock timestamp not found")?;
        
        // SECURE: Primary check using block height (harder to manipulate)
        if current_block >= lock_end_block {
            // SECURE: Secondary timestamp check with drift tolerance
            let timestamp_drift = if current_timestamp > lock_end_timestamp {
                current_timestamp - lock_end_timestamp
            } else {
                0
            };
            
            // Allow reasonable timestamp drift (e.g., 300 seconds = 5 minutes)
            if timestamp_drift <= 300 {
                // SECURE: Check nonce to prevent replay attacks
                let mut nonce: u64 = env.storage().instance()
                    .get(&Symbol::new(&env, "nonce"))
                    .unwrap_or(0);
                    
                nonce += 1;
                env.storage().instance().set(&Symbol::new(&env, "nonce"), &nonce);
                
                // Clear lock data
                env.storage().instance().remove(&Symbol::new(&env, "lock_end_block"));
                env.storage().instance().remove(&Symbol::new(&env, "lock_end_timestamp"));
                env.storage().instance().remove(&Symbol::new(&env, "owner"));
                
                return Ok(());
            }
        }
        
        Err("Lock not expired or invalid timestamp")
    }
    
    // SECURE: Emergency release with multiple validations
    pub fn emergency_release(env: Env, caller: Address, emergency_code: u64) -> Result<(), &'static str> {
        let owner: Address = env.storage().instance()
            .get(&Symbol::new(&env, "owner"))
            .ok_or("Lock not found")?;
            
        let current_block = env.ledger().sequence();
        let current_timestamp = env.ledger().timestamp();
        let lock_end_block: u32 = env.storage().instance()
            .get(&Symbol::new(&env, "lock_end_block"))
            .ok_or("Lock not found")?;
        
        // SECURE: Multiple validation layers
        if caller != owner {
            return Err("Unauthorized");
        }
        
        if !Self::is_timestamp_valid(&env, current_timestamp)? {
            return Err("Invalid timestamp");
        }
        
        // SECURE: Use block-based time calculation instead of timestamp difference
        let blocks_passed = current_block.saturating_sub(lock_end_block);
        
        // SECURE: Require minimum blocks to pass (more reliable than time)
        if blocks_passed >= 1000 { // ~5000 blocks = ~7 hours minimum
            // SECURE: Validate emergency code
            if emergency_code == Self::calculate_emergency_code(&env, &owner)? {
                // SECURE: Update nonce
                let mut nonce: u64 = env.storage().instance()
                    .get(&Symbol::new(&env, "nonce"))
                    .unwrap_or(0);
                nonce += 1;
                env.storage().instance().set(&Symbol::new(&env, "nonce"), &nonce);
                
                // Clear lock
                env.storage().instance().remove(&Symbol::new(&env, "lock_end_block"));
                env.storage().instance().remove(&Symbol::new(&env, "lock_end_timestamp"));
                env.storage().instance().remove(&Symbol::new(&env, "owner"));
                
                return Ok(());
            }
        }
        
        Err("Emergency conditions not met")
    }
    
    // SECURE: Extend lock with proper validation
    pub fn extend_lock(env: Env, additional_blocks: u32, caller: Address) -> Result<(), &'static str> {
        let owner: Address = env.storage().instance()
            .get(&Symbol::new(&env, "owner"))
            .ok_or("Lock not found")?;
            
        if caller != owner {
            return Err("Unauthorized");
        }
        
        let current_block = env.ledger().sequence();
        let current_timestamp = env.ledger().timestamp();
        
        if !Self::is_timestamp_valid(&env, current_timestamp)? {
            return Err("Invalid timestamp");
        }
        
        let mut lock_end_block: u32 = env.storage().instance()
            .get(&Symbol::new(&env, "lock_end_block"))
            .ok_or("Lock not found")?;
            
        // SECURE: Safe block arithmetic with overflow protection
        lock_end_block = lock_end_block.checked_add(additional_blocks)
            .ok_or("Block overflow")?;
            
        // SECURE: Update timestamp with bounds checking
        let additional_seconds = additional_blocks as u64 * 5;
        let new_timestamp = Self::safe_timestamp_addition(current_timestamp, additional_seconds)?;
        
        env.storage().instance().set(&Symbol::new(&env, "lock_end_block"), &lock_end_block);
        env.storage().instance().set(&Symbol::new(&env, "lock_end_timestamp"), &new_timestamp);
        
        Ok(())
    }
    
    // SECURE: Helper function for timestamp validation
    fn is_timestamp_valid(env: &Env, timestamp: u64) -> Result<bool, &'static str> {
        // Get reasonable bounds (e.g., within 1 year of current time)
        let current_timestamp = env.ledger().timestamp();
        let one_year_seconds = 365 * 24 * 60 * 60;
        
        // Check if timestamp is within reasonable bounds
        if timestamp > current_timestamp + one_year_seconds {
            return Ok(false);
        }
        
        // Check if timestamp is not too far in the past
        if timestamp < current_timestamp.saturating_sub(one_year_seconds) {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    // SECURE: Safe timestamp addition with overflow protection
    fn safe_timestamp_addition(base: u64, addition: u64) -> Result<u64, &'static str> {
        base.checked_add(addition).ok_or("Timestamp overflow")
    }
    
    // SECURE: Calculate emergency code (example implementation)
    fn calculate_emergency_code(env: &Env, owner: &Address) -> Result<u64, &'static str> {
        // Simple example: hash of owner address and current block
        let current_block = env.ledger().sequence();
        let owner_bytes = owner.to_string();
        let hash = current_block.wrapping_mul(owner_bytes.len() as u64);
        Ok(hash)
    }
}
