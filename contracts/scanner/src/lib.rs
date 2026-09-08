#![no_std]
extern crate alloc;
use alloc::format;
use alloc::string::ToString;
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, vec, xdr::ToXdr,
    Address, Bytes, BytesN, Env, Map, String, Symbol, Vec,
};

// Contract state keys
const ADMIN: Symbol = symbol_short!("ADMIN");
const TOKEN: Symbol = symbol_short!("TOKEN");
const BOUNTY_POOL: Symbol = symbol_short!("BOUNTY");
const MIN_HIGH_BOUNTY_APPROVALS: u64 = 2;
#[allow(dead_code)]
const VULNERABILITIES: Symbol = symbol_short!("VULNS");
#[allow(dead_code)]
const REPUTATION: Symbol = symbol_short!("REPUT");
const ESCROW: Symbol = symbol_short!("ESCROW");
const EMERGENCY_POOL: Symbol = symbol_short!("EMERG");
const REPORTS: Symbol = symbol_short!("RPTS");
const ESCROWS: Symbol = symbol_short!("ESCRS");
const EMERGENCY_ALERTS: Symbol = symbol_short!("EALRTS");
const REPORT_COUNTER: Symbol = symbol_short!("RPTCTR");
const ESCROW_COUNTER: Symbol = symbol_short!("ESCCTR");
const ALERT_COUNTER: Symbol = symbol_short!("ALRTCTR");
const REPORT_NONCES: Symbol = symbol_short!("RPTNONC");
const ESCROW_NONCES: Symbol = symbol_short!("ESCNONC");
const ALERT_NONCES: Symbol = symbol_short!("ALRNONC");
const MAX_TEXT_LEN: u32 = 280;

// Enum-like values are stored as `Symbol`s instead of `String`s. Symbols are
// fixed-size (max 9 bytes when short-encoded) and far cheaper to store,
// compare and XDR-encode than `String`s, so using them for status, purpose and
// severity fields reduces on-chain storage footprint and gas.
const STATUS_PENDING: Symbol = symbol_short!("pending");
const STATUS_VERIFIED: Symbol = symbol_short!("verified");
const STATUS_RELEASED: Symbol = symbol_short!("released");
const STATUS_REFUNDED: Symbol = symbol_short!("refunded");
// "false_positive" is 14 chars — too long for `symbol_short!` (9 max).
const STATUS_FALSE_POSITIVE: &str = "false_positive";

const SEVERITY_CRITICAL: Symbol = symbol_short!("critical");
const SEVERITY_EMERGENCY: Symbol = symbol_short!("emergency");

const PURPOSE_BOUNTY: Symbol = symbol_short!("bounty");
const PURPOSE_REWARD: Symbol = symbol_short!("reward");
const PURPOSE_EMERGENCY: Symbol = symbol_short!("emergency");

// Role-based access control keys
const ADMIN_ROLES: Symbol = symbol_short!("ADM_ROLE");
const MULTI_SIG_PROPOSALS: Symbol = symbol_short!("MS_PROPS");
const MULTI_SIG_COUNTER: Symbol = symbol_short!("MS_CNTR");
const ROLE_PERMISSIONS: Symbol = symbol_short!("ROLE_PER");
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

// Contract errors
//
// Structured, gas-efficient error system. Every failure state maps to a
// stable numerical code via [`ContractError::code`], with the code space
// partitioned by domain so off-chain tooling can react to specific failure
// states numerically (up to 200 distinct codes):
//
// | Range   | Domain                        |
// |---------|-------------------------------|
// | 1-19    | Authentication & authorization |
// | 20-39   | Input validation              |
// | 40-59   | Lookup & storage              |
// | 60-79   | Funds & pools                 |
// | 80-99   | Escrow lifecycle              |
// | 100-119 | Multi-signature proposals     |
// | 120-139 | Roles & permissions           |
// | 140-159 | Emergency operations          |
// | 160-179 | Arithmetic                    |
// | 180-200 | Reserved for future use       |
//
// Legacy discriminants `1..=17` are preserved for ABI compatibility; new
// discriminants continue from 18.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ContractError {
    // 1-19: Authentication & authorization
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
    AlreadyVerified = 17,
    // 18+: new discriminants (domain ranges are reflected in `code()`)
    DefaultAddress = 18,
    TokenNotConfigured = 19,
    ContractNotInitialized = 20,
    TextTooLong = 21,
    InvalidSeverity = 22,
    InvalidPurpose = 23,
    ProposalNotReady = 24,
    RoleAlreadyGranted = 25,
    EscrowAlreadyReleased = 26,
    EscrowAlreadyRefunded = 27,
    SignatureMissing = 28,
    SignatureInvalid = 29,
    ZeroAmount = 30,
}

impl ContractError {
    /// Highest code available in the 200-code error space.
    pub const MAX_ERROR_CODE: u32 = 200;

    /// Number of distinct failure states currently defined.
    pub const COUNT: u32 = 30;

    /// Map a failure state to its structured numerical code (`1..=200`).
    /// Codes are stable: adding new errors never renumbers existing ones.
    pub fn code(&self) -> u32 {
        match self {
            // Authentication & authorization (1-19)
            ContractError::Unauthorized => 1,
            ContractError::InsufficientPermissions => 2,
            // Input validation (20-39)
            ContractError::InvalidInput => 20,
            ContractError::DefaultAddress => 21,
            ContractError::TextTooLong => 22,
            ContractError::InvalidSeverity => 23,
            ContractError::InvalidPurpose => 24,
            ContractError::ZeroAmount => 25,
            // Lookup & storage (40-59)
            ContractError::NotFound => 40,
            ContractError::TokenNotConfigured => 41,
            ContractError::ContractNotInitialized => 42,
            // Funds & pools (60-79)
            ContractError::InsufficientFunds => 60,
            // Escrow lifecycle (80-99)
            ContractError::EscrowLocked => 80,
            ContractError::InvalidEscrowStatus => 81,
            ContractError::EscrowAlreadyReleased => 82,
            ContractError::EscrowAlreadyRefunded => 83,
            // Multi-signature proposals (100-119)
            ContractError::ProposalNotFound => 100,
            ContractError::ProposalAlreadyExecuted => 101,
            ContractError::ProposalNotReady => 102,
            ContractError::TimeLockNotExpired => 103,
            ContractError::AlreadyApproved => 104,
            ContractError::MultiSigRequired => 105,
            ContractError::SignatureMissing => 106,
            ContractError::SignatureInvalid => 107,
            // Roles & permissions (120-139)
            ContractError::InvalidRole => 120,
            ContractError::RoleAlreadyGranted => 121,
            // Emergency operations (140-159)
            ContractError::EmergencyModeActive => 140,
            ContractError::AlreadyVerified => 141,
            // Arithmetic (160-179)
            ContractError::Overflow => 160,
            ContractError::ExternalCallFailed => 161,
        }
    }

