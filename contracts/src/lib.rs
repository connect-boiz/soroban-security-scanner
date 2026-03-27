use soroban_sdk::{contractimpl, Address, BytesN, Env, Symbol, Vec, Map, String, I128, U128};

// Contract state keys
const ADMIN: Symbol = Symbol::short("ADMIN");
const BOUNTY_POOL: Symbol = Symbol::short("BOUNTY");
const VULNERABILITIES: Symbol = Symbol::short("VULNS");
const REPUTATION: Symbol = Symbol::short("REPUT");
const ESCROW: Symbol = Symbol::short("ESCROW");
const EMERGENCY_POOL: Symbol = Symbol::short("EMERG");

// Contract errors
pub enum ContractError {
    Unauthorized = 1,
    InvalidInput = 2,
    NotFound = 3,
    InsufficientFunds = 4,
    EscrowLocked = 5,
    InvalidEscrowStatus = 6,
    EmergencyModeActive = 7,
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
}

// Reputation tracking
#[derive(Clone)]
pub struct Reputation {
    pub researcher: Address,
    pub score: u64,
    pub successful_reports: u64,
    pub total_earnings: i128,
}

// Escrow structure
#[derive(Clone)]
pub struct EscrowEntry {
    pub id: u64,
    pub depositor: Address,
    pub beneficiary: Address,
    pub amount: i128,
    pub purpose: String, // "bounty", "reward", "emergency"
    pub status: String,  // "pending", "locked", "released", "refunded"
    pub created_at: u64,
    pub lock_until: u64,
    pub conditions_met: bool,
    pub release_signature: Option<BytesN<32>>,
}

