use soroban_sdk::{contract, contractimpl, Address, Env, Symbol, contractclient};

#[contract]
pub struct Escrow;

#[contractclient(name = "EscrowClient")]
pub trait EscrowTrait {
    fn create_escrow(env: Env, beneficiary: Address, amount: i128);
    fn release(env: Env);
    fn get_escrow_info(env: Env) -> EscrowData;
}

#[derive(Clone)]
pub struct EscrowData {
    pub depositor: Address,
    pub beneficiary: Address,
    pub amount: i128,
    pub released: bool,
}

#[contractimpl]
impl Escrow {
    /// Create a new escrow with the specified beneficiary and amount
    pub fn create_escrow(env: Env, beneficiary: Address, amount: i128) {
        let depositor = env.current_contract_address();
        
        let escrow_data = EscrowData {
            depositor: depositor.clone(),
            beneficiary,
            amount,
            released: false,
        };
        
        // Store escrow data
        env.storage().instance().set(&Symbol::new(&env, "escrow"), &escrow_data);
        
        // Transfer funds to contract
        env.storage().instance().set(&Symbol::new(&env, "balance"), &amount);
    }
    
    /// SECURE: Release funds to beneficiary - updates state BEFORE external call
    pub fn release(env: Env) {
        let escrow_key = Symbol::new(&env, "escrow");
        let escrow_data: EscrowData = env.storage().instance()
            .get(&escrow_key)
            .expect("Escrow not found");
        
        if escrow_data.released {
            panic!("Escrow already released");
        }
        
        let balance_key = Symbol::new(&env, "balance");
        let balance: i128 = env.storage().instance()
            .get(&balance_key)
            .expect("No balance found");
        
        // SECURITY: Update state BEFORE external call to prevent reentrancy
        let mut updated_escrow = escrow_data.clone();
        updated_escrow.released = true;
        env.storage().instance().set(&escrow_key, &updated_escrow);
        
        // Clear balance immediately
        env.storage().instance().remove(&balance_key);
        
        // External calls AFTER state update - secure from reentrancy
        env.current_contract_address()
            .require_auth_for_args((&escrow_data.beneficiary, balance));
        
        // Transfer funds to beneficiary (external call simulation)
        // In real implementation, this would be an actual token transfer
        self::transfer_funds(&env, &escrow_data.beneficiary, balance);
    }
    
        
    /// Helper function to simulate external fund transfer
    fn transfer_funds(env: &Env, recipient: &Address, amount: i128) {
        // In a real implementation, this would call a token contract
        // For demonstration, we'll just log the transfer
        env.storage().instance().set(
            &Symbol::new(env, "last_transfer"), 
            &(recipient.clone(), amount)
        );
    }
    
    /// Get escrow information
    pub fn get_escrow_info(env: Env) -> EscrowData {
        let escrow_key = Symbol::new(&env, "escrow");
        env.storage().instance()
            .get(&escrow_key)
            .expect("Escrow not found")
    }
}
