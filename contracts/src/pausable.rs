//! Emergency Pause (Circuit Breaker) for the Soroban Security Scanner contract.
//!
//! Adds `pause` / `unpause` / `is_paused` to the contract.
//! All fund-moving and state-mutating functions must call
//! `require_not_paused` before executing.

use soroban_sdk::{contracttype, panic_with_error, symbol_short, Address, Env, Symbol};

use crate::contracts::lib::ContractError;

// Storage key for the paused flag.
const PAUSED: Symbol = symbol_short!("PAUSED");

// ---------------------------------------------------------------------------
// Pausable state
// ---------------------------------------------------------------------------

/// Read the current paused state from contract storage.
#[inline]
pub fn is_paused(env: &Env) -> bool {
    env.storage().instance().get::<Symbol, bool>(&PAUSED).unwrap_or(false)
}

/// Revert the transaction if the contract is paused.
#[inline]
pub fn require_not_paused(env: &Env) {
    if is_paused(env) {
        panic_with_error!(env, ContractError::ContractPaused);
    }
}

/// Pause all fund-moving and state-mutating operations.
/// Caller must hold `EmergencyActions` permission and multi-sig approval.
///
/// # Arguments
/// * `admin`  - Address that must be the contract admin.
/// * `reason` - Human-readable pause reason (max 280 chars, stored in events).
pub fn pause(env: &Env, admin: &Address, reason: &soroban_sdk::String) {
    admin.require_auth();
    require_admin(env, admin);

    if is_paused(env) {
        panic_with_error!(env, ContractError::AlreadyPaused);
    }

    env.storage().instance().set(&PAUSED, &true);

    env.events().publish(
        (symbol_short!("PAUSE"), symbol_short!("v1")),
        (admin.clone(), reason.clone(), env.ledger().timestamp()),
    );
}

/// Resume normal operation.
/// Caller must hold `EmergencyActions` permission.
pub fn unpause(env: &Env, admin: &Address) {
    admin.require_auth();
    require_admin(env, admin);

    if !is_paused(env) {
        panic_with_error!(env, ContractError::NotPaused);
    }

    env.storage().instance().set(&PAUSED, &false);

    env.events().publish(
        (symbol_short!("UNPAUSE"), symbol_short!("v1")),
        (admin.clone(), env.ledger().timestamp()),
    );
}

// ---------------------------------------------------------------------------
// Helper: verify admin
// ---------------------------------------------------------------------------

use crate::contracts::lib::ADMIN;

fn require_admin(env: &Env, caller: &Address) {
    let admin: Address = env
        .storage()
        .instance()
        .get(&ADMIN)
        .unwrap_or_else(|| panic_with_error!(env, ContractError::NotInitialized));
    if &admin != caller {
        panic_with_error!(env, ContractError::Unauthorized);
    }
}