// Emergency alert structure
#[derive(Clone)]
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
        env.storage().instance().set(&BOUNTY_POOL, &0i128);
        env.storage().instance().set(&EMERGENCY_POOL, &0i128);
        
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

        // Get vulnerability report
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
    pub fn fund_bounty_pool(env: Env, funder: Address, amount: i128) -> Result<(), ContractError> {
        funder.require_auth();
        
        let mut current_pool: i128 = env.storage().instance().get(&BOUNTY_POOL).unwrap_or(0i128);
        current_pool += amount;
        env.storage().instance().set(&BOUNTY_POOL, &current_pool);

        Ok(())
    }

    /// Get bounty pool balance
    pub fn get_bounty_pool(env: Env) -> i128 {
        env.storage().instance().get(&BOUNTY_POOL).unwrap_or(0i128)
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

    /// Create escrow entry for bounty payment
    pub fn create_escrow(
        env: Env,
        depositor: Address,
        beneficiary: Address,
        amount: i128,
        purpose: String,
        lock_duration: u64,
    ) -> Result<u64, ContractError> {
        depositor.require_auth();
        
        if amount <= 0 {
            return Err(ContractError::InvalidInput);
        }

        let escrow_id = env.ledger().sequence();
        let current_time = env.ledger().timestamp();
        
        let escrow = EscrowEntry {
            id: escrow_id,
            depositor: depositor.clone(),
            beneficiary: beneficiary.clone(),
            amount,
            purpose: purpose.clone(),
            status: String::from_slice(&env, "pending"),
            created_at: current_time,
            lock_until: current_time + lock_duration,
            conditions_met: false,
            release_signature: None,
        };

        // Store escrow entry
        let escrow_key = Symbol::short(&format!("ESCROW_{:?}", escrow_id));
        env.storage().instance().set(&escrow_key, &escrow);

        // Add to escrow pool tracking
        let mut escrow_pool: i128 = env.storage().instance().get(&ESCROW).unwrap_or(0i128);
        escrow_pool += amount;
        env.storage().instance().set(&ESCROW, &escrow_pool);

        Ok(escrow_id)
    }

    /// Release escrow funds to beneficiary
    pub fn release_escrow(
        env: Env,
        escrow_id: u64,
        depositor: Address,
        signature: Option<BytesN<32>>,
    ) -> Result<(), ContractError> {
        depositor.require_auth();

        let escrow_key = Symbol::short(&format!("ESCROW_{:?}", escrow_id));
        let mut escrow: EscrowEntry = env.storage().instance()
            .get(&escrow_key)
            .ok_or(ContractError::NotFound)?;

        // Verify depositor authorization
        if escrow.depositor != depositor {
            return Err(ContractError::Unauthorized);
        }

        // Check if escrow can be released
        if escrow.status == String::from_slice(&env, "released") {
            return Err(ContractError::InvalidEscrowStatus);
        }

        let current_time = env.ledger().timestamp();
        
        // Allow release if conditions are met or lock period has expired
        if !escrow.conditions_met && current_time < escrow.lock_until {
            return Err(ContractError::EscrowLocked);
        }

        // Update escrow status
        escrow.status = String::from_slice(&env, "released");
        escrow.release_signature = signature;
        env.storage().instance().set(&escrow_key, &escrow);

        // Update escrow pool
        let mut escrow_pool: i128 = env.storage().instance().get(&ESCROW).unwrap_or(0i128);
        escrow_pool -= escrow.amount;
        env.storage().instance().set(&ESCROW, &escrow_pool);

        // In a real implementation, you would transfer the tokens here
        // For now, we just update the state

        Ok(())
    }

    /// Refund escrow funds to depositor
    pub fn refund_escrow(
        env: Env,
        escrow_id: u64,
        depositor: Address,
    ) -> Result<(), ContractError> {
        depositor.require_auth();

        let escrow_key = Symbol::short(&format!("ESCROW_{:?}", escrow_id));
        let mut escrow: EscrowEntry = env.storage().instance()
            .get(&escrow_key)
            .ok_or(ContractError::NotFound)?;

        // Verify depositor authorization
        if escrow.depositor != depositor {
            return Err(ContractError::Unauthorized);
        }

        // Check if escrow can be refunded
        if escrow.status == String::from_slice(&env, "released") {
            return Err(ContractError::InvalidEscrowStatus);
        }

        let current_time = env.ledger().timestamp();
        
        // Only allow refund if lock period has expired and conditions not met
        if current_time < escrow.lock_until || escrow.conditions_met {
            return Err(ContractError::EscrowLocked);
        }

        // Update escrow status
        escrow.status = String::from_slice(&env, "refunded");
        env.storage().instance().set(&escrow_key, &escrow);

        // Update escrow pool
        let mut escrow_pool: i128 = env.storage().instance().get(&ESCROW).unwrap_or(0i128);
        escrow_pool -= escrow.amount;
        env.storage().instance().set(&ESCROW, &escrow_pool);

        // In a real implementation, you would transfer tokens back to depositor
        // For now, we just update the state

        Ok(())
    }

    /// Mark escrow conditions as met (for bounty completion)
    pub fn mark_escrow_conditions_met(
        env: Env,
        escrow_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        // Verify admin authorization
        let contract_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if contract_admin != admin {
            return Err(ContractError::Unauthorized);
        }
        
        admin.require_auth();

        let escrow_key = Symbol::short(&format!("ESCROW_{:?}", escrow_id));
        let mut escrow: EscrowEntry = env.storage().instance()
            .get(&escrow_key)
            .ok_or(ContractError::NotFound)?;

        escrow.conditions_met = true;
        env.storage().instance().set(&escrow_key, &escrow);

        Ok(())
    }

    /// Report emergency vulnerability
    pub fn report_emergency_vulnerability(
        env: Env,
        reporter: Address,
        contract_id: BytesN<32>,
        vulnerability_type: String,
        severity: String,
        description: String,
        location: String,
    ) -> Result<u64, ContractError> {
        // Verify severity is critical or emergency
        if severity != String::from_slice(&env, "critical") && severity != String::from_slice(&env, "emergency") {
            return Err(ContractError::InvalidInput);
        }

        reporter.require_auth();

        let alert_id = env.ledger().sequence();
        let emergency_reward = if severity == String::from_slice(&env, "emergency") { 
            10000000i128 // 10 XLM for emergency
        } else { 
            5000000i128 // 5 XLM for critical
        };

        let alert = EmergencyAlert {
            id: alert_id,
            reporter: reporter.clone(),
            contract_id,
            vulnerability_type: vulnerability_type.clone(),
            severity: severity.clone(),
            description: description.clone(),
            location: location.clone(),
            timestamp: env.ledger().timestamp(),
            status: String::from_slice(&env, "pending"),
            emergency_reward,
            verified_by: None,
        };

        // Store emergency alert
        let alert_key = Symbol::short(&format!("EMERG_{:?}", alert_id));
        env.storage().instance().set(&alert_key, &alert);

        Ok(alert_id)
    }

    /// Verify emergency vulnerability and trigger immediate reward
    pub fn verify_emergency_vulnerability(
        env: Env,
        admin: Address,
        alert_id: u64,
        verified: bool,
    ) -> Result<(), ContractError> {
        // Verify admin authorization
        let contract_admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        if contract_admin != admin {
            return Err(ContractError::Unauthorized);
        }
        
        admin.require_auth();

        let alert_key = Symbol::short(&format!("EMERG_{:?}", alert_id));
        let mut alert: EmergencyAlert = env.storage().instance()
            .get(&alert_key)
            .ok_or(ContractError::NotFound)?;

        if verified {
            alert.status = String::from_slice(&env, "verified");
            alert.verified_by = Some(admin.clone());

            // Create immediate escrow for emergency reward
            let escrow_id = Self::create_escrow(
                env,
                admin.clone(), // Admin deposits on behalf of the platform
                alert.reporter.clone(),
                alert.emergency_reward,
                String::from_slice(&env, "emergency"),
                0, // No lock period for emergency rewards
            )?;

            // Immediately mark conditions as met and release
            Self::mark_escrow_conditions_met(env, escrow_id, admin)?;
            Self::release_escrow(env, escrow_id, admin, None)?;

            // Update reputation
            Self::update_reputation(env, alert.reporter, 1, alert.emergency_reward)?;
        } else {
            alert.status = String::from_slice(&env, "false_positive");
        }

        env.storage().instance().set(&alert_key, &alert);

        Ok(())
    }

    /// Get escrow details
    pub fn get_escrow(env: Env, escrow_id: u64) -> Result<EscrowEntry, ContractError> {
        let escrow_key = Symbol::short(&format!("ESCROW_{:?}", escrow_id));
        env.storage().instance()
            .get(&escrow_key)
            .ok_or(ContractError::NotFound)
    }

    /// Get emergency alert details
    pub fn get_emergency_alert(env: Env, alert_id: u64) -> Result<EmergencyAlert, ContractError> {
        let alert_key = Symbol::short(&format!("EMERG_{:?}", alert_id));
        env.storage().instance()
            .get(&alert_key)
            .ok_or(ContractError::NotFound)
    }

    /// Get escrow pool balance
    pub fn get_escrow_pool_balance(env: Env) -> i128 {
        env.storage().instance().get(&ESCROW).unwrap_or(0i128)
    }

    /// Get emergency pool balance
    pub fn get_emergency_pool_balance(env: Env) -> i128 {
        env.storage().instance().get(&EMERGENCY_POOL).unwrap_or(0i128)
    }

    /// Fund emergency pool
    pub fn fund_emergency_pool(env: Env, funder: Address, amount: i128) -> Result<(), ContractError> {
        funder.require_auth();
        
        let mut current_pool: i128 = env.storage().instance().get(&EMERGENCY_POOL).unwrap_or(0i128);
        current_pool += amount;
        env.storage().instance().set(&EMERGENCY_POOL, &current_pool);

        Ok(())
    }
}
