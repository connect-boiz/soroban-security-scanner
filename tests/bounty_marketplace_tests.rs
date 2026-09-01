//! Comprehensive tests for the Security Bounty Marketplace smart contract

use soroban_sdk::{Address, Env, Symbol, contracterror};
use stellar_security_scanner::bounty_marketplace::{
    BountyMarketplace, Bounty, BountyStatus, Severity, MultiSigApproval
};

#[test]
fn test_contract_initialization() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyMarketplace);
    let client = BountyMarketplaceClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let owner = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin, &owner);

    // Verify initialization
    assert_eq!(client.get_admin(), admin);
    assert_eq!(client.get_owner(), owner);
}

#[test]
fn test_create_bounty() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyMarketplace);
    let client = BountyMarketplaceClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let creator = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin, &owner);

    // Create bounty
    let amount = 1000i128;
    let title = Symbol::new(&env, "Critical Bug in Token Contract");
    let description = Symbol::new(&env, "Critical vulnerability found in token contract");
    let severity = Severity::Critical;

    let bounty_id = client.create_bounty(&creator, &amount, &title, &description, &severity);

    // Verify bounty creation
    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.id, bounty_id);
    assert_eq!(bounty.creator, creator);
    assert_eq!(bounty.amount, amount);
    assert_eq!(bounty.title, title);
    assert_eq!(bounty.description, description);
    assert_eq!(bounty.severity, severity);
    assert_eq!(bounty.status, BountyStatus::Timelocked);
}

#[test]
fn test_create_bounty_validation() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyMarketplace);
    let client = BountyMarketplaceClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let creator = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin, &owner);

    // Test negative amount
    let result = env.try_invoke_contract::<u64>(
        &contract_id,
        &Symbol::new(&env, "create_bounty"),
        (creator.clone(), -100i128, Symbol::new(&env, "Test"), Symbol::new(&env, "Test"), Severity::Critical)
    );
    assert!(result.is_err());

    // Test empty title
    let result = env.try_invoke_contract::<u64>(
        &contract_id,
        &Symbol::new(&env, "create_bounty"),
        (creator.clone(), 100i128, Symbol::new(&env, ""), Symbol::new(&env, "Test"), Severity::Critical)
    );
    assert!(result.is_err());

    // Test empty description
    let result = env.try_invoke_contract::<u64>(
        &contract_id,
        &Symbol::new(&env, "create_bounty"),
        (creator, 100i128, Symbol::new(&env, "Test"), Symbol::new(&env, ""), Severity::Critical)
    );
    assert!(result.is_err());
}

#[test]
fn test_timelock_mechanism() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyMarketplace);
    let client = BountyMarketplaceClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let creator = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin, &owner);

    // Create bounty
    let bounty_id = client.create_bounty(
        &creator,
        &1000i128,
        &Symbol::new(&env, "Test Bounty"),
        &Symbol::new(&env, "Test Description"),
        &Severity::Critical
    );

    // Check initial status (should be Timelocked)
    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.status, BountyStatus::Timelocked);

    // Advance time beyond timelock period (7 days)
    env.ledger().set_timestamp(env.ledger().timestamp() + 7 * 24 * 60 * 60 + 1);

    // Check timelock
    let timelock_passed = client.check_timelock(&bounty_id);
    assert!(timelock_passed);

    // Verify status changed to Active
    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.status, BountyStatus::Active);
}

