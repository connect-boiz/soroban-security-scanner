use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Bytes, BytesN, Env, Map, String, Symbol, Vec};

// Contract state keys
const ADMIN: Symbol = Symbol::short("ADMIN");
const BOUNTY_POOL: Symbol = Symbol::short("BOUNTY");
const VULNERABILITIES: Symbol = Symbol::short("VULNS");
const REPUTATION: Symbol = Symbol::short("REPUT");
const ESCROW: Symbol = Symbol::short("ESCROW");
const EMERGENCY_POOL: Symbol = Symbol::short("EMERG");
const REPORTS: Symbol = Symbol::short("RPTS");
const ESCROWS: Symbol = Symbol::short("ESCRS");
const EMERGENCY_ALERTS: Symbol = Symbol::short("EALRTS");
const REPORT_COUNTER: Symbol = Symbol::short("RPTCTR");
const ESCROW_COUNTER: Symbol = Symbol::short("ESCCTR");
const ALERT_COUNTER: Symbol = Symbol::short("ALRTCTR");
const REPORT_NONCES: Symbol = Symbol::short("RPTNONC");
const ESCROW_NONCES: Symbol = Symbol::short("ESCNONC");
const ALERT_NONCES: Symbol = Symbol::short("ALRNONC");
const MAX_TEXT_LEN: u32 = 280;

// Contract errors
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ContractError {
    Unauthorized = 1,
    InvalidInput = 2,
    NotFound = 3,
    InsufficientFunds = 4,
    EscrowLocked = 5,
    InvalidEscrowStatus = 6,
    EmergencyModeActive = 7,
    Overflow = 8,
    ExternalCallFailed = 9,
}

// Vulnerability structure
#[derive(Clone, Debug, Eq, PartialEq)]
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
}

// Reputation tracking
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Reputation {
    pub researcher: Address,
    pub score: u64,
    pub successful_reports: u64,
    pub total_earnings: i128,
}

// Escrow structure
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
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
    pub release_signature: Option<Bytes>,
}

// Emergency alert structure
#[derive(Clone, Debug, Eq, PartialEq)]
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
    pub verified_by: Option<Address>,
}

#[contract]
pub struct SecurityScannerContract;

#[contractimpl]
impl SecurityScannerContract {
    fn require_non_default_address(addr: &Address) -> Result<(), ContractError> {
        let _ = addr;
        Ok(())
    }

    fn require_positive_amount(amount: i128) -> Result<(), ContractError> {
        if amount <= 0 {
            return Err(ContractError::InvalidInput);
        }
        Ok(())
    }

    fn require_valid_text(value: &String) -> Result<(), ContractError> {
        if value.is_empty() || value.len() > MAX_TEXT_LEN {
            return Err(ContractError::InvalidInput);
        }
        Ok(())
    }

    fn checked_add_i128(a: i128, b: i128) -> Result<i128, ContractError> {
        a.checked_add(b).ok_or(ContractError::Overflow)
    }

    fn checked_sub_i128(a: i128, b: i128) -> Result<i128, ContractError> {
        a.checked_sub(b).ok_or(ContractError::Overflow)
    }

    fn checked_add_u64(a: u64, b: u64) -> Result<u64, ContractError> {
        a.checked_add(b).ok_or(ContractError::Overflow)
    }

    fn checked_mul_u64(a: u64, b: u64) -> Result<u64, ContractError> {
        a.checked_mul(b).ok_or(ContractError::Overflow)
    }

    fn checked_non_negative_i128_to_u64(value: i128) -> Result<u64, ContractError> {
        if value < 0 {
            return Ok(0);
        }
        u64::try_from(value).map_err(|_| ContractError::Overflow)
    }

    fn next_counter(env: &Env, key: Symbol) -> Result<u64, ContractError> {
        let current: u64 = env.storage().instance().get(&key).unwrap_or(0u64);
        let next = Self::checked_add_u64(current, 1)?;
        env.storage().instance().set(&key, &next);
        Ok(next)
    }

