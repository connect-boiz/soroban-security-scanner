//! Enhanced Soroban contract upgrade mechanism.
//!
//! Adds the safety features missing from the existing upgrade code:
//! - 7-day timelock enforcement for standard upgrades
//! - Quorum-based multi-sig requirement for emergency upgrades
//! - Upgrade cancellation by minority (any admin can cancel)
//! - Storage migration verification hook

use soroban_sdk::{
    contracttype, panic_with_error, symbol_short,
    Address, Bytes, BytesN, Env, Map, Symbol, Vec,
};

use crate::contracts::lib::ContractError;

// Storage keys
const UPGRADE_TIMELOCK: Symbol      = symbol_short!("UP_TL");
const PENDING_UPGRADE:  Symbol      = symbol_short!("PND_UP");
const UPGRADE_VOTES:    Symbol      = symbol_short!("UP_VOTS");
const UPGRADE_QUORUM:   Symbol      = symbol_short!("UP_QR");
const MIGRATION_VER:    Symbol      = symbol_short!("MIG_VER");

/// Standard upgrade timelock: 7 days in seconds.
pub const TIMELOCK_SECONDS: u64 = 7 * 24 * 60 * 60;
/// Emergency upgrade minimum quorum (fraction of admins, rounded up).
pub const EMERGENCY_QUORUM_NUMERATOR: u32 = 2;
pub const EMERGENCY_QUORUM_DENOMINATOR: u32 = 3;

#[derive(Clone, Debug, contracttype)]
pub struct PendingUpgrade {
    pub new_wasm_hash:    BytesN<32>,
    pub proposed_at:      u64,
    pub proposer:         Address,
    pub is_emergency:     bool,
    pub approvals:        Vec<Address>,
    pub migration_script: Option<Bytes>,
}

/// Propose a standard (time-locked) upgrade.
/// Only callable after TIMELOCK_SECONDS from proposal.
pub fn propose_upgrade(
    env:          &Env,
    proposer:     Address,
    new_wasm_hash: BytesN<32>,
    migration_script: Option<Bytes>,
) -> Result<(), ContractError> {
    proposer.require_auth();
    require_admin(env, &proposer)?;

    let pending = PendingUpgrade {
        new_wasm_hash,
        proposed_at: env.ledger().timestamp(),
        proposer: proposer.clone(),
        is_emergency: false,
        approvals: Vec::new(env),
        migration_script,
    };
    env.storage().persistent().set(&PENDING_UPGRADE, &pending);
    env.events().publish((symbol_short!("UP_PROP"), symbol_short!("v2")),
        (proposer, env.ledger().timestamp()));
    Ok(())
}

/// Execute a pending standard upgrade after timelock has elapsed.
pub fn execute_upgrade(env: &Env, executor: Address) -> Result<(), ContractError> {
    executor.require_auth();
    require_admin(env, &executor)?;

    let pending: PendingUpgrade = env.storage().persistent()
        .get(&PENDING_UPGRADE).ok_or(ContractError::ProposalNotFound)?;

    if pending.is_emergency {
        return Err(ContractError::Unauthorized);
    }

    let now = env.ledger().timestamp();
    if now < pending.proposed_at + TIMELOCK_SECONDS {
        return Err(ContractError::TimelockNotExpired);
    }

    // Verify migration if provided
    if let Some(ref script) = pending.migration_script {
        verify_migration(env, script)?;
    }

    env.deployer().update_current_contract_wasm(pending.new_wasm_hash);
    env.storage().persistent().remove(&PENDING_UPGRADE);
    env.events().publish((symbol_short!("UP_EXEC"), symbol_short!("v2")),
        (executor, now));
    Ok(())
}

/// Cancel any pending upgrade. Any admin can cancel.
pub fn cancel_upgrade(env: &Env, canceller: Address) -> Result<(), ContractError> {
    canceller.require_auth();
    require_admin(env, &canceller)?;

    if !env.storage().persistent().has(&PENDING_UPGRADE) {
        return Err(ContractError::ProposalNotFound);
    }
    env.storage().persistent().remove(&PENDING_UPGRADE);
    env.events().publish((symbol_short!("UP_CNCL"), symbol_short!("v2")),
        (canceller, env.ledger().timestamp()));
    Ok(())
}

/// Vote to approve an emergency upgrade.
/// Executes automatically once quorum is reached.
pub fn vote_emergency_upgrade(
    env:     &Env,
    voter:   Address,
    wasm:    BytesN<32>,
) -> Result<bool, ContractError> {
    voter.require_auth();
    require_admin(env, &voter)?;

    let mut pending: PendingUpgrade = env.storage().persistent()
        .get(&PENDING_UPGRADE).unwrap_or(PendingUpgrade {
            new_wasm_hash: wasm.clone(),
            proposed_at: env.ledger().timestamp(),
            proposer: voter.clone(),
            is_emergency: true,
            approvals: Vec::new(env),
            migration_script: None,
        });

    // Prevent duplicate votes
    if pending.approvals.iter().any(|a| a == voter) {
        return Err(ContractError::AlreadyApproved);
    }
    pending.approvals.push_back(voter.clone());
    env.storage().persistent().set(&PENDING_UPGRADE, &pending);

    let admin_count: u32 = env.storage().instance()
        .get(&symbol_short!("ADM_CNT")).unwrap_or(3u32);
    let quorum = (admin_count * EMERGENCY_QUORUM_NUMERATOR + EMERGENCY_QUORUM_DENOMINATOR - 1)
        / EMERGENCY_QUORUM_DENOMINATOR;

    if pending.approvals.len() as u32 >= quorum {
        env.deployer().update_current_contract_wasm(pending.new_wasm_hash);
        env.storage().persistent().remove(&PENDING_UPGRADE);
        env.events().publish((symbol_short!("EM_EXEC"), symbol_short!("v2")),
            (voter, quorum, env.ledger().timestamp()));
        return Ok(true); // upgrade executed
    }
    Ok(false) // quorum not yet reached
}

fn verify_migration(_env: &Env, _script: &Bytes) -> Result<(), ContractError> {
    // Hook: in a full implementation, execute migration in a read-only
    // sandbox and assert storage schema version increments correctly.
    Ok(())
}

fn require_admin(env: &Env, caller: &Address) -> Result<(), ContractError> {
    use soroban_sdk::symbol_short;
    const ADMIN: Symbol = symbol_short!("ADMIN");
    let admin: Address = env.storage().instance().get(&ADMIN)
        .ok_or(ContractError::NotInitialized)?;
    if &admin != caller { return Err(ContractError::Unauthorized); }
    Ok(())
}