#[test]
fn test_multi_sig_approval() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyMarketplace);
    let client = BountyMarketplaceClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let creator = Address::generate(&env);
    let researcher = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin, &owner);

    // Create bounty and advance time to activate
    let bounty_id = client.create_bounty(
        &creator,
        &1000i128,
        &Symbol::new(&env, "Test Bounty"),
        &Symbol::new(&env, "Test Description"),
        &Severity::Critical
    );

    // Advance time beyond timelock
    env.ledger().set_timestamp(env.ledger().timestamp() + 7 * 24 * 60 * 60 + 1);
    client.check_timelock(&bounty_id);

    // Assign researcher
    client.assign_researcher(&bounty_id, &researcher);

    // Test admin approval
    client.admin_approve(&admin, &bounty_id);

    // Bounty should still not be fully approved
    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.status, BountyStatus::Active);

    // Test owner approval
    client.owner_approve(&owner, &bounty_id);

    // Now bounty should be fully approved
    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.status, BountyStatus::Approved);
}

#[test]
fn test_partial_rewards() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyMarketplace);
    let client = BountyMarketplaceClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let creator = Address::generate(&env);
    let researcher = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin, &owner);

    // Test Medium severity bounty (60% reward)
    let medium_bounty_id = client.create_bounty(
        &creator,
        &1000i128,
        &Symbol::new(&env, "Medium Bounty"),
        &Symbol::new(&env, "Medium severity issue"),
        &Severity::Medium
    );

    // Advance time and activate
    env.ledger().set_timestamp(env.ledger().timestamp() + 7 * 24 * 60 * 60 + 1);
    client.check_timelock(&medium_bounty_id);
    client.assign_researcher(&medium_bounty_id, &researcher);
    client.admin_approve(&admin, &medium_bounty_id);
    client.owner_approve(&owner, &medium_bounty_id);

    // Test Low severity bounty (30% reward)
    let low_bounty_id = client.create_bounty(
        &creator,
        &1000i128,
        &Symbol::new(&env, "Low Bounty"),
        &Symbol::new(&env, "Low severity issue"),
        &Severity::Low
    );

    // Advance time and activate
    env.ledger().set_timestamp(env.ledger().timestamp() + 7 * 24 * 60 * 60 + 1);
    client.check_timelock(&low_bounty_id);
    client.assign_researcher(&low_bounty_id, &researcher);
    client.admin_approve(&admin, &low_bounty_id);
    client.owner_approve(&owner, &low_bounty_id);

    // Verify reward calculations
    assert_eq!(Severity::Medium.reward_percentage(), 60);
    assert_eq!(Severity::Low.reward_percentage(), 30);
}

#[test]
fn test_researcher_assignments() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyMarketplace);
    let client = BountyMarketplaceClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let creator = Address::generate(&env);
    let researcher1 = Address::generate(&env);
    let researcher2 = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin, &owner);

    // Create multiple bounties
    let bounty1_id = client.create_bounty(
        &creator,
        &1000i128,
        &Symbol::new(&env, "Bounty 1"),
        &Symbol::new(&env, "First bounty"),
        &Severity::Critical
    );

    let bounty2_id = client.create_bounty(
        &creator,
        &1500i128,
        &Symbol::new(&env, "Bounty 2"),
        &Symbol::new(&env, "Second bounty"),
        &Severity::High
    );

    // Advance time and activate
    env.ledger().set_timestamp(env.ledger().timestamp() + 7 * 24 * 60 * 60 + 1);
    client.check_timelock(&bounty1_id);
    client.check_timelock(&bounty2_id);

    // Assign researchers
    client.assign_researcher(&bounty1_id, &researcher1);
    client.assign_researcher(&bounty2_id, &researcher2);

    // Verify assignments
    let researcher1_bounties = client.get_researcher_bounties(&researcher1);
    assert_eq!(researcher1_bounties.len(), 1);
    assert_eq!(researcher1_bounties.get(0).unwrap(), bounty1_id);

    let researcher2_bounties = client.get_researcher_bounties(&researcher2);
    assert_eq!(researcher2_bounties.len(), 1);
    assert_eq!(researcher2_bounties.get(0).unwrap(), bounty2_id);
}

