use soroban_sdk::{contractimpl, Address, BytesN, Env, Symbol, Vec, Map, String};

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
}
