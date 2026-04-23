use soroban_sdk::{contractimpl, Address, BytesN, Env, Symbol, Vec, Map, String, token, contracttype, try_contract, ConversionError};
use soroban_sdk::token::{TokenClient, StellarAssetClient};

// Contract state keys
const ADMIN: Symbol = Symbol::short("ADMIN");
const BOUNTY_POOL: Symbol = Symbol::short("BOUNTY");
const VULNERABILITIES: Symbol = Symbol::short("VULNS");
const REPUTATION: Symbol = Symbol::short("REPUT");
const PRICE_ORACLE: Symbol = Symbol::short("ORACLE");
const SUPPORTED_TOKENS: Symbol = Symbol::short("TOKENS");
const LIQUIDITY_THRESHOLD: Symbol = Symbol::short("LIQ_THR");
const EMERGENCY_REWARDS: Symbol = Symbol::short("EMERG_RW");
const SLIPPAGE_TOLERANCE: Symbol = Symbol::short("SLIP_TOL");

// Contract errors
pub enum ContractError {
    Unauthorized = 1,
    InvalidInput = 2,
    NotFound = 3,
    InsufficientFunds = 4,
    OracleError = 5,
    TokenNotSupported = 6,
    SlippageExceeded = 7,
    InsufficientLiquidity = 8,
    TransferFailed = 9,
}

// Token interface structure
#[derive(Clone, contracttype)]
pub struct TokenInfo {
    pub token_address: Address,
    pub decimals: u32,
    pub is_active: bool,
    pub minimum_liquidity: i128,
}

// Liquidity pool structure
#[derive(Clone, contracttype)]
pub struct LiquidityPool {
    pub token_address: Address,
    pub balance: i128,
    pub last_updated: u64,
}

// Emergency reward configuration
#[derive(Clone, contracttype)]
pub struct EmergencyRewardConfig {
    pub base_amount: i128,
    pub severity_multiplier: Map<String, i128>,
    pub oracle_enabled: bool,
    pub price_feed_address: Address,
}

// Vulnerability structure
#[derive(Clone)]
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
    pub token_address: Address,
    pub slippage_protection: i128,
}

// Reputation tracking
#[derive(Clone)]
pub struct Reputation {
    pub researcher: Address,
    pub score: u64,
    pub successful_reports: u64,
    pub total_earnings: i128,
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
        env.storage().instance().set(&BOUNTY_POOL, &0i128);
        
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

