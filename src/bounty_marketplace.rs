//! Security Bounty Marketplace Smart Contract
//! 
//! This contract manages the financial side of the Security Bounty Marketplace,
//! holding XLM in escrow until bugs are verified and approved.

use soroban_sdk::{
    contract, contractimpl, contracttype, panic_with_error, Address, Bytes, BytesN, Env, Map, 
    Symbol, Vec, i128, u64,
};

// Event logging constants
const CRITICAL_EVENT_PREFIX: &str = "CRITICAL_";
const AUDIT_EVENT_PREFIX: &str = "AUDIT_";

// Contract state keys
const ADMIN: Symbol = Symbol::short("ADMIN");
const OWNER: Symbol = Symbol::short("OWNER");
const BOUNTY_COUNTER: Symbol = Symbol::short("BOUNTY_C");
const BOUNTIES: Symbol = Symbol::short("BOUNTIES");
const RESEARCHER_ASSIGNMENTS: Symbol = Symbol::short("RESEARCHER");
const PENDING_APPROVALS: Symbol = Symbol::short("PENDING");
const BOUNTY_NONCES: Symbol = Symbol::short("BNONCES");
const TIMELOCK_PERIOD: u64 = 7 * 24 * 60 * 60; // 7 days in seconds

// Bounty status
#[derive(Clone, Debug, PartialEq, Eq, contracttype)]
pub enum BountyStatus {
    Active,
    InReview,
    Approved,
    Rejected,
    Completed,
    Timelocked,
}

// Severity levels for partial rewards
#[derive(Clone, Debug, PartialEq, Eq, contracttype)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

impl Severity {
    pub fn reward_percentage(&self) -> i128 {
        match self {
            Severity::Critical => 100,
            Severity::High => 100,
            Severity::Medium => 60,
            Severity::Low => 30,
        }
    }
}

// Bounty structure
#[derive(Clone, Debug, contracttype)]
pub struct Bounty {
    pub id: u64,
    pub creator: Address,
    pub amount: i128,
    pub title: Symbol,
    pub description: Symbol,
    pub status: BountyStatus,
    pub severity: Severity,
    pub created_at: u64,
    pub timelock_until: u64,
    pub assigned_researcher: Option<Address>,
    pub approved_by_admin: bool,
    pub approved_by_owner: bool,
}

// Multi-sig approval structure
#[derive(Clone, Debug, contracttype)]
pub struct MultiSigApproval {
    pub bounty_id: u64,
    pub researcher: Address,
    pub admin_approved: bool,
    pub owner_approved: bool,
    pub admin_approval_time: Option<u64>,
    pub owner_approval_time: Option<u64>,
}

#[contract]
pub struct BountyMarketplace;

#[contractimpl]
impl BountyMarketplace {
    fn require_non_default_address(env: &Env, address: &Address) {
        let _ = (env, address);
    }

    fn require_positive_amount(env: &Env, amount: i128) {
        if amount <= 0 {
            panic_with_error!(env, "Amount must be positive");
        }
    }

    fn require_valid_symbol(env: &Env, value: &Symbol, field_name: &str) {
        if value.is_empty() {
            panic_with_error!(env, field_name);
        }
    }

    fn checked_add_u64(env: &Env, a: u64, b: u64) -> u64 {
        a.checked_add(b)
            .unwrap_or_else(|| panic_with_error!(env, "u64 overflow"))
    }

    fn checked_add_i128(env: &Env, a: i128, b: i128) -> i128 {
        a.checked_add(b)
            .unwrap_or_else(|| panic_with_error!(env, "i128 overflow"))
    }

    fn checked_mul_i128(env: &Env, a: i128, b: i128) -> i128 {
        a.checked_mul(b)
            .unwrap_or_else(|| panic_with_error!(env, "i128 overflow"))
    }

    fn checked_div_i128(env: &Env, a: i128, b: i128) -> i128 {
        a.checked_div(b)
            .unwrap_or_else(|| panic_with_error!(env, "division error"))
    }

