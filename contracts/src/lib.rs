


// Contract state keys
const ADMIN: Symbol = Symbol::short("ADMIN");
const BOUNTY_POOL: Symbol = Symbol::short("BOUNTY");
const VULNERABILITIES: Symbol = Symbol::short("VULNS");
const REPUTATION: Symbol = Symbol::short("REPUT");
const SUPPORTED_TOKENS: Symbol = Symbol::short("TOKENS");
const LIQUIDITY_THRESHOLD: Symbol = Symbol::short("LIQ_TH");
const SLIPPAGE_TOLERANCE: Symbol = Symbol::short("SLIP_TOL");
const EMERGENCY_REWARDS: Symbol = Symbol::short("EMERG");
const ORACLE: Symbol = Symbol::short("ORACLE");
const TOKENS: Symbol = Symbol::short("TOKENS");
const CONTRACT_VERSION: Symbol = Symbol::short("VERSION");
const UPGRADE_AUTHORITY: Symbol = Symbol::short("UPGRADE");
const UPGRADE_DELAY: Symbol = Symbol::short("UP_DELAY");
const PENDING_UPGRADE: Symbol = Symbol::short("PENDING");
const UPGRADE_HISTORY: Symbol = Symbol::short("UP_HISTORY");



// Contract errors
pub enum ContractError {
    Unauthorized = 1,
    InvalidInput = 2,
    NotFound = 3,
    InsufficientFunds = 4,
    TokenNotSupported = 5,
    UpgradeInProgress = 6,
    UpgradeNotReady = 7,
    InvalidUpgrade = 8,
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

// Token information structure
#[derive(Clone)]
#[contracttype]
pub struct TokenInfo {
    pub address: Address,
    pub decimals: u32,
    pub minimum_liquidity: i128,
    pub enabled: bool,
}

// Emergency reward configuration
#[derive(Clone)]
#[contracttype]
pub struct EmergencyRewardConfig {
    pub base_amount: i128,
    pub severity_multiplier: Map<String, i128>,
    pub oracle_enabled: bool,
    pub price_feed_address: Address,
}

// Upgrade request structure
#[derive(Clone)]
#[contracttype]
pub struct UpgradeRequest {
    pub new_contract_address: Address,
    pub proposed_by: Address,
    pub timestamp: u64,
    pub ready_at: u64,
    pub reason: String,
    pub version: String,
}

// Upgrade history entry
#[derive(Clone)]
#[contracttype]
pub struct UpgradeHistory {
    pub from_version: String,
    pub to_version: String,
    pub timestamp: u64,
    pub upgraded_by: Address,
    pub old_contract: Address,
    pub new_contract: Address,
}

// Contract state snapshot for migration
#[derive(Clone)]
#[contracttype]
pub struct ContractStateSnapshot {
    pub admin: Address,
    pub version: String,
    pub bounty_pools: Map<Address, i128>,
    pub supported_tokens: Map<Address, TokenInfo>,
    pub emergency_rewards: EmergencyRewardConfig,
    pub upgrade_authority: Address,
    pub upgrade_delay: u64,
    pub upgrade_history: Vec<UpgradeHistory>,
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
        
        // Set contract version
        env.storage().instance().set(&CONTRACT_VERSION, &"1.0.0");
        
        // Set upgrade authority (initially admin)
        env.storage().instance().set(&UPGRADE_AUTHORITY, &admin);
        
