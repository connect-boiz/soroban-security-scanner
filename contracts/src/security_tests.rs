#[cfg(test)]
mod security_tests {
    use soroban_sdk::{symbol_short, Address, BytesN, Env, String};
    use crate::{
        ContractError, SecurityScannerContract, Role, Permission,
        MultiSigProposal, ADMIN_ROLES, MULTI_SIG_PROPOSALS
    };

    #[test]
    fn test_role_based_access_control() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let verifier = Address::generate(&env);
        let escrow_manager = Address::generate(&env);
        let unauthorized_user = Address::generate(&env);
        
        // Initialize contract
        SecurityScannerContract::initialize(env.clone(), admin.clone()).unwrap();
        
        // Grant Verifier role through multi-sig process
        let proposal_id = SecurityScannerContract::propose_role_grant(
            env.clone(),
            admin.clone(),
            verifier.clone(),
            Role::Verifier,
            2, // required approvals
            0,  // no delay for test
        ).unwrap();
        
        // Approve proposal (admin approves)
        SecurityScannerContract::approve_role_grant(env.clone(), admin.clone(), proposal_id).unwrap();
        
        // Execute role grant
        SecurityScannerContract::execute_role_grant(env.clone(), admin.clone(), proposal_id).unwrap();
        
        // Verify role was granted
        let roles = SecurityScannerContract::get_user_roles(env.clone(), verifier.clone()).unwrap();
        assert!(roles.contains(&Role::Verifier));
        
        // Test that verifier can verify vulnerability but unauthorized user cannot
        let contract_id = BytesN::from_array(&env, &[1; 32]);
        let report_id = SecurityScannerContract::report_vulnerability(
            env.clone(),
            Address::generate(&env),
            contract_id,
            String::from_str(&env, "reentrancy"),
            String::from_str(&env, "high"),
            String::from_str(&env, "Test vulnerability"),
            String::from_str(&env, "function xyz"),
        ).unwrap();
        
        // Verifier should be able to verify (with small bounty)
        let result = SecurityScannerContract::verify_vulnerability(
            env.clone(),
            verifier.clone(),
            report_id,
            100000, // Small bounty, no multi-sig required
        );
        assert!(result.is_ok());
        