    /// Reverse lookup from a structured numerical code back to its error.
    /// Returns `None` for codes that are not yet assigned.
    pub fn from_code(code: u32) -> Option<ContractError> {
        Some(match code {
            1 => ContractError::Unauthorized,
            2 => ContractError::InsufficientPermissions,
            20 => ContractError::InvalidInput,
            21 => ContractError::DefaultAddress,
            22 => ContractError::TextTooLong,
            23 => ContractError::InvalidSeverity,
            24 => ContractError::InvalidPurpose,
            25 => ContractError::ZeroAmount,
            40 => ContractError::NotFound,
            41 => ContractError::TokenNotConfigured,
            42 => ContractError::ContractNotInitialized,
            60 => ContractError::InsufficientFunds,
            80 => ContractError::EscrowLocked,
            81 => ContractError::InvalidEscrowStatus,
            82 => ContractError::EscrowAlreadyReleased,
            83 => ContractError::EscrowAlreadyRefunded,
            100 => ContractError::ProposalNotFound,
            101 => ContractError::ProposalAlreadyExecuted,
            102 => ContractError::ProposalNotReady,
            103 => ContractError::TimeLockNotExpired,
            104 => ContractError::AlreadyApproved,
            105 => ContractError::MultiSigRequired,
            106 => ContractError::SignatureMissing,
            107 => ContractError::SignatureInvalid,
            120 => ContractError::InvalidRole,
            121 => ContractError::RoleAlreadyGranted,
            140 => ContractError::EmergencyModeActive,
            141 => ContractError::AlreadyVerified,
            160 => ContractError::Overflow,
            161 => ContractError::ExternalCallFailed,
            _ => return None,
        })
    }
}

// Vulnerability structure
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct VulnerabilityReport {
    pub reporter: Address,
    pub contract_id: BytesN<32>,
    pub vulnerability_type: String,
    pub severity: Symbol,
    pub description: String,
    pub location: String,
    pub timestamp: u64,
    pub status: Symbol, // STATUS_PENDING | STATUS_VERIFIED | STATUS_REJECTED
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
    pub purpose: Symbol, // PURPOSE_BOUNTY | PURPOSE_REWARD | PURPOSE_EMERGENCY
    pub status: Symbol,  // STATUS_PENDING | STATUS_RELEASED | STATUS_REFUNDED
    pub created_at: u64,
    pub lock_until: u64,
    pub conditions_met: bool,
    /// Optional ed25519 public key that must authorize a release. When set,
    /// `release_escrow` requires a valid signature over the escrow's canonical
    /// release message; when `None`, release falls back to depositor auth only.
    pub release_signer: Option<BytesN<32>>,
    /// The verified ed25519 signature that authorized the release. Only ever
    /// populated with a signature that passed `ed25519_verify` (Issue #481).
    pub release_signature: Option<BytesN<64>>,
}

// Emergency alert structure
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct EmergencyAlert {
    pub id: u64,
    pub reporter: Address,
    pub contract_id: BytesN<32>,
    pub vulnerability_type: String,
    pub severity: Symbol, // SEVERITY_CRITICAL | SEVERITY_EMERGENCY
    pub description: String,
    pub location: String,
    pub timestamp: u64,
    pub status: Symbol, // STATUS_PENDING | STATUS_VERIFIED | "false_positive"
    pub emergency_reward: i128,
    pub verified_by: Option<Address>,
}

#[contract]
pub struct SecurityScannerContract;

#[contractimpl]
impl SecurityScannerContract {
    /// Convert an SDK string to a Rust `String`, returning [`ContractError::InvalidInput`]
    /// on invalid UTF-8 instead of panicking. A panic here traps the whole
    /// transaction, so untrusted proposal parameters must never be able to trigger one.
    fn sdk_string_to_rust(s: soroban_sdk::String) -> Result<alloc::string::String, ContractError> {
        let bytes = s.to_bytes();
        let vec = bytes.to_alloc_vec();
        alloc::string::String::from_utf8(vec).map_err(|_| ContractError::InvalidInput)
    }

    /// Fetch parameter `idx` from a proposal parameter list, erroring instead of
    /// panicking when the index is out of range.
    fn proposal_param(parameters: &Vec<String>, idx: u32) -> Result<String, ContractError> {
        parameters.get(idx).ok_or(ContractError::InvalidInput)
    }

    /// Parse proposal parameter `idx` as `u64` without panicking.
    fn parse_param_u64(parameters: &Vec<String>, idx: u32) -> Result<u64, ContractError> {
        Self::sdk_string_to_rust(Self::proposal_param(parameters, idx)?)?
            .parse()
            .map_err(|_| ContractError::InvalidInput)
    }

    /// Parse proposal parameter `idx` as `i128` without panicking.
    fn parse_param_i128(parameters: &Vec<String>, idx: u32) -> Result<i128, ContractError> {
        Self::sdk_string_to_rust(Self::proposal_param(parameters, idx)?)?
            .parse()
            .map_err(|_| ContractError::InvalidInput)
    }

    /// Parse proposal parameter `idx` as `bool` without panicking.
    fn parse_param_bool(parameters: &Vec<String>, idx: u32) -> Result<bool, ContractError> {
        Self::sdk_string_to_rust(Self::proposal_param(parameters, idx)?)?
            .parse()
            .map_err(|_| ContractError::InvalidInput)
    }

    fn require_non_default_address(env: &Env, addr: &Address) -> Result<(), ContractError> {
        let null_address = String::from_str(
            env,
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
        );
        if addr == &Address::from_string(&null_address) {
            return Err(ContractError::DefaultAddress);
        }
        Ok(())
    }

