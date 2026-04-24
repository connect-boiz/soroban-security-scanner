use soroban_sdk::{Address, BytesN, Vec, Symbol, Env};
use crate::{SorobanSecurityScanner, ContractError, Escrow, Config, BatchOperation};

#[test]
fn test_initialize() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let config = Config {
        max_batch_size: 50,
        max_escrows_per_address: 100,
        storage_quota: 10000,
        cleanup_threshold: 86400, // 1 day
    };
    
    assert_eq!(SorobanSecurityScanner::initialize(&env, admin.clone(), config.clone()), Ok(()));
    
    // Test duplicate initialization
    assert_eq!(
        SorobanSecurityScanner::initialize(&env, admin.clone(), config),
        Err(ContractError::AlreadyExists)
    );
}

#[test]
fn test_create_escrow() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let config = Config {
        max_batch_size: 50,
        max_escrows_per_address: 100,
        storage_quota: 10000,
        cleanup_threshold: 86400,
    };
    
    // Initialize
    SorobanSecurityScanner::initialize(&env, admin.clone(), config).unwrap();
    
    // Create escrow
    let conditions = Vec::new(&env);
    let escrow_id = SorobanSecurityScanner::create_escrow(
        &env,
        depositor.clone(),
        recipient.clone(),
        1000,
        env.ledger().timestamp() + 3600, // 1 hour from now
        conditions,
    ).unwrap();
    
    // Verify escrow was created
    let escrow = SorobanSecurityScanner::get_escrow(&env, escrow_id).unwrap();
    assert_eq!(escrow.depositor, depositor);
    assert_eq!(escrow.recipient, recipient);
    assert_eq!(escrow.amount, 1000);
    assert!(!escrow.is_completed);
    assert!(!escrow.is_verified);
}

#[test]
fn test_invalid_escrow_amount() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let config = Config {
        max_batch_size: 50,
        max_escrows_per_address: 100,
        storage_quota: 10000,
        cleanup_threshold: 86400,
    };
    
    // Initialize
    SorobanSecurityScanner::initialize(&env, admin, config).unwrap();
    
    // Try to create escrow with invalid amount
    let conditions = Vec::new(&env);
    let result = SorobanSecurityScanner::create_escrow(
        &env,
        depositor,
        recipient,
        0, // Invalid amount
        env.ledger().timestamp() + 3600,
        conditions,
    );
    
    assert_eq!(result, Err(ContractError::InvalidEscrowAmount));
}

#[test]
fn test_verify_escrow() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let config = Config {
        max_batch_size: 50,
        max_escrows_per_address: 100,
        storage_quota: 10000,
        cleanup_threshold: 86400,
    };
    
    // Initialize
    SorobanSecurityScanner::initialize(&env, admin.clone(), config).unwrap();
    
    // Create escrow
    let conditions = Vec::new(&env);
    let escrow_id = SorobanSecurityScanner::create_escrow(
        &env,
        depositor.clone(),
        recipient.clone(),
        1000,
        env.ledger().timestamp() + 3600,
        conditions,
    ).unwrap();
    
    // Verify escrow as admin
    assert_eq!(SorobanSecurityScanner::verify_escrow(&env, escrow_id.clone(), admin.clone()), Ok(()));
    
    // Check escrow is verified
    let escrow = SorobanSecurityScanner::get_escrow(&env, escrow_id).unwrap();
    assert!(escrow.is_verified);
    
    // Try to verify again
    assert_eq!(
        SorobanSecurityScanner::verify_escrow(&env, escrow_id, admin),
        Err(ContractError::AlreadyExists)
    );
}

#[test]
fn test_unauthorized_verify() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let unauthorized = Address::generate(&env);
    let config = Config {
        max_batch_size: 50,
        max_escrows_per_address: 100,
        storage_quota: 10000,
        cleanup_threshold: 86400,
    };
    
    // Initialize
    SorobanSecurityScanner::initialize(&env, admin, config).unwrap();
    
    // Create escrow
    let conditions = Vec::new(&env);
    let escrow_id = SorobanSecurityScanner::create_escrow(
        &env,
        depositor,
        recipient,
        1000,
        env.ledger().timestamp() + 3600,
        conditions,
    ).unwrap();
    
    // Try to verify with unauthorized user
    assert_eq!(
        SorobanSecurityScanner::verify_escrow(&env, escrow_id, unauthorized),
        Err(ContractError::Unauthorized)
    );
}

#[test]
fn test_release_escrow() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let config = Config {
        max_batch_size: 50,
        max_escrows_per_address: 100,
        storage_quota: 10000,
        cleanup_threshold: 86400,
    };
    
    // Initialize
    SorobanSecurityScanner::initialize(&env, admin.clone(), config).unwrap();
    
    // Create escrow
    let conditions = Vec::new(&env);
    let escrow_id = SorobanSecurityScanner::create_escrow(
        &env,
        depositor.clone(),
        recipient.clone(),
        1000,
        env.ledger().timestamp() + 3600,
        conditions,
    ).unwrap();
    
    // Verify escrow
    SorobanSecurityScanner::verify_escrow(&env, escrow_id.clone(), admin.clone()).unwrap();
    
    // Release escrow
    assert_eq!(SorobanSecurityScanner::release_escrow(&env, escrow_id.clone(), depositor.clone()), Ok(()));
    
    // Check escrow is completed
    let escrow = SorobanSecurityScanner::get_escrow(&env, escrow_id).unwrap();
    assert!(escrow.is_completed);
    
    // Try to release again
    assert_eq!(
        SorobanSecurityScanner::release_escrow(&env, escrow_id, depositor),
        Err(ContractError::EscrowAlreadyCompleted)
    );
}

