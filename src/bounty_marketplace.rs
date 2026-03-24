//! Security Bounty Marketplace Smart Contract
//! 
//! This contract manages the financial side of the Security Bounty Marketplace,
//! holding XLM in escrow until bugs are verified and approved.

use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Env, Symbol, panic_with_error, 
    Map, Vec, i128, u64
};

// Contract state keys
const ADMIN: Symbol = Symbol::short("ADMIN");
const OWNER: Symbol = Symbol::short("OWNER");
const BOUNTY_COUNTER: Symbol = Symbol::short("BOUNTY_C");
const BOUNTIES: Symbol = Symbol::short("BOUNTIES");
const RESEARCHER_ASSIGNMENTS: Symbol = Symbol::short("RESEARCHER");
const PENDING_APPROVALS: Symbol = Symbol::short("PENDING");
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
    // Initialize the contract with admin and owner
    pub fn initialize(env: Env, admin: Address, owner: Address) {
        // Check if already initialized
        if env.storage().persistent().has(&ADMIN) {
            panic_with_error!(&env, "Contract already initialized");
        }

        // Validate addresses
        if admin == Address::default() || owner == Address::default() {
            panic_with_error!(&env, "Invalid addresses provided");
        }

        // Set admin and owner
        env.storage().persistent().set(&ADMIN, &admin);
        env.storage().persistent().set(&OWNER, &owner);
        env.storage().persistent().set(&BOUNTY_COUNTER, &0u64);

        // Initialize empty maps
        env.storage().persistent().set(&BOUNTIES, &Map::<Address, Vec<Bounty>>::new(&env));
        env.storage().persistent().set(&RESEARCHER_ASSIGNMENTS, &Map::<Address, Vec<u64>>::new(&env));
        env.storage().persistent().set(&PENDING_APPROVALS, &Map::<u64, MultiSigApproval>::new(&env));

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
        if amount <= 0 {
            panic_with_error!(&env, "Amount must be positive");
        }
        if title.is_empty() || description.is_empty() {
            panic_with_error!(&env, "Title and description cannot be empty");
        }

        // Check authorization
        creator.require_auth();

        // Get current time
        let current_time = env.ledger().timestamp();
        let timelock_until = current_time + TIMELOCK_PERIOD;

        // Generate bounty ID
        let mut bounty_counter: u64 = env.storage().persistent().get(&BOUNTY_COUNTER).unwrap_or(0);
        bounty_counter += 1;
        env.storage().persistent().set(&BOUNTY_COUNTER, &bounty_counter);

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
        // Check authorization
        researcher.require_auth();

        // Get bounty
        let bounty = Self::get_bounty_internal(&env, bounty_id);
        
        // Validate bounty status
        if bounty.status != BountyStatus::Approved {
            panic_with_error!(&env, "Bounty must be approved to claim reward");
        }

        // Check if researcher is assigned
        if bounty.assigned_researcher != Some(researcher.clone()) {
            panic_with_error!(&env, "Researcher not assigned to this bounty");
        }

        // Get multi-sig approval
        let mut pending_approvals: Map<u64, MultiSigApproval> = env.storage().persistent().get(&PENDING_APPROVALS)
            .unwrap_or_else(|| Map::new(&env));
        
        let approval = pending_approvals.get(bounty_id)
            .unwrap_or_else(|| panic_with_error!(&env, "Approval not found"));

        // Check multi-sig approval
        if !(approval.admin_approved && approval.owner_approved) {
            panic_with_error!(&env, "Multi-sig approval required (Admin + Owner)");
        }

        // Calculate reward amount based on severity
        let reward_amount = match bounty.severity {
            Severity::Critical => bounty.amount,
            Severity::High => bounty.amount,
            Severity::Medium => bounty.amount * bounty.severity.reward_percentage() / 100,
            Severity::Low => bounty.amount * bounty.severity.reward_percentage() / 100,
        };

        // Update bounty status to completed
        Self::update_bounty_status(&env, bounty_id, BountyStatus::Completed);

        // Remove from pending approvals
        pending_approvals.remove(bounty_id);
        env.storage().persistent().set(&PENDING_APPROVALS, &pending_approvals);

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

        // Validate amount
        if amount <= 0 {
            panic_with_error!(&env, "Amount must be positive");
        }

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
                    Severity::Medium => bounty.amount * bounty.severity.reward_percentage() / 100,
                    Severity::Low => bounty.amount * bounty.severity.reward_percentage() / 100,
                };
                total_available += reward;
            }
        }

        if total_available < amount {
            panic_with_error!(&env, "Insufficient available rewards");
        }

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