        // Unauthorized user should not be able to verify
        let result = SecurityScannerContract::verify_vulnerability(
            env.clone(),
            unauthorized_user.clone(),
            report_id,
            100000,
        );
        assert_eq!(result, Err(ContractError::InsufficientPermissions));
    }

    #[test]
    fn test_multi_signature_requirements() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let verifier1 = Address::generate(&env);
        let verifier2 = Address::generate(&env);
        
        // Initialize contract
        SecurityScannerContract::initialize(env.clone(), admin.clone()).unwrap();
        
        // Grant Verifier roles
        for verifier in [&verifier1, &verifier2] {
            let proposal_id = SecurityScannerContract::propose_role_grant(
                env.clone(),
                admin.clone(),
                verifier.clone(),
                Role::Verifier,
                2,
                0,
            ).unwrap();
            
            SecurityScannerContract::approve_role_grant(env.clone(), admin.clone(), proposal_id).unwrap();
            SecurityScannerContract::execute_role_grant(env.clone(), admin.clone(), proposal_id).unwrap();
        }
        
        // Create a vulnerability report
        let contract_id = BytesN::from_array(&env, &[2; 32]);
        let report_id = SecurityScannerContract::report_vulnerability(
            env.clone(),
            Address::generate(&env),
            contract_id,
            String::from_str(&env, "overflow"),
            String::from_str(&env, "critical"),
            String::from_str(&env, "Test overflow"),
            String::from_str(&env, "function abc"),
        ).unwrap();
        
        // High bounty should require multi-sig
        let result = SecurityScannerContract::verify_vulnerability(
            env.clone(),
            verifier1.clone(),
            report_id,
            2000000, // High bounty (> 1M)
        );
        assert_eq!(result, Err(ContractError::MultiSigRequired));
        
        // Create multi-sig proposal for high bounty
        let proposal_id = SecurityScannerContract::propose_high_bounty_verification(
            env.clone(),
            verifier1.clone(),
            report_id,
            2000000,
            2, // required approvals
            0,  // no delay for test
        ).unwrap();
        
        // Single approval should not be enough
        SecurityScannerContract::approve_bounty_verification(env.clone(), verifier1.clone(), proposal_id).unwrap();
        
        let can_execute = SecurityScannerContract::can_execute_proposal_check(env.clone(), proposal_id).unwrap();
        assert!(!can_execute); // Should not be executable yet
        
        // Second approval should make it executable
        SecurityScannerContract::approve_bounty_verification(env.clone(), verifier2.clone(), proposal_id).unwrap();
        
        let can_execute = SecurityScannerContract::can_execute_proposal_check(env.clone(), proposal_id).unwrap();
        assert!(can_execute); // Should be executable now
        
        // Execute the high bounty verification
        SecurityScannerContract::execute_high_bounty_verification(env.clone(), verifier1.clone(), proposal_id).unwrap();
        
        // Verify vulnerability was actually verified
        let report = SecurityScannerContract::get_vulnerability(env.clone(), report_id).unwrap();
        assert_eq!(report.status, String::from_str(&env, "verified"));
        assert_eq!(report.bounty_amount, 2000000);
    }

    #[test]
    fn test_emergency_verification_multi_sig() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let verifier1 = Address::generate(&env);
        let verifier2 = Address::generate(&env);
        let verifier3 = Address::generate(&env);
        
        // Initialize contract
        SecurityScannerContract::initialize(env.clone(), admin.clone()).unwrap();
        
        // Grant Verifier roles
        for verifier in [&verifier1, &verifier2, &verifier3] {
            let proposal_id = SecurityScannerContract::propose_role_grant(
                env.clone(),
                admin.clone(),
                verifier.clone(),
                Role::Verifier,
                2,
                0,
            ).unwrap();
            
            SecurityScannerContract::approve_role_grant(env.clone(), admin.clone(), proposal_id).unwrap();
            SecurityScannerContract::execute_role_grant(env.clone(), admin.clone(), proposal_id).unwrap();
        }
        
        // Report emergency vulnerability
        let contract_id = BytesN::from_array(&env, &[3; 32]);
        let alert_id = SecurityScannerContract::report_emergency_vulnerability(
            env.clone(),
            Address::generate(&env),
            contract_id,
            String::from_str(&env, "critical_vuln"),
            String::from_str(&env, "emergency"),
            String::from_str(&env, "Critical emergency vulnerability"),
            String::from_str(&env, "core contract"),
        ).unwrap();
        
        // Emergency verification should always require multi-sig
        let result = SecurityScannerContract::verify_emergency_vulnerability(
            env.clone(),
            verifier1.clone(),
            alert_id,
            true,
        );
        assert_eq!(result, Err(ContractError::MultiSigRequired));
        
        // Create emergency verification proposal
        let proposal_id = SecurityScannerContract::propose_emergency_verification(
            env.clone(),
            verifier1.clone(),
            alert_id,
            true,
            3, // minimum 3 approvals for emergency
            3600, // minimum 1 hour delay
        ).unwrap();
        
        // Get proposal details to verify requirements
        let proposal = SecurityScannerContract::get_proposal(env.clone(), proposal_id).unwrap();
        assert_eq!(proposal.required_approvals, 3); // Should enforce minimum
        assert_eq!(proposal.execution_delay, 3600); // Should enforce minimum
        
        // Approve with all 3 verifiers
        for verifier in [&verifier1, &verifier2, &verifier3] {
            SecurityScannerContract::approve_emergency_verification(env.clone(), verifier.clone(), proposal_id).unwrap();
        }
        
        // Should not be executable yet due to time delay
        let can_execute = SecurityScannerContract::can_execute_proposal_check(env.clone(), proposal_id).unwrap();
        assert!(!can_execute);
        
        // Fast forward time
        env.ledger().set_timestamp(env.ledger().timestamp() + 3600 + 1);
        
        // Now should be executable
        let can_execute = SecurityScannerContract::can_execute_proposal_check(env.clone(), proposal_id).unwrap();
        assert!(can_execute);
        
        // Execute emergency verification
        SecurityScannerContract::execute_emergency_verification(env.clone(), verifier1.clone(), proposal_id).unwrap();
        
        // Verify emergency alert was verified
        let alert = SecurityScannerContract::get_emergency_alert(env.clone(), alert_id).unwrap();
        assert_eq!(alert.status, String::from_str(&env, "verified"));
        assert_eq!(alert.emergency_reward, 10000000); // Emergency reward
    }

    #[test]
    fn test_escrow_manager_role() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let escrow_manager = Address::generate(&env);
        let unauthorized_user = Address::generate(&env);
        
        // Initialize contract
        SecurityScannerContract::initialize(env.clone(), admin.clone()).unwrap();
        
        // Grant EscrowManager role
        let proposal_id = SecurityScannerContract::propose_role_grant(
            env.clone(),
            admin.clone(),
            escrow_manager.clone(),
            Role::EscrowManager,
            2,
            0,
        ).unwrap();
        
        SecurityScannerContract::approve_role_grant(env.clone(), admin.clone(), proposal_id).unwrap();
        SecurityScannerContract::execute_role_grant(env.clone(), admin.clone(), proposal_id).unwrap();
        
        // Create escrow
        let escrow_id = SecurityScannerContract::create_escrow(
            env.clone(),
            Address::generate(&env),
            Address::generate(&env),
            1000000,
            String::from_str(&env, "bounty"),
            86400, // 1 day lock
        ).unwrap();
        
        // Escrow manager should be able to mark conditions as met
        let result = SecurityScannerContract::mark_escrow_conditions_met(
            env.clone(),
            escrow_id,
            escrow_manager.clone(),
        );
        assert!(result.is_ok());
        
        // Unauthorized user should not be able to mark conditions as met
        let result = SecurityScannerContract::mark_escrow_conditions_met(
            env.clone(),
            escrow_id,
            unauthorized_user.clone(),
        );
        assert_eq!(result, Err(ContractError::InsufficientPermissions));
    }

    #[test]
    fn test_treasury_manager_role() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let treasury_manager = Address::generate(&env);
        let unauthorized_user = Address::generate(&env);
        
        // Initialize contract
        SecurityScannerContract::initialize(env.clone(), admin.clone()).unwrap();
        
        // Grant TreasuryManager role
        let proposal_id = SecurityScannerContract::propose_role_grant(
            env.clone(),
            admin.clone(),
            treasury_manager.clone(),
            Role::TreasuryManager,
            2,
            0,
        ).unwrap();
        
        SecurityScannerContract::approve_role_grant(env.clone(), admin.clone(), proposal_id).unwrap();
        SecurityScannerContract::execute_role_grant(env.clone(), admin.clone(), proposal_id).unwrap();
        
        // Treasury manager should be able to fund emergency pool
        let result = SecurityScannerContract::fund_emergency_pool(
            env.clone(),
            treasury_manager.clone(),
            5000000,
        );
        assert!(result.is_ok());
        
        // Verify pool was funded
        let pool_balance = SecurityScannerContract::get_emergency_pool_balance(env.clone());
        assert_eq!(pool_balance, 5000000);
        
        // Unauthorized user should not be able to fund emergency pool
        let result = SecurityScannerContract::fund_emergency_pool(
            env.clone(),
            unauthorized_user.clone(),
            1000000,
        );
        assert_eq!(result, Err(ContractError::InsufficientPermissions));
    }

    #[test]
    fn test_role_management_security() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let unauthorized_user = Address::generate(&env);
        let new_user = Address::generate(&env);
        
        // Initialize contract
        SecurityScannerContract::initialize(env.clone(), admin.clone()).unwrap();
        
        // Direct role grant should require multi-sig
        let result = SecurityScannerContract::grant_role(
            env.clone(),
            admin.clone(),
            new_user.clone(),
            Role::Verifier,
        );
        assert_eq!(result, Err(ContractError::MultiSigRequired));
        
        // Unauthorized user should not be able to propose role grants
        let result = SecurityScannerContract::propose_role_grant(
            env.clone(),
            unauthorized_user.clone(),
            new_user.clone(),
            Role::Verifier,
            2,
            0,
        );
        assert_eq!(result, Err(ContractError::InsufficientPermissions));
        
        // Role management should have minimum delay and approval requirements
        let proposal_id = SecurityScannerContract::propose_role_grant(
            env.clone(),
            admin.clone(),
            new_user.clone(),
            Role::Verifier,
            1, // below minimum
            100, // below minimum
        ).unwrap();
        
        let proposal = SecurityScannerContract::get_proposal(env.clone(), proposal_id).unwrap();
        assert_eq!(proposal.required_approvals, 2); // Should enforce minimum
        assert_eq!(proposal.execution_delay, 86400); // Should enforce minimum 24 hours
    }
}