        // Set upgrade delay (7 days in seconds)
        env.storage().instance().set(&UPGRADE_DELAY, &604800u64);
        
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
        bounty_amount: i128,
    ) -> Result<(), ContractError> {
        // Verify admin authorization
        let contract_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if contract_admin != admin {
            return Err(ContractError::Unauthorized);
        }
        admin.require_auth();

        // Get the vulnerability report
        let report_key = Symbol::short(&report_id.to_string());
        let mut report: VulnerabilityReport = env.storage().instance()
            .get(&report_key)
            .ok_or(ContractError::NotFound)?;

        // Update status and bounty
        report.status = String::from_slice(&env, "verified");
        report.bounty_amount = bounty_amount;
        
        // Store updated report
        env.storage().instance().set(&report_key, &report);

        // Update researcher reputation
        Self::update_reputation(env, report.reporter, 1, bounty_amount)?;

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
    pub fn add_bounty_funds(
        env: Env,
        funder: Address,
        token: Address,
        amount: i128,
    ) -> Result<(), ContractError> {
        funder.require_auth();

        let client = token::Client::new(&env, &token);
        client.transfer(&funder, &env.current_contract_address(), &amount);

        let mut pools: Map<Address, i128> = env.storage().instance().get(&BOUNTY_POOL).unwrap_or(Map::new(&env));
        let current_balance = pools.get(token).unwrap_or(0i128);
        pools.set(token, current_balance + amount);
        env.storage().instance().set(&BOUNTY_POOL, &pools);

        Ok(())
    }

    /// Get bounty pool balance
    pub fn get_bounty_pool(env: Env, token: Address) -> i128 {
        let pools: Map<Address, i128> = env.storage().instance().get(&BOUNTY_POOL).unwrap_or(Map::new(&env));
        pools.get(token).unwrap_or(0i128)
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
            .unwrap_or(Map::new(&env));

        let token_info = TokenInfo {
            address: token_address.clone(),
            decimals,
            minimum_liquidity,
            enabled: true,
        };

        supported_tokens.set(token_address, token_info);
        env.storage().instance().set(&SUPPORTED_TOKENS, &supported_tokens);

        Ok(())
    }

    /// Configure oracle settings
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

    /// Set slippage tolerance
    pub fn set_slippage_tolerance(
        env: Env,
        admin: Address,
        tolerance_bps: i128,
    ) -> Result<(), ContractError> {
        // Verify admin authorization
        let contract_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if contract_admin != admin {
            return Err(ContractError::Unauthorized);
        }
        admin.require_auth();

        // Validate tolerance is between 0 and 10000 (100%)
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

    // ===== UPGRADE MECHANISM FUNCTIONS =====

    /// Get current contract version
    pub fn get_version(env: Env) -> String {
        env.storage().instance()
            .get(&CONTRACT_VERSION)
            .unwrap_or_else(|| "1.0.0".into_val(&env))
    }

    /// Propose a contract upgrade
    pub fn propose_upgrade(
        env: Env,
        proposer: Address,
        new_contract_address: Address,
        new_version: String,
        reason: String,
    ) -> Result<(), ContractError> {
        // Check if proposer is upgrade authority
        let upgrade_authority: Address = env.storage().instance().get(&UPGRADE_AUTHORITY).unwrap();
        if proposer != upgrade_authority {
            return Err(ContractError::Unauthorized);
        }
        proposer.require_auth();

        // Check if there's already a pending upgrade
        if env.storage().instance().has(&PENDING_UPGRADE) {
            return Err(ContractError::UpgradeInProgress);
        }

        // Get upgrade delay
        let upgrade_delay: u64 = env.storage().instance().get(&UPGRADE_DELAY).unwrap_or(604800);
        let current_time = env.ledger().timestamp();
        let ready_time = current_time + upgrade_delay;

        // Create upgrade request
        let upgrade_request = UpgradeRequest {
            new_contract_address: new_contract_address.clone(),
            proposed_by: proposer,
            timestamp: current_time,
            ready_at: ready_time,
            reason: reason.clone(),
            version: new_version.clone(),
        };

        env.storage().instance().set(&PENDING_UPGRADE, &upgrade_request);

        Ok(())
    }

    /// Execute a pending upgrade
    pub fn execute_upgrade(
        env: Env,
        executor: Address,
    ) -> Result<(), ContractError> {
        // Check if executor is upgrade authority
        let upgrade_authority: Address = env.storage().instance().get(&UPGRADE_AUTHORITY).unwrap();
        if executor != upgrade_authority {
            return Err(ContractError::Unauthorized);
        }
        executor.require_auth();

        // Get pending upgrade
        let pending_upgrade: UpgradeRequest = env.storage().instance()
            .get(&PENDING_UPGRADE)
            .ok_or(ContractError::NotFound)?;

        // Check if upgrade delay has passed
        let current_time = env.ledger().timestamp();
        if current_time < pending_upgrade.ready_at {
            return Err(ContractError::UpgradeNotReady);
        }

        // Get current version and contract address
        let current_version: String = env.storage().instance().get(&CONTRACT_VERSION).unwrap();
        let current_contract = env.current_contract_address();

        // Create upgrade history entry
        let history_entry = UpgradeHistory {
            from_version: current_version.clone(),
            to_version: pending_upgrade.version.clone(),
            timestamp: current_time,
            upgraded_by: executor,
            old_contract: current_contract.clone(),
            new_contract: pending_upgrade.new_contract_address.clone(),
        };

        // Add to upgrade history
        let mut history: Vec<UpgradeHistory> = env.storage().instance()
            .get(&UPGRADE_HISTORY)
            .unwrap_or(Vec::new(&env));
        history.push_back(history_entry);
        env.storage().instance().set(&UPGRADE_HISTORY, &history);

        // Clear pending upgrade
        env.storage().instance().remove(&PENDING_UPGRADE);

        // Migrate state to new contract (this would be implemented in the new contract)
        Self::migrate_state(env, pending_upgrade.new_contract_address)?;

        Ok(())
    }

    /// Cancel a pending upgrade
    pub fn cancel_upgrade(
        env: Env,
        canceler: Address,
    ) -> Result<(), ContractError> {
        // Check if canceler is upgrade authority
        let upgrade_authority: Address = env.storage().instance().get(&UPGRADE_AUTHORITY).unwrap();
        if canceler != upgrade_authority {
            return Err(ContractError::Unauthorized);
        }
        canceler.require_auth();

        // Remove pending upgrade
        env.storage().instance().remove(&PENDING_UPGRADE);

        Ok(())
    }

    /// Set upgrade authority
    pub fn set_upgrade_authority(
        env: Env,
        admin: Address,
        new_authority: Address,
    ) -> Result<(), ContractError> {
        // Check if caller is admin
        let contract_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if contract_admin != admin {
            return Err(ContractError::Unauthorized);
        }
        admin.require_auth();

        env.storage().instance().set(&UPGRADE_AUTHORITY, &new_authority);
        Ok(())
    }

    /// Set upgrade delay
    pub fn set_upgrade_delay(
        env: Env,
        admin: Address,
        delay_seconds: u64,
    ) -> Result<(), ContractError> {
        // Check if caller is admin
        let contract_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if contract_admin != admin {
            return Err(ContractError::Unauthorized);
        }
        admin.require_auth();

        // Minimum delay of 24 hours
        if delay_seconds < 86400 {
            return Err(ContractError::InvalidInput);
        }

        env.storage().instance().set(&UPGRADE_DELAY, &delay_seconds);
        Ok(())
    }

    /// Get pending upgrade information
    pub fn get_pending_upgrade(env: Env) -> Result<UpgradeRequest, ContractError> {
        env.storage().instance()
            .get(&PENDING_UPGRADE)
            .ok_or(ContractError::NotFound)
    }

    /// Get upgrade history
    pub fn get_upgrade_history(env: Env) -> Vec<UpgradeHistory> {
        env.storage().instance()
            .get(&UPGRADE_HISTORY)
            .unwrap_or(Vec::new(&env))
    }

    /// Migrate state to new contract
    fn migrate_state(env: Env, new_contract: Address) -> Result<(), ContractError> {
        // Create a snapshot of all critical state
        let state_snapshot = ContractStateSnapshot {
            admin: env.storage().instance().get(&ADMIN).unwrap(),
            version: env.storage().instance().get(&CONTRACT_VERSION).unwrap(),
            bounty_pools: env.storage().instance().get(&BOUNTY_POOL).unwrap_or(Map::new(&env)),
            supported_tokens: env.storage().instance().get(&SUPPORTED_TOKENS).unwrap_or(Map::new(&env)),
            emergency_rewards: env.storage().instance().get(&EMERGENCY_REWARDS).unwrap_or(EmergencyRewardConfig {
                base_amount: 1000000i128,
                severity_multiplier: Map::new(&env),
                oracle_enabled: false,
                price_feed_address: env.storage().instance().get(&ADMIN).unwrap(),
            }),
            upgrade_authority: env.storage().instance().get(&UPGRADE_AUTHORITY).unwrap(),
            upgrade_delay: env.storage().instance().get(&UPGRADE_DELAY).unwrap_or(604800),
            upgrade_history: env.storage().instance().get(&UPGRADE_HISTORY).unwrap_or(Vec::new(&env)),
        };

        // In a real implementation, you would:
        // 1. Call the new contract's initialize_migration function
        // 2. Transfer the state snapshot
        // 3. Verify the migration was successful
        // 4. Clear old state if migration succeeded
        
        // For this example, we'll store the migration intent
        env.storage().instance().set(&Symbol::short("MIGRATION_INTENT"), &new_contract);
        env.storage().instance().set(&Symbol::short("MIGRATION_TIME"), &env.ledger().timestamp());
        
        Ok(())
    }

    /// Get migration status
    pub fn get_migration_status(env: Env) -> Option<(Address, u64)> {
        let contract: Option<Address> = env.storage().instance().get(&Symbol::short("MIGRATION_INTENT"));
        let time: Option<u64> = env.storage().instance().get(&Symbol::short("MIGRATION_TIME"));
        
        match (contract, time) {
            (Some(c), Some(t)) => Some((c, t)),
            _ => None,
        }
    }

    /// Emergency upgrade for critical security patches
    pub fn emergency_upgrade(
        env: Env,
        admin: Address,
        new_contract_address: Address,
        new_version: String,
        reason: String,
    ) -> Result<(), ContractError> {
        // Check if caller is admin
        let contract_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if contract_admin != admin {
            return Err(ContractError::Unauthorized);
        }
        admin.require_auth();

        // Check if there's already a pending upgrade
        if env.storage().instance().has(&PENDING_UPGRADE) {
            return Err(ContractError::UpgradeInProgress);
        }

        // Create immediate upgrade request (no delay)
        let current_time = env.ledger().timestamp();
        let upgrade_request = UpgradeRequest {
            new_contract_address: new_contract_address.clone(),
            proposed_by: admin,
            timestamp: current_time,
            ready_at: current_time, // No delay for emergency
            reason: format!("EMERGENCY: {}", reason),
            version: new_version,
        };

        env.storage().instance().set(&PENDING_UPGRADE, &upgrade_request);

        // Immediately execute the upgrade
        Self::execute_upgrade(env, admin)
    }

    // ===== GOVERNANCE FUNCTIONS =====

    /// Create upgrade proposal with multi-sig support
    pub fn create_upgrade_proposal(
        env: Env,
        proposer: Address,
        new_contract_address: Address,
        new_version: String,
        reason: String,
        required_signatures: u32,
    ) -> Result<u64, ContractError> {
        // Check if proposer is upgrade authority
        let upgrade_authority: Address = env.storage().instance().get(&UPGRADE_AUTHORITY).unwrap();
        if proposer != upgrade_authority {
            return Err(ContractError::Unauthorized);
        }
        proposer.require_auth();

        // Check if there's already a pending upgrade
        if env.storage().instance().has(&PENDING_UPGRADE) {
            return Err(ContractError::UpgradeInProgress);
        }

        // Create proposal with governance tracking
        let proposal_id = env.ledger().sequence();
        let proposal_key = Symbol::short(&format!("PROP_{}", proposal_id));
        
        let governance_proposal = GovernanceUpgradeProposal {
            proposal_id,
            new_contract_address,
            proposed_by: proposer,
            timestamp: env.ledger().timestamp(),
            ready_at: env.ledger().timestamp() + env.storage().instance().get(&UPGRADE_DELAY).unwrap_or(604800),
            reason,
            version: new_version,
            required_signatures,
            collected_signatures: Vec::new(&env),
            status: String::from_str(&env, "pending"),
        };

        env.storage().instance().set(&proposal_key, &governance_proposal);
        Ok(proposal_id)
    }

    /// Sign upgrade proposal
    pub fn sign_upgrade_proposal(
        env: Env,
        signer: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        signer.require_auth();

        let proposal_key = Symbol::short(&format!("PROP_{}", proposal_id));
        let mut proposal: GovernanceUpgradeProposal = env.storage().instance()
            .get(&proposal_key)
            .ok_or(ContractError::NotFound)?;

        // Check if already signed
        if proposal.collected_signatures.contains(&signer) {
            return Err(ContractError::InvalidInput);
        }

        // Add signature
        proposal.collected_signatures.push_back(signer);
        env.storage().instance().set(&proposal_key, &proposal);

        Ok(())
    }

    /// Execute governance upgrade
    pub fn execute_governance_upgrade(
        env: Env,
        executor: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        // Check if executor is upgrade authority
        let upgrade_authority: Address = env.storage().instance().get(&UPGRADE_AUTHORITY).unwrap();
        if executor != upgrade_authority {
            return Err(ContractError::Unauthorized);
        }
        executor.require_auth();

        let proposal_key = Symbol::short(&format!("PROP_{}", proposal_id));
        let proposal: GovernanceUpgradeProposal = env.storage().instance()
            .get(&proposal_key)
            .ok_or(ContractError::NotFound)?;

        // Check if enough signatures collected
        if proposal.collected_signatures.len() < proposal.required_signatures as usize {
            return Err(ContractError::InvalidInput);
        }

        // Check if delay has passed
        let current_time = env.ledger().timestamp();
        if current_time < proposal.ready_at {
            return Err(ContractError::UpgradeNotReady);
        }

        // Execute the upgrade
        let upgrade_request = UpgradeRequest {
            new_contract_address: proposal.new_contract_address,
            proposed_by: proposal.proposed_by,
            timestamp: proposal.timestamp,
            ready_at: proposal.ready_at,
            reason: proposal.reason,
            version: proposal.version,
        };

        env.storage().instance().set(&PENDING_UPGRADE, &upgrade_request);
        Self::execute_upgrade(env, executor)?;

        // Clean up proposal
        env.storage().instance().remove(&proposal_key);

        Ok(())
    }
}

// Governance proposal structure
#[derive(Clone)]
#[contracttype]
pub struct GovernanceUpgradeProposal {
    pub proposal_id: u64,
    pub new_contract_address: Address,
    pub proposed_by: Address,
    pub timestamp: u64,
    pub ready_at: u64,
    pub reason: String,
    pub version: String,
    pub required_signatures: u32,
    pub collected_signatures: Vec<Address>,
    pub status: String,
}

// Include test module
#[cfg(test)]
mod tests;
