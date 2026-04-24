


// Contract state keys
const ADMIN: Symbol = Symbol::short("ADMIN");
const BOUNTY_POOL: Symbol = Symbol::short("BOUNTY");
const VULNERABILITIES: Symbol = Symbol::short("VULNS");
const REPUTATION: Symbol = Symbol::short("REPUT");



// Contract errors
pub enum ContractError {
    Unauthorized = 1,
    InvalidInput = 2,
    NotFound = 3,
    InsufficientFunds = 4,

}

// Vulnerability structure
#[derive(Clone)]
#[contracttype]
pub struct VulnerabilityReport {
    pub reporter: Address,
    pub contract_id: BytesN<32>,
    pub vulnerability_type: String,
    pub severity: String,
    pub description: String,
    pub location: String,
    pub timestamp: u64,
    pub status: String, // "pending", "verified", "rejected"
    pub bounty_amount: i128,



// Reputation tracking
#[derive(Clone)]
#[contracttype]
pub struct Reputation {
    pub researcher: Address,
    pub score: u64,
    pub successful_reports: u64,
    pub total_earnings: i128,
}


// Escrow structure
#[derive(Clone)]
#[contracttype]
pub struct EscrowEntry {
    pub id: u64,
    pub depositor: Address,
    pub beneficiary: Address,
    pub amount: i128,
    pub token: Address,
    pub purpose: String, // "bounty", "reward", "emergency"
    pub status: String,  // "pending", "locked", "released", "refunded"
    pub created_at: u64,
    pub lock_until: u64,
    pub conditions_met: bool,
    pub release_signature: Option<BytesN<32>>,
}


// Emergency alert structure
#[derive(Clone)]
#[contracttype]
pub struct EmergencyAlert {
    pub id: u64,
    pub reporter: Address,
    pub contract_id: BytesN<32>,
    pub vulnerability_type: String,
    pub severity: String, // "critical", "emergency"
    pub description: String,
    pub location: String,
    pub timestamp: u64,
    pub status: String, // "pending", "verified", "false_positive"
    pub emergency_reward: i128,
    pub token: Address,
    pub verified_by: Option<Address>,
}


pub struct SecurityScannerContract;

#[contractimpl]
impl SecurityScannerContract {
    
    /// Initialize the contract with admin address
    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        if env.storage().instance().has(&ADMIN) {
            return Err(ContractError::Unauthorized);
        }
        
        env.storage().instance().set(&ADMIN, &admin);
        
        // Initialize default configuration
        let supported_tokens: Map<Address, TokenInfo> = Map::new(&env);
        env.storage().instance().set(&SUPPORTED_TOKENS, &supported_tokens);
        
        // Set default liquidity threshold (1000 tokens)
        env.storage().instance().set(&LIQUIDITY_THRESHOLD, &1000000000i128);
        
        // Set default slippage tolerance (5%)
        env.storage().instance().set(&SLIPPAGE_TOLERANCE, &500i128);
        
        // Initialize emergency reward config
        let mut severity_multipliers = Map::new(&env);
        severity_multipliers.set(String::from_str(&env, "low"), &1000000i128);
        severity_multipliers.set(String::from_str(&env, "medium"), &5000000i128);
        severity_multipliers.set(String::from_str(&env, "high"), &10000000i128);
        severity_multipliers.set(String::from_str(&env, "critical"), &50000000i128);
        
        let emergency_config = EmergencyRewardConfig {
            base_amount: 1000000i128,
            severity_multiplier: severity_multipliers,
            oracle_enabled: false,
            price_feed_address: admin.clone(), // Placeholder
        };
        env.storage().instance().set(&EMERGENCY_REWARDS, &emergency_config);
        
        Ok(())
    }

    /// Set the oracle contract address
    pub fn set_oracle(env: Env, admin: Address, oracle: Address) -> Result<(), ContractError> {
        let contract_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if contract_admin != admin {
            return Err(ContractError::Unauthorized);
        }
        admin.require_auth();
        
        env.storage().instance().set(&ORACLE, &oracle);
        Ok(())
    }