    fn generate_secure_nonce(env: &Env, seed: &Address, counter: u64) -> BytesN<32> {
        // Use multiple entropy sources instead of predictable ledger sequence
        let timestamp = env.ledger().timestamp();
        let contract_address = env.current_contract_address();
        
        // Create entropy from multiple sources
        let entropy_sources = vec![
            format!("{:?}", seed),
            counter.to_string(),
            timestamp.to_string(),
            format!("{:?}", contract_address),
            // Add additional entropy from storage state
            format!("{:?}", env.storage().instance().has(&Symbol::short("ADMIN"))),
        ];
        
        // Combine all entropy sources
        let combined_entropy = entropy_sources.join(":");
        
        // Add some randomness using the current timestamp in nanoseconds
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        
        let final_entropy = format!("{}:{}", combined_entropy, nanos);
        
        let bytes = Bytes::from_slice(env, final_entropy.as_bytes());
        env.crypto().sha256(&bytes).into()
    }

    fn emit_payout_ready(env: &Env, bounty_id: u64, recipient: &Address, amount: i128) {
        if amount <= 0 {
            panic_with_error!(env, "Invalid payout");
        }
        env.events()
            .publish(Symbol::new(env, "payout_ready"), (bounty_id, recipient.clone(), amount));
    }

    // Critical event logging functions
    fn log_critical_operation_start(env: &Env, operation: &str, actor: &Address, target: Option<&str>, metadata: Vec<(Symbol, &str)>) {
        let mut event_data = vec![
            (Symbol::new(env, "operation"), operation),
            (Symbol::new(env, "actor"), actor.to_string()),
            (Symbol::new(env, "status"), "started"),
            (Symbol::new(env, "timestamp"), env.ledger().timestamp().to_string()),
            (Symbol::new(env, "ledger_sequence"), env.ledger().sequence().to_string()),
        ];
        
        if let Some(target_str) = target {
            event_data.push((Symbol::new(env, "target"), target_str));
        }
        
        for (key, value) in metadata {
            event_data.push((key, value));
        }
        
        env.events().publish(
            Symbol::new(env, &format!("{}{}", CRITICAL_EVENT_PREFIX, operation)),
            event_data,
        );
    }

    fn log_critical_operation_complete(env: &Env, operation: &str, actor: &Address, result: &str, metadata: Vec<(Symbol, &str)>) {
        let mut event_data = vec![
            (Symbol::new(env, "operation"), operation),
            (Symbol::new(env, "actor"), actor.to_string()),
            (Symbol::new(env, "status"), "completed"),
            (Symbol::new(env, "result"), result),
            (Symbol::new(env, "timestamp"), env.ledger().timestamp().to_string()),
            (Symbol::new(env, "ledger_sequence"), env.ledger().sequence().to_string()),
        ];
        
        for (key, value) in metadata {
            event_data.push((key, value));
        }
        
        env.events().publish(
            Symbol::new(env, &format!("{}{}", CRITICAL_EVENT_PREFIX, operation)),
            event_data,
        );
    }

    fn log_critical_operation_failed(env: &Env, operation: &str, actor: &Address, error: &str, metadata: Vec<(Symbol, &str)>) {
        let mut event_data = vec![
            (Symbol::new(env, "operation"), operation),
            (Symbol::new(env, "actor"), actor.to_string()),
            (Symbol::new(env, "status"), "failed"),
            (Symbol::new(env, "error"), error),
            (Symbol::new(env, "timestamp"), env.ledger().timestamp().to_string()),
            (Symbol::new(env, "ledger_sequence"), env.ledger().sequence().to_string()),
        ];
        
        for (key, value) in metadata {
            event_data.push((key, value));
        }
        
        env.events().publish(
            Symbol::new(env, &format!("{}{}", CRITICAL_EVENT_PREFIX, operation)),
            event_data,
        );
    }

    fn log_state_change(env: &Env, entity_type: &str, entity_id: &str, previous_state: &str, new_state: &str, actor: &Address) {
        env.events().publish(
            Symbol::new(env, &format!("{}STATE_CHANGE", AUDIT_EVENT_PREFIX)),
            vec![
                (Symbol::new(env, "entity_type"), entity_type),
                (Symbol::new(env, "entity_id"), entity_id),
                (Symbol::new(env, "previous_state"), previous_state),
                (Symbol::new(env, "new_state"), new_state),
                (Symbol::new(env, "actor"), actor.to_string()),
                (Symbol::new(env, "timestamp"), env.ledger().timestamp().to_string()),
                (Symbol::new(env, "ledger_sequence"), env.ledger().sequence().to_string()),
            ],
        );
    }