#[test]
fn test_batch_verify_escrows() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let config = Config {
        max_batch_size: 50,
        max_escrows_per_address: 100,
        storage_quota: 10000,
        cleanup_threshold: 86400,
    };
    
    // Initialize
    SorobanSecurityScanner::initialize(&env, admin.clone(), config).unwrap();
    
    // Create multiple escrows
    let mut escrow_ids = Vec::new(&env);
    let conditions = Vec::new(&env);
    
    for i in 0..3 {
        let escrow_id = SorobanSecurityScanner::create_escrow(
            &env,
            depositor.clone(),
            recipient.clone(),
            1000 + i as i128,
            env.ledger().timestamp() + 3600,
            conditions.clone(),
        ).unwrap();
        escrow_ids.push_back(escrow_id);
    }
    
    // Batch verify
    let batch_id = SorobanSecurityScanner::batch_verify_escrows(&env, escrow_ids.clone(), admin.clone()).unwrap();
    
    // Check batch operation
    let batch_op = SorobanSecurityScanner::get_batch_operation(&env, batch_id).unwrap();
    assert_eq!(batch_op.operation_type, Symbol::short("VERIFY"));
    assert_eq!(batch_op.escrow_ids, escrow_ids);
    assert_eq!(batch_op.status, Symbol::short("COMPLETED"));
    
    // Verify all escrows are marked as verified
    for escrow_id in escrow_ids.iter() {
        let escrow = SorobanSecurityScanner::get_escrow(&env, escrow_id).unwrap();
        assert!(escrow.is_verified);
    }
}

#[test]
fn test_batch_release_escrows() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let config = Config {
        max_batch_size: 50,
        max_escrows_per_address: 100,
        storage_quota: 10000,
        cleanup_threshold: 86400,
    };
    
    // Initialize
    SorobanSecurityScanner::initialize(&env, admin.clone(), config).unwrap();
    
    // Create multiple escrows
    let mut escrow_ids = Vec::new(&env);
    let conditions = Vec::new(&env);
    
    for i in 0..3 {
        let escrow_id = SorobanSecurityScanner::create_escrow(
            &env,
            depositor.clone(),
            recipient.clone(),
            1000 + i as i128,
            env.ledger().timestamp() + 3600,
            conditions.clone(),
        ).unwrap();
        escrow_ids.push_back(escrow_id);
    }
    
    // Verify all escrows first
    SorobanSecurityScanner::batch_verify_escrows(&env, escrow_ids.clone(), admin.clone()).unwrap();
    
    // Batch release
    let batch_id = SorobanSecurityScanner::batch_release_escrows(&env, escrow_ids.clone(), depositor.clone()).unwrap();
    
    // Check batch operation
    let batch_op = SorobanSecurityScanner::get_batch_operation(&env, batch_id).unwrap();
    assert_eq!(batch_op.operation_type, Symbol::short("RELEASE"));
    assert_eq!(batch_op.escrow_ids, escrow_ids);
    assert_eq!(batch_op.status, Symbol::short("COMPLETED"));
    
    // Verify all escrows are marked as completed
    for escrow_id in escrow_ids.iter() {
        let escrow = SorobanSecurityScanner::get_escrow(&env, escrow_id).unwrap();
        assert!(escrow.is_completed);
    }
}

#[test]
fn test_batch_size_exceeded() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let config = Config {
        max_batch_size: 2, // Small batch size for testing
        max_escrows_per_address: 100,
        storage_quota: 10000,
        cleanup_threshold: 86400,
    };
    
    // Initialize
    SorobanSecurityScanner::initialize(&env, admin.clone(), config).unwrap();
    
    // Create more escrows than batch size
    let mut escrow_ids = Vec::new(&env);
    let conditions = Vec::new(&env);
    
    for i in 0..3 {
        let escrow_id = SorobanSecurityScanner::create_escrow(
            &env,
            depositor.clone(),
            recipient.clone(),
            1000 + i as i128,
            env.ledger().timestamp() + 3600,
            conditions.clone(),
        ).unwrap();
        escrow_ids.push_back(escrow_id);
    }
    
    // Try batch verify with too many escrows
    assert_eq!(
        SorobanSecurityScanner::batch_verify_escrows(&env, escrow_ids, admin),
        Err(ContractError::BatchSizeExceeded)
    );
}

#[test]
fn test_storage_limit_exceeded() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let config = Config {
        max_batch_size: 50,
        max_escrows_per_address: 2, // Small limit for testing
        storage_quota: 10000,
        cleanup_threshold: 86400,
    };
    
    // Initialize
    SorobanSecurityScanner::initialize(&env, admin.clone(), config).unwrap();
    
    // Create escrows up to the limit
    let conditions = Vec::new(&env);
    
    for i in 0..2 {
        SorobanSecurityScanner::create_escrow(
            &env,
            depositor.clone(),
            recipient.clone(),
            1000 + i as i128,
            env.ledger().timestamp() + 3600,
            conditions.clone(),
        ).unwrap();
    }
    
    // Try to create one more escrow (should fail)
    assert_eq!(
        SorobanSecurityScanner::create_escrow(
            &env,
            depositor,
            recipient,
            1002,
            env.ledger().timestamp() + 3600,
            conditions,
        ),
        Err(ContractError::StorageLimitExceeded)
    );
}