    /// Report a new vulnerability
    pub fn report_vulnerability(
        env: Env,
        reporter: Address,
        contract_id: BytesN<32>,
        vulnerability_type: String,
        severity: String,
        description: String,
        location: String,

    ) -> Result<u64, ContractError> {
        // Verify reporter is authorized
        reporter.require_auth();

        // Check if token is whitelisted (#125)
        Self::check_token_whitelisted(&env, &token)?;
        
        // Check if token is supported
        let supported_tokens: Map<Address, TokenInfo> = env.storage().instance()
            .get(&SUPPORTED_TOKENS)
            .unwrap_or_else(|| Map::new(&env));
        
        if !supported_tokens.contains_key(token_address) {
            return Err(ContractError::TokenNotSupported);
        }
        
        // Create vulnerability report
        let report = VulnerabilityReport {
            reporter: reporter.clone(),
            contract_id,
            vulnerability_type: vulnerability_type.clone(),
            severity: severity.clone(),
            description: description.clone(),
            location: location.clone(),
            timestamp: env.ledger().timestamp(),
            status: String::from_slice(&env, "pending"),
            bounty_amount: 0i128,

        };


        // Store the report
        let report_id = env.ledger().sequence();
        env.storage().instance().set(&Symbol::short(&report_id.to_string()), &report);

        // Update reputation
        Self::update_reputation(env, reporter, 0, 0)?;

        Ok(report_id)
    }

    /// Verify a vulnerability and award bounty
    pub fn verify_vulnerability(
        env: Env,
        admin: Address,
        report_id: u64,


        // Update status and bounty
        report.status = String::from_slice(&env, "verified");
        report.bounty_amount = bounty_amount;
        
        // Store updated report
        env.storage().instance().set(&report_key, &report);

        // Deduct from bounty pool (#127)
        let mut pools: Map<Address, i128> = env.storage().instance().get(&BOUNTY_POOL).unwrap_or(Map::new(&env));
        pools.set(report.token.clone(), pool_balance - bounty_amount);
        env.storage().instance().set(&BOUNTY_POOL, &pools);

        // Update researcher reputation
        Self::update_reputation(env, report.reporter, 1, bounty_amount)?;

        // Transfer bounty from pool to researcher
        Self::transfer_bounty(env, report.token_address, report.reporter, bounty_amount)?;

        Ok(())
    }

    /// Get vulnerability report
    pub fn get_vulnerability(env: Env, report_id: u64) -> Result<VulnerabilityReport, ContractError> {
        let report_key = Symbol::short(&report_id.to_string());
        env.storage().instance()
            .get(&report_key)
            .ok_or(ContractError::NotFound)
    }

    /// Get researcher reputation
    pub fn get_reputation(env: Env, researcher: Address) -> Result<Reputation, ContractError> {
        let rep_key = Symbol::short(&format!("REP_{:?}", researcher));
        env.storage().instance()
            .get(&rep_key)
            .ok_or(ContractError::NotFound)
    }

    /// Add funds to bounty pool