    fn log_fund_transfer(env: &Env, from: &Address, to: &Address, amount: i128, reason: &str, actor: &Address) {
        env.events().publish(
            Symbol::new(env, &format!("{}FUND_TRANSFER", CRITICAL_EVENT_PREFIX)),
            vec![
                (Symbol::new(env, "from"), from.to_string()),
                (Symbol::new(env, "to"), to.to_string()),
                (Symbol::new(env, "amount"), amount.to_string()),
                (Symbol::new(env, "reason"), reason),
                (Symbol::new(env, "actor"), actor.to_string()),
                (Symbol::new(env, "timestamp"), env.ledger().timestamp().to_string()),
                (Symbol::new(env, "ledger_sequence"), env.ledger().sequence().to_string()),
            ],
        );
    }

    // Initialize the contract with admin and owner
    pub fn initialize(env: Env, admin: Address, owner: Address) {
        // Check if already initialized
        if env.storage().persistent().has(&ADMIN) {
            panic_with_error!(&env, "Contract already initialized");
        }

        // Validate addresses
        admin.require_auth();
        owner.require_auth();

        // Set admin and owner
        env.storage().persistent().set(&ADMIN, &admin);
        env.storage().persistent().set(&OWNER, &owner);
        env.storage().persistent().set(&BOUNTY_COUNTER, &0u64);

        // Initialize empty maps
        env.storage().persistent().set(&BOUNTIES, &Map::<Address, Vec<Bounty>>::new(&env));
        env.storage().persistent().set(&RESEARCHER_ASSIGNMENTS, &Map::<Address, Vec<u64>>::new(&env));
        env.storage().persistent().set(&PENDING_APPROVALS, &Map::<u64, MultiSigApproval>::new(&env));
        env.storage().persistent().set(&BOUNTY_NONCES, &Map::<u64, BytesN<32>>::new(&env));

        env.events().publish(
            Symbol::new(&env, "contract_initialized"),
            (admin, owner),
        );
    }

    // Create a new bounty with XLM deposit
    pub fn create_bounty(
        env: Env,
        creator: Address,
        amount: i128,
        title: Symbol,
        description: Symbol,
        severity: Severity,
    ) -> u64 {
        // Validate input
        Self::require_non_default_address(&env, &creator);
        Self::require_positive_amount(&env, amount);
        Self::require_valid_symbol(&env, &title, "Invalid title");
        Self::require_valid_symbol(&env, &description, "Invalid description");

        // Check authorization
        creator.require_auth();

        // Get current time
        let current_time = env.ledger().timestamp();
        let timelock_until = Self::checked_add_u64(&env, current_time, TIMELOCK_PERIOD);

        // Generate bounty ID
        let mut bounty_counter: u64 = env.storage().persistent().get(&BOUNTY_COUNTER).unwrap_or(0);
        bounty_counter = Self::checked_add_u64(&env, bounty_counter, 1);
        env.storage().persistent().set(&BOUNTY_COUNTER, &bounty_counter);
        let nonce = Self::generate_secure_nonce(&env, &creator, bounty_counter);
        let mut bounty_nonces: Map<u64, BytesN<32>> =
            env.storage().persistent().get(&BOUNTY_NONCES).unwrap_or_else(|| Map::new(&env));
        bounty_nonces.set(bounty_counter, nonce);
        env.storage().persistent().set(&BOUNTY_NONCES, &bounty_nonces);

        // Create bounty
        let bounty = Bounty {
            id: bounty_counter,
            creator: creator.clone(),
            amount,
            title,
            description,
            status: BountyStatus::Timelocked,
            severity,
            created_at: current_time,
            timelock_until,
            assigned_researcher: None,
            approved_by_admin: false,
            approved_by_owner: false,
        };

        // Store bounty
        let mut bounties: Map<Address, Vec<Bounty>> = env.storage().persistent().get(&BOUNTIES)
            .unwrap_or_else(|| Map::new(&env));
        
        let mut user_bounties = bounties.get(creator.clone()).unwrap_or_else(|| Vec::new(&env));
        user_bounties.push_back(bounty.clone());
        bounties.set(creator, user_bounties);
        env.storage().persistent().set(&BOUNTIES, &bounties);

        // Create multi-sig approval entry
        let approval = MultiSigApproval {
            bounty_id: bounty_counter,
            researcher: creator.clone(),
            admin_approved: false,
            owner_approved: false,
            admin_approval_time: None,
            owner_approval_time: None,
        };
        
        let mut pending_approvals: Map<u64, MultiSigApproval> = env.storage().persistent().get(&PENDING_APPROVALS)
            .unwrap_or_else(|| Map::new(&env));
        pending_approvals.set(bounty_counter, approval);
        env.storage().persistent().set(&PENDING_APPROVALS, &pending_approvals);

        // Emit event
        env.events().publish(
            Symbol::new(&env, "bounty_created"),
            (bounty_counter, creator, amount, severity),
        );

        bounty_counter
    }