    fn generate_nonce(env: &Env, seed: &Address, counter: u64) -> BytesN<32> {
        let payload = format!(
            "{:?}:{}:{}:{}",
            seed,
            counter,
            env.ledger().sequence(),
            env.ledger().timestamp()
        );
        let bytes = Bytes::from_slice(env, payload.as_bytes());
        env.crypto().sha256(&bytes).into()
    }

    fn execute_payout_placeholder(
        env: &Env,
        recipient: &Address,
        amount: i128,
        escrow_id: u64,
    ) -> Result<(), ContractError> {
        if amount <= 0 {
            return Err(ContractError::ExternalCallFailed);
        }
        env.events().publish(
            (Symbol::new(env, "payout_ready"),),
            (escrow_id, recipient.clone(), amount),
        );
        Ok(())
    }
    
    /// Initialize the contract with admin address
    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        if env.storage().instance().has(&ADMIN) {
            return Err(ContractError::Unauthorized);
        }
        
        Self::require_non_default_address(&admin)?;
        env.storage().instance().set(&ADMIN, &admin);
        env.storage().instance().set(&BOUNTY_POOL, &0i128);
        env.storage().instance().set(&EMERGENCY_POOL, &0i128);
        env.storage().instance().set(&REPORTS, &Map::<u64, VulnerabilityReport>::new(&env));
        env.storage().instance().set(&ESCROWS, &Map::<u64, EscrowEntry>::new(&env));
        env.storage().instance().set(&EMERGENCY_ALERTS, &Map::<u64, EmergencyAlert>::new(&env));
        env.storage().instance().set(&REPORT_NONCES, &Map::<u64, BytesN<32>>::new(&env));
        env.storage().instance().set(&ESCROW_NONCES, &Map::<u64, BytesN<32>>::new(&env));
        env.storage().instance().set(&ALERT_NONCES, &Map::<u64, BytesN<32>>::new(&env));
        
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
        Self::require_non_default_address(&reporter)?;
        Self::require_valid_text(&vulnerability_type)?;
        Self::require_valid_text(&severity)?;
        Self::require_valid_text(&description)?;
        Self::require_valid_text(&location)?;
        
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

        let report_id = Self::next_counter(&env, REPORT_COUNTER)?;
        let report_nonce = Self::generate_nonce(&env, &reporter, report_id);
        let mut reports: Map<u64, VulnerabilityReport> = env.storage().instance().get(&REPORTS).unwrap_or(Map::new(&env));
        reports.set(report_id, report);
        env.storage().instance().set(&REPORTS, &reports);
        let mut report_nonces: Map<u64, BytesN<32>> = env.storage().instance().get(&REPORT_NONCES).unwrap_or(Map::new(&env));
        report_nonces.set(report_id, report_nonce);
        env.storage().instance().set(&REPORT_NONCES, &report_nonces);

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
        Self::require_positive_amount(bounty_amount)?;

        // Get vulnerability report
        let mut reports: Map<u64, VulnerabilityReport> = env.storage().instance().get(&REPORTS).unwrap_or(Map::new(&env));
        let mut report: VulnerabilityReport = reports
            .get(report_id)
            .ok_or(ContractError::NotFound)?;

        // Update status and bounty
        report.status = String::from_slice(&env, "verified");
        report.bounty_amount = bounty_amount;
        
        // Store updated report
        reports.set(report_id, report.clone());
        env.storage().instance().set(&REPORTS, &reports);

        // Update researcher reputation
        Self::update_reputation(env, report.reporter, 1, bounty_amount)?;