        Ok(())
    }


        
        match liquidity_pools.get(token_address) {
            Some(pool) => Ok(pool.balance),
            None => Ok(0i128),
        }

        env: Env,
        admin: Address,
        token_address: Address,
        decimals: u32,
        minimum_liquidity: i128,
    ) -> Result<(), ContractError> {
        // Verify admin authorization
        let contract_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if contract_admin != admin {
            return Err(ContractError::Unauthorized);
        }
        

        env: Env,
        admin: Address,
        oracle_address: Address,
        enabled: bool,
    ) -> Result<(), ContractError> {
        // Verify admin authorization
        let contract_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if contract_admin != admin {
            return Err(ContractError::Unauthorized);
        }
        
        admin.require_auth();
        
        let mut emergency_config: EmergencyRewardConfig = env.storage().instance()
            .get(&EMERGENCY_REWARDS)
            .unwrap_or_else(|| EmergencyRewardConfig {
                base_amount: 1000000i128,
                severity_multiplier: Map::new(&env),
                oracle_enabled: false,
                price_feed_address: admin.clone(),
            });
        
        emergency_config.oracle_enabled = enabled;
        emergency_config.price_feed_address = oracle_address;
        
        env.storage().instance().set(&EMERGENCY_REWARDS, &emergency_config);
        
        Ok(())
    }

        // Verify admin authorization
        let contract_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if contract_admin != admin {
            return Err(ContractError::Unauthorized);
        }
        
        admin.require_auth();

        }
        
        env.storage().instance().set(&SLIPPAGE_TOLERANCE, &tolerance_bps);
        
        Ok(())
    }

    /// Helper function to update reputation
    fn update_reputation(
        env: Env,
        researcher: Address,
        successful_reports: u64,
        earnings: i128,
    ) -> Result<(), ContractError> {
        let rep_key = Symbol::short(&format!("REP_{:?}", researcher));
        
        let mut reputation: Reputation = env.storage().instance()
            .get(&rep_key)
            .unwrap_or(Reputation {
                researcher: researcher.clone(),
                score: 0,
                successful_reports: 0,
                total_earnings: 0,
            });

        reputation.successful_reports += successful_reports;
        reputation.total_earnings += earnings;
        reputation.score = reputation.successful_reports * 10 + (reputation.total_earnings / 1000000) as u64;


    }

    /// Admin withdraw function for liquidity management (#127)
    pub fn admin_withdraw(
        env: Env,
        admin: Address,
        token: Address,
        amount: i128,
        to: Address,
    ) -> Result<(), ContractError> {
        let contract_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if contract_admin != admin {
            return Err(ContractError::Unauthorized);
        }
        admin.require_auth();

        let client = token::Client::new(&env, &token);
        let contract_balance = client.balance(&env.current_contract_address());
        
        // Liquidity Management (#127): Maintain a 10% reserve threshold
        let reserve_threshold = contract_balance / 10;
        if contract_balance - amount < reserve_threshold {
            return Err(ContractError::InsufficientFunds);
        }

        client.transfer(&env.current_contract_address(), &to, &amount);
        Ok(())
    }

    /// Add a token to the whitelist (#125)
    pub fn add_token(env: Env, admin: Address, token: Address) -> Result<(), ContractError> {
        let contract_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if contract_admin != admin {
            return Err(ContractError::Unauthorized);
        }
        admin.require_auth();

        let mut tokens: Vec<Address> = env.storage().instance().get(&TOKENS).unwrap_or(Vec::new(&env));
        if !tokens.contains(&token) {
            tokens.push_back(token);
            env.storage().instance().set(&TOKENS, &tokens);
        }
        Ok(())
    }

    /// Remove a token from the whitelist (#125)
    pub fn remove_token(env: Env, admin: Address, token: Address) -> Result<(), ContractError> {
        let contract_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if contract_admin != admin {
            return Err(ContractError::Unauthorized);
        }
        admin.require_auth();

        let tokens: Vec<Address> = env.storage().instance().get(&TOKENS).unwrap_or(Vec::new(&env));
        let mut new_tokens = Vec::new(&env);
        for t in tokens.iter() {
            if t != token {
                new_tokens.push_back(t);
            }
        }
        env.storage().instance().set(&TOKENS, &new_tokens);
        Ok(())
    }

    /// Helper to check if token is whitelisted (#125)
    fn check_token_whitelisted(env: &Env, token: &Address) -> Result<(), ContractError> {
        let tokens: Vec<Address> = env.storage().instance().get(&TOKENS).unwrap_or(Vec::new(&env));
        if !tokens.contains(token) {
            return Err(ContractError::InvalidInput);
        }
        Ok(())
    }

    /// Get liquidity status report (#127)
    pub fn get_liquidity_status(env: Env, token: Address) -> (i128, i128) {
        let bounty_pool = Self::get_bounty_pool(env.clone(), token.clone());
        let client = token::Client::new(&env, &token);
        let contract_balance = client.balance(&env.current_contract_address());
        (bounty_pool, contract_balance)
    }
}