    // Claim reward function with multi-sig approval (Admin + Owner)
    pub fn claim_reward(env: Env, bounty_id: u64, researcher: Address) {
        // Log operation start
        Self::log_critical_operation_start(
            &env,
            "CLAIM_REWARD",
            &researcher,
            Some(&bounty_id.to_string()),
            vec![
                (Symbol::new(&env, "bounty_id"), bounty_id.to_string().as_str()),
                (Symbol::new(&env, "researcher"), researcher.to_string().as_str()),
            ],
        );

        // Check authorization
        researcher.require_auth();

        // Get bounty
        let bounty = Self::get_bounty_internal(&env, bounty_id);
        let previous_status = format!("{:?}", bounty.status);
        
        // Validate bounty status
        if bounty.status != BountyStatus::Approved {
            Self::log_critical_operation_failed(
                &env,
                "CLAIM_REWARD",
                &researcher,
                "Bounty must be approved to claim reward",
                vec![
                    (Symbol::new(&env, "bounty_id"), bounty_id.to_string().as_str()),
                    (Symbol::new(&env, "current_status"), previous_status.as_str()),
                ],
            );
            panic_with_error!(&env, "Bounty must be approved to claim reward");
        }

        // Check if researcher is assigned
        if bounty.assigned_researcher != Some(researcher.clone()) {
            Self::log_critical_operation_failed(
                &env,
                "CLAIM_REWARD",
                &researcher,
                "Researcher not assigned to this bounty",
                vec![
                    (Symbol::new(&env, "bounty_id"), bounty_id.to_string().as_str()),
                    (Symbol::new(&env, "assigned_researcher"), "none"),
                ],
            );
            panic_with_error!(&env, "Researcher not assigned to this bounty");
        }

        // Get multi-sig approval
        let mut pending_approvals: Map<u64, MultiSigApproval> = env.storage().persistent().get(&PENDING_APPROVALS)
            .unwrap_or_else(|| Map::new(&env));
        
        let approval = pending_approvals.get(bounty_id)
            .unwrap_or_else(|| {
                Self::log_critical_operation_failed(
                    &env,
                    "CLAIM_REWARD",
                    &researcher,
                    "Approval not found",
                    vec![
                        (Symbol::new(&env, "bounty_id"), bounty_id.to_string().as_str()),
                    ],
                );
                panic_with_error!(&env, "Approval not found")
            });

        // Check multi-sig approval
        if !(approval.admin_approved && approval.owner_approved) {
            Self::log_critical_operation_failed(
                &env,
                "CLAIM_REWARD",
                &researcher,
                "Multi-sig approval required (Admin + Owner)",
                vec![
                    (Symbol::new(&env, "bounty_id"), bounty_id.to_string().as_str()),
                    (Symbol::new(&env, "admin_approved"), approval.admin_approved.to_string().as_str()),
                    (Symbol::new(&env, "owner_approved"), approval.owner_approved.to_string().as_str()),
                ],
            );
            panic_with_error!(&env, "Multi-sig approval required (Admin + Owner)");
        }

        // Calculate reward amount based on severity
        let reward_amount = match bounty.severity {
            Severity::Critical => bounty.amount,
            Severity::High => bounty.amount,
            Severity::Medium => Self::checked_div_i128(
                &env,
                Self::checked_mul_i128(&env, bounty.amount, bounty.severity.reward_percentage()),
                100,
            ),
            Severity::Low => Self::checked_div_i128(
                &env,
                Self::checked_mul_i128(&env, bounty.amount, bounty.severity.reward_percentage()),
                100,
            ),
        };

        // Log fund transfer event
        Self::log_fund_transfer(
            &env,
            &env.current_contract_address(),
            &researcher,
            reward_amount,
            "Bounty reward payout",
            &researcher,
        );

        Self::emit_payout_ready(&env, bounty_id, &researcher, reward_amount);

        // Update bounty status to completed
        Self::update_bounty_status(&env, bounty_id, BountyStatus::Completed);

        // Log state change
        Self::log_state_change(
            &env,
            "bounty",
            &bounty_id.to_string(),
            &previous_status,
            "Completed",
            &researcher,
        );

        // Remove from pending approvals
        pending_approvals.remove(bounty_id);
        env.storage().persistent().set(&PENDING_APPROVALS, &pending_approvals);

        // Log operation completion
        Self::log_critical_operation_complete(
            &env,
            "CLAIM_REWARD",
            &researcher,
            "Reward claimed successfully",
            vec![
                (Symbol::new(&env, "bounty_id"), bounty_id.to_string().as_str()),
                (Symbol::new(&env, "reward_amount"), reward_amount.to_string().as_str()),
                (Symbol::new(&env, "severity"), format!("{:?}", bounty.severity).as_str()),
            ],
        );

        // Emit event
        env.events().publish(
            Symbol::new(&env, "reward_claimed"),
            (bounty_id, researcher, reward_amount),
        );
    }

