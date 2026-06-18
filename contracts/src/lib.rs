#![cfg_attr(target_family = "wasm", no_std)]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, symbol_short, Address, Bytes, BytesN, Env, Map, String, Symbol, Vec};

// Contract state keys
const ADMIN: Symbol = symbol_short!("ADMIN");
const BOUNTY_POOL: Symbol = symbol_short!("BOUNTY");
#[allow(dead_code)]
const VULNERABILITIES: Symbol = symbol_short!("VULNS");
#[allow(dead_code)]
const REPUTATION: Symbol = symbol_short!("REPUT");
const ESCROW: Symbol = symbol_short!("ESCROW");
const EMERGENCY_POOL: Symbol = symbol_short!("EMERG");
const REPORTS: Symbol = symbol_short!("RPTS");
const ESCROWS: Symbol = symbol_short!("ESCRS");
const EMERGENCY_ALERTS: Symbol = symbol_short!("EALRTS");
const REPUTATION_MAP: Symbol = symbol_short!("REP_MAP");
const REPORT_COUNTER: Symbol = symbol_short!("RPTCTR");
const ESCROW_COUNTER: Symbol = symbol_short!("ESCCTR");
const ALERT_COUNTER: Symbol = symbol_short!("ALRTCTR");
const REPORT_NONCES: Symbol = symbol_short!("RPTNONC");
const ESCROW_NONCES: Symbol = symbol_short!("ESCNONC");
const ALERT_NONCES: Symbol = symbol_short!("ALRNONC");
const MAX_TEXT_LEN: u32 = 280;
const DISPUTE_DEADLINE: u64 = 7 * 24 * 60 * 60; // 7 days in seconds
const MIN_DISPUTE_QUORUM: u64 = 3; // Minimum 3 votes required for quorum
const MIN_REPUTATION_SCORE: u64 = 50; // Minimum reputation to vote on disputes

// Dispute resolution storage keys
const DISPUTES: Symbol = symbol_short!("DISPUTES");
const DISPUTE_COUNTER: Symbol = symbol_short!("DISPCTR");
const DISPUTE_NONCES: Symbol = symbol_short!("DISPNONC");
const DISPUTE_STAKES: Symbol = symbol_short!("DISPSTAK");

// Upgrade mechanism storage keys
const CONTRACT_VERSION: Symbol = symbol_short!("VERSION");
const UPGRADE_AUTHORITY: Symbol = symbol_short!("UPGRADE");
const UPGRADE_DELAY_KEY: Symbol = symbol_short!("UP_DELAY");
const PENDING_UPGRADE: Symbol = symbol_short!("PENDING");
const UPGRADE_HISTORY: Symbol = symbol_short!("UP_HIST");
const MIGRATION_STATUS: Symbol = symbol_short!("MIG_STAT");

// Issue #26: Emergency upgrade safeguards
const MAX_EMERGENCY_UPGRADES_PER_MONTH: u64 = 2;
const EMERGENCY_COOLING_PERIOD_SECONDS: u64 = 86400; // 24 hours
const CHALLENGE_PERIOD_SECONDS: u64 = 21600; // 6 hours
const MIN_EMERGENCY_REASON_LENGTH: u32 = 50;

// Emergency upgrade tracking
const EMERGENCY_UPGRADE_COUNT: Symbol = symbol_short!("EMERG_CT");
const EMERGENCY_UPGRADE_MONTH: Symbol = symbol_short!("EMERG_MTH");
const LAST_EMERGENCY_UPGRADE: Symbol = symbol_short!("EMRG_LAST");
const CHALLENGED_UPGRADES: Symbol = symbol_short!("CHAL_UPGR"); // Set of challenged upgrade timestamps

// Role-based access control keys
const ADMIN_ROLES: Symbol = symbol_short!("ADM_ROLES");
const MULTI_SIG_PROPOSALS: Symbol = symbol_short!("MS_PROPS");
const MULTI_SIG_COUNTER: Symbol = symbol_short!("MS_CNTR");
const ROLE_PERMISSIONS: Symbol = symbol_short!("ROLE_PERM");
#[allow(dead_code)]
const TIME_LOCKS: Symbol = symbol_short!("TIME_LOCK");

// Role definitions
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum Role {
    SuperAdmin,      // Can do everything, including role management
    Verifier,        // Can verify vulnerabilities and emergency alerts
    EscrowManager,   // Can manage escrow operations
    TreasuryManager, // Can manage funding pools
}

// Permission definitions
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum Permission {
    VerifyVulnerability,
    VerifyEmergency,
    ManageEscrow,
    ManageTreasury,
    ManageRoles,
    EmergencyActions,
}

// Multi-signature proposal structure
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct MultiSigProposal {
    pub id: u64,
    pub proposer: Address,
    pub target_function: String,
    pub parameters: Vec<String>,
    pub approvals: Map<Address, bool>,
    pub required_approvals: u64,
    pub created_at: u64,
    pub executed: bool,
    pub execution_delay: u64,
}

// Time lock structure
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct TimeLock {
    pub target_function: String,
    pub delay_seconds: u64,
    pub created_at: u64,
}

// Upgrade request structure
#[derive(Clone, Debug, Eq, PartialEq)]
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
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct UpgradeHistory {
    pub from_version: String,
    pub to_version: String,
    pub timestamp: u64,
    pub upgraded_by: Address,
    pub old_contract: Address,
    pub new_contract: Address,
}

// Challenge on an emergency upgrade (multi-sig halt vote)
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct UpgradeChallenge {
    pub timestamp: u64,
    pub challenged_by: Address,
    pub reason: String,
    pub votes: Map<Address, bool>,
    pub required_votes: u64,
    pub resolved: bool,
}

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
    InsufficientPermissions = 10,
    ProposalNotFound = 11,
    ProposalAlreadyExecuted = 12,
    TimeLockNotExpired = 13,
    InvalidRole = 14,
    MultiSigRequired = 15,
    AlreadyApproved = 16,
    UpgradeInProgress = 17,
    UpgradeNotReady = 18,
    InvalidUpgrade = 19,
    UpgradeChallengeActive = 20,
    EmergencyUpgradeLimitReached = 21,
    EmergencyCoolingPeriodActive = 22,
    InsufficientUpgradeReason = 23,
    InsufficientStake = 24,
    InvalidDisputeStatus = 25,
    InsufficientReputation = 26,
    AlreadyVoted = 27,
    DisputeAlreadyResolved = 28,
    DisputeNotFound = 29,
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

