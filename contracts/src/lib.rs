


// Contract state keys
const ADMIN: Symbol = Symbol::short("ADMIN");
const BOUNTY_POOL: Symbol = Symbol::short("BOUNTY");
const VULNERABILITIES: Symbol = Symbol::short("VULNS");
const REPUTATION: Symbol = Symbol::short("REPUT");

// Gas limit configuration keys
const GAS_CONFIG: Symbol = Symbol::short("GAS_CONFIG");
const ESCROW_BATCH: Symbol = Symbol::short("ESCROW_BATCH");
const EMERGENCY_BATCH: Symbol = Symbol::short("EMERGENCY_BATCH");

// Default gas limits (in Soroban fee units)
const DEFAULT_SINGLE_TRANSFER_GAS: u64 = 100_000;
const DEFAULT_BATCH_TRANSFER_GAS: u64 = 500_000;
const DEFAULT_EMERGENCY_GAS: u64 = 200_000;
const MAX_BATCH_SIZE: u32 = 50;



// Contract errors
pub enum ContractError {
    Unauthorized = 1,
    InvalidInput = 2,
    NotFound = 3,
    InsufficientFunds = 4,
    GasLimitExceeded = 5,
    BatchTooLarge = 6,
    InsufficientGas = 7,

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

// Gas configuration structure
#[derive(Clone)]
#[contracttype]
pub struct GasConfig {
    pub single_transfer_limit: u64,
    pub batch_transfer_limit: u64,
    pub emergency_limit: u64,
    pub max_batch_size: u32,
    pub gas_price_multiplier: u32, // For dynamic gas adjustment
}

// Batch operation structure for escrow releases
#[derive(Clone)]
#[contracttype]
pub struct BatchRelease {
    pub id: u64,
    pub recipients: Vec<Address>,
    pub amounts: Vec<i128>,
    pub tokens: Vec<Address>,
    pub total_amount: i128,
    pub gas_used: u64,
    pub status: String, // "pending", "processing", "completed", "failed"
    pub created_at: u64,
}

// Batch operation structure for emergency rewards
#[derive(Clone)]
#[contracttype]
pub struct EmergencyBatch {
    pub id: u64,
    pub alert_ids: Vec<u64>,
    pub total_reward: i128,
    pub token: Address,
    pub gas_used: u64,
    pub status: String, // "pending", "processing", "completed", "failed"
    pub created_at: u64,
}

// Event structures for critical operations
#[derive(Clone)]
#[contracttype]
pub struct VulnerabilityReportedEvent {
    pub report_id: u64,
    pub reporter: Address,
    pub contract_id: BytesN<32>,
    pub vulnerability_type: String,
    pub severity: String,
    pub timestamp: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct VulnerabilityVerifiedEvent {
    pub report_id: u64,
    pub verifier: Address,
    pub reporter: Address,
    pub bounty_amount: i128,
    pub token: Address,
    pub timestamp: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct FundTransferEvent {
    pub from: Address,
    pub to: Address,
    pub amount: i128,
    pub token: Address,
    pub purpose: String, // "bounty", "reward", "escrow"
    pub reference_id: Option<u64>,
    pub timestamp: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct EscrowCreatedEvent {
    pub escrow_id: u64,
    pub depositor: Address,
    pub beneficiary: Address,
    pub amount: i128,
    pub token: Address,
    pub purpose: String,
    pub timestamp: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct EscrowReleasedEvent {
    pub escrow_id: u64,
    pub beneficiary: Address,
    pub amount: i128,
    pub token: Address,
    pub released_by: Address,
    pub timestamp: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct BatchEscrowReleaseEvent {
    pub batch_id: u64,
    pub recipient_count: u32,
    pub total_amount: i128,
    pub token: Address,
    pub gas_used: u64,
    pub initiated_by: Address,
    pub timestamp: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct EmergencyRewardDistributedEvent {
    pub batch_id: u64,
    pub alert_count: u32,
    pub total_reward: i128,
    pub token: Address,
    pub gas_used: u64,
    pub distributed_by: Address,
    pub timestamp: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct ReputationUpdatedEvent {
    pub researcher: Address,
    pub score_change: u64,
    pub new_score: u64,
    pub successful_reports: u64,
    pub total_earnings: i128,
    pub timestamp: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct BountyPoolFundedEvent {
    pub funder: Address,
    pub token: Address,
    pub amount: i128,
    pub new_balance: i128,
    pub timestamp: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct GasConfigUpdatedEvent {
    pub updated_by: Address,
    pub single_transfer_limit: u64,
    pub batch_transfer_limit: u64,
    pub emergency_limit: u64,
    pub max_batch_size: u32,
    pub timestamp: u64,
}

pub struct SecurityScannerContract;

#[contractimpl]
impl SecurityScannerContract {
    // ... rest of the code remains the same ...
}
    
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
        
        // Initialize gas configuration
        let gas_config = GasConfig {
            single_transfer_limit: DEFAULT_SINGLE_TRANSFER_GAS,
            batch_transfer_limit: DEFAULT_BATCH_TRANSFER_GAS,
            emergency_limit: DEFAULT_EMERGENCY_GAS,
            max_batch_size: MAX_BATCH_SIZE,
            gas_price_multiplier: 100, // 1x multiplier
        };
        env.storage().instance().set(&GAS_CONFIG, &gas_config);
        
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

        // Emit vulnerability reported event
        let reported_event = VulnerabilityReportedEvent {
            report_id,
            reporter: reporter.clone(),
            contract_id,
            vulnerability_type: vulnerability_type.clone(),
            severity: severity.clone(),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((Symbol::short("VULNERABILITY_REPORTED"), reported_event));

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

        // Emit vulnerability verified event
        let verified_event = VulnerabilityVerifiedEvent {
            report_id,
            verifier: admin.clone(),
            reporter: report.reporter.clone(),
            bounty_amount,
            token: report.token_address.clone(),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((Symbol::short("VULNERABILITY_VERIFIED"), verified_event));

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

        // Emit reputation updated event
        let reputation_event = ReputationUpdatedEvent {
            researcher: researcher.clone(),
            score_change: successful_reports * 10 + (earnings / 1000000) as u64 - (reputation.score - successful_reports * 10 - ((reputation.total_earnings - earnings) / 1000000) as u64),
            new_score: reputation.score,
            successful_reports: reputation.successful_reports,
            total_earnings: reputation.total_earnings,
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((Symbol::short("REPUTATION_UPDATED"), reputation_event));

        env.storage().instance().set(&rep_key, &reputation);
        Ok(())
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

    /// Transfer bounty with gas limit consideration (#112)
    fn transfer_bounty(env: &Env, token_address: &Address, recipient: &Address, amount: i128) -> Result<(), ContractError> {
        let gas_config: GasConfig = env.storage().instance()
            .get(&GAS_CONFIG)
            .unwrap_or_else(|| GasConfig {
                single_transfer_limit: DEFAULT_SINGLE_TRANSFER_GAS,
                batch_transfer_limit: DEFAULT_BATCH_TRANSFER_GAS,
                emergency_limit: DEFAULT_EMERGENCY_GAS,
                max_batch_size: MAX_BATCH_SIZE,
                gas_price_multiplier: 100,
            });

        // Check current gas usage and limits
        let current_gas = env.budget().gas_left();
        if current_gas < gas_config.single_transfer_limit {
            return Err(ContractError::InsufficientGas);
        }

        let client = token::Client::new(env, token_address);
        
        // Perform transfer with gas tracking
        let initial_gas = env.budget().gas_left();
        client.transfer(&env.current_contract_address(), recipient, &amount);
        let gas_used = initial_gas - env.budget().gas_left();

        // Emit fund transfer event
        let transfer_event = FundTransferEvent {
            from: env.current_contract_address(),
            to: recipient.clone(),
            amount,
            token: token_address.clone(),
            purpose: String::from_slice(env, "bounty"),
            reference_id: None,
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((Symbol::short("FUND_TRANSFER"), transfer_event));

        // Log gas usage for monitoring
        env.log().format(
            &format!("Bounty transfer completed. Gas used: {}, Gas limit: {}, Amount: {}", 
                gas_used, gas_config.single_transfer_limit, amount)
        );

        Ok(())
    }

    /// Batch escrow release with gas limit management (#112)
    pub fn batch_escrow_release(
        env: Env,
        admin: Address,
        recipients: Vec<Address>,
        amounts: Vec<i128>,
        tokens: Vec<Address>,
    ) -> Result<u64, ContractError> {
        let contract_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if contract_admin != admin {
            return Err(ContractError::Unauthorized);
        }
        admin.require_auth();

        // Validate batch size
        if recipients.len() > MAX_BATCH_SIZE as usize {
            return Err(ContractError::BatchTooLarge);
        }

        if recipients.len() != amounts.len() || recipients.len() != tokens.len() {
            return Err(ContractError::InvalidInput);
        }

        let gas_config: GasConfig = env.storage().instance()
            .get(&GAS_CONFIG)
            .unwrap_or_else(|| GasConfig {
                single_transfer_limit: DEFAULT_SINGLE_TRANSFER_GAS,
                batch_transfer_limit: DEFAULT_BATCH_TRANSFER_GAS,
                emergency_limit: DEFAULT_EMERGENCY_GAS,
                max_batch_size: MAX_BATCH_SIZE,
                gas_price_multiplier: 100,
            });

        // Check if enough gas for batch operation
        let current_gas = env.budget().gas_left();
        let estimated_gas = gas_config.batch_transfer_limit * (recipients.len() as u64);
        
        if current_gas < estimated_gas {
            return Err(ContractError::InsufficientGas);
        }

        // Create batch record
        let batch_id = env.ledger().sequence();
        let total_amount: i128 = amounts.iter().sum();
        
        let batch = BatchRelease {
            id: batch_id,
            recipients: recipients.clone(),
            amounts: amounts.clone(),
            tokens: tokens.clone(),
            total_amount,
            gas_used: 0,
            status: String::from_slice(&env, "processing"),
            created_at: env.ledger().timestamp(),
        };

        env.storage().instance().set(&Symbol::short(&format!("BATCH_{}", batch_id)), &batch);

        // Process transfers with gas tracking
        let initial_gas = env.budget().gas_left();
        
        for (i, recipient) in recipients.iter().enumerate() {
            Self::transfer_bounty(&env, &tokens[i], recipient, amounts[i])?;
            
            // Check gas remaining after each transfer
            let remaining_gas = env.budget().gas_left();
            let estimated_remaining = initial_gas - (gas_config.single_transfer_limit * ((i + 1) as u64));
            
            if remaining_gas < estimated_remaining {
                // Update batch status to failed
                let mut updated_batch = batch.clone();
                updated_batch.status = String::from_slice(&env, "failed");
                updated_batch.gas_used = initial_gas - remaining_gas;
                env.storage().instance().set(&Symbol::short(&format!("BATCH_{}", batch_id)), &updated_batch);
                
                return Err(ContractError::GasLimitExceeded);
            }
        }

        // Mark batch as completed
        let mut completed_batch = batch.clone();
        completed_batch.status = String::from_slice(&env, "completed");
        completed_batch.gas_used = initial_gas - env.budget().gas_left();
        env.storage().instance().set(&Symbol::short(&format!("BATCH_{}", batch_id)), &completed_batch);

        // Emit batch escrow release event
        let batch_event = BatchEscrowReleaseEvent {
            batch_id,
            recipient_count: recipients.len() as u32,
            total_amount: completed_batch.total_amount,
            token: tokens[0].clone(), // All tokens should be same in batch
            gas_used: completed_batch.gas_used,
            initiated_by: admin.clone(),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((Symbol::short("BATCH_ESCROW_RELEASE"), batch_event));

        env.log().format(&format!("Batch escrow release {} completed. Total transfers: {}, Total gas used: {}", 
            batch_id, recipients.len(), completed_batch.gas_used));

        Ok(batch_id)
    }

    /// Emergency reward distribution with gas limit consideration (#112)
    pub fn emergency_reward_distribution(
        env: Env,
        admin: Address,
        alert_ids: Vec<u64>,
        token: Address,
    ) -> Result<u64, ContractError> {
        let contract_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if contract_admin != admin {
            return Err(ContractError::Unauthorized);
        }
        admin.require_auth();

        // Validate batch size
        if alert_ids.len() > MAX_BATCH_SIZE as usize {
            return Err(ContractError::BatchTooLarge);
        }

        let gas_config: GasConfig = env.storage().instance()
            .get(&GAS_CONFIG)
            .unwrap_or_else(|| GasConfig {
                single_transfer_limit: DEFAULT_SINGLE_TRANSFER_GAS,
                batch_transfer_limit: DEFAULT_BATCH_TRANSFER_GAS,
                emergency_limit: DEFAULT_EMERGENCY_GAS,
                max_batch_size: MAX_BATCH_SIZE,
                gas_price_multiplier: 100,
            });

        // Check if enough gas for emergency operation
        let current_gas = env.budget().gas_left();
        let estimated_gas = gas_config.emergency_limit * (alert_ids.len() as u64);
        
        if current_gas < estimated_gas {
            return Err(ContractError::InsufficientGas);
        }

        // Get emergency reward configuration
        let emergency_config: EmergencyRewardConfig = env.storage().instance()
            .get(&EMERGENCY_REWARDS)
            .unwrap_or_else(|| EmergencyRewardConfig {
                base_amount: 1000000i128,
                severity_multiplier: Map::new(&env),
                oracle_enabled: false,
                price_feed_address: admin.clone(),
            });

        // Calculate total reward amount
        let mut total_reward = 0i128;
        for alert_id in alert_ids.iter() {
            let alert_key = Symbol::short(&format!("ALERT_{}", alert_id));
            if let Some(alert) = env.storage().instance().get::<EmergencyAlert>(&alert_key) {
                let multiplier = emergency_config.severity_multiplier
                    .get(String::from_str(&env, &alert.severity))
                    .unwrap_or(&1000000i128);
                total_reward += emergency_config.base_amount * *multiplier / 1000000i128;
            }
        }

        // Create emergency batch record
        let batch_id = env.ledger().sequence();
        let emergency_batch = EmergencyBatch {
            id: batch_id,
            alert_ids: alert_ids.clone(),
            total_reward,
            token: token.clone(),
            gas_used: 0,
            status: String::from_slice(&env, "processing"),
            created_at: env.ledger().timestamp(),
        };

        env.storage().instance().set(&Symbol::short(&format!("EMERGENCY_BATCH_{}", batch_id)), &emergency_batch);

        // Process emergency rewards with gas tracking
        let initial_gas = env.budget().gas_left();
        
        for alert_id in alert_ids.iter() {
            let alert_key = Symbol::short(&format!("ALERT_{}", alert_id));
            if let Some(mut alert) = env.storage().instance().get::<EmergencyAlert>(&alert_key) {
                let multiplier = emergency_config.severity_multiplier
                    .get(String::from_str(&env, &alert.severity))
                    .unwrap_or(&1000000i128);
                let reward_amount = emergency_config.base_amount * *multiplier / 1000000i128;

                // Transfer emergency reward
                Self::transfer_bounty(&env, &token, &alert.reporter, reward_amount)?;
                
                // Update alert status
                alert.status = String::from_slice(&env, "verified");
                env.storage().instance().set(&alert_key, &alert);

                // Check gas remaining
                let remaining_gas = env.budget().gas_left();
                let estimated_remaining = initial_gas - (gas_config.emergency_limit * ((alert_ids.len() - alert_ids.iter().position(|&id| id == *alert_id).unwrap() + 1) as u64));
                
                if remaining_gas < estimated_remaining {
                    // Update batch status to failed
                    let mut updated_batch = emergency_batch.clone();
                    updated_batch.status = String::from_slice(&env, "failed");
                    updated_batch.gas_used = initial_gas - remaining_gas;
                    env.storage().instance().set(&Symbol::short(&format!("EMERGENCY_BATCH_{}", batch_id)), &updated_batch);
                    
                    return Err(ContractError::GasLimitExceeded);
                }
            }
        }

        // Mark emergency batch as completed
        let mut completed_batch = emergency_batch.clone();
        completed_batch.status = String::from_slice(&env, "completed");
        completed_batch.gas_used = initial_gas - env.budget().gas_left();
        env.storage().instance().set(&Symbol::short(&format!("EMERGENCY_BATCH_{}", batch_id)), &completed_batch);

        // Emit emergency reward distributed event
        let emergency_event = EmergencyRewardDistributedEvent {
            batch_id,
            alert_count: alert_ids.len() as u32,
            total_reward: completed_batch.total_reward,
            token: token.clone(),
            gas_used: completed_batch.gas_used,
            distributed_by: admin.clone(),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((Symbol::short("EMERGENCY_REWARD_DISTRIBUTED"), emergency_event));

        env.log().format(&format!("Emergency reward distribution {} completed. Total alerts: {}, Total reward: {}, Total gas used: {}", 
            batch_id, alert_ids.len(), total_reward, completed_batch.gas_used));

        Ok(batch_id)
    }

    /// Update gas configuration
    pub fn update_gas_config(
        env: Env,
        admin: Address,
        single_transfer_limit: Option<u64>,
        batch_transfer_limit: Option<u64>,
        emergency_limit: Option<u64>,
        max_batch_size: Option<u32>,
        gas_price_multiplier: Option<u32>,
    ) -> Result<(), ContractError> {
        let contract_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if contract_admin != admin {
            return Err(ContractError::Unauthorized);
        }
        admin.require_auth();

        let mut gas_config: GasConfig = env.storage().instance()
            .get(&GAS_CONFIG)
            .unwrap_or_else(|| GasConfig {
                single_transfer_limit: DEFAULT_SINGLE_TRANSFER_GAS,
                batch_transfer_limit: DEFAULT_BATCH_TRANSFER_GAS,
                emergency_limit: DEFAULT_EMERGENCY_GAS,
                max_batch_size: MAX_BATCH_SIZE,
                gas_price_multiplier: 100,
            });

        if let Some(limit) = single_transfer_limit {
            gas_config.single_transfer_limit = limit;
        }
        if let Some(limit) = batch_transfer_limit {
            gas_config.batch_transfer_limit = limit;
        }
        if let Some(limit) = emergency_limit {
            gas_config.emergency_limit = limit;
        }
        if let Some(size) = max_batch_size {
            gas_config.max_batch_size = size;
        }
        if let Some(multiplier) = gas_price_multiplier {
            gas_config.gas_price_multiplier = multiplier;
        }

        // Emit gas config updated event
        let gas_config_event = GasConfigUpdatedEvent {
            updated_by: admin.clone(),
            single_transfer_limit: gas_config.single_transfer_limit,
            batch_transfer_limit: gas_config.batch_transfer_limit,
            emergency_limit: gas_config.emergency_limit,
            max_batch_size: gas_config.max_batch_size,
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((Symbol::short("GAS_CONFIG_UPDATED"), gas_config_event));

        env.storage().instance().set(&GAS_CONFIG, &gas_config);
        Ok(())
    }

    /// Get gas configuration
    pub fn get_gas_config(env: Env) -> GasConfig {
        env.storage().instance()
            .get(&GAS_CONFIG)
            .unwrap_or_else(|| GasConfig {
                single_transfer_limit: DEFAULT_SINGLE_TRANSFER_GAS,
                batch_transfer_limit: DEFAULT_BATCH_TRANSFER_GAS,
                emergency_limit: DEFAULT_EMERGENCY_GAS,
                max_batch_size: MAX_BATCH_SIZE,
                gas_price_multiplier: 100,
            })
    }

    /// Get batch status
    pub fn get_batch_status(env: Env, batch_id: u64) -> Result<BatchRelease, ContractError> {
        let batch_key = Symbol::short(&format!("BATCH_{}", batch_id));
        env.storage().instance()
            .get(&batch_key)
            .ok_or(ContractError::NotFound)
    }

    /// Get emergency batch status
    pub fn get_emergency_batch_status(env: Env, batch_id: u64) -> Result<EmergencyBatch, ContractError> {
        let batch_key = Symbol::short(&format!("EMERGENCY_BATCH_{}", batch_id));
        env.storage().instance()
            .get(&batch_key)
            .ok_or(ContractError::NotFound)
    }
}