#[test]
fn test_claim_reward() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyMarketplace);
    let client = BountyMarketplaceClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let creator = Address::generate(&env);
    let researcher = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin, &owner);

    // Create bounty
    let bounty_id = client.create_bounty(
        &creator,
        &1000i128,
        &Symbol::new(&env, "Test Bounty"),
        &Symbol::new(&env, "Test Description"),
        &Severity::Critical
    );

    // Advance time and activate
    env.ledger().set_timestamp(env.ledger().timestamp() + 7 * 24 * 60 * 60 + 1);
    client.check_timelock(&bounty_id);

    // Assign researcher and get approvals
    client.assign_researcher(&bounty_id, &researcher);
    client.admin_approve(&admin, &bounty_id);
    client.owner_approve(&owner, &bounty_id);

    // Claim reward
    client.claim_reward(&bounty_id, &researcher);

    // Verify bounty is completed
    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.status, BountyStatus::Completed);
}

#[test]
fn test_withdraw_function() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyMarketplace);
    let client = BountyMarketplaceClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let creator = Address::generate(&env);
    let researcher = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin, &owner);

    // Create multiple approved bounties
    let bounty1_id = client.create_bounty(
        &creator,
        &1000i128,
        &Symbol::new(&env, "Bounty 1"),
        &Symbol::new(&env, "Critical issue"),
        &Severity::Critical
    );

    let bounty2_id = client.create_bounty(
        &creator,
        &1000i128,
        &Symbol::new(&env, "Bounty 2"),
        &Symbol::new(&env, "Medium issue"),
        &Severity::Medium
    );

    // Advance time and activate
    env.ledger().set_timestamp(env.ledger().timestamp() + 7 * 24 * 60 * 60 + 1);
    client.check_timelock(&bounty1_id);
    client.check_timelock(&bounty2_id);

    // Assign researcher and get approvals
    client.assign_researcher(&bounty1_id, &researcher);
    client.assign_researcher(&bounty2_id, &researcher);
    client.admin_approve(&admin, &bounty1_id);
    client.owner_approve(&owner, &bounty1_id);
    client.admin_approve(&admin, &bounty2_id);
    client.owner_approve(&owner, &bounty2_id);

    // Test withdrawal (should succeed with available rewards)
    client.withdraw(&researcher, &1300i128); // 1000 (critical) + 600 (medium) = 1600 available

    // Test withdrawal with insufficient funds
    let result = env.try_invoke_contract::<()>(
        &contract_id,
        &Symbol::new(&env, "withdraw"),
        (researcher, 2000i128) // More than available
    );
    assert!(result.is_err());
}

#[test]
fn test_access_control() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyMarketplace);
    let client = BountyMarketplaceClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    let creator = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin, &owner);

    // Create bounty
    let bounty_id = client.create_bounty(
        &creator,
        &1000i128,
        &Symbol::new(&env, "Test Bounty"),
        &Symbol::new(&env, "Test Description"),
        &Severity::Critical
    );

    // Test unauthorized admin approval
    let result = env.try_invoke_contract::<()>(
        &contract_id,
        &Symbol::new(&env, "admin_approve"),
        (unauthorized_user, bounty_id)
    );
    assert!(result.is_err());

    // Test unauthorized owner approval
    let result = env.try_invoke_contract::<()>(
        &contract_id,
        &Symbol::new(&env, "owner_approve"),
        (unauthorized_user, bounty_id)
    );
    assert!(result.is_err());
}

#[test]
fn test_edge_cases() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyMarketplace);
    let client = BountyMarketplaceClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let owner = Address::generate(&env);

    // Test double initialization
    client.initialize(&admin, &owner);
    let result = env.try_invoke_contract::<()>(
        &contract_id,
        &Symbol::new(&env, "initialize"),
        (admin, owner)
    );
    assert!(result.is_err());

    // Test zero addresses
    let result = env.try_invoke_contract::<()>(
        &contract_id,
        &Symbol::new(&env, "initialize"),
        (Address::default(), owner)
    );
    assert!(result.is_err());
}