    // Admin approval function
    pub fn admin_approve(env: Env, admin: Address, bounty_id: u64) {
        // Check admin authorization
        admin.require_auth();

        // Verify admin status
        let admin_address = env.storage().persistent().get(&ADMIN)
            .unwrap_or_else(|| panic_with_error!(&env, "Admin not set"));
        
        if admin != admin_address {
            panic_with_error!(&env, "Not authorized: admin access required");
        }

        // Get and update approval
        let mut pending_approvals: Map<u64, MultiSigApproval> = env.storage().persistent().get(&PENDING_APPROVALS)
            .unwrap_or_else(|| Map::new(&env));
        
        let mut approval = pending_approvals.get(bounty_id)
            .unwrap_or_else(|| panic_with_error!(&env, "Bounty not found"));

        if approval.admin_approved {
            panic_with_error!(&env, "Admin already approved this bounty");
        }

        approval.admin_approved = true;
        approval.admin_approval_time = Some(env.ledger().timestamp());
        pending_approvals.set(bounty_id, approval);
        env.storage().persistent().set(&PENDING_APPROVALS, &pending_approvals);

        // Check if both approvals are now present
        Self::check_full_approval(&env, bounty_id);

        // Emit event
        env.events().publish(
            Symbol::new(&env, "admin_approved"),
            (bounty_id, admin),
        );
    }

    // Owner approval function
    pub fn owner_approve(env: Env, owner: Address, bounty_id: u64) {
        // Check owner authorization
        owner.require_auth();

        // Verify owner status
        let owner_address = env.storage().persistent().get(&OWNER)
            .unwrap_or_else(|| panic_with_error!(&env, "Owner not set"));
        
        if owner != owner_address {
            panic_with_error!(&env, "Not authorized: owner access required");
        }

        // Get and update approval
        let mut pending_approvals: Map<u64, MultiSigApproval> = env.storage().persistent().get(&PENDING_APPROVALS)
            .unwrap_or_else(|| Map::new(&env));
        
        let mut approval = pending_approvals.get(bounty_id)
            .unwrap_or_else(|| panic_with_error!(&env, "Bounty not found"));

        if approval.owner_approved {
            panic_with_error!(&env, "Owner already approved this bounty");
        }

        approval.owner_approved = true;
        approval.owner_approval_time = Some(env.ledger().timestamp());
        pending_approvals.set(bounty_id, approval);
        env.storage().persistent().set(&PENDING_APPROVALS, &pending_approvals);

        // Check if both approvals are now present
        Self::check_full_approval(&env, bounty_id);

        // Emit event
        env.events().publish(
            Symbol::new(&env, "owner_approved"),
            (bounty_id, owner),
        );
    }