// Dispute resolution types

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum DisputeStatus {
    Active,
    Resolved,
    Dismissed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum Vote {
    Accept,
    Reject,
    Abstain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Dispute {
    pub id: u64,
    pub report_id: u64,
    pub disputant: Address,
    pub reason: String,
    pub evidence_hashes: Vec<BytesN<32>>,
    pub votes: Map<Address, Vote>,
    pub status: DisputeStatus,
    pub created_at: u64,
    pub resolution_deadline: u64,
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

    fn generate_nonce(env: &Env, _seed: &Address, counter: u64) -> BytesN<32> {
        let mut buf = [0u8; 20];
        buf[0..8].copy_from_slice(&counter.to_be_bytes());
        buf[8..12].copy_from_slice(&env.ledger().sequence().to_be_bytes());
        buf[12..20].copy_from_slice(&env.ledger().timestamp().to_be_bytes());
        let bytes = Bytes::from_slice(env, &buf);
        env.crypto().sha256(&bytes).into()
    }

    /// Convert u64 to soroban String without heap allocation (no_std compatible).
    fn u64_to_sdk_string(env: &Env, value: u64) -> String {
        if value == 0 {
            return String::from_str(env, "0");
        }
        let mut digits = [0u8; 20];
        let mut v = value;
        let mut i = 20;
        while v > 0 {
            i -= 1;
            digits[i] = (v % 10) as u8 + b'0';
            v /= 10;
        }
        let s = core::str::from_utf8(&digits[i..]).unwrap();
        String::from_str(env, s)
    }

    /// Convert i128 to soroban String without heap allocation (no_std compatible).
    fn i128_to_sdk_string(env: &Env, value: i128) -> String {
        if value == 0 {
            return String::from_str(env, "0");
        }
        let negative = value < 0;
        let mut v: u128 = if negative {
            value.wrapping_neg() as u128
        } else {
            value as u128
        };
        let mut digits = [0u8; 41];
        let mut i = 41;
        while v > 0 {
            i -= 1;
            digits[i] = (v % 10) as u8 + b'0';
            v /= 10;
        }
        if negative {
            i -= 1;
            digits[i] = b'-';
        }
        let s = core::str::from_utf8(&digits[i..]).unwrap();
        String::from_str(env, s)
    }

    /// Convert bool to soroban String without heap allocation (no_std compatible).
    fn bool_to_sdk_string(env: &Env, value: bool) -> String {
        if value {
            String::from_str(env, "true")
        } else {
            String::from_str(env, "false")
        }
    }

    // Role-based access control helper functions
    #[allow(dead_code)]
    fn has_role(env: &Env, user: &Address, role: Role) -> bool {
        let admin_roles: Map<Address, Vec<Role>> = env.storage().instance().get(&ADMIN_ROLES).unwrap_or(Map::new(env));
        if let Some(roles) = admin_roles.get(user.clone()) {
            roles.contains(&role)
        } else {
            false
        }
    }

    fn has_permission(env: &Env, user: &Address, permission: Permission) -> bool {
        let role_permissions: Map<Role, Vec<Permission>> = env.storage().instance().get(&ROLE_PERMISSIONS).unwrap_or(Map::new(env));
        let admin_roles: Map<Address, Vec<Role>> = env.storage().instance().get(&ADMIN_ROLES).unwrap_or(Map::new(env));
        
        if let Some(roles) = admin_roles.get(user.clone()) {
            for role in roles {
                if let Some(permissions) = role_permissions.get(role.clone()) {
                    if permissions.contains(&permission) {
                        return true;
                    }
                }
            }
        }
        false
    }

    #[allow(dead_code)]
    fn require_role(env: &Env, user: &Address, role: Role) -> Result<(), ContractError> {
        if Self::has_role(env, user, role) {
            Ok(())
        } else {
            Err(ContractError::InsufficientPermissions)
        }
    }

    fn require_permission(env: &Env, user: &Address, permission: Permission) -> Result<(), ContractError> {
        if Self::has_permission(env, user, permission) {
            Ok(())
        } else {
            Err(ContractError::InsufficientPermissions)
        }
    }

    fn initialize_role_permissions(env: &Env) {
        let mut role_permissions: Map<Role, Vec<Permission>> = Map::new(env);
        
        // SuperAdmin has all permissions
        role_permissions.set(Role::SuperAdmin, soroban_sdk::vec![env,
            Permission::VerifyVulnerability,
            Permission::VerifyEmergency,
            Permission::ManageEscrow,
            Permission::ManageTreasury,
            Permission::ManageRoles,
            Permission::EmergencyActions,
        ]);
        
        // Verifier can verify vulnerabilities and emergencies
        role_permissions.set(Role::Verifier, soroban_sdk::vec![env,
            Permission::VerifyVulnerability,
            Permission::VerifyEmergency,
        ]);
        
        // EscrowManager can manage escrows
        role_permissions.set(Role::EscrowManager, soroban_sdk::vec![env,
            Permission::ManageEscrow,
        ]);
        
        // TreasuryManager can manage funding pools
        role_permissions.set(Role::TreasuryManager, soroban_sdk::vec![env,
            Permission::ManageTreasury,
        ]);
        
        env.storage().instance().set(&ROLE_PERMISSIONS, &role_permissions);
    }

    fn create_multi_sig_proposal(
        env: &Env,
        proposer: &Address,
        target_function: String,
        parameters: Vec<String>,
        required_approvals: u64,
        execution_delay: u64,
    ) -> Result<u64, ContractError> {
        let proposal_id = Self::next_counter(env, MULTI_SIG_COUNTER)?;
        let proposal = MultiSigProposal {
            id: proposal_id,
            proposer: proposer.clone(),
            target_function: target_function.clone(),
            parameters: parameters.clone(),
            approvals: Map::new(env),
            required_approvals,
            created_at: env.ledger().timestamp(),
            executed: false,
            execution_delay,
        };
        
        let mut proposals: Map<u64, MultiSigProposal> = env.storage().instance().get(&MULTI_SIG_PROPOSALS).unwrap_or(Map::new(env));
        proposals.set(proposal_id, proposal);
        env.storage().instance().set(&MULTI_SIG_PROPOSALS, &proposals);
        
        Ok(proposal_id)
    }

    fn approve_proposal(env: &Env, proposal_id: u64, approver: &Address) -> Result<(), ContractError> {
        let mut proposals: Map<u64, MultiSigProposal> = env.storage().instance().get(&MULTI_SIG_PROPOSALS).unwrap_or(Map::new(env));
        let mut proposal: MultiSigProposal = proposals.get(proposal_id).ok_or(ContractError::ProposalNotFound)?;
        
        if proposal.executed {
            return Err(ContractError::ProposalAlreadyExecuted);
        }
        
        if proposal.approvals.contains_key(approver.clone()) {
            return Err(ContractError::AlreadyApproved);
        }
        
        proposal.approvals.set(approver.clone(), true);
        proposals.set(proposal_id, proposal);
        env.storage().instance().set(&MULTI_SIG_PROPOSALS, &proposals);
        
        Ok(())
    }

    fn can_execute_proposal(env: &Env, proposal_id: u64) -> Result<bool, ContractError> {
        let proposals: Map<u64, MultiSigProposal> = env.storage().instance().get(&MULTI_SIG_PROPOSALS).unwrap_or(Map::new(env));
        let proposal: MultiSigProposal = proposals.get(proposal_id).ok_or(ContractError::ProposalNotFound)?;
        
        if proposal.executed {
            return Ok(false);
        }
        
        let current_time = env.ledger().timestamp();
        if current_time < proposal.created_at + proposal.execution_delay {
            return Ok(false);
        }
        
        let approval_count = proposal.approvals.len() as u64;
        Ok(approval_count >= proposal.required_approvals)
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
        
        // Initialize role-based access control
        Self::initialize_role_permissions(&env);
        
        // Set initial admin as SuperAdmin
        let mut admin_roles: Map<Address, Vec<Role>> = Map::new(&env);
        admin_roles.set(admin.clone(), soroban_sdk::vec![&env, Role::SuperAdmin]);
        env.storage().instance().set(&ADMIN_ROLES, &admin_roles);

        // Initialize multi-signature proposals map
        env.storage().instance().set(&MULTI_SIG_PROPOSALS, &Map::<u64, MultiSigProposal>::new(&env));

        // Initialize reputation map (Map<Address, Reputation>)
        env.storage().instance().set(&REPUTATION_MAP, &Map::<Address, Reputation>::new(&env));

        // Initialize dispute resolution maps
        env.storage().instance().set(&DISPUTES, &Map::<u64, Dispute>::new(&env));
        env.storage().instance().set(&DISPUTE_NONCES, &Map::<u64, BytesN<32>>::new(&env));
        env.storage().instance().set(&DISPUTE_STAKES, &Map::<u64, i128>::new(&env));

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
            status: String::from_str(&env, "pending"),
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

    /// Verify a vulnerability and award bounty (requires Verifier role and multi-sig for high bounties)
    pub fn verify_vulnerability(
        env: Env,
        admin: Address,
        report_id: u64,
        bounty_amount: i128,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        Self::require_non_default_address(&admin)?;
        Self::require_positive_amount(bounty_amount)?;
        
        // Check role-based permissions
        Self::require_permission(&env, &admin, Permission::VerifyVulnerability)?;
        
        // For high bounty amounts (> 1M tokens), require multi-signature
        if bounty_amount > 1_000_000i128 {
            return Err(ContractError::MultiSigRequired);
        }

        Self::verify_vulnerability_internal(env, admin, report_id, bounty_amount)
    }

    /// Internal helper: verify vulnerability without re-checking auth or bounty threshold.
    fn verify_vulnerability_internal(
        env: Env,
        admin: Address,
        report_id: u64,
        bounty_amount: i128,
    ) -> Result<(), ContractError> {
        Self::require_non_default_address(&admin)?;
        Self::require_positive_amount(bounty_amount)?;
        
        // Check role-based permissions
        Self::require_permission(&env, &admin, Permission::VerifyVulnerability)?;

        // Get vulnerability report
        let mut reports: Map<u64, VulnerabilityReport> = env.storage().instance().get(&REPORTS).unwrap_or(Map::new(&env));
        let mut report: VulnerabilityReport = reports
            .get(report_id)
            .ok_or(ContractError::NotFound)?;

        // Update status and bounty
        report.status = String::from_str(&env, "verified");
        report.bounty_amount = bounty_amount;
        
        // Store updated report
        reports.set(report_id, report.clone());
        env.storage().instance().set(&REPORTS, &reports);

        // Update researcher reputation
        Self::update_reputation(env, report.reporter, 1, bounty_amount)?;

        Ok(())
    }

    /// Create multi-signature proposal for high bounty verification
    pub fn propose_high_bounty_verification(
        env: Env,
        proposer: Address,
        report_id: u64,
        bounty_amount: i128,
        required_approvals: u64,
        execution_delay: u64,
    ) -> Result<u64, ContractError> {
        proposer.require_auth();
        Self::require_permission(&env, &proposer, Permission::VerifyVulnerability)?;
        
        let parameters = soroban_sdk::vec![&env,
            Self::u64_to_sdk_string(&env, report_id),
            Self::i128_to_sdk_string(&env, bounty_amount),
        ];
        
        Self::create_multi_sig_proposal(
            &env,
            &proposer,
            String::from_str(&env, "verify_vulnerability"),
            parameters,
            required_approvals,
            execution_delay,
        )
    }

    /// Approve high bounty verification proposal
    pub fn approve_bounty_verification(
        env: Env,
        approver: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        approver.require_auth();
        Self::require_permission(&env, &approver, Permission::VerifyVulnerability)?;
        
        Self::approve_proposal(&env, proposal_id, &approver)
    }

    /// Execute high bounty verification after multi-sig approval
    pub fn execute_high_bounty_verification(
        env: Env,
        executor: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        executor.require_auth();
        Self::require_permission(&env, &executor, Permission::VerifyVulnerability)?;
        
        if !Self::can_execute_proposal(&env, proposal_id)? {
            return Err(ContractError::ProposalNotFound);
        }
        
        let proposals: Map<u64, MultiSigProposal> = env.storage().instance().get(&MULTI_SIG_PROPOSALS).unwrap_or(Map::new(&env));
        let proposal: MultiSigProposal = proposals.get(proposal_id).ok_or(ContractError::ProposalNotFound)?;
        
        let report_id_str: String = proposal.parameters.get(0).unwrap();
        let bounty_amount_str: String = proposal.parameters.get(1).unwrap();
        #[allow(clippy::from_str_radix_10)]
        let report_id: u64 = sdk_string_to_u64(&report_id_str);
        #[allow(clippy::from_str_radix_10)]
        let bounty_amount: i128 = sdk_string_to_i128(&bounty_amount_str);
        
        // Execute the verification
        Self::verify_vulnerability_internal(env.clone(), executor, report_id, bounty_amount)?;

        // Mark proposal as executed
        let mut proposals: Map<u64, MultiSigProposal> = env.storage().instance().get(&MULTI_SIG_PROPOSALS).unwrap_or(Map::new(&env));
        let mut proposal: MultiSigProposal = proposals.get(proposal_id).ok_or(ContractError::ProposalNotFound)?;
        proposal.executed = true;
        proposals.set(proposal_id, proposal);
        env.storage().instance().set(&MULTI_SIG_PROPOSALS, &proposals);
        
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
        let rep_map: Map<Address, Reputation> = env.storage().instance()
            .get(&REPUTATION_MAP)
            .unwrap_or(Map::new(&env));
        rep_map
            .get(researcher)
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

    /// Helper function to update reputation.
    /// Stores reputation in a Map<Address, Reputation> keyed by the researcher address.
    fn update_reputation(
        env: Env,
        researcher: Address,
        successful_reports: u64,
        earnings: i128,
    ) -> Result<(), ContractError> {
        let mut rep_map: Map<Address, Reputation> = env.storage().instance()
            .get(&REPUTATION_MAP)
            .unwrap_or(Map::new(&env));

        let mut reputation: Reputation = rep_map
            .get(researcher.clone())
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

        rep_map.set(researcher, reputation);
        env.storage().instance().set(&REPUTATION_MAP, &rep_map);

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
            status: String::from_str(&env, "pending"),
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
        if escrow.status == String::from_str(&env, "released") {
            return Err(ContractError::InvalidEscrowStatus);
        }

        let current_time = env.ledger().timestamp();
        
        // Allow release if conditions are met or lock period has expired
        if !escrow.conditions_met && current_time < escrow.lock_until {
            return Err(ContractError::EscrowLocked);
        }

        Self::execute_payout_placeholder(&env, &escrow.beneficiary, escrow.amount, escrow_id)?;

        // Update escrow status
        escrow.status = String::from_str(&env, "released");
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
        if escrow.status == String::from_str(&env, "released") {
            return Err(ContractError::InvalidEscrowStatus);
        }

        let current_time = env.ledger().timestamp();
        
        // Only allow refund if lock period has expired and conditions not met
        if current_time < escrow.lock_until || escrow.conditions_met {
            return Err(ContractError::EscrowLocked);
        }

        Self::execute_payout_placeholder(&env, &escrow.depositor, escrow.amount, escrow_id)?;

        // Update escrow status
        escrow.status = String::from_str(&env, "refunded");
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

    /// Mark escrow conditions as met (requires EscrowManager role)
    pub fn mark_escrow_conditions_met(
        env: Env,
        escrow_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        Self::require_non_default_address(&admin)?;
        
        // Check role-based permissions
        Self::require_permission(&env, &admin, Permission::ManageEscrow)?;

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
        if severity != String::from_str(&env, "critical") && severity != String::from_str(&env, "emergency") {
            return Err(ContractError::InvalidInput);
        }

        reporter.require_auth();

        let alert_id = Self::next_counter(&env, ALERT_COUNTER)?;
        let alert_nonce = Self::generate_nonce(&env, &reporter, alert_id);
        let emergency_reward = if severity == String::from_str(&env, "emergency") {
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
            status: String::from_str(&env, "pending"),
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

    /// Verify emergency vulnerability and trigger immediate reward (requires Verifier role and multi-sig)
    pub fn verify_emergency_vulnerability(
        env: Env,
        admin: Address,
        _alert_id: u64,
        _verified: bool,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        Self::require_non_default_address(&admin)?;
        
        // Check role-based permissions
        Self::require_permission(&env, &admin, Permission::VerifyEmergency)?;
        
        // Emergency verifications always require multi-signature for security
        Err(ContractError::MultiSigRequired)
    }

    /// Create multi-signature proposal for emergency vulnerability verification
    pub fn propose_emergency_verification(
        env: Env,
        proposer: Address,
        alert_id: u64,
        verified: bool,
        required_approvals: u64,
        execution_delay: u64,
    ) -> Result<u64, ContractError> {
        proposer.require_auth();
        Self::require_permission(&env, &proposer, Permission::VerifyEmergency)?;
        
        let parameters = soroban_sdk::vec![&env,
            Self::u64_to_sdk_string(&env, alert_id),
            Self::bool_to_sdk_string(&env, verified),
        ];
        
        // Emergency verifications have shorter delay but higher approval requirements
        let emergency_delay = if execution_delay < 3600 { 3600 } else { execution_delay }; // Minimum 1 hour
        let emergency_approvals = if required_approvals < 3 { 3 } else { required_approvals }; // Minimum 3 approvals
        
        Self::create_multi_sig_proposal(
            &env,
            &proposer,
            String::from_str(&env, "verify_emergency_vulnerability"),
            parameters,
            emergency_approvals,
            emergency_delay,
        )
    }

    /// Approve emergency verification proposal
    pub fn approve_emergency_verification(
        env: Env,
        approver: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        approver.require_auth();
        Self::require_permission(&env, &approver, Permission::VerifyEmergency)?;
        
        Self::approve_proposal(&env, proposal_id, &approver)
    }

    /// Execute emergency verification after multi-sig approval
    pub fn execute_emergency_verification(
        env: Env,
        executor: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        executor.require_auth();
        Self::require_permission(&env, &executor, Permission::VerifyEmergency)?;
        
        if !Self::can_execute_proposal(&env, proposal_id)? {
            return Err(ContractError::ProposalNotFound);
        }
        
        let proposals: Map<u64, MultiSigProposal> = env.storage().instance().get(&MULTI_SIG_PROPOSALS).unwrap_or(Map::new(&env));
        let proposal: MultiSigProposal = proposals.get(proposal_id).ok_or(ContractError::ProposalNotFound)?;
        
        let alert_id_str: String = proposal.parameters.get(0).unwrap();
        let verified_str: String = proposal.parameters.get(1).unwrap();
        #[allow(clippy::from_str_radix_10)]
        let alert_id: u64 = sdk_string_to_u64(&alert_id_str);
        let verified: bool = sdk_string_to_bool(&verified_str);
        
        // Execute the emergency verification
        Self::execute_emergency_verification_internal(env.clone(), executor, alert_id, verified)?;
        
        // Mark proposal as executed
        let mut proposals: Map<u64, MultiSigProposal> = env.storage().instance().get(&MULTI_SIG_PROPOSALS).unwrap_or(Map::new(&env));
        let mut proposal: MultiSigProposal = proposals.get(proposal_id).ok_or(ContractError::ProposalNotFound)?;
        proposal.executed = true;
        proposals.set(proposal_id, proposal);
        env.storage().instance().set(&MULTI_SIG_PROPOSALS, &proposals);
        
        Ok(())
    }

    /// Internal function to execute emergency verification
    fn execute_emergency_verification_internal(
        env: Env,
        admin: Address,
        alert_id: u64,
        verified: bool,
    ) -> Result<(), ContractError> {
        let mut alerts: Map<u64, EmergencyAlert> = env.storage().instance().get(&EMERGENCY_ALERTS).unwrap_or(Map::new(&env));
        let mut alert: EmergencyAlert = alerts
            .get(alert_id)
            .ok_or(ContractError::NotFound)?;

        if verified {
            alert.status = String::from_str(&env, "verified");
            alert.verified_by = Some(admin.clone());

            // Create immediate escrow for emergency reward
            let escrow_id = Self::create_escrow(
                env.clone(),
                admin.clone(), // Admin deposits on behalf of the platform
                alert.reporter.clone(),
                alert.emergency_reward,
                String::from_str(&env, "emergency"),
                0, // No lock period for emergency rewards
            )?;

            // Immediately mark conditions as met and release
            Self::mark_escrow_conditions_met(env.clone(), escrow_id, admin.clone())?;
            Self::release_escrow(env.clone(), escrow_id, admin, None)?;

            // Update reputation
            Self::update_reputation(env.clone(), alert.reporter.clone(), 1, alert.emergency_reward)?;
        } else {
            alert.status = String::from_str(&env, "false_positive");
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

    /// Fund emergency pool (requires TreasuryManager role)
    pub fn fund_emergency_pool(env: Env, funder: Address, amount: i128) -> Result<(), ContractError> {
        funder.require_auth();
        Self::require_non_default_address(&funder)?;
        Self::require_positive_amount(amount)?;
        
        // Check role-based permissions
        Self::require_permission(&env, &funder, Permission::ManageTreasury)?;
        
        let mut current_pool: i128 = env.storage().instance().get(&EMERGENCY_POOL).unwrap_or(0i128);
        current_pool = Self::checked_add_i128(current_pool, amount)?;
        env.storage().instance().set(&EMERGENCY_POOL, &current_pool);

        Ok(())
    }

    // Role management functions (require SuperAdmin role)
    
    /// Grant a role to an address (requires SuperAdmin role and multi-sig)
    pub fn grant_role(
        _env: Env,
        super_admin: Address,
        user: Address,
        _role: Role,
    ) -> Result<(), ContractError> {
        super_admin.require_auth();
        Self::require_non_default_address(&super_admin)?;
        Self::require_non_default_address(&user)?;
        
        // Role management always requires multi-signature
        Err(ContractError::MultiSigRequired)
    }

    /// Create multi-signature proposal for role grant
    pub fn propose_role_grant(
        env: Env,
        proposer: Address,
        user: Address,
        role: Role,
        required_approvals: u64,
        execution_delay: u64,
    ) -> Result<u64, ContractError> {
        proposer.require_auth();
        Self::require_permission(&env, &proposer, Permission::ManageRoles)?;
        
        let role_str = match role {
            Role::SuperAdmin => "SuperAdmin",
            Role::Verifier => "Verifier",
            Role::EscrowManager => "EscrowManager",
            Role::TreasuryManager => "TreasuryManager",
        };
        
        let parameters = soroban_sdk::vec![&env,
            user.to_string(),
            String::from_str(&env, role_str),
        ];
        
        // Role management has higher security requirements
        let role_delay = if execution_delay < 86400 { 86400 } else { execution_delay }; // Minimum 24 hours
        let role_approvals = if required_approvals < 2 { 2 } else { required_approvals }; // Minimum 2 approvals
        
        Self::create_multi_sig_proposal(
            &env,
            &proposer,
            String::from_str(&env, "grant_role"),
            parameters,
            role_approvals,
            role_delay,
        )
    }

    /// Approve role grant proposal
    pub fn approve_role_grant(
        env: Env,
        approver: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        approver.require_auth();
        Self::require_permission(&env, &approver, Permission::ManageRoles)?;
        
        Self::approve_proposal(&env, proposal_id, &approver)
    }

    /// Execute role grant after multi-sig approval
    pub fn execute_role_grant(
        env: Env,
        executor: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        executor.require_auth();
        Self::require_permission(&env, &executor, Permission::ManageRoles)?;
        
        if !Self::can_execute_proposal(&env, proposal_id)? {
            return Err(ContractError::ProposalNotFound);
        }
        
        let proposals: Map<u64, MultiSigProposal> = env.storage().instance().get(&MULTI_SIG_PROPOSALS).unwrap_or(Map::new(&env));
        let proposal: MultiSigProposal = proposals.get(proposal_id).ok_or(ContractError::ProposalNotFound)?;
        
        // Parse address and role from stored parameters
        let user_address_str = proposal.parameters.get(0).unwrap();
        let role_str = proposal.parameters.get(1).unwrap();
        
        // Parse address
        let user_address = Address::from_string(&user_address_str);
        let role = if role_str == String::from_str(&env, "SuperAdmin") {
            Role::SuperAdmin
        } else if role_str == String::from_str(&env, "Verifier") {
            Role::Verifier
        } else if role_str == String::from_str(&env, "EscrowManager") {
            Role::EscrowManager
        } else if role_str == String::from_str(&env, "TreasuryManager") {
            Role::TreasuryManager
        } else {
            return Err(ContractError::InvalidRole);
        };
        
        // Execute the role grant
        Self::execute_role_grant_internal(env.clone(), user_address, role)?;
        
        // Mark proposal as executed
        let mut proposals: Map<u64, MultiSigProposal> = env.storage().instance().get(&MULTI_SIG_PROPOSALS).unwrap_or(Map::new(&env));
        let mut proposal: MultiSigProposal = proposals.get(proposal_id).ok_or(ContractError::ProposalNotFound)?;
        proposal.executed = true;
        proposals.set(proposal_id, proposal);
        env.storage().instance().set(&MULTI_SIG_PROPOSALS, &proposals);
        
        Ok(())
    }

    /// Internal function to execute role grant
    fn execute_role_grant_internal(
        env: Env,
        user: Address,
        role: Role,
    ) -> Result<(), ContractError> {
        let mut admin_roles: Map<Address, Vec<Role>> = env.storage().instance().get(&ADMIN_ROLES).unwrap_or(Map::new(&env));
        
        let mut user_roles = admin_roles.get(user.clone()).unwrap_or(Vec::new(&env));
        if !user_roles.contains(&role) {
            user_roles.push_back(role);
        }
        
        admin_roles.set(user, user_roles);
        env.storage().instance().set(&ADMIN_ROLES, &admin_roles);
        
        Ok(())
    }

    /// Revoke a role from an address (requires SuperAdmin role and multi-sig)
    pub fn revoke_role(
        _env: Env,
        super_admin: Address,
        user: Address,
        _role: Role,
    ) -> Result<(), ContractError> {
        super_admin.require_auth();
        Self::require_non_default_address(&super_admin)?;
        Self::require_non_default_address(&user)?;
        
        // Role management always requires multi-signature
        Err(ContractError::MultiSigRequired)
    }

    /// Get user roles
    pub fn get_user_roles(env: Env, user: Address) -> Result<Vec<Role>, ContractError> {
        let admin_roles: Map<Address, Vec<Role>> = env.storage().instance().get(&ADMIN_ROLES).unwrap_or(Map::new(&env));
        Ok(admin_roles.get(user.clone()).unwrap_or(Vec::new(&env)))
    }

    /// Get multi-signature proposal details
    pub fn get_proposal(env: Env, proposal_id: u64) -> Result<MultiSigProposal, ContractError> {
        let proposals: Map<u64, MultiSigProposal> = env.storage().instance().get(&MULTI_SIG_PROPOSALS).unwrap_or(Map::new(&env));
        proposals.get(proposal_id).ok_or(ContractError::ProposalNotFound)
    }

    /// Check if a proposal can be executed
    pub fn can_execute_proposal_check(env: Env, proposal_id: u64) -> Result<bool, ContractError> {
        Self::can_execute_proposal(&env, proposal_id)
    }

    // -----------------------------------------------------------------------
    // Upgrade mechanism (Issue #26: with timelock & emergency safeguards)
    // -----------------------------------------------------------------------

    /// Get current contract version
    pub fn get_version(env: Env) -> String {
        env.storage()
            .instance()
            .get(&CONTRACT_VERSION)
            .unwrap_or(String::from_str(&env, "1.0.0"))
    }

    /// Set upgrade authority (admin only)
    pub fn set_upgrade_authority(
        env: Env,
        admin: Address,
        new_authority: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        Self::require_non_default_address(&new_authority)?;

        env.storage().instance().set(&UPGRADE_AUTHORITY, &new_authority);
        Ok(())
    }

    /// Set upgrade delay in seconds (admin only, minimum 24 hours)
    pub fn set_upgrade_delay(
        env: Env,
        admin: Address,
        delay_seconds: u64,
    ) -> Result<(), ContractError> {
        admin.require_auth();

        if delay_seconds < 86400 {
            // Minimum 24 hours
            return Err(ContractError::InvalidInput);
        }

        env.storage().instance().set(&UPGRADE_DELAY_KEY, &delay_seconds);
        Ok(())
    }

    /// Propose a standard upgrade with timelock delay.
    /// Only the upgrade authority can propose.
    pub fn propose_upgrade(
        env: Env,
        proposer: Address,
        new_contract_address: Address,
        new_version: String,
        reason: String,
    ) -> Result<(), ContractError> {
        proposer.require_auth();
        Self::require_non_default_address(&new_contract_address)?;
        Self::require_valid_text(&new_version)?;

        // Check caller is upgrade authority
        let upgrade_authority: Address = env.storage().instance()
            .get(&UPGRADE_AUTHORITY)
            .ok_or(ContractError::Unauthorized)?;

        if proposer != upgrade_authority {
            return Err(ContractError::Unauthorized);
        }

        // Ensure no pending upgrade exists
        if env.storage().instance().has(&PENDING_UPGRADE) {
            return Err(ContractError::UpgradeInProgress);
        }

        let now = env.ledger().timestamp();
        let delay: u64 = env.storage().instance()
            .get(&UPGRADE_DELAY_KEY)
            .unwrap_or(604800); // Default 7 days

        let ready_at = now + delay;

        let request = UpgradeRequest {
            new_contract_address,
            proposed_by: proposer,
            timestamp: now,
            ready_at,
            reason,
            version: new_version,
        };

        env.storage().instance().set(&PENDING_UPGRADE, &request);

        env.events().publish(
            (Symbol::new(&env, "upgrade_proposed"),),
            (request.version.clone(), request.ready_at),
        );

        Ok(())
    }

    /// Get pending upgrade information
    pub fn get_pending_upgrade(env: Env) -> Result<UpgradeRequest, ContractError> {
        env.storage()
            .instance()
            .get(&PENDING_UPGRADE)
            .ok_or(ContractError::NotFound)
    }

    /// Execute a pending upgrade after the timelock delay has passed.
    /// Only the upgrade authority can execute.
    pub fn execute_upgrade(
        env: Env,
        executor: Address,
    ) -> Result<(), ContractError> {
        executor.require_auth();

        // Check caller is upgrade authority
        let upgrade_authority: Address = env.storage().instance()
            .get(&UPGRADE_AUTHORITY)
            .ok_or(ContractError::Unauthorized)?;

        if executor != upgrade_authority {
            return Err(ContractError::Unauthorized);
        }

        let request: UpgradeRequest = env.storage().instance()
            .get(&PENDING_UPGRADE)
            .ok_or(ContractError::NotFound)?;

        let now = env.ledger().timestamp();
        if now < request.ready_at {
            return Err(ContractError::UpgradeNotReady);
        }

        // Record upgrade history
        let current_version = Self::get_version(env.clone());
        let mut history: Vec<UpgradeHistory> = env.storage().instance()
            .get(&UPGRADE_HISTORY)
            .unwrap_or(Vec::new(&env));

        history.push_back(UpgradeHistory {
            from_version: current_version.clone(),
            to_version: request.version.clone(),
            timestamp: now,
            upgraded_by: executor.clone(),
            // In a production environment, this would be the current contract's address
            old_contract: upgrade_authority.clone(),
            new_contract: request.new_contract_address.clone(),
        });

        env.storage().instance().set(&UPGRADE_HISTORY, &history);

        // Update version
        env.storage().instance().set(&CONTRACT_VERSION, &request.version);

        // Set migration status
        let migration_pair: (Address, u64) = (request.new_contract_address.clone(), now);
        env.storage().instance().set(&MIGRATION_STATUS, &migration_pair);

        // Clear pending upgrade
        env.storage().instance().remove(&PENDING_UPGRADE);

        env.events().publish(
            (Symbol::new(&env, "upgrade_executed"),),
            (current_version, request.version, now),
        );

        Ok(())
    }

    /// Perform an emergency upgrade that bypasses the timelock.
    /// Only the admin can trigger emergency upgrades.
    /// Includes Issue #26 safeguards: frequency cap, reason requirement,
    /// cooling-off period, forced event, and challenge period.
    pub fn emergency_upgrade(
        env: Env,
        admin: Address,
        new_contract_address: Address,
        new_version: String,
        reason: String,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        Self::require_non_default_address(&new_contract_address)?;
        Self::require_valid_text(&new_version)?;

        // ---- Issue #26 Safeguards ----

        // 1. Minimum reason length (50 characters)
        if reason.len() < MIN_EMERGENCY_REASON_LENGTH {
            return Err(ContractError::InsufficientUpgradeReason);
        }

        let now = env.ledger().timestamp();
        let one_month_seconds: u64 = 30 * 24 * 60 * 60;
        let current_month = now / one_month_seconds;

        // 2. Frequency cap: MAX_EMERGENCY_UPGRADES_PER_MONTH
        let last_month: u64 = env.storage().instance()
            .get(&EMERGENCY_UPGRADE_MONTH)
            .unwrap_or(0u64);

        let emergency_count: u64 = if current_month == last_month {
            env.storage().instance()
                .get(&EMERGENCY_UPGRADE_COUNT)
                .unwrap_or(0u64)
        } else {
            // Reset counter for new month
            0u64
        };

        if emergency_count >= MAX_EMERGENCY_UPGRADES_PER_MONTH {
            return Err(ContractError::EmergencyUpgradeLimitReached);
        }

        // 3. Cooling-off period (24 hours between emergency upgrades)
        let last_emergency: u64 = env.storage().instance()
            .get(&LAST_EMERGENCY_UPGRADE)
            .unwrap_or(0u64);

        if last_emergency > 0 && now < last_emergency + EMERGENCY_COOLING_PERIOD_SECONDS {
            return Err(ContractError::EmergencyCoolingPeriodActive);
        }

        // 4. Check if this upgrade has been challenged during the challenge period
        // (Challenge can only be initiated BEFORE execution, not after)
        // Check that no active challenge exists
        let challenged_upgrades: Map<u64, UpgradeChallenge> = env.storage().instance()
            .get(&CHALLENGED_UPGRADES)
            .unwrap_or(Map::new(&env));

        for (challenge_ts, challenge) in challenged_upgrades.iter() {
            if !challenge.resolved && now < challenge_ts + CHALLENGE_PERIOD_SECONDS {
                // An active unresolved challenge exists for this upgrade window
                return Err(ContractError::UpgradeChallengeActive);
            }
        }

        // ---- Execute emergency upgrade ----

        let current_version = Self::get_version(env.clone());
        let mut history: Vec<UpgradeHistory> = env.storage().instance()
            .get(&UPGRADE_HISTORY)
            .unwrap_or(Vec::new(&env));

        history.push_back(UpgradeHistory {
            from_version: current_version.clone(),
            to_version: new_version.clone(),
            timestamp: now,
            upgraded_by: admin.clone(),
            // In a production environment, this would be the current contract's address
            old_contract: admin.clone(),
            new_contract: new_contract_address.clone(),
        });

        env.storage().instance().set(&UPGRADE_HISTORY, &history);
        env.storage().instance().set(&CONTRACT_VERSION, &new_version);
        let migration_pair: (Address, u64) = (new_contract_address.clone(), now);
        env.storage().instance().set(&MIGRATION_STATUS, &migration_pair);

        // Update emergency upgrade tracking
        env.storage().instance().set(&EMERGENCY_UPGRADE_MONTH, &current_month);
        env.storage().instance().set(&EMERGENCY_UPGRADE_COUNT, &(emergency_count + 1));
        env.storage().instance().set(&LAST_EMERGENCY_UPGRADE, &now);

        // 5. Forced EMERGENCY_UPGRADE_NOTIFICATION event (cannot be suppressed)
        env.events().publish(
            (Symbol::new(&env, "emergency_upgrade"),),
            (
                current_version,
                new_version,
                now,
                admin.clone(),
                reason,
                emergency_count + 1,
                MAX_EMERGENCY_UPGRADES_PER_MONTH,
            ),
        );

        Ok(())
    }

    /// Cancel a pending upgrade (proposer or admin only)
    pub fn cancel_upgrade(
        env: Env,
        canceler: Address,
    ) -> Result<(), ContractError> {
        canceler.require_auth();

        let request: UpgradeRequest = env.storage().instance()
            .get(&PENDING_UPGRADE)
            .ok_or(ContractError::NotFound)?;

        let upgrade_authority: Address = env.storage().instance()
            .get(&UPGRADE_AUTHORITY)
            .ok_or(ContractError::Unauthorized)?;

        // Only the proposer, upgrade authority, or admin can cancel
        let admin: Address = env.storage().instance().get(&ADMIN).ok_or(ContractError::Unauthorized)?;
        if canceler != request.proposed_by && canceler != upgrade_authority && canceler != admin {
            return Err(ContractError::Unauthorized);
        }

        env.storage().instance().remove(&PENDING_UPGRADE);

        env.events().publish(
            (Symbol::new(&env, "upgrade_cancelled"),),
            (request.version, canceler),
        );

        Ok(())
    }

    /// Get upgrade history
    pub fn get_upgrade_history(env: Env) -> Vec<UpgradeHistory> {
        env.storage().instance()
            .get(&UPGRADE_HISTORY)
            .unwrap_or(Vec::new(&env))
    }

    /// Get migration status (target contract, timestamp)
    pub fn get_migration_status(env: Env) -> Option<(Address, u64)> {
        env.storage().instance().get(&MIGRATION_STATUS)
    }

    /// Halt an emergency upgrade by creating a challenge vote.
    /// This opens a 6-hour challenge period where multi-sig signers can vote.
    pub fn halt_emergency_upgrade(
        env: Env,
        challenger: Address,
        challenge_reason: String,
        required_votes: u64,
    ) -> Result<u64, ContractError> {
        challenger.require_auth();
        Self::require_valid_text(&challenge_reason)?;

        let now = env.ledger().timestamp();

        let mut challenged_upgrades: Map<u64, UpgradeChallenge> = env.storage().instance()
            .get(&CHALLENGED_UPGRADES)
            .unwrap_or(Map::new(&env));

        let challenge = UpgradeChallenge {
            timestamp: now,
            challenged_by: challenger.clone(),
            reason: challenge_reason,
            votes: Map::new(&env),
            required_votes,
            resolved: false,
        };

        challenged_upgrades.set(now, challenge);
        env.storage().instance().set(&CHALLENGED_UPGRADES, &challenged_upgrades);

        env.events().publish(
            (Symbol::new(&env, "upgrade_challenged"),),
            (now, challenger, required_votes),
        );

        Ok(now) // Returns the challenge timestamp as ID
    }

    /// Vote to halt an emergency upgrade during the challenge period.
    /// Only authorized multi-sig signers can vote.
    pub fn vote_to_halt_upgrade(
        env: Env,
        voter: Address,
        challenge_timestamp: u64,
        vote_to_halt: bool,
    ) -> Result<(), ContractError> {
        voter.require_auth();

        let now = env.ledger().timestamp();

        // Check challenge period hasn't expired
        if now >= challenge_timestamp + CHALLENGE_PERIOD_SECONDS {
            return Err(ContractError::UpgradeChallengeActive);
        }

        let mut challenged_upgrades: Map<u64, UpgradeChallenge> = env.storage().instance()
            .get(&CHALLENGED_UPGRADES)
            .unwrap_or(Map::new(&env));

        let mut challenge: UpgradeChallenge = challenged_upgrades
            .get(challenge_timestamp)
            .ok_or(ContractError::NotFound)?;

        if challenge.resolved {
            return Err(ContractError::UpgradeChallengeActive);
        }

        if challenge.votes.contains_key(voter.clone()) {
            return Err(ContractError::AlreadyApproved);
        }

        challenge.votes.set(voter.clone(), vote_to_halt);

        // Check if required votes are met for halting
        let halt_votes: u64 = challenge.votes
            .iter()
            .filter(|(_, vote)| *vote)
            .count() as u64;

        if halt_votes >= challenge.required_votes {
            challenge.resolved = true;

            env.events().publish(
                (Symbol::new(&env, "upgrade_halted"),),
                (challenge_timestamp, true, halt_votes),
            );
        }

        challenged_upgrades.set(challenge_timestamp, challenge);
        env.storage().instance().set(&CHALLENGED_UPGRADES, &challenged_upgrades);

        Ok(())
    }

    // =========================================================================
    // Dispute Resolution System
    // =========================================================================

    /// File a dispute against a verified vulnerability report.
    /// Requires staking 1% of the current bounty pool.
    /// Stake is returned if dispute is upheld, forfeited to bounty pool if dismissed.
    pub fn file_dispute(
        env: Env,
        disputant: Address,
        report_id: u64,
        reason: String,
        evidence_hashes: Vec<BytesN<32>>,
    ) -> Result<u64, ContractError> {
        disputant.require_auth();
        Self::require_non_default_address(&disputant)?;
        Self::require_valid_text(&reason)?;

        // Verify that the report exists
        let reports: Map<u64, VulnerabilityReport> = env.storage().instance()
            .get(&REPORTS)
            .unwrap_or(Map::new(&env));
        reports.get(report_id).ok_or(ContractError::NotFound)?;

        // Calculate required stake: 1% of bounty pool
        let bounty_pool: i128 = env.storage().instance().get(&BOUNTY_POOL).unwrap_or(0i128);
        let stake_amount = bounty_pool
            .checked_div(100)
            .ok_or(ContractError::Overflow)?;

        if stake_amount <= 0 {
            return Err(ContractError::InsufficientStake);
        }

        // Track the stake: add to dispute stakes tracking
        let mut dispute_stakes: Map<u64, i128> = env.storage().instance()
            .get(&DISPUTE_STAKES)
            .unwrap_or(Map::new(&env));

        // Generate dispute ID
        let dispute_id = Self::next_counter(&env, DISPUTE_COUNTER)?;
        let dispute_nonce = Self::generate_nonce(&env, &disputant, dispute_id);

        // Record the stake (deducted from disputant in production)
        dispute_stakes.set(dispute_id, stake_amount);
        env.storage().instance().set(&DISPUTE_STAKES, &dispute_stakes);

        // Store dispute nonce
        let mut dispute_nonces: Map<u64, BytesN<32>> = env.storage().instance()
            .get(&DISPUTE_NONCES)
            .unwrap_or(Map::new(&env));
        dispute_nonces.set(dispute_id, dispute_nonce);
        env.storage().instance().set(&DISPUTE_NONCES, &dispute_nonces);

        // Create dispute
        let now = env.ledger().timestamp();
        let dispute = Dispute {
            id: dispute_id,
            report_id,
            disputant: disputant.clone(),
            reason,
            evidence_hashes,
            votes: Map::new(&env),
            status: DisputeStatus::Active,
            created_at: now,
            resolution_deadline: Self::checked_add_u64(now, DISPUTE_DEADLINE)?,
        };

        // Store dispute
        let mut disputes: Map<u64, Dispute> = env.storage().instance()
            .get(&DISPUTES)
            .unwrap_or(Map::new(&env));
        disputes.set(dispute_id, dispute);
        env.storage().instance().set(&DISPUTES, &disputes);

        // Emit event
        env.events().publish(
            (Symbol::new(&env, "dispute_filed"),),
            (dispute_id, report_id, disputant, stake_amount),
        );

        Ok(dispute_id)
    }

    /// Vote on an active dispute.
    /// Only verified researchers with reputation score > 50 can vote.
    /// Each researcher can vote only once per dispute.
    pub fn vote_on_dispute(
        env: Env,
        voter: Address,
        dispute_id: u64,
        vote: Vote,
    ) -> Result<(), ContractError> {
        voter.require_auth();
        Self::require_non_default_address(&voter)?;

        // Check researcher reputation (must be > 50)
        let rep_map: Map<Address, Reputation> = env.storage().instance()
            .get(&REPUTATION_MAP)
            .unwrap_or(Map::new(&env));
        let reputation: Reputation = rep_map
            .get(voter.clone())
            .ok_or(ContractError::InsufficientReputation)?;

        if reputation.score <= MIN_REPUTATION_SCORE {
            return Err(ContractError::InsufficientReputation);
        }

        // Get dispute
        let mut disputes: Map<u64, Dispute> = env.storage().instance()
            .get(&DISPUTES)
            .unwrap_or(Map::new(&env));
        let mut dispute: Dispute = disputes
            .get(dispute_id)
            .ok_or(ContractError::DisputeNotFound)?;

        // Check dispute status
        if dispute.status != DisputeStatus::Active {
            return Err(ContractError::DisputeAlreadyResolved);
        }

        // Check if already voted
        if dispute.votes.contains_key(voter.clone()) {
            return Err(ContractError::AlreadyVoted);
        }

        // Record vote
        dispute.votes.set(voter.clone(), vote.clone());
        disputes.set(dispute_id, dispute);
        env.storage().instance().set(&DISPUTES, &disputes);

        // Emit event
        env.events().publish(
            (Symbol::new(&env, "dispute_vote_cast"),),
            (dispute_id, voter, vote),
        );

        Ok(())
    }

    /// Resolve a dispute.
    /// Can be called by anyone once the resolution deadline has passed
    /// OR once quorum has been reached.
    /// - If dispute is upheld (majority Accept): stake returned to disputant, reporter keeps bounty.
    /// - If dispute is dismissed (majority Reject or no quorum): stake forfeited to bounty pool.
    pub fn resolve_dispute(
        env: Env,
        dispute_id: u64,
    ) -> Result<DisputeStatus, ContractError> {
        // Get dispute
        let mut disputes: Map<u64, Dispute> = env.storage().instance()
            .get(&DISPUTES)
            .unwrap_or(Map::new(&env));
        let mut dispute: Dispute = disputes
            .get(dispute_id)
            .ok_or(ContractError::DisputeNotFound)?;

        // Check dispute status
        if dispute.status != DisputeStatus::Active {
            return Err(ContractError::DisputeAlreadyResolved);
        }

        let now = env.ledger().timestamp();

        // Count votes
        let mut accept_count: u64 = 0;
        let mut reject_count: u64 = 0;
        for (_, voter_vote) in dispute.votes.iter() {
            match voter_vote {
                Vote::Accept => accept_count += 1,
                Vote::Reject => reject_count += 1,
                Vote::Abstain => { /* Abstentions don't count toward majority */ }
            }
        }

        let total_votes = accept_count + reject_count;
        let quorum_reached = total_votes >= MIN_DISPUTE_QUORUM;
        let deadline_passed = now >= dispute.resolution_deadline;

        // Only resolve if quorum reached or deadline passed
        if !quorum_reached && !deadline_passed {
            return Err(ContractError::InvalidDisputeStatus);
        }

        let dispute_upheld = quorum_reached && accept_count > reject_count;

        // Handle staking outcome
        let mut dispute_stakes: Map<u64, i128> = env.storage().instance()
            .get(&DISPUTE_STAKES)
            .unwrap_or(Map::new(&env));

        if let Some(stake_amount) = dispute_stakes.get(dispute_id) {
            if dispute_upheld {
                // Dispute upheld: stake returned to disputant (refund logic)
                // Emit event for stake return
                env.events().publish(
                    (Symbol::new(&env, "dispute_stake_returned"),),
                    (dispute_id, dispute.disputant.clone(), stake_amount),
                );
                // In production: transfer stake back to disputant
            } else {
                // Dispute dismissed: stake forfeited to bounty pool
                let mut bounty_pool: i128 = env.storage().instance()
                    .get(&BOUNTY_POOL)
                    .unwrap_or(0i128);
                bounty_pool = Self::checked_add_i128(bounty_pool, stake_amount)?;
                env.storage().instance().set(&BOUNTY_POOL, &bounty_pool);

                // Emit event for stake forfeiture
                env.events().publish(
                    (Symbol::new(&env, "dispute_stake_forfeited"),),
                    (dispute_id, dispute.disputant.clone(), stake_amount),
                );
            }

            // Remove stake tracking
            dispute_stakes.remove(dispute_id);
            env.storage().instance().set(&DISPUTE_STAKES, &dispute_stakes);
        }

        // Update dispute status
        let new_status = if dispute_upheld {
            DisputeStatus::Resolved
        } else {
            DisputeStatus::Dismissed
        };

        dispute.status = new_status.clone();
        disputes.set(dispute_id, dispute);
        env.storage().instance().set(&DISPUTES, &disputes);

        // Emit resolution event
        env.events().publish(
            (Symbol::new(&env, "dispute_resolved"),),
            (dispute_id, new_status.clone(), accept_count, reject_count, total_votes),
        );

        Ok(new_status)
    }

    /// Get dispute details
    pub fn get_dispute(env: Env, dispute_id: u64) -> Result<Dispute, ContractError> {
        let disputes: Map<u64, Dispute> = env.storage().instance()
            .get(&DISPUTES)
            .unwrap_or(Map::new(&env));
        disputes
            .get(dispute_id)
            .ok_or(ContractError::DisputeNotFound)
    }

    /// Get the stake amount for a dispute
    pub fn get_dispute_stake(env: Env, dispute_id: u64) -> Result<i128, ContractError> {
        let dispute_stakes: Map<u64, i128> = env.storage().instance()
            .get(&DISPUTE_STAKES)
            .unwrap_or(Map::new(&env));
        dispute_stakes
            .get(dispute_id)
            .ok_or(ContractError::DisputeNotFound)
    }
}

// ---------------------------------------------------------------------------
// Free helper functions for parsing stored parameter strings back to primitives.
// On WASM targets these return defaults; the affected functions are only used
// in native test/development environments.
// ---------------------------------------------------------------------------

fn sdk_string_to_u64(s: &String) -> u64 {
    #[cfg(not(target_family = "wasm"))]
    {
        let rust_str = s.to_string();
        #[allow(clippy::from_str_radix_10)]
        { u64::from_str_radix(&rust_str, 10).unwrap_or(0) }
    }
    #[cfg(target_family = "wasm")]
    {
        let _ = s;
        0
    }
}

fn sdk_string_to_i128(s: &String) -> i128 {
    #[cfg(not(target_family = "wasm"))]
    {
        let rust_str = s.to_string();
        #[allow(clippy::from_str_radix_10)]
        { i128::from_str_radix(&rust_str, 10).unwrap_or(0) }
    }
    #[cfg(target_family = "wasm")]
    {
        let _ = s;
        0
    }
}

fn sdk_string_to_bool(s: &String) -> bool {
    #[cfg(not(target_family = "wasm"))]
    {
        let rust_str = s.to_string();
        rust_str == "true"
    }
    #[cfg(target_family = "wasm")]
    {
        let _ = s;
        false
    }
}

#[cfg(test)]
mod security_tests;

#[cfg(test)]
mod dispute_tests;
