#[cfg(test)]
mod batch_verify_tests {
    use crate::{
        ContractError, Permission, SecurityScannerContract, SecurityScannerContractClient,
        VulnerabilityReport, REPORTS, ROLE_PERMISSIONS,
    };
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, BytesN, Env, Map, String, Symbol, Vec};

    fn test_address(env: &Env, seed: u64) -> Address {
        Address::generate(env)
    }

    fn grant_verifier_role(env: &Env, contract_id: &Address, user: &Address) {
        env.as_contract(contract_id, || {
            let mut permissions: Map<Address, Vec<Permission>> = env
                .storage()
                .instance()
                .get(&ROLE_PERMISSIONS)
                .unwrap_or(Map::new(env));
            let mut user_perms = permissions.get(user.clone()).unwrap_or(Vec::new(env));
            if !user_perms.contains(&Permission::VerifyVulnerability) {
                user_perms.push_back(Permission::VerifyVulnerability);
            }
            permissions.set(user.clone(), user_perms);
            env.storage()
                .instance()
                .set(&ROLE_PERMISSIONS, &permissions);
        });
    }

    fn create_test_report(
        env: &Env,
        contract_id: &Address,
        report_id: u64,
        reporter: &Address,
        bounty: i128,
    ) {
        env.as_contract(contract_id, || {
            let mut reports: Map<u64, VulnerabilityReport> = env
                .storage()
                .instance()
                .get(&REPORTS)
                .unwrap_or(Map::new(env));

            let report = VulnerabilityReport {
                reporter: reporter.clone(),
                contract_id: BytesN::from_array(env, &[0u8; 32]),
                vulnerability_type: String::from_str(env, "reentrancy"),
                severity: String::from_str(env, "high"),
                description: String::from_str(env, "Test vulnerability"),
                location: String::from_str(env, "function_x"),
                timestamp: 1000000,
                status: String::from_str(env, "pending"),
                bounty_amount: 0,
            };

            reports.set(report_id, report);
            env.storage().instance().set(&REPORTS, &reports);
        });
    }

    #[test]
    fn test_batch_verify_vulnerabilities_success() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let reporter = test_address(&env, 2);

        // Initialize and grant permissions
        client.initialize(&admin);
        grant_verifier_role(&env, &contract_id, &admin);

        // Create test reports
        for i in 1..=5 {
            create_test_report(&env, &contract_id, i, &reporter, 0);
        }

        // Prepare batch data
        let mut report_ids = Vec::new(&env);
        let mut bounty_amounts = Vec::new(&env);
        for i in 1..=5 {
            report_ids.push_back(i);
            bounty_amounts.push_back(100_000i128);
        }

        // Execute batch verification
        let successful = client.batch_verify_vulnerabilities(&admin, &report_ids, &bounty_amounts);

        // Verify all were successful
        assert_eq!(successful.len(), 5);
        for i in 1..=5 {
            assert!(successful.contains(&i));
        }
    }

    #[test]
    fn test_batch_verify_empty_batch() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        client.initialize(&admin);
        grant_verifier_role(&env, &contract_id, &admin);

        let report_ids: Vec<u64> = Vec::new(&env);
        let bounty_amounts: Vec<i128> = Vec::new(&env);

        // Empty batch should fail
        assert_eq!(
            client.try_batch_verify_vulnerabilities(&admin, &report_ids, &bounty_amounts),
            Err(Ok(ContractError::InvalidInput))
        );
    }

    #[test]
    fn test_batch_verify_oversized_batch() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        client.initialize(&admin);
        grant_verifier_role(&env, &contract_id, &admin);

        // Create batch larger than MAX_BATCH_SIZE (50)
        let mut report_ids = Vec::new(&env);
        let mut bounty_amounts = Vec::new(&env);
        for i in 1..=51 {
            report_ids.push_back(i as u64);
            bounty_amounts.push_back(100_000i128);
        }

        // Oversized batch should fail
        assert_eq!(
            client.try_batch_verify_vulnerabilities(&admin, &report_ids, &bounty_amounts),
            Err(Ok(ContractError::InvalidInput))
        );
    }

    #[test]
    fn test_batch_verify_length_mismatch() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        client.initialize(&admin);
        grant_verifier_role(&env, &contract_id, &admin);

        let mut report_ids = Vec::new(&env);
        let mut bounty_amounts = Vec::new(&env);
        for i in 1..=5 {
            report_ids.push_back(i);
        }
        for i in 1..=3 {
            // Only 3 bounties instead of 5
            bounty_amounts.push_back(100_000i128);
        }

        // Mismatched lengths should fail
        assert_eq!(
            client.try_batch_verify_vulnerabilities(&admin, &report_ids, &bounty_amounts),
            Err(Ok(ContractError::InvalidInput))
        );
    }

    #[test]
    fn test_batch_verify_duplicates() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        client.initialize(&admin);
        grant_verifier_role(&env, &contract_id, &admin);

        // Create reports
        for i in 1..=3 {
            create_test_report(&env, &contract_id, i, &test_address(&env, 2), 0);
        }

        let mut report_ids = Vec::new(&env);
        let mut bounty_amounts = Vec::new(&env);

        // Add duplicate IDs
        report_ids.push_back(1);
        report_ids.push_back(2);
        report_ids.push_back(1); // Duplicate!
        bounty_amounts.push_back(100_000i128);
        bounty_amounts.push_back(100_000i128);
        bounty_amounts.push_back(100_000i128);

        // Duplicate IDs should fail
        assert_eq!(
            client.try_batch_verify_vulnerabilities(&admin, &report_ids, &bounty_amounts),
            Err(Ok(ContractError::InvalidInput))
        );
    }

    #[test]
    fn test_batch_verify_high_bounty_skipped() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let reporter = test_address(&env, 2);

        client.initialize(&admin);
        grant_verifier_role(&env, &contract_id, &admin);

        // Create test reports
        for i in 1..=3 {
            create_test_report(&env, &contract_id, i, &reporter, 0);
        }

        let mut report_ids = Vec::new(&env);
        let mut bounty_amounts = Vec::new(&env);

        report_ids.push_back(1);
        bounty_amounts.push_back(100_000i128); // Normal bounty

        report_ids.push_back(2);
        bounty_amounts.push_back(2_000_000i128); // High bounty - should be skipped

        report_ids.push_back(3);
        bounty_amounts.push_back(100_000i128); // Normal bounty

        let successful = client.batch_verify_vulnerabilities(&admin, &report_ids, &bounty_amounts);

        // Only reports 1 and 3 should succeed
        assert_eq!(successful.len(), 2);
        assert!(successful.contains(&1));
        assert!(!successful.contains(&2)); // Skipped due to high bounty
        assert!(successful.contains(&3));
    }

    #[test]
    fn test_batch_verify_partial_failure() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let reporter = test_address(&env, 2);

        client.initialize(&admin);
        grant_verifier_role(&env, &contract_id, &admin);

        // Create only reports 1 and 3 (report 2 doesn't exist)
        create_test_report(&env, &contract_id, 1, &reporter, 0);
        create_test_report(&env, &contract_id, 3, &reporter, 0);

        let mut report_ids = Vec::new(&env);
        let mut bounty_amounts = Vec::new(&env);

        report_ids.push_back(1);
        bounty_amounts.push_back(100_000i128);

        report_ids.push_back(2); // Doesn't exist
        bounty_amounts.push_back(100_000i128);

        report_ids.push_back(3);
        bounty_amounts.push_back(100_000i128);

        let successful = client.batch_verify_vulnerabilities(&admin, &report_ids, &bounty_amounts);

        // Only existing reports should be successful
        assert_eq!(successful.len(), 2);
        assert!(successful.contains(&1));
        assert!(!successful.contains(&2));
        assert!(successful.contains(&3));
    }

    #[test]
    fn test_batch_verify_invalid_bounty() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let reporter = test_address(&env, 2);

        client.initialize(&admin);
        grant_verifier_role(&env, &contract_id, &admin);

        // Create test reports
        create_test_report(&env, &contract_id, 1, &reporter, 0);
        create_test_report(&env, &contract_id, 2, &reporter, 0);

        let mut report_ids = Vec::new(&env);
        let mut bounty_amounts = Vec::new(&env);

        report_ids.push_back(1);
        bounty_amounts.push_back(100_000i128); // Valid

        report_ids.push_back(2);
        bounty_amounts.push_back(-50_000i128); // Invalid - negative

        let successful = client.batch_verify_vulnerabilities(&admin, &report_ids, &bounty_amounts);

        // Only report 1 should be successful
        assert_eq!(successful.len(), 1);
        assert!(successful.contains(&1));
        assert!(!successful.contains(&2));
    }

    #[test]
    fn test_batch_verify_max_batch_size_allowed() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let reporter = test_address(&env, 2);

        client.initialize(&admin);
        grant_verifier_role(&env, &contract_id, &admin);

        // Create exactly 50 reports (MAX_BATCH_SIZE)
        for i in 1..=50 {
            create_test_report(&env, &contract_id, i, &reporter, 0);
        }

        let mut report_ids = Vec::new(&env);
        let mut bounty_amounts = Vec::new(&env);
        for i in 1..=50 {
            report_ids.push_back(i);
            bounty_amounts.push_back(10_000i128);
        }

        // Max batch size should succeed
        let successful = client.batch_verify_vulnerabilities(&admin, &report_ids, &bounty_amounts);
        assert_eq!(successful.len(), 50);
    }
}