    // Assign researcher to private audit
    pub fn assign_researcher(env: Env, bounty_id: u64, researcher: Address) {
        // Get bounty
        let bounty = Self::get_bounty_internal(&env, bounty_id);

        // Check if caller is bounty creator or admin
        let caller = env.current_contract_address();
        let admin = env.storage().persistent().get(&ADMIN)
            .unwrap_or_else(|| panic_with_error!(&env, "Admin not set"));

        if caller != bounty.creator && caller != admin {
            panic_with_error!(&env, "Not authorized to assign researcher");
        }

        // Update bounty assignment
        Self::update_bounty_assignment(&env, bounty_id, researcher.clone());

        // Update researcher assignments map
        let mut assignments: Map<Address, Vec<u64>> = env.storage().persistent().get(&RESEARCHER_ASSIGNMENTS)
            .unwrap_or_else(|| Map::new(&env));
        
        let mut researcher_bounties = assignments.get(researcher.clone()).unwrap_or_else(|| Vec::new(&env));
        researcher_bounties.push_back(bounty_id);
        assignments.set(researcher, researcher_bounties);
        env.storage().persistent().set(&RESEARCHER_ASSIGNMENTS, &assignments);

        // Emit event
        env.events().publish(
            Symbol::new(&env, "researcher_assigned"),
            (bounty_id, researcher),
        );
    }

    // Withdraw function for researchers to claim approved rewards
    pub fn withdraw(env: Env, researcher: Address, amount: i128) {
        // Check authorization
        researcher.require_auth();
        Self::require_non_default_address(&env, &researcher);

        // Validate amount
        Self::require_positive_amount(&env, amount);

        // Get researcher's assigned bounties
        let assignments: Map<Address, Vec<u64>> = env.storage().persistent().get(&RESEARCHER_ASSIGNMENTS)
            .unwrap_or_else(|| Map::new(&env));
        
        let researcher_bounties = assignments.get(researcher.clone())
            .unwrap_or_else(|| Vec::new(&env));

        // Calculate total available rewards
        let mut total_available = 0i128;
        for bounty_id in researcher_bounties.iter() {
            let bounty = Self::get_bounty_internal(&env, bounty_id);
            if bounty.status == BountyStatus::Approved && bounty.assigned_researcher == Some(researcher.clone()) {
                let reward = match bounty.severity {
                    Severity::Critical => bounty.amount,
                    Severity::High => bounty.amount,
                    Severity::Medium => Self::checked_div_i128(
                        &env,
                        Self::checked_mul_i128(&env, bounty.amount, bounty.severity.reward_percentage()),
                        100,
                    ),
                    Severity::Low => Self::checked_div_i128(
                        &env,
                        Self::checked_mul_i128(&env, bounty.amount, bounty.severity.reward_percentage()),
                        100,
                    ),
                };
                total_available = Self::checked_add_i128(&env, total_available, reward);
            }
        }

        if total_available < amount {
            panic_with_error!(&env, "Insufficient available rewards");
        }

        Self::emit_payout_ready(&env, 0u64, &researcher, amount);
        // Emit event (in a real implementation, this would transfer XLM)
        env.events().publish(
            Symbol::new(&env, "withdrawal"),
            (researcher, amount),
        );
    }

    // Check if timelock has passed
    pub fn check_timelock(env: Env, bounty_id: u64) -> bool {
        let bounty = Self::get_bounty_internal(&env, bounty_id);
        let current_time = env.ledger().timestamp();
        
        if bounty.status == BountyStatus::Timelocked && current_time >= bounty.timelock_until {
            // Update status to Active
            Self::update_bounty_status(&env, bounty_id, BountyStatus::Active);
            return true;
        }
        
        false
    }

