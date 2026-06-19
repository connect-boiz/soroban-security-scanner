#[cfg(test)]
mod security {
    use crate::{
        ContractError, Role, SecurityScannerContract, SecurityScannerContractClient, ADMIN_ROLES,
    };
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, BytesN, Env, String};

    fn test_address(env: &Env, _seed: u64) -> Address {
        Address::generate(env)
    }

    /// Directly grant a role to a user by manipulating storage, bypassing multi-sig.
    fn grant_role(env: &Env, contract_id: &Address, user: &Address, role: &Role) {
        env.as_contract(contract_id, || {
            let mut admin_roles: soroban_sdk::Map<Address, soroban_sdk::Vec<Role>> = env
                .storage()
                .instance()
                .get(&ADMIN_ROLES)
                .unwrap_or(soroban_sdk::Map::new(env));
            let mut user_roles = admin_roles
                .get(user.clone())
                .unwrap_or(soroban_sdk::Vec::new(env));
            if !user_roles.contains(role) {
                user_roles.push_back(role.clone());
            }
            admin_roles.set(user.clone(), user_roles);
            env.storage().instance().set(&ADMIN_ROLES, &admin_roles);
        });
    }

    #[test]
    fn test_role_based_access_control() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let verifier = test_address(&env, 2);
        let unauthorized_user = test_address(&env, 3);

        client.initialize(&admin);

        // Grant Verifier role to verifier
        grant_role(&env, &contract_id, &verifier, &Role::Verifier);

        let roles = client.get_user_roles(&verifier);
        assert!(roles.contains(&Role::Verifier));

        // Report a vulnerability
        let contract_id_bytes = BytesN::from_array(&env, &[1; 32]);
        let report_id = client.report_vulnerability(
            &test_address(&env, 10),
            &contract_id_bytes,
            &String::from_str(&env, "reentrancy"),
            &String::from_str(&env, "high"),
            &String::from_str(&env, "Test vulnerability"),
            &String::from_str(&env, "function xyz"),
        );

        // Verifier can verify
        client.verify_vulnerability(&verifier, &report_id, &100000i128);

        // Unauthorized user cannot verify
        assert_eq!(
            client.try_verify_vulnerability(&unauthorized_user, &report_id, &100000i128),
            Err(Ok(ContractError::InsufficientPermissions))
        );
    }

    #[test]
    fn test_multi_signature_requirements() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let verifier1 = test_address(&env, 2);
        let verifier2 = test_address(&env, 3);

        client.initialize(&admin);
        grant_role(&env, &contract_id, &verifier1, &Role::Verifier);
        grant_role(&env, &contract_id, &verifier2, &Role::Verifier);

        // Report a vulnerability
        let contract_id_bytes = BytesN::from_array(&env, &[2; 32]);
        let report_id = client.report_vulnerability(
            &test_address(&env, 10),
            &contract_id_bytes,
            &String::from_str(&env, "overflow"),
            &String::from_str(&env, "critical"),
            &String::from_str(&env, "Test overflow"),
            &String::from_str(&env, "function abc"),
        );

        // High bounty (> 1M) requires multi-sig
        assert_eq!(
            client.try_verify_vulnerability(&verifier1, &report_id, &2000000i128),
            Err(Ok(ContractError::MultiSigRequired))
        );

        // Create multi-sig proposal
        let proposal_id =
            client.propose_high_bounty_verification(&verifier1, &report_id, &2000000i128, &2, &0);

        // First approval
        client.approve_bounty_verification(&verifier1, &proposal_id);
        let can_execute = client.can_execute_proposal_check(&proposal_id);
        assert!(!can_execute);

        // Second approval reaches quorum
        client.approve_bounty_verification(&verifier2, &proposal_id);
        let can_execute = client.can_execute_proposal_check(&proposal_id);
        assert!(can_execute);

        // Execute and verify
        client.execute_high_bounty_verification(&verifier1, &proposal_id);
        let report = client.get_vulnerability(&report_id);
        assert_eq!(report.status, String::from_str(&env, "verified"));
        assert_eq!(report.bounty_amount, 2000000);
    }

    #[test]
    fn test_emergency_verification_multi_sig() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let verifier1 = test_address(&env, 2);
        let verifier2 = test_address(&env, 3);
        let verifier3 = test_address(&env, 4);

        client.initialize(&admin);
        grant_role(&env, &contract_id, &verifier1, &Role::Verifier);
        grant_role(&env, &contract_id, &verifier2, &Role::Verifier);
        grant_role(&env, &contract_id, &verifier3, &Role::Verifier);

        // Report emergency vulnerability
        let contract_id_bytes = BytesN::from_array(&env, &[3; 32]);
        let alert_id = client.report_emergency_vulnerability(
            &test_address(&env, 10),
            &contract_id_bytes,
            &String::from_str(&env, "critical_vuln"),
            &String::from_str(&env, "emergency"),
            &String::from_str(&env, "Critical emergency vulnerability"),
            &String::from_str(&env, "core contract"),
        );

        // Emergency verification requires multi-sig
        assert_eq!(
            client.try_verify_emergency_vulnerability(&verifier1, &alert_id, &true),
            Err(Ok(ContractError::MultiSigRequired))
        );

        // Create multi-sig proposal for emergency verification
        let proposal_id =
            client.propose_emergency_verification(&verifier1, &alert_id, &true, &3, &3600);

        let proposal = client.get_proposal(&proposal_id);
        assert_eq!(proposal.required_approvals, 3);
        assert_eq!(proposal.execution_delay, 3600);

        // All three verifiers approve
        for verifier in [&verifier1, &verifier2, &verifier3] {
            client.approve_emergency_verification(verifier, &proposal_id);
        }

        // Cannot execute yet (delay not passed)
        let can_execute = client.can_execute_proposal_check(&proposal_id);
        assert!(!can_execute);

        // Alert still pending
        let alert = client.get_emergency_alert(&alert_id);
        assert_eq!(alert.status, String::from_str(&env, "pending"));
        assert_eq!(alert.emergency_reward, 10000000);
    }

    #[test]
    fn test_escrow_manager_role() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let escrow_manager = test_address(&env, 2);
        let unauthorized_user = test_address(&env, 3);

        client.initialize(&admin);

        grant_role(&env, &contract_id, &escrow_manager, &Role::EscrowManager);

        // Create escrow
        let escrow_id = client.create_escrow(
            &test_address(&env, 10),
            &test_address(&env, 11),
            &1_000_000i128,
            &String::from_str(&env, "bounty"),
            &86400,
        );

        // Escrow manager can mark conditions met
        client.mark_escrow_conditions_met(&escrow_id, &escrow_manager);

        // Unauthorized user cannot
        assert_eq!(
            client.try_mark_escrow_conditions_met(&escrow_id, &unauthorized_user),
            Err(Ok(ContractError::InsufficientPermissions))
        );
    }

    #[test]
    fn test_treasury_manager_role() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let treasury_manager = test_address(&env, 2);
        let unauthorized_user = test_address(&env, 3);

        client.initialize(&admin);

        grant_role(
            &env,
            &contract_id,
            &treasury_manager,
            &Role::TreasuryManager,
        );

        // Treasury manager can fund emergency pool
        client.fund_emergency_pool(&treasury_manager, &5_000_000i128);
        let pool_balance = client.get_emergency_pool_balance();
        assert_eq!(pool_balance, 5000000);

        // Unauthorized user cannot
        assert_eq!(
            client.try_fund_emergency_pool(&unauthorized_user, &1_000_000i128),
            Err(Ok(ContractError::InsufficientPermissions))
        );
    }

    #[test]
    fn test_role_management_security() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let unauthorized_user = test_address(&env, 2);
        let new_user = test_address(&env, 3);

        client.initialize(&admin);

        // Direct grant always requires multi-sig
        assert_eq!(
            client.try_grant_role(&admin, &new_user, &Role::Verifier),
            Err(Ok(ContractError::MultiSigRequired))
        );

        // Unauthorized user cannot propose role grants
        assert_eq!(
            client.try_propose_role_grant(&unauthorized_user, &new_user, &Role::Verifier, &2, &0),
            Err(Ok(ContractError::InsufficientPermissions))
        );

        // Propose with low values gets upgraded to minimums
        let proposal_id = client.propose_role_grant(&admin, &new_user, &Role::Verifier, &1, &100);
        let proposal = client.get_proposal(&proposal_id);
        assert_eq!(proposal.required_approvals, 2);
        assert_eq!(proposal.execution_delay, 86400);
    }
}