    /// Report a new vulnerability
    pub fn report_vulnerability(
        env: Env,
        reporter: Address,
        contract_id: BytesN<32>,
        vulnerability_type: String,
        severity: String,
        description: String,
        location: String,
        token_address: Address,
    ) -> Result<u64, ContractError> {
        // Verify reporter is authorized
        reporter.require_auth();
        
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
            token_address,
            slippage_protection: 0i128,
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
        custom_bounty_amount: Option<i128>,
    ) -> Result<(), ContractError> {
        // Verify admin authorization
        let contract_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if contract_admin != admin {
            return Err(ContractError::Unauthorized);
        }
        
        admin.require_auth();

        // Get vulnerability report
        let report_key = Symbol::short(&report_id.to_string());
        let mut report: VulnerabilityReport = env.storage().instance()
            .get(&report_key)
            .ok_or(ContractError::NotFound)?;

        // Calculate bounty amount using oracle or custom amount
        let bounty_amount = match custom_bounty_amount {
            Some(amount) => amount,
            None => Self::calculate_emergency_reward(env, &report.severity)?,
        };

        // Check liquidity for the token
        Self::check_liquidity(env, report.token_address, bounty_amount)?;

        // Apply slippage protection
        let slippage_tolerance: i128 = env.storage().instance()
            .get(&SLIPPAGE_TOLERANCE)
            .unwrap_or(500i128); // 5% default
        
        let slippage_amount = (bounty_amount * slippage_tolerance) / 10000i128;
        report.slippage_protection = slippage_amount;

        // Update status and bounty
        report.status = String::from_slice(&env, "verified");
        report.bounty_amount = bounty_amount;
        
        // Store updated report
        env.storage().instance().set(&report_key, &report);

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
    pub fn fund_bounty_pool(env: Env, funder: Address, token_address: Address, amount: i128) -> Result<(), ContractError> {
        funder.require_auth();
        
        // Check if token is supported
        let supported_tokens: Map<Address, TokenInfo> = env.storage().instance()
            .get(&SUPPORTED_TOKENS)
            .unwrap_or_else(|| Map::new(&env));
        
        if !supported_tokens.contains_key(token_address) {
            return Err(ContractError::TokenNotSupported);
        }
        
        // Transfer tokens to contract
        let token_client = TokenClient::new(&env, &token_address);
        token_client.transfer(&funder, &env.current_contract_address(), &amount);
        
        // Update liquidity pool
        Self::update_liquidity_pool(env, token_address, amount, true)?;

        Ok(())
    }

    /// Get bounty pool balance for specific token
    pub fn get_bounty_pool(env: Env, token_address: Address) -> Result<i128, ContractError> {
        let liquidity_pools: Map<Address, LiquidityPool> = env.storage().instance()
            .get(&Symbol::short("LIQ_POOLS"))
            .unwrap_or_else(|| Map::new(&env));
        
        match liquidity_pools.get(token_address) {
            Some(pool) => Ok(pool.balance),
            None => Ok(0i128),
        }
    }
    
    /// Add supported token
    pub fn add_supported_token(
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
        
        admin.require_auth();
        
        let mut supported_tokens: Map<Address, TokenInfo> = env.storage().instance()
            .get(&SUPPORTED_TOKENS)
            .unwrap_or_else(|| Map::new(&env));
        
        let token_info = TokenInfo {
            token_address,
            decimals,
            is_active: true,
            minimum_liquidity,
        };
        
        supported_tokens.set(token_address, &token_info);
        env.storage().instance().set(&SUPPORTED_TOKENS, &supported_tokens);
        
        Ok(())
    }
    
    /// Configure price oracle
    pub fn configure_oracle(
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
    
    /// Set slippage tolerance (in basis points, e.g., 500 = 5%)
    pub fn set_slippage_tolerance(env: Env, admin: Address, tolerance_bps: i128) -> Result<(), ContractError> {
        // Verify admin authorization
        let contract_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if contract_admin != admin {
            return Err(ContractError::Unauthorized);
        }
        
        admin.require_auth();
        
        if tolerance_bps < 0 || tolerance_bps > 10000 {
            return Err(ContractError::InvalidInput);
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

        env.storage().instance().set(&rep_key, &reputation);

        Ok(())
    }
    
    /// Calculate emergency reward amount using oracle or fixed amounts
    fn calculate_emergency_reward(env: Env, severity: &String) -> Result<i128, ContractError> {
        let emergency_config: EmergencyRewardConfig = env.storage().instance()
            .get(&EMERGENCY_REWARDS)
            .ok_or(ContractError::NotFound)?;
        
        let base_amount = if emergency_config.oracle_enabled {
            // Try to get price from oracle
            match Self::get_price_from_oracle(env, emergency_config.price_feed_address) {
                Ok(price) => price * emergency_config.base_amount / 1000000i128, // Normalize
                Err(_) => emergency_config.base_amount, // Fallback to base amount
            }
        } else {
            emergency_config.base_amount
        };
        
        // Apply severity multiplier
        let multiplier = emergency_config.severity_multiplier
            .get(severity.clone())
            .unwrap_or(1000000i128); // Default 1x multiplier
        
        Ok(base_amount * multiplier / 1000000i128)
    }
    
    /// Get price from oracle (mock implementation)
    fn get_price_from_oracle(env: Env, oracle_address: Address) -> Result<i128, ContractError> {
        // This is a mock oracle implementation
        // In a real implementation, this would call an actual price oracle contract
        // For now, we'll return a fixed price to demonstrate the concept
        Ok(1000000i128) // 1 USD = 1,000,000 units (6 decimals)
    }
    
    /// Check if sufficient liquidity exists
    fn check_liquidity(env: Env, token_address: Address, required_amount: i128) -> Result<(), ContractError> {
        let liquidity_pools: Map<Address, LiquidityPool> = env.storage().instance()
            .get(&Symbol::short("LIQ_POOLS"))
            .unwrap_or_else(|| Map::new(&env));
        
        let pool_balance = match liquidity_pools.get(token_address) {
            Some(pool) => pool.balance,
            None => 0i128,
        };
        
        let threshold: i128 = env.storage().instance()
            .get(&LIQUIDITY_THRESHOLD)
            .unwrap_or(1000000000i128); // 1000 tokens default
        
        if pool_balance < required_amount + threshold {
            return Err(ContractError::InsufficientLiquidity);
        }
        
        Ok(())
    }
    
    /// Update liquidity pool
    fn update_liquidity_pool(env: Env, token_address: Address, amount: i128, is_deposit: bool) -> Result<(), ContractError> {
        let mut liquidity_pools: Map<Address, LiquidityPool> = env.storage().instance()
            .get(&Symbol::short("LIQ_POOLS"))
            .unwrap_or_else(|| Map::new(&env));
        
        let mut pool = liquidity_pools.get(token_address).unwrap_or(LiquidityPool {
            token_address,
            balance: 0i128,
            last_updated: env.ledger().timestamp(),
        });
        
        if is_deposit {
            pool.balance += amount;
        } else {
            pool.balance -= amount;
        }
        
        pool.last_updated = env.ledger().timestamp();
        liquidity_pools.set(token_address, &pool);
        env.storage().instance().set(&Symbol::short("LIQ_POOLS"), &liquidity_pools);
        
        Ok(())
    }
    
    /// Transfer bounty with slippage protection
    fn transfer_bounty(env: Env, token_address: Address, recipient: Address, amount: i128) -> Result<(), ContractError> {
        let token_client = TokenClient::new(&env, &token_address);
        
        // Get current balance to check for slippage
        let contract_balance = token_client.balance(&env.current_contract_address());
        
        if contract_balance < amount {
            return Err(ContractError::InsufficientFunds);
        }
        
        // Transfer tokens
        token_client.transfer(&env.current_contract_address(), &recipient, &amount);
        
        // Update liquidity pool
        Self::update_liquidity_pool(env, token_address, amount, false)?;
        
        Ok(())
    }
    
    /// Get supported tokens
    pub fn get_supported_tokens(env: Env) -> Map<Address, TokenInfo> {
        env.storage().instance()
            .get(&SUPPORTED_TOKENS)
            .unwrap_or_else(|| Map::new(&env))
    }
    
    /// Get emergency reward configuration
    pub fn get_emergency_config(env: Env) -> EmergencyRewardConfig {
        env.storage().instance()
            .get(&EMERGENCY_REWARDS)
            .unwrap_or_else(|| EmergencyRewardConfig {
                base_amount: 1000000i128,
                severity_multiplier: Map::new(&env),
                oracle_enabled: false,
                price_feed_address: Address::generate(&env),
            })
    }
    
    /// Get liquidity information
    pub fn get_liquidity_info(env: Env, token_address: Address) -> Result<LiquidityPool, ContractError> {
        let liquidity_pools: Map<Address, LiquidityPool> = env.storage().instance()
            .get(&Symbol::short("LIQ_POOLS"))
            .unwrap_or_else(|| Map::new(&env));
        
        liquidity_pools.get(token_address)
            .ok_or(ContractError::NotFound)
    }
}