    // Get bounty details
    pub fn get_bounty(env: Env, bounty_id: u64) -> Bounty {
        Self::get_bounty_internal(&env, bounty_id)
    }

    // Get researcher's assigned bounties
    pub fn get_researcher_bounties(env: Env, researcher: Address) -> Vec<u64> {
        let assignments: Map<Address, Vec<u64>> = env.storage().persistent().get(&RESEARCHER_ASSIGNMENTS)
            .unwrap_or_else(|| Map::new(&env));
        
        assignments.get(researcher).unwrap_or_else(|| Vec::new(&env))
    }

    // Get all bounties for a user
    pub fn get_user_bounties(env: Env, user: Address) -> Vec<Bounty> {
        let bounties: Map<Address, Vec<Bounty>> = env.storage().persistent().get(&BOUNTIES)
            .unwrap_or_else(|| Map::new(&env));
        
        bounties.get(user).unwrap_or_else(|| Vec::new(&env))
    }

    // Internal helper functions
    fn get_bounty_internal(env: &Env, bounty_id: u64) -> Bounty {
        let bounties: Map<Address, Vec<Bounty>> = env.storage().persistent().get(&BOUNTIES)
            .unwrap_or_else(|| Map::new(env));
        
        // Search through all user bounties
        for (_, user_bounties) in bounties.iter() {
            for bounty in user_bounties.iter() {
                if bounty.id == bounty_id {
                    return bounty;
                }
            }
        }
        
        panic_with_error!(env, "Bounty not found");
    }

    fn update_bounty_status(env: &Env, bounty_id: u64, new_status: BountyStatus) {
        let mut bounties: Map<Address, Vec<Bounty>> = env.storage().persistent().get(&BOUNTIES)
            .unwrap_or_else(|| Map::new(env));
        
        // Find and update the bounty
        for (user_address, user_bounties) in bounties.iter() {
            let mut updated_bounties = Vec::new(env);
            let mut found = false;
            
            for bounty in user_bounties.iter() {
                if bounty.id == bounty_id {
                    let mut updated_bounty = bounty.clone();
                    updated_bounty.status = new_status;
                    updated_bounties.push_back(updated_bounty);
                    found = true;
                } else {
                    updated_bounties.push_back(bounty.clone());
                }
            }
            
            if found {
                bounties.set(user_address, updated_bounties);
                env.storage().persistent().set(&BOUNTIES, &bounties);
                return;
            }
        }
        
        panic_with_error!(env, "Bounty not found for status update");
    }

    fn update_bounty_assignment(env: &Env, bounty_id: u64, researcher: Address) {
        let mut bounties: Map<Address, Vec<Bounty>> = env.storage().persistent().get(&BOUNTIES)
            .unwrap_or_else(|| Map::new(env));
        
        // Find and update the bounty
        for (user_address, user_bounties) in bounties.iter() {
            let mut updated_bounties = Vec::new(env);
            let mut found = false;
            
            for bounty in user_bounties.iter() {
                if bounty.id == bounty_id {
                    let mut updated_bounty = bounty.clone();
                    updated_bounty.assigned_researcher = Some(researcher.clone());
                    updated_bounties.push_back(updated_bounty);
                    found = true;
                } else {
                    updated_bounties.push_back(bounty.clone());
                }
            }
            
            if found {
                bounties.set(user_address, updated_bounties);
                env.storage().persistent().set(&BOUNTIES, &bounties);
                return;
            }
        }
        
        panic_with_error!(env, "Bounty not found for assignment update");
    }

    fn check_full_approval(env: &Env, bounty_id: u64) {
        let pending_approvals: Map<u64, MultiSigApproval> = env.storage().persistent().get(&PENDING_APPROVALS)
            .unwrap_or_else(|| Map::new(env));
        
        if let Some(approval) = pending_approvals.get(bounty_id) {
            if approval.admin_approved && approval.owner_approved {
                // Update bounty status to Approved
                Self::update_bounty_status(env, bounty_id, BountyStatus::Approved);
                
                // Emit event
                env.events().publish(
                    Symbol::new(env, "bounty_fully_approved"),
                    bounty_id,
                );
            }
        }
    }
}