        Ok(())
    }

    /// Get vulnerability report
    pub fn get_vulnerability(env: Env, report_id: u64) -> Result<VulnerabilityReport, ContractError> {
        let reports: Map<u64, VulnerabilityReport> = env.storage().instance().get(&REPORTS).unwrap_or(Map::new(&env));
        reports
            .get(report_id)
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
        Self::require_non_default_address(&funder)?;
        Self::require_positive_amount(amount)?;
        
        let mut current_pool: i128 = env.storage().instance().get(&BOUNTY_POOL).unwrap_or(0i128);
        current_pool = Self::checked_add_i128(current_pool, amount)?;
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

        reputation.successful_reports = Self::checked_add_u64(reputation.successful_reports, successful_reports)?;
        reputation.total_earnings = Self::checked_add_i128(reputation.total_earnings, earnings)?;
        let score_from_reports = Self::checked_mul_u64(reputation.successful_reports, 10)?;
        let score_from_earnings = Self::checked_non_negative_i128_to_u64(reputation.total_earnings / 1_000_000)?;
        reputation.score = Self::checked_add_u64(score_from_reports, score_from_earnings)?;

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
        Self::require_non_default_address(&depositor)?;
        Self::require_non_default_address(&beneficiary)?;
        Self::require_positive_amount(amount)?;
        Self::require_valid_text(&purpose)?;

        let escrow_id = Self::next_counter(&env, ESCROW_COUNTER)?;
        let current_time = env.ledger().timestamp();
        let lock_until = Self::checked_add_u64(current_time, lock_duration)?;
        let escrow_nonce = Self::generate_nonce(&env, &beneficiary, escrow_id);
        
        let escrow = EscrowEntry {
            id: escrow_id,
            depositor: depositor.clone(),
            beneficiary: beneficiary.clone(),
            amount,
            purpose: purpose.clone(),
            status: String::from_slice(&env, "pending"),
            created_at: current_time,
            lock_until,
            conditions_met: false,
            release_signature: None,
        };

        let mut escrows: Map<u64, EscrowEntry> = env.storage().instance().get(&ESCROWS).unwrap_or(Map::new(&env));
        escrows.set(escrow_id, escrow);
        env.storage().instance().set(&ESCROWS, &escrows);
        let mut escrow_nonces: Map<u64, BytesN<32>> = env.storage().instance().get(&ESCROW_NONCES).unwrap_or(Map::new(&env));
        escrow_nonces.set(escrow_id, escrow_nonce);
        env.storage().instance().set(&ESCROW_NONCES, &escrow_nonces);

        // Add to escrow pool tracking
        let mut escrow_pool: i128 = env.storage().instance().get(&ESCROW).unwrap_or(0i128);
        escrow_pool = Self::checked_add_i128(escrow_pool, amount)?;
        env.storage().instance().set(&ESCROW, &escrow_pool);

        Ok(escrow_id)
    }

    /// Release escrow funds to beneficiary
    pub fn release_escrow(
        env: Env,
        escrow_id: u64,
        depositor: Address,
        signature: Option<Bytes>,
    ) -> Result<(), ContractError> {
        depositor.require_auth();

        let mut escrows: Map<u64, EscrowEntry> = env.storage().instance().get(&ESCROWS).unwrap_or(Map::new(&env));
        let mut escrow: EscrowEntry = escrows
            .get(escrow_id)
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

        Self::execute_payout_placeholder(&env, &escrow.beneficiary, escrow.amount, escrow_id)?;

        // Update escrow status
        escrow.status = String::from_slice(&env, "released");
        escrow.release_signature = signature;
        escrows.set(escrow_id, escrow.clone());
        env.storage().instance().set(&ESCROWS, &escrows);

        // Update escrow pool
        let mut escrow_pool: i128 = env.storage().instance().get(&ESCROW).unwrap_or(0i128);
        escrow_pool = Self::checked_sub_i128(escrow_pool, escrow.amount)?;
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

        let mut escrows: Map<u64, EscrowEntry> = env.storage().instance().get(&ESCROWS).unwrap_or(Map::new(&env));
        let mut escrow: EscrowEntry = escrows
            .get(escrow_id)
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

        Self::execute_payout_placeholder(&env, &escrow.depositor, escrow.amount, escrow_id)?;

        // Update escrow status
        escrow.status = String::from_slice(&env, "refunded");
        escrows.set(escrow_id, escrow.clone());
        env.storage().instance().set(&ESCROWS, &escrows);

        // Update escrow pool
        let mut escrow_pool: i128 = env.storage().instance().get(&ESCROW).unwrap_or(0i128);
        escrow_pool = Self::checked_sub_i128(escrow_pool, escrow.amount)?;
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

        let mut escrows: Map<u64, EscrowEntry> = env.storage().instance().get(&ESCROWS).unwrap_or(Map::new(&env));
        let mut escrow: EscrowEntry = escrows
            .get(escrow_id)
            .ok_or(ContractError::NotFound)?;

        escrow.conditions_met = true;
        escrows.set(escrow_id, escrow);
        env.storage().instance().set(&ESCROWS, &escrows);

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
        Self::require_non_default_address(&reporter)?;
        Self::require_valid_text(&vulnerability_type)?;
        Self::require_valid_text(&severity)?;
        Self::require_valid_text(&description)?;
        Self::require_valid_text(&location)?;
        // Verify severity is critical or emergency
        if severity != String::from_slice(&env, "critical") && severity != String::from_slice(&env, "emergency") {
            return Err(ContractError::InvalidInput);
        }

        reporter.require_auth();

        let alert_id = Self::next_counter(&env, ALERT_COUNTER)?;
        let alert_nonce = Self::generate_nonce(&env, &reporter, alert_id);
        let emergency_reward = if severity == String::from_slice(&env, "emergency") {
            10_000_000i128
        } else {
            5_000_000i128
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

        let mut alerts: Map<u64, EmergencyAlert> = env.storage().instance().get(&EMERGENCY_ALERTS).unwrap_or(Map::new(&env));
        alerts.set(alert_id, alert);
        env.storage().instance().set(&EMERGENCY_ALERTS, &alerts);
        let mut alert_nonces: Map<u64, BytesN<32>> = env.storage().instance().get(&ALERT_NONCES).unwrap_or(Map::new(&env));
        alert_nonces.set(alert_id, alert_nonce);
        env.storage().instance().set(&ALERT_NONCES, &alert_nonces);

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

        let mut alerts: Map<u64, EmergencyAlert> = env.storage().instance().get(&EMERGENCY_ALERTS).unwrap_or(Map::new(&env));
        let mut alert: EmergencyAlert = alerts
            .get(alert_id)
            .ok_or(ContractError::NotFound)?;

        if verified {
            alert.status = String::from_slice(&env, "verified");
            alert.verified_by = Some(admin.clone());

            // Create immediate escrow for emergency reward
            let escrow_id = Self::create_escrow(
                env.clone(),
                admin.clone(), // Admin deposits on behalf of the platform
                alert.reporter.clone(),
                alert.emergency_reward,
                String::from_slice(&env, "emergency"),
                0, // No lock period for emergency rewards
            )?;

            // Immediately mark conditions as met and release
            Self::mark_escrow_conditions_met(env.clone(), escrow_id, admin.clone())?;
            Self::release_escrow(env.clone(), escrow_id, admin, None)?;

            // Update reputation
            Self::update_reputation(env.clone(), alert.reporter.clone(), 1, alert.emergency_reward)?;
        } else {
            alert.status = String::from_slice(&env, "false_positive");
        }

        alerts.set(alert_id, alert);
        env.storage().instance().set(&EMERGENCY_ALERTS, &alerts);

        Ok(())
    }

    /// Get escrow details
    pub fn get_escrow(env: Env, escrow_id: u64) -> Result<EscrowEntry, ContractError> {
        let escrows: Map<u64, EscrowEntry> = env.storage().instance().get(&ESCROWS).unwrap_or(Map::new(&env));
        escrows
            .get(escrow_id)
            .ok_or(ContractError::NotFound)
    }

    /// Get emergency alert details
    pub fn get_emergency_alert(env: Env, alert_id: u64) -> Result<EmergencyAlert, ContractError> {
        let alerts: Map<u64, EmergencyAlert> = env.storage().instance().get(&EMERGENCY_ALERTS).unwrap_or(Map::new(&env));
        alerts
            .get(alert_id)
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
        Self::require_non_default_address(&funder)?;
        Self::require_positive_amount(amount)?;
        
        let mut current_pool: i128 = env.storage().instance().get(&EMERGENCY_POOL).unwrap_or(0i128);
        current_pool = Self::checked_add_i128(current_pool, amount)?;
        env.storage().instance().set(&EMERGENCY_POOL, &current_pool);

        Ok(())
    }
}