    fn token_client(env: &Env) -> token::Client<'_> {
        let token_addr: Address = env
            .storage()
            .instance()
            .get(&TOKEN)
            .expect("token address not configured");
        token::Client::new(env, &token_addr)
    }

    fn receive_tokens(env: &Env, from: &Address, amount: i128) -> Result<(), ContractError> {
        Self::require_positive_amount(amount)?;
        let contract = env.current_contract_address();
        Self::token_client(env).transfer(from, &contract, &amount);
        Ok(())
    }

    fn transfer_tokens(env: &Env, to: &Address, amount: i128) -> Result<(), ContractError> {
        Self::require_positive_amount(amount)?;
        let contract = env.current_contract_address();
        Self::token_client(env).transfer(&contract, to, &amount);
        Ok(())
    }

    fn reputations(env: &Env) -> Map<Address, Reputation> {
        env.storage()
            .instance()
            .get(&REPUTATION)
            .unwrap_or(Map::new(env))
    }

    fn require_positive_amount(amount: i128) -> Result<(), ContractError> {
        if amount <= 0 {
            return Err(ContractError::ZeroAmount);
        }
        Ok(())
    }

    fn require_valid_text(value: &String) -> Result<(), ContractError> {
        if value.is_empty() || value.len() > MAX_TEXT_LEN {
            return Err(ContractError::TextTooLong);
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

    /// Load the per-escrow nonce created in `create_escrow`.
    fn escrow_nonce(env: &Env, escrow_id: u64) -> Result<BytesN<32>, ContractError> {
        let escrow_nonces: Map<u64, BytesN<32>> = env
            .storage()
            .instance()
            .get(&ESCROW_NONCES)
            .unwrap_or(Map::new(env));
        escrow_nonces.get(escrow_id).ok_or(ContractError::NotFound)
    }

    /// Build the canonical message a release signer must sign to authorize a
    /// release. Binding the escrow's unique nonce, id, beneficiary and amount
    /// ties the signature to exactly one payout and prevents it from being
    /// replayed against a different escrow or a mutated amount.
    fn build_release_message(env: &Env, escrow: &EscrowEntry, nonce: &BytesN<32>) -> Bytes {
        let mut message = Bytes::from_array(env, &nonce.to_array());
        message.extend_from_array(&escrow.id.to_be_bytes());
        message.append(&escrow.beneficiary.clone().to_xdr(env));
        message.extend_from_array(&escrow.amount.to_be_bytes());
        message
    }

    // Role-based access control helper functions
    #[allow(dead_code)]
    fn has_role(env: &Env, user: &Address, role: Role) -> bool {
        let admin_roles: Map<Address, Vec<Role>> = env
            .storage()
            .instance()
            .get(&ADMIN_ROLES)
            .unwrap_or(Map::new(env));
        if let Some(roles) = admin_roles.get(user.clone()) {
            roles.contains(&role)
        } else {
            false
        }
    }

    fn has_permission(env: &Env, user: &Address, permission: Permission) -> bool {
        let role_permissions: Map<Role, Vec<Permission>> = env
            .storage()
            .instance()
            .get(&ROLE_PERMISSIONS)
            .unwrap_or(Map::new(env));
        let admin_roles: Map<Address, Vec<Role>> = env
            .storage()
            .instance()
            .get(&ADMIN_ROLES)
            .unwrap_or(Map::new(env));

        if let Some(roles) = admin_roles.get(user.clone()) {
            for role in roles {
                if let Some(permissions) = role_permissions.get(role) {
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

    fn require_permission(
        env: &Env,
        user: &Address,
        permission: Permission,
    ) -> Result<(), ContractError> {
        if Self::has_permission(env, user, permission) {
            Ok(())
        } else {
            Err(ContractError::InsufficientPermissions)
        }
    }

    fn initialize_role_permissions(env: &Env) {
        let mut role_permissions: Map<Role, Vec<Permission>> = Map::new(env);

        // SuperAdmin has all permissions
        role_permissions.set(
            Role::SuperAdmin,
            vec![
                &env,
                Permission::VerifyVulnerability,
                Permission::VerifyEmergency,
                Permission::ManageEscrow,
                Permission::ManageTreasury,
                Permission::ManageRoles,
                Permission::EmergencyActions,
            ],
        );

        // Verifier can verify vulnerabilities and emergencies
        role_permissions.set(
            Role::Verifier,
            vec![
                &env,
                Permission::VerifyVulnerability,
                Permission::VerifyEmergency,
            ],
        );

        // EscrowManager can manage escrows
        role_permissions.set(Role::EscrowManager, vec![&env, Permission::ManageEscrow]);

        // TreasuryManager can manage funding pools
        role_permissions.set(
            Role::TreasuryManager,
            vec![&env, Permission::ManageTreasury],
        );

        env.storage()
            .instance()
            .set(&ROLE_PERMISSIONS, &role_permissions);
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

        let mut proposals: Map<u64, MultiSigProposal> = env
            .storage()
            .instance()
            .get(&MULTI_SIG_PROPOSALS)
            .unwrap_or(Map::new(env));
        proposals.set(proposal_id, proposal);
        env.storage()
            .instance()
            .set(&MULTI_SIG_PROPOSALS, &proposals);

        Ok(proposal_id)
    }

    fn approve_proposal(
        env: &Env,
        proposal_id: u64,
        approver: &Address,
    ) -> Result<(), ContractError> {
        let mut proposals: Map<u64, MultiSigProposal> = env
            .storage()
            .instance()
            .get(&MULTI_SIG_PROPOSALS)
            .unwrap_or(Map::new(env));
        let mut proposal: MultiSigProposal = proposals
            .get(proposal_id)
            .ok_or(ContractError::ProposalNotFound)?;

        if proposal.executed {
            return Err(ContractError::ProposalAlreadyExecuted);
        }

        if proposal.approvals.contains_key(approver.clone()) {
            return Err(ContractError::AlreadyApproved);
        }

        proposal.approvals.set(approver.clone(), true);
        proposals.set(proposal_id, proposal);
        env.storage()
            .instance()
            .set(&MULTI_SIG_PROPOSALS, &proposals);

        Ok(())
    }

    fn can_execute_proposal(env: &Env, proposal_id: u64) -> Result<bool, ContractError> {
        let proposals: Map<u64, MultiSigProposal> = env
            .storage()
            .instance()
            .get(&MULTI_SIG_PROPOSALS)
            .unwrap_or(Map::new(env));
        let proposal: MultiSigProposal = proposals
            .get(proposal_id)
            .ok_or(ContractError::ProposalNotFound)?;

        if proposal.executed {
            return Ok(false);
        }

        let current_time = env.ledger().timestamp();
        let ready_after = Self::checked_add_u64(proposal.created_at, proposal.execution_delay)?;
        if current_time < ready_after {
            return Ok(false);
        }

        let approval_count = proposal.approvals.len() as u64;
        Ok(approval_count >= proposal.required_approvals)
    }

    fn execute_payout(
        env: &Env,
        recipient: &Address,
        amount: i128,
        escrow_id: u64,
    ) -> Result<(), ContractError> {
        Self::transfer_tokens(env, recipient, amount)?;
        #[allow(deprecated)]
        env.events().publish(
            (symbol_short!("payout"),),
            (escrow_id, recipient.clone(), amount),
        );
        Ok(())
    }

    /// Initialize the contract with admin address and payment token
    pub fn initialize(env: Env, admin: Address, token: Address) -> Result<(), ContractError> {
        if env.storage().instance().has(&ADMIN) {
            return Err(ContractError::Unauthorized);
        }

        admin.require_auth();
        Self::require_non_default_address(&env, &admin)?;
        Self::require_non_default_address(&env, &token)?;
        env.storage().instance().set(&ADMIN, &admin);
        env.storage().instance().set(&TOKEN, &token);
        env.storage().instance().set(&BOUNTY_POOL, &0i128);
        env.storage().instance().set(&EMERGENCY_POOL, &0i128);
        env.storage()
            .instance()
            .set(&REPUTATION, &Map::<Address, Reputation>::new(&env));
        env.storage()
            .instance()
            .set(&REPORTS, &Map::<u64, VulnerabilityReport>::new(&env));
        env.storage()
            .instance()
            .set(&ESCROWS, &Map::<u64, EscrowEntry>::new(&env));
        env.storage()
            .instance()
            .set(&EMERGENCY_ALERTS, &Map::<u64, EmergencyAlert>::new(&env));
        env.storage()
            .instance()
            .set(&REPORT_NONCES, &Map::<u64, BytesN<32>>::new(&env));
        env.storage()
            .instance()
            .set(&ESCROW_NONCES, &Map::<u64, BytesN<32>>::new(&env));
        env.storage()
            .instance()
            .set(&ALERT_NONCES, &Map::<u64, BytesN<32>>::new(&env));

        // Initialize role-based access control
        Self::initialize_role_permissions(&env);

        // Set initial admin as SuperAdmin
        let mut admin_roles: Map<Address, Vec<Role>> = Map::new(&env);
        admin_roles.set(admin.clone(), vec![&env, Role::SuperAdmin]);
        env.storage().instance().set(&ADMIN_ROLES, &admin_roles);

        // Initialize multi-signature proposals map
        env.storage().instance().set(
            &MULTI_SIG_PROPOSALS,
            &Map::<u64, MultiSigProposal>::new(&env),
        );

        Ok(())
    }

    /// Report a new vulnerability
    pub fn report_vulnerability(
        env: Env,
        reporter: Address,
        contract_id: BytesN<32>,
        vulnerability_type: String,
        severity: Symbol,
        description: String,
        location: String,
    ) -> Result<u64, ContractError> {
        // Verify reporter is authorized
        reporter.require_auth();
        Self::require_non_default_address(&env, &reporter)?;
        Self::require_valid_text(&vulnerability_type)?;
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
            status: STATUS_PENDING,
            bounty_amount: 0i128,
        };

        let report_id = Self::next_counter(&env, REPORT_COUNTER)?;
        let report_nonce = Self::generate_nonce(&env, &reporter, report_id);
        let mut reports: Map<u64, VulnerabilityReport> = env
            .storage()
            .instance()
            .get(&REPORTS)
            .unwrap_or(Map::new(&env));
        reports.set(report_id, report);
        env.storage().instance().set(&REPORTS, &reports);
        let mut report_nonces: Map<u64, BytesN<32>> = env
            .storage()
            .instance()
            .get(&REPORT_NONCES)
            .unwrap_or(Map::new(&env));
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
        Self::verify_vulnerability_internal(env, &admin, report_id, bounty_amount, true)
    }

    /// Same as [`Self::verify_vulnerability`] but without `require_auth`, for
    /// use after an outer call (e.g. multi-sig execution) has already
    /// authorized the caller. Requiring auth again for the same address inside
    /// the same transaction is redundant and costs gas.
    ///
    /// `enforce_high_bounty_gate` is `true` on the direct public path (high
    /// bounties then require multi-sig) and `false` on the multi-sig execution
    /// path, which is the sanctioned route for high bounties.
    fn verify_vulnerability_internal(
        env: Env,
        admin: &Address,
        report_id: u64,
        bounty_amount: i128,
        enforce_high_bounty_gate: bool,
    ) -> Result<(), ContractError> {
        Self::require_non_default_address(&env, admin)?;
        Self::require_positive_amount(bounty_amount)?;

        // Check role-based permissions
        Self::require_permission(&env, admin, Permission::VerifyVulnerability)?;

        // For high bounty amounts (> 1M tokens), require multi-signature
        if enforce_high_bounty_gate && bounty_amount > 1_000_000i128 {
            return Err(ContractError::MultiSigRequired);
        }

        // Get vulnerability report
        let mut reports: Map<u64, VulnerabilityReport> = env
            .storage()
            .instance()
            .get(&REPORTS)
            .unwrap_or(Map::new(&env));
        let mut report: VulnerabilityReport =
            reports.get(report_id).ok_or(ContractError::NotFound)?;

        if report.status == STATUS_VERIFIED {
            return Err(ContractError::AlreadyVerified);
        }

        let mut pool: i128 = env.storage().instance().get(&BOUNTY_POOL).unwrap_or(0i128);
        if pool < bounty_amount {
            return Err(ContractError::InsufficientFunds);
        }
        pool = Self::checked_sub_i128(pool, bounty_amount)?;
        env.storage().instance().set(&BOUNTY_POOL, &pool);

        // Update status and bounty
        report.status = STATUS_VERIFIED;
        report.bounty_amount = bounty_amount;

        // Store updated report
        reports.set(report_id, report.clone());
        env.storage().instance().set(&REPORTS, &reports);

        // Pay bounty from contract custody and update researcher reputation
        Self::transfer_tokens(&env, &report.reporter, bounty_amount)?;
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

        let parameters = vec![
            &env,
            String::from_str(&env, &report_id.to_string()),
            String::from_str(&env, &bounty_amount.to_string()),
        ];

        let bounty_approvals = if required_approvals < MIN_HIGH_BOUNTY_APPROVALS {
            MIN_HIGH_BOUNTY_APPROVALS
        } else {
            required_approvals
        };

        Self::create_multi_sig_proposal(
            &env,
            &proposer,
            String::from_str(&env, "verify_vulnerability"),
            parameters,
            bounty_approvals,
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

        // Load the proposal store once and keep it in memory for the readiness
        // check, the parameter parse and the final state write (previously the
        // map was re-read from storage three times per execution).
        let mut proposals: Map<u64, MultiSigProposal> = env
            .storage()
            .instance()
            .get(&MULTI_SIG_PROPOSALS)
            .unwrap_or(Map::new(&env));
        let mut proposal: MultiSigProposal = proposals
            .get(proposal_id)
            .ok_or(ContractError::ProposalNotFound)?;

        if proposal.executed {
            return Err(ContractError::ProposalAlreadyExecuted);
        }
        let current_time = env.ledger().timestamp();
        let ready_after = Self::checked_add_u64(proposal.created_at, proposal.execution_delay)?;
        if current_time < ready_after
            || (proposal.approvals.len() as u64) < proposal.required_approvals
        {
            return Err(ContractError::ProposalNotReady);
        }

        let report_id: u64 = Self::parse_param_u64(&proposal.parameters, 0)?;
        let bounty_amount: i128 = Self::parse_param_i128(&proposal.parameters, 1)?;

        // Execute the verification. The executor has already been authorized at
        // the top of this function, so skip the redundant nested `require_auth`,
        // and skip the high-bounty gate because multi-sig approval is the
        // sanctioned route for high bounties.
        Self::verify_vulnerability_internal(
            env.clone(),
            &executor,
            report_id,
            bounty_amount,
            false,
        )?;

        // Mark proposal as executed and write back in a single pass
        proposal.executed = true;
        proposals.set(proposal_id, proposal);
        env.storage()
            .instance()
            .set(&MULTI_SIG_PROPOSALS, &proposals);

        Ok(())
    }

    /// Get vulnerability report
    pub fn get_vulnerability(
        env: Env,
        report_id: u64,
    ) -> Result<VulnerabilityReport, ContractError> {
        let reports: Map<u64, VulnerabilityReport> = env
            .storage()
            .instance()
            .get(&REPORTS)
            .unwrap_or(Map::new(&env));
        reports.get(report_id).ok_or(ContractError::NotFound)
    }

    /// Get researcher reputation
    pub fn get_reputation(env: Env, researcher: Address) -> Result<Reputation, ContractError> {
        Self::reputations(&env)
            .get(researcher)
            .ok_or(ContractError::NotFound)
    }

    /// Add funds to bounty pool
    pub fn fund_bounty_pool(env: Env, funder: Address, amount: i128) -> Result<(), ContractError> {
        funder.require_auth();
        Self::require_non_default_address(&env, &funder)?;
        Self::require_positive_amount(amount)?;

        Self::receive_tokens(&env, &funder, amount)?;

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
        let mut reputations = Self::reputations(&env);

        let mut reputation: Reputation =
            reputations.get(researcher.clone()).unwrap_or(Reputation {
                researcher: researcher.clone(),
                score: 0,
                successful_reports: 0,
                total_earnings: 0,
            });

        reputation.successful_reports =
            Self::checked_add_u64(reputation.successful_reports, successful_reports)?;
        reputation.total_earnings = Self::checked_add_i128(reputation.total_earnings, earnings)?;
        let score_from_reports = Self::checked_mul_u64(reputation.successful_reports, 10)?;
        let score_from_earnings =
            Self::checked_non_negative_i128_to_u64(reputation.total_earnings / 1_000_000)?;
        reputation.score = Self::checked_add_u64(score_from_reports, score_from_earnings)?;

        reputations.set(researcher, reputation);
        env.storage().instance().set(&REPUTATION, &reputations);

        Ok(())
    }

    /// Create escrow entry for bounty payment
    pub fn create_escrow(
        env: Env,
        depositor: Address,
        beneficiary: Address,
        amount: i128,
        purpose: Symbol,
        lock_duration: u64,
        release_signer: Option<BytesN<32>>,
    ) -> Result<u64, ContractError> {
        depositor.require_auth();
        Self::create_escrow_internal(
            env,
            &depositor,
            &beneficiary,
            amount,
            purpose,
            lock_duration,
            release_signer,
        )
    }

    /// Same as [`Self::create_escrow`] without `require_auth`, for use after an
    /// outer call (e.g. multi-sig emergency execution) has already authorized
    /// the caller.
    fn create_escrow_internal(
        env: Env,
        depositor: &Address,
        beneficiary: &Address,
        amount: i128,
        purpose: Symbol,
        lock_duration: u64,
        release_signer: Option<BytesN<32>>,
    ) -> Result<u64, ContractError> {
        Self::require_non_default_address(&env, depositor)?;
        Self::require_non_default_address(&env, beneficiary)?;
        Self::require_positive_amount(amount)?;
        if purpose != PURPOSE_BOUNTY && purpose != PURPOSE_REWARD && purpose != PURPOSE_EMERGENCY {
            return Err(ContractError::InvalidPurpose);
        }

        let escrow_id = Self::next_counter(&env, ESCROW_COUNTER)?;
        let current_time = env.ledger().timestamp();
        let lock_until = Self::checked_add_u64(current_time, lock_duration)?;
        let escrow_nonce = Self::generate_nonce(&env, beneficiary, escrow_id);

        let escrow = EscrowEntry {
            id: escrow_id,
            depositor: depositor.clone(),
            beneficiary: beneficiary.clone(),
            amount,
            purpose: purpose.clone(),
            status: STATUS_PENDING,
            created_at: current_time,
            lock_until,
            conditions_met: false,
            release_signer,
            release_signature: None,
        };

        let mut escrows: Map<u64, EscrowEntry> = env
            .storage()
            .instance()
            .get(&ESCROWS)
            .unwrap_or(Map::new(&env));
        escrows.set(escrow_id, escrow);
        env.storage().instance().set(&ESCROWS, &escrows);
        let mut escrow_nonces: Map<u64, BytesN<32>> = env
            .storage()
            .instance()
            .get(&ESCROW_NONCES)
            .unwrap_or(Map::new(&env));
        escrow_nonces.set(escrow_id, escrow_nonce);
        env.storage().instance().set(&ESCROW_NONCES, &escrow_nonces);

        Self::receive_tokens(&env, depositor, amount)?;

        // Add to escrow pool tracking
        let mut escrow_pool: i128 = env.storage().instance().get(&ESCROW).unwrap_or(0i128);
        escrow_pool = Self::checked_add_i128(escrow_pool, amount)?;
        env.storage().instance().set(&ESCROW, &escrow_pool);

        Ok(escrow_id)
    }

    fn verify_release_signature(
        env: &Env,
        escrow: &EscrowEntry,
        signature: Option<BytesN<64>>,
    ) -> Result<Option<BytesN<64>>, ContractError> {
        match &escrow.release_signer {
            Some(signer) => {
                let signature = signature.ok_or(ContractError::SignatureMissing)?;
                let nonce = Self::escrow_nonce(env, escrow.id)?;
                let message = Self::build_release_message(env, escrow, &nonce);
                // `ed25519_verify` traps (aborts the whole transaction) when
                // the signature does not verify, so a forged signature can
                // never release an escrow (Issue #481). The failure state is
                // exposed to off-chain tooling as the structured error code
                // `ContractError::SignatureInvalid`.
                env.crypto().ed25519_verify(signer, &message, &signature);
                Ok(Some(signature))
            }
            None => Ok(None),
        }
    }

    fn release_escrow_internal(
        env: &Env,
        escrow_id: u64,
        depositor: &Address,
        release_signature: Option<BytesN<64>>,
    ) -> Result<(), ContractError> {
        let mut escrows: Map<u64, EscrowEntry> = env
            .storage()
            .instance()
            .get(&ESCROWS)
            .unwrap_or(Map::new(env));
        let mut escrow: EscrowEntry = escrows.get(escrow_id).ok_or(ContractError::NotFound)?;

        if escrow.depositor != *depositor {
            return Err(ContractError::Unauthorized);
        }

        if escrow.status == STATUS_RELEASED {
            return Err(ContractError::EscrowAlreadyReleased);
        }

        let current_time = env.ledger().timestamp();
        if !escrow.conditions_met && current_time < escrow.lock_until {
            return Err(ContractError::EscrowLocked);
        }

        let verified_signature = Self::verify_release_signature(env, &escrow, release_signature)?;

        Self::execute_payout(env, &escrow.beneficiary, escrow.amount, escrow_id)?;

        escrow.status = STATUS_RELEASED;
        escrow.release_signature = verified_signature;
        escrows.set(escrow_id, escrow.clone());
        env.storage().instance().set(&ESCROWS, &escrows);

        let mut escrow_pool: i128 = env.storage().instance().get(&ESCROW).unwrap_or(0i128);
        escrow_pool = Self::checked_sub_i128(escrow_pool, escrow.amount)?;
        env.storage().instance().set(&ESCROW, &escrow_pool);

        Ok(())
    }

    /// Release escrow funds to beneficiary
    pub fn release_escrow(
        env: Env,
        escrow_id: u64,
        depositor: Address,
        signature: Option<BytesN<64>>,
    ) -> Result<(), ContractError> {
        depositor.require_auth();

        // Signature verification happens inside `release_escrow_internal` so the
        // escrow entry is loaded from storage only once for the whole release.
        Self::release_escrow_internal(&env, escrow_id, &depositor, signature)
    }

    /// Refund escrow funds to depositor
    pub fn refund_escrow(
        env: Env,
        escrow_id: u64,
        depositor: Address,
    ) -> Result<(), ContractError> {
        depositor.require_auth();

        let mut escrows: Map<u64, EscrowEntry> = env
            .storage()
            .instance()
            .get(&ESCROWS)
            .unwrap_or(Map::new(&env));
        let mut escrow: EscrowEntry = escrows.get(escrow_id).ok_or(ContractError::NotFound)?;

        // Verify depositor authorization
        if escrow.depositor != depositor {
            return Err(ContractError::Unauthorized);
        }

        // Check if escrow can be refunded
        if escrow.status == STATUS_RELEASED {
            return Err(ContractError::InvalidEscrowStatus);
        }
        if escrow.status == STATUS_REFUNDED {
            return Err(ContractError::EscrowAlreadyRefunded);
        }

        let current_time = env.ledger().timestamp();

        // Only allow refund if lock period has expired and conditions not met
        if current_time < escrow.lock_until || escrow.conditions_met {
            return Err(ContractError::EscrowLocked);
        }

        Self::execute_payout(&env, &escrow.depositor, escrow.amount, escrow_id)?;

        // Update escrow status
        escrow.status = STATUS_REFUNDED;
        escrows.set(escrow_id, escrow.clone());
        env.storage().instance().set(&ESCROWS, &escrows);

        // Update escrow pool
        let mut escrow_pool: i128 = env.storage().instance().get(&ESCROW).unwrap_or(0i128);
        escrow_pool = Self::checked_sub_i128(escrow_pool, escrow.amount)?;
        env.storage().instance().set(&ESCROW, &escrow_pool);

        Ok(())
    }

    /// Mark escrow conditions as met (requires EscrowManager role)
    pub fn mark_escrow_conditions_met(
        env: Env,
        escrow_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        Self::mark_escrow_conditions_met_internal(env, escrow_id, &admin)
    }

    /// Same as [`Self::mark_escrow_conditions_met`] without `require_auth`, for
    /// use after an outer call (e.g. multi-sig emergency execution) has already
    /// authorized the caller.
    fn mark_escrow_conditions_met_internal(
        env: Env,
        escrow_id: u64,
        admin: &Address,
    ) -> Result<(), ContractError> {
        Self::require_non_default_address(&env, admin)?;

        // Check role-based permissions
        Self::require_permission(&env, admin, Permission::ManageEscrow)?;

        let mut escrows: Map<u64, EscrowEntry> = env
            .storage()
            .instance()
            .get(&ESCROWS)
            .unwrap_or(Map::new(&env));
        let mut escrow: EscrowEntry = escrows.get(escrow_id).ok_or(ContractError::NotFound)?;

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
        severity: Symbol,
        description: String,
        location: String,
    ) -> Result<u64, ContractError> {
        Self::require_non_default_address(&env, &reporter)?;
        Self::require_valid_text(&vulnerability_type)?;
        Self::require_valid_text(&description)?;
        Self::require_valid_text(&location)?;
        // Verify severity is critical or emergency
        if severity != SEVERITY_CRITICAL && severity != SEVERITY_EMERGENCY {
            return Err(ContractError::InvalidSeverity);
        }

        reporter.require_auth();

        let alert_id = Self::next_counter(&env, ALERT_COUNTER)?;
        let alert_nonce = Self::generate_nonce(&env, &reporter, alert_id);
        let emergency_reward = if severity == SEVERITY_EMERGENCY {
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
            status: STATUS_PENDING,
            emergency_reward,
            verified_by: None,
        };

        let mut alerts: Map<u64, EmergencyAlert> = env
            .storage()
            .instance()
            .get(&EMERGENCY_ALERTS)
            .unwrap_or(Map::new(&env));
        alerts.set(alert_id, alert);
        env.storage().instance().set(&EMERGENCY_ALERTS, &alerts);
        let mut alert_nonces: Map<u64, BytesN<32>> = env
            .storage()
            .instance()
            .get(&ALERT_NONCES)
            .unwrap_or(Map::new(&env));
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
        Self::require_non_default_address(&env, &admin)?;

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

        let parameters = vec![
            &env,
            String::from_str(&env, &alert_id.to_string()),
            String::from_str(&env, &verified.to_string()),
        ];

        // Emergency verifications have shorter delay but higher approval requirements
        let emergency_delay = if execution_delay < 3600 {
            3600
        } else {
            execution_delay
        }; // Minimum 1 hour
        let emergency_approvals = if required_approvals < 3 {
            3
        } else {
            required_approvals
        }; // Minimum 3 approvals

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

        // Single storage load for the whole execution (see
        // `execute_high_bounty_verification`).
        let mut proposals: Map<u64, MultiSigProposal> = env
            .storage()
            .instance()
            .get(&MULTI_SIG_PROPOSALS)
            .unwrap_or(Map::new(&env));
        let mut proposal: MultiSigProposal = proposals
            .get(proposal_id)
            .ok_or(ContractError::ProposalNotFound)?;

        if proposal.executed {
            return Err(ContractError::ProposalAlreadyExecuted);
        }
        let current_time = env.ledger().timestamp();
        let ready_after = Self::checked_add_u64(proposal.created_at, proposal.execution_delay)?;
        if current_time < ready_after
            || (proposal.approvals.len() as u64) < proposal.required_approvals
        {
            return Err(ContractError::ProposalNotReady);
        }

        let alert_id: u64 = Self::parse_param_u64(&proposal.parameters, 0)?;
        let verified: bool = Self::parse_param_bool(&proposal.parameters, 1)?;

        // Execute the emergency verification
        Self::execute_emergency_verification_internal(env.clone(), executor, alert_id, verified)?;

        // Mark proposal as executed and write back in a single pass
        proposal.executed = true;
        proposals.set(proposal_id, proposal);
        env.storage()
            .instance()
            .set(&MULTI_SIG_PROPOSALS, &proposals);

        Ok(())
    }

    /// Internal function to execute emergency verification
    fn execute_emergency_verification_internal(
        env: Env,
        admin: Address,
        alert_id: u64,
        verified: bool,
    ) -> Result<(), ContractError> {
        let mut alerts: Map<u64, EmergencyAlert> = env
            .storage()
            .instance()
            .get(&EMERGENCY_ALERTS)
            .unwrap_or(Map::new(&env));
        let mut alert: EmergencyAlert = alerts.get(alert_id).ok_or(ContractError::NotFound)?;

        if verified {
            alert.status = STATUS_VERIFIED;
            alert.verified_by = Some(admin.clone());

            // Create immediate escrow for emergency reward. The executor has
            // already been authorized by the outer multi-sig call, so use the
            // internal variants to avoid redundant nested `require_auth` calls
            // for the same address.
            let escrow_id = Self::create_escrow_internal(
                env.clone(),
                &admin, // Admin deposits on behalf of the platform
                &alert.reporter,
                alert.emergency_reward,
                PURPOSE_EMERGENCY,
                0,    // No lock period for emergency rewards
                None, // Emergency releases are authorized by admin auth, not a signer
            )?;

            // Immediately mark conditions as met and release
            Self::mark_escrow_conditions_met_internal(env.clone(), escrow_id, &admin)?;
            Self::release_escrow_internal(&env, escrow_id, &admin, None)?;

            // Update reputation
            Self::update_reputation(
                env.clone(),
                alert.reporter.clone(),
                1,
                alert.emergency_reward,
            )?;
        } else {
            alert.status = Symbol::new(&env, STATUS_FALSE_POSITIVE);
        }

        alerts.set(alert_id, alert);
        env.storage().instance().set(&EMERGENCY_ALERTS, &alerts);

        Ok(())
    }

    /// Get escrow details
    pub fn get_escrow(env: Env, escrow_id: u64) -> Result<EscrowEntry, ContractError> {
        let escrows: Map<u64, EscrowEntry> = env
            .storage()
            .instance()
            .get(&ESCROWS)
            .unwrap_or(Map::new(&env));
        escrows.get(escrow_id).ok_or(ContractError::NotFound)
    }

    /// Canonical message that an escrow's registered `release_signer` must sign
    /// (raw, unhashed) to authorize a release via `release_escrow`. Returns
    /// `NotFound` if the escrow does not exist.
    pub fn escrow_release_message(env: Env, escrow_id: u64) -> Result<Bytes, ContractError> {
        let escrow = Self::get_escrow(env.clone(), escrow_id)?;
        let nonce = Self::escrow_nonce(&env, escrow_id)?;
        Ok(Self::build_release_message(&env, &escrow, &nonce))
    }

    /// Get emergency alert details
    pub fn get_emergency_alert(env: Env, alert_id: u64) -> Result<EmergencyAlert, ContractError> {
        let alerts: Map<u64, EmergencyAlert> = env
            .storage()
            .instance()
            .get(&EMERGENCY_ALERTS)
            .unwrap_or(Map::new(&env));
        alerts.get(alert_id).ok_or(ContractError::NotFound)
    }

    /// Get escrow pool balance
    pub fn get_escrow_pool_balance(env: Env) -> i128 {
        env.storage().instance().get(&ESCROW).unwrap_or(0i128)
    }

    /// Get emergency pool balance
    pub fn get_emergency_pool_balance(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&EMERGENCY_POOL)
            .unwrap_or(0i128)
    }

    /// Fund emergency pool (requires TreasuryManager role)
    pub fn fund_emergency_pool(
        env: Env,
        funder: Address,
        amount: i128,
    ) -> Result<(), ContractError> {
        funder.require_auth();
        Self::require_non_default_address(&env, &funder)?;
        Self::require_positive_amount(amount)?;

        // Check role-based permissions
        Self::require_permission(&env, &funder, Permission::ManageTreasury)?;

        Self::receive_tokens(&env, &funder, amount)?;

        let mut current_pool: i128 = env
            .storage()
            .instance()
            .get(&EMERGENCY_POOL)
            .unwrap_or(0i128);
        current_pool = Self::checked_add_i128(current_pool, amount)?;
        env.storage().instance().set(&EMERGENCY_POOL, &current_pool);

        Ok(())
    }

    // Role management functions (require SuperAdmin role)

    /// Grant a role to an address (requires SuperAdmin role and multi-sig)
    pub fn grant_role(
        env: Env,
        super_admin: Address,
        user: Address,
        _role: Role,
    ) -> Result<(), ContractError> {
        super_admin.require_auth();
        Self::require_non_default_address(&env, &super_admin)?;
        Self::require_non_default_address(&env, &user)?;

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

        let parameters = vec![&env, user.to_string(), String::from_str(&env, role_str)];

        // Role management has higher security requirements
        let role_delay = if execution_delay < 86400 {
            86400
        } else {
            execution_delay
        }; // Minimum 24 hours
        let role_approvals = if required_approvals < 2 {
            2
        } else {
            required_approvals
        }; // Minimum 2 approvals

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

        // Single storage load for the whole execution.
        let mut proposals: Map<u64, MultiSigProposal> = env
            .storage()
            .instance()
            .get(&MULTI_SIG_PROPOSALS)
            .unwrap_or(Map::new(&env));
        let mut proposal: MultiSigProposal = proposals
            .get(proposal_id)
            .ok_or(ContractError::ProposalNotFound)?;

        if proposal.executed {
            return Err(ContractError::ProposalAlreadyExecuted);
        }
        let current_time = env.ledger().timestamp();
        let ready_after = Self::checked_add_u64(proposal.created_at, proposal.execution_delay)?;
        if current_time < ready_after
            || (proposal.approvals.len() as u64) < proposal.required_approvals
        {
            return Err(ContractError::ProposalNotReady);
        }

        // The user address is stored in canonical strkey form (see
        // `propose_role_grant`), so reconstruct it directly from the stored SDK
        // string. The previous code stored `format!("{:?}", user)` and rebuilt the
        // address from that `Debug` output, which is not a valid strkey — so the
        // granted address could differ from the one that was proposed and approved.
        let user_address = Address::from_string(&Self::proposal_param(&proposal.parameters, 0)?);
        let role_s = Self::sdk_string_to_rust(Self::proposal_param(&proposal.parameters, 1)?)?;
        let role = match role_s.as_str() {
            "SuperAdmin" => Role::SuperAdmin,
            "Verifier" => Role::Verifier,
            "EscrowManager" => Role::EscrowManager,
            "TreasuryManager" => Role::TreasuryManager,
            _ => return Err(ContractError::InvalidRole),
        };

        // Execute the role grant
        Self::execute_role_grant_internal(env.clone(), user_address, role)?;

        // Mark proposal as executed and write back in a single pass
        proposal.executed = true;
        proposals.set(proposal_id, proposal);
        env.storage()
            .instance()
            .set(&MULTI_SIG_PROPOSALS, &proposals);

        Ok(())
    }

    /// Internal function to execute role grant
    fn execute_role_grant_internal(
        env: Env,
        user: Address,
        role: Role,
    ) -> Result<(), ContractError> {
        let mut admin_roles: Map<Address, Vec<Role>> = env
            .storage()
            .instance()
            .get(&ADMIN_ROLES)
            .unwrap_or(Map::new(&env));

        let mut user_roles = admin_roles.get(user.clone()).unwrap_or(Vec::new(&env));
        if user_roles.contains(&role) {
            return Err(ContractError::RoleAlreadyGranted);
        }
        user_roles.push_back(role);

        admin_roles.set(user, user_roles);
        env.storage().instance().set(&ADMIN_ROLES, &admin_roles);

        Ok(())
    }

    /// Revoke a role from an address (requires SuperAdmin role and multi-sig)
    pub fn revoke_role(
        env: Env,
        super_admin: Address,
        user: Address,
        _role: Role,
    ) -> Result<(), ContractError> {
        super_admin.require_auth();
        Self::require_non_default_address(&env, &super_admin)?;
        Self::require_non_default_address(&env, &user)?;

        // Role management always requires multi-signature
        Err(ContractError::MultiSigRequired)
    }

    /// Get user roles
    pub fn get_user_roles(env: Env, user: Address) -> Result<Vec<Role>, ContractError> {
        let admin_roles: Map<Address, Vec<Role>> = env
            .storage()
            .instance()
            .get(&ADMIN_ROLES)
            .unwrap_or(Map::new(&env));
        Ok(admin_roles.get(user).unwrap_or(Vec::new(&env)))
    }

    /// Get multi-signature proposal details
    pub fn get_proposal(env: Env, proposal_id: u64) -> Result<MultiSigProposal, ContractError> {
        let proposals: Map<u64, MultiSigProposal> = env
            .storage()
            .instance()
            .get(&MULTI_SIG_PROPOSALS)
            .unwrap_or(Map::new(&env));
        proposals
            .get(proposal_id)
            .ok_or(ContractError::ProposalNotFound)
    }

    /// Check if a proposal can be executed
    pub fn can_execute_proposal_check(env: Env, proposal_id: u64) -> Result<bool, ContractError> {
        Self::can_execute_proposal(&env, proposal_id)
    }
}

#[cfg(test)]
mod test;
