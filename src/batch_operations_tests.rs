//! Tests for Batch Operations Module

use soroban_sdk::{Address, Env, Symbol, Vec, i128, u64};
use crate::batch_operations::{
    BatchOperations, BatchOperationStatus, BatchEscrowReleaseRequest, 
    BatchVerificationRequest, BatchOperationResult, BatchOperationSummary
};
use crate::contracts::lib::{EscrowEntry, VulnerabilityReport, ContractError};

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as TestAddress, Ledger as TestLedger};

    #[test]
    fn test_batch_operations_initialization() {
        let env = Env::default();
        let admin = Address::generate(&env);
        
        // Initialize batch operations
        BatchOperations::initialize(env.clone());
        
        // Verify initialization
        let batch_counter: u64 = env.storage().instance().get(&Symbol::short("BATCH_CNT")).unwrap();
        assert_eq!(batch_counter, 0);
    }

    #[test]
    fn test_create_batch_escrow_release() {
        let env = Env::default();
        let requester = Address::generate(&env);
        
        // Initialize
        BatchOperations::initialize(env.clone());
        
        // Create escrow IDs for batch
        let mut escrow_ids = Vec::new(&env);
        for i in 1..=5 {
            escrow_ids.push_back(i);
        }
        
        // Create batch escrow release
        let batch_id = BatchOperations::create_batch_escrow_release(
            env.clone(),
            escrow_ids.clone(),
            requester.clone(),
        );
        
        // Verify batch was created
        assert_eq!(batch_id, 1);
        
        // Get batch summary
        let summary = BatchOperations::get_batch_summary(env.clone(), batch_id);
        assert_eq!(summary.batch_id, batch_id);
        assert_eq!(summary.total_items, 5);
        assert_eq!(summary.successful_items, 0);
        assert_eq!(summary.failed_items, 0);
        assert!(matches!(summary.status, BatchOperationStatus::Pending));
    }

    #[test]
    fn test_create_batch_verification() {
        let env = Env::default();
        let verifier = Address::generate(&env);
        
        // Initialize
        BatchOperations::initialize(env.clone());
        
        // Create vulnerability IDs for batch
        let mut vuln_ids = Vec::new(&env);
        for i in 1..=3 {
            vuln_ids.push_back(i);
        }
        
        // Create batch verification
        let batch_id = BatchOperations::create_batch_verification(
            env.clone(),
            vuln_ids.clone(),
            verifier.clone(),
        );
        
        // Verify batch was created
        assert_eq!(batch_id, 1);
        
        // Get batch summary
        let summary = BatchOperations::get_batch_summary(env.clone(), batch_id);
        assert_eq!(summary.batch_id, batch_id);
        assert_eq!(summary.total_items, 3);
        assert_eq!(summary.successful_items, 0);
        assert_eq!(summary.failed_items, 0);
        assert!(matches!(summary.status, BatchOperationStatus::Pending));
    }

    #[test]
    fn test_batch_size_validation() {
        let env = Env::default();
        let requester = Address::generate(&env);
        
        // Initialize
        BatchOperations::initialize(env.clone());
        
        // Create escrow IDs exceeding batch limit
        let mut escrow_ids = Vec::new(&env);
        for i in 1..=101 { // Exceeds limit of 100
            escrow_ids.push_back(i);
        }
        
        // Should panic due to batch size limit
        let result = std::panic::catch_unwind(|| {
            BatchOperations::create_batch_escrow_release(
                env.clone(),
                escrow_ids,
                requester,
            )
        });
        
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_batch_validation() {
        let env = Env::default();
        let requester = Address::generate(&env);
        
        // Initialize
        BatchOperations::initialize(env.clone());
        
        // Create empty escrow IDs
        let escrow_ids = Vec::new(&env);
        
        // Should panic due to empty batch
        let result = std::panic::catch_unwind(|| {
            BatchOperations::create_batch_escrow_release(
                env.clone(),
                escrow_ids,
                requester,
            )
        });
        
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_batch_escrow_release_success() {
        let env = Env::default();
        let requester = Address::generate(&env);
        let executor = Address::generate(&env);
        
        // Initialize
        BatchOperations::initialize(env.clone());
        
        // Create mock escrow entries
        let mut escrow_ids = Vec::new(&env);
        for i in 1..=3 {
            escrow_ids.push_back(i);
            
            // Create mock escrow entry
            let escrow = EscrowEntry {
                id: i,
                depositor: requester.clone(),
                beneficiary: Address::generate(&env),
                amount: 1000i128,
                token: Address::generate(&env),
                purpose: "bounty".to_string(),
                status: "locked".to_string(),
                created_at: env.ledger().timestamp(),
                lock_until: env.ledger().timestamp() - 1000, // Expired
                conditions_met: true,
                release_signature: None,
            };
            
            let escrow_key = Symbol::short(&format!("ESCROW_{}", i));
            env.storage().instance().set(&escrow_key, &escrow);
        }
        
        // Create batch
        let batch_id = BatchOperations::create_batch_escrow_release(
            env.clone(),
            escrow_ids.clone(),
            requester.clone(),
        );
        
        // Execute batch
        let summary = BatchOperations::execute_batch_escrow_release(
            env.clone(),
            batch_id,
            executor.clone(),
        );
        
        // Verify results
        assert_eq!(summary.batch_id, batch_id);
        assert_eq!(summary.total_items, 3);
        assert_eq!(summary.successful_items, 3);
        assert_eq!(summary.failed_items, 0);
        assert!(matches!(summary.status, BatchOperationStatus::Completed));
        assert_eq!(summary.results.len(), 3);
        
        // Verify all operations succeeded
        for result in summary.results.iter() {
            assert!(result.success);
            assert!(result.error_message.is_none());
        }
    }

    #[test]
    fn test_execute_batch_verification_partial_success() {
        let env = Env::default();
        let verifier = Address::generate(&env);
        let executor = Address::generate(&env);
        
        // Initialize
        BatchOperations::initialize(env.clone());
        
        // Create mock vulnerability entries (only some exist)
        let mut vuln_ids = Vec::new(&env);
        for i in 1..=3 {
            vuln_ids.push_back(i);
            
            if i <= 2 { // Only create first 2 vulnerabilities
                let vulnerability = VulnerabilityReport {
                    reporter: Address::generate(&env),
                    contract_id: [0u8; 32].into(),
                    vulnerability_type: "reentrancy".to_string(),
                    severity: "high".to_string(),
                    description: "Test vulnerability".to_string(),
                    location: "function test()".to_string(),
                    timestamp: env.ledger().timestamp(),
                    status: "pending".to_string(),
                    bounty_amount: 1000i128,
                };
                
                let vuln_key = Symbol::short(&i.to_string());
                env.storage().instance().set(&vuln_key, &vulnerability);
            }
        }
        
        // Create batch
        let batch_id = BatchOperations::create_batch_verification(
            env.clone(),
            vuln_ids.clone(),
            verifier.clone(),
        );
        
        // Execute batch
        let summary = BatchOperations::execute_batch_verification(
            env.clone(),
            batch_id,
            executor.clone(),
        );
        
        // Verify partial success
        assert_eq!(summary.batch_id, batch_id);
        assert_eq!(summary.total_items, 3);
        assert_eq!(summary.successful_items, 2);
        assert_eq!(summary.failed_items, 1);
        assert!(matches!(summary.status, BatchOperationStatus::PartiallyCompleted));
        assert_eq!(summary.results.len(), 3);
        
        // Verify operation results
        let mut success_count = 0;
        let mut failure_count = 0;
        
        for result in summary.results.iter() {
            if result.success {
                success_count += 1;
                assert!(result.error_message.is_none());
            } else {
                failure_count += 1;
                assert!(result.error_message.is_some());
            }
        }
        
        assert_eq!(success_count, 2);
        assert_eq!(failure_count, 1);
    }

    #[test]
    fn test_get_user_batches() {
        let env = Env::default();
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        
        // Initialize
        BatchOperations::initialize(env.clone());
        
        // Create batches for user1
        let mut escrow_ids1 = Vec::new(&env);
        escrow_ids1.push_back(1);
        escrow_ids1.push_back(2);
        
        let batch1 = BatchOperations::create_batch_escrow_release(
            env.clone(),
            escrow_ids1,
            user1.clone(),
        );
        
        let mut vuln_ids1 = Vec::new(&env);
        vuln_ids1.push_back(10);
        
        let batch2 = BatchOperations::create_batch_verification(
            env.clone(),
            vuln_ids1,
            user1.clone(),
        );
        
        // Create batch for user2
        let mut escrow_ids2 = Vec::new(&env);
        escrow_ids2.push_back(3);
        
        let batch3 = BatchOperations::create_batch_escrow_release(
            env.clone(),
            escrow_ids2,
            user2.clone(),
        );
        
        // Get user1's batches
        let user1_batches = BatchOperations::get_user_batches(env.clone(), user1);
        assert_eq!(user1_batches.len(), 2);
        assert!(user1_batches.contains(&batch1));
        assert!(user1_batches.contains(&batch2));
        
        // Get user2's batches
        let user2_batches = BatchOperations::get_user_batches(env.clone(), user2);
        assert_eq!(user2_batches.len(), 1);
        assert!(user2_batches.contains(&batch3));
    }

    #[test]
    fn test_batch_execution_idempotency() {
        let env = Env::default();
        let requester = Address::generate(&env);
        let executor = Address::generate(&env);
        
        // Initialize
        BatchOperations::initialize(env.clone());
        
        // Create mock escrow entries
        let mut escrow_ids = Vec::new(&env);
        escrow_ids.push_back(1);
        
        let escrow = EscrowEntry {
            id: 1,
            depositor: requester.clone(),
            beneficiary: Address::generate(&env),
            amount: 1000i128,
            token: Address::generate(&env),
            purpose: "bounty".to_string(),
            status: "locked".to_string(),
            created_at: env.ledger().timestamp(),
            lock_until: env.ledger().timestamp() - 1000,
            conditions_met: true,
            release_signature: None,
        };
        
        let escrow_key = Symbol::short(&format!("ESCROW_{}", 1));
        env.storage().instance().set(&escrow_key, &escrow);
        
        // Create batch
        let batch_id = BatchOperations::create_batch_escrow_release(
            env.clone(),
            escrow_ids.clone(),
            requester.clone(),
        );
        
        // Execute batch twice
        let summary1 = BatchOperations::execute_batch_escrow_release(
            env.clone(),
            batch_id,
            executor.clone(),
        );
        
        let summary2 = BatchOperations::execute_batch_escrow_release(
            env.clone(),
            batch_id,
            executor.clone(),
        );
        
        // Both executions should return the same result
        assert_eq!(summary1.batch_id, summary2.batch_id);
        assert_eq!(summary1.total_items, summary2.total_items);
        assert_eq!(summary1.successful_items, summary2.successful_items);
        assert_eq!(summary1.failed_items, summary2.failed_items);
        assert!(matches!(summary1.status, BatchOperationStatus::Completed));
        assert!(matches!(summary2.status, BatchOperationStatus::Completed));
    }

    #[test]
    fn test_gas_tracking() {
        let env = Env::default();
        let requester = Address::generate(&env);
        let executor = Address::generate(&env);
        
        // Initialize
        BatchOperations::initialize(env.clone());
        
        // Create mock escrow entries
        let mut escrow_ids = Vec::new(&env);
        for i in 1..=2 {
            escrow_ids.push_back(i);
            
            let escrow = EscrowEntry {
                id: i,
                depositor: requester.clone(),
                beneficiary: Address::generate(&env),
                amount: 1000i128,
                token: Address::generate(&env),
                purpose: "bounty".to_string(),
                status: "locked".to_string(),
                created_at: env.ledger().timestamp(),
                lock_until: env.ledger().timestamp() - 1000,
                conditions_met: true,
                release_signature: None,
            };
            
            let escrow_key = Symbol::short(&format!("ESCROW_{}", i));
            env.storage().instance().set(&escrow_key, &escrow);
        }
        
        // Create batch
        let batch_id = BatchOperations::create_batch_escrow_release(
            env.clone(),
            escrow_ids.clone(),
            requester.clone(),
        );
        
        // Execute batch
        let summary = BatchOperations::execute_batch_escrow_release(
            env.clone(),
            batch_id,
            executor.clone(),
        );
        
        // Verify gas tracking
        assert!(summary.total_gas_used > 0);
        assert_eq!(summary.results.len(), 2);
        
        for result in summary.results.iter() {
            assert!(result.gas_used > 0);
        }
    }
}
