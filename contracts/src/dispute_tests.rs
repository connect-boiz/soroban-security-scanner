#[cfg(test)]
#[allow(clippy::module_inception)]
mod dispute_tests {
    use crate::{
        ContractError, DisputeStatus, Reputation, Role, SecurityScannerContract,
        SecurityScannerContractClient, Vote, ADMIN_ROLES, DISPUTE_DEADLINE, REPUTATION_MAP,
        UPGRADE_DELAY_KEY,
    };
    use soroban_sdk::testutils::{Address as _, Ledger as _};
    use soroban_sdk::{Address, BytesN, Env, String};

    fn test_address(env: &Env, _seed: u64) -> Address {
        Address::generate(env)
    }

    fn advance_timestamp(env: &Env, delta: u64) {
        let current = env.ledger().timestamp();
        env.ledger().with_mut(|li| li.timestamp = current + delta);
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

    fn set_reputation(env: &Env, contract_id: &Address, researcher: &Address, score: u64) {
        env.as_contract(contract_id, || {
            let mut rep_map: soroban_sdk::Map<Address, Reputation> = env
                .storage()
                .instance()
                .get(&REPUTATION_MAP)
                .unwrap_or(soroban_sdk::Map::new(env));
            let reputation = Reputation {
                researcher: researcher.clone(),
                score,
                successful_reports: score / 10,
                total_earnings: 0,
            };
            rep_map.set(researcher.clone(), reputation);
            env.storage().instance().set(&REPUTATION_MAP, &rep_map);
        });
    }

    fn setup_dispute_env(
        env: &Env,
        client: &SecurityScannerContractClient,
    ) -> (Address, Address, u64) {
        let admin = test_address(env, 10);
        let researcher = test_address(env, 20);
        client.initialize(&admin);
        client.fund_bounty_pool(&admin, &10_000_000i128);
        let contract_id = BytesN::from_array(env, &[10; 32]);
        let report_id = client.report_vulnerability(
            &researcher,
            &contract_id,
            &String::from_str(env, "reentrancy"),
            &String::from_str(env, "critical"),
            &String::from_str(env, "Critical reentrancy vulnerability found"),
            &String::from_str(env, "function withdraw"),
        );
        client.verify_vulnerability(&admin, &report_id, &500_000i128);
        (admin, researcher, report_id)
    }

    #[test]
    fn test_upgrade_mechanism() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let upgrade_authority = test_address(&env, 2);
        let new_contract = test_address(&env, 3);

        client.initialize(&admin);
        client.set_upgrade_authority(&admin, &upgrade_authority);
        assert_eq!(client.get_version(), String::from_str(&env, "1.0.0"));
        client.propose_upgrade(
            &upgrade_authority,
            &new_contract,
            &String::from_str(&env, "2.0.0"),
            &String::from_str(&env, "Security improvements and new features"),
        );
        let pending = client.get_pending_upgrade();
        assert_eq!(pending.new_contract_address, new_contract);
        assert_eq!(pending.version, String::from_str(&env, "2.0.0"));
        assert_eq!(pending.proposed_by, upgrade_authority);
        assert_eq!(
            client.try_execute_upgrade(&upgrade_authority),
            Err(Ok(ContractError::UpgradeNotReady))
        );
        advance_timestamp(&env, 604800 + 1);
        client.execute_upgrade(&upgrade_authority);
        let history = client.get_upgrade_history();
        assert_eq!(history.len(), 1);
        assert_eq!(
            history.get(0).unwrap().from_version,
            String::from_str(&env, "1.0.0")
        );
        assert_eq!(
            history.get(0).unwrap().to_version,
            String::from_str(&env, "2.0.0")
        );
        assert!(client.try_get_pending_upgrade().is_err());
    }

    #[test]
    fn test_emergency_upgrade() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let approver1 = test_address(&env, 2);
        let approver2 = test_address(&env, 3);
        let approver3 = test_address(&env, 4);
        let new_contract = test_address(&env, 5);

        client.initialize(&admin);

        // Grant SuperAdmin to approvers for EmergencyActions permission
        grant_role(&env, &contract_id, &approver1, &Role::SuperAdmin);
        grant_role(&env, &contract_id, &approver2, &Role::SuperAdmin);
        grant_role(&env, &contract_id, &approver3, &Role::SuperAdmin);

        // Direct call now requires multi-sig
        assert_eq!(
            client.try_emergency_upgrade(
                &admin,
                &new_contract,
                &String::from_str(&env, "1.0.1"),
                &String::from_str(
                    &env,
                    "Critical security patch for a severe vulnerability that must be fixed immediately",
                ),
            ),
            Err(Ok(ContractError::MultiSigRequired))
        );

        // Create multi-sig proposal
        let proposal_id = client.propose_emergency_upgrade(
            &approver1,
            &new_contract,
            &String::from_str(&env, "1.0.1"),
            &String::from_str(
                &env,
                "Critical security patch for a severe vulnerability that must be fixed immediately",
            ),
            &3,
            &0,
        );

        // All three approve
        client.approve_emerg_upgrade(&approver1, &proposal_id);
        client.approve_emerg_upgrade(&approver2, &proposal_id);
        client.approve_emerg_upgrade(&approver3, &proposal_id);

        // Wait for execution delay to pass
        advance_timestamp(&env, 3601);

        // Execute
        client.execute_emergency_upgrade(&approver1, &proposal_id);

        assert_eq!(client.get_upgrade_history().len(), 1);
    }

    #[test]
    fn test_upgrade_authority_controls() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let unauthorized_user = test_address(&env, 2);
        let new_contract = test_address(&env, 3);
        client.initialize(&admin);
        assert_eq!(
            client.try_propose_upgrade(
                &unauthorized_user,
                &new_contract,
                &String::from_str(&env, "2.0.0"),
                &String::from_str(&env, "Unauthorized upgrade")
            ),
            Err(Ok(ContractError::Unauthorized))
        );
        let new_authority = test_address(&env, 4);
        client.set_upgrade_authority(&admin, &new_authority);
        assert_eq!(
            client.try_propose_upgrade(
                &admin,
                &new_contract,
                &String::from_str(&env, "2.0.0"),
                &String::from_str(&env, "Admin attempt")
            ),
            Err(Ok(ContractError::Unauthorized))
        );
        client.propose_upgrade(
            &new_authority,
            &new_contract,
            &String::from_str(&env, "2.0.0"),
            &String::from_str(&env, "Authorized upgrade"),
        );
    }

    #[test]
    fn test_upgrade_delay_configuration() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        client.initialize(&admin);
        let default_delay: u64 = env.as_contract(&contract_id, || {
            env.storage()
                .instance()
                .get(&UPGRADE_DELAY_KEY)
                .unwrap_or(604800)
        });
        assert_eq!(default_delay, 604800);
        client.set_upgrade_delay(&admin, &259200);
        let new_delay: u64 = env.as_contract(&contract_id, || {
            env.storage()
                .instance()
                .get(&UPGRADE_DELAY_KEY)
                .unwrap_or(0)
        });
        assert_eq!(new_delay, 259200);
        assert_eq!(
            client.try_set_upgrade_delay(&admin, &3600),
            Err(Ok(ContractError::InvalidInput))
        );
    }

    #[test]
    fn test_concurrent_upgrade_prevention() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let upgrade_authority = test_address(&env, 2);
        let new_contract1 = test_address(&env, 3);
        let new_contract2 = test_address(&env, 4);
        client.initialize(&admin);
        client.set_upgrade_authority(&admin, &upgrade_authority);
        client.propose_upgrade(
            &upgrade_authority,
            &new_contract1,
            &String::from_str(&env, "2.0.0"),
            &String::from_str(&env, "First upgrade"),
        );
        assert_eq!(
            client.try_propose_upgrade(
                &upgrade_authority,
                &new_contract2,
                &String::from_str(&env, "3.0.0"),
                &String::from_str(&env, "Second upgrade")
            ),
            Err(Ok(ContractError::UpgradeInProgress))
        );
        client.cancel_upgrade(&upgrade_authority);
        client.propose_upgrade(
            &upgrade_authority,
            &new_contract2,
            &String::from_str(&env, "3.0.0"),
            &String::from_str(&env, "Second upgrade"),
        );
    }

    #[test]
    fn test_state_migration_tracking() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let upgrade_authority = test_address(&env, 2);
        let new_contract = test_address(&env, 3);
        client.initialize(&admin);
        client.set_upgrade_authority(&admin, &upgrade_authority);
        assert!(client.get_migration_status().is_none());
        client.propose_upgrade(
            &upgrade_authority,
            &new_contract,
            &String::from_str(&env, "2.0.0"),
            &String::from_str(&env, "Test migration"),
        );
        advance_timestamp(&env, 604800 + 1);
        client.execute_upgrade(&upgrade_authority);
        let (contract_addr, _timestamp) = client.get_migration_status().unwrap();
        assert_eq!(contract_addr, new_contract);
    }

    // Dispute Resolution Tests
    #[test]
    fn test_file_dispute_with_stake() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let disputant = test_address(&env, 100);
        let (_admin, _researcher, report_id) = setup_dispute_env(&env, &client);
        let evidence: soroban_sdk::Vec<BytesN<32>> = soroban_sdk::Vec::new(&env);
        let dispute_id = client.file_dispute(
            &disputant,
            &report_id,
            &String::from_str(
                &env,
                "This report is invalid, the vulnerability was already patched",
            ),
            &evidence,
        );
        let dispute = client.get_dispute(&dispute_id);
        assert_eq!(dispute.report_id, report_id);
        assert_eq!(dispute.disputant, disputant);
        assert_eq!(dispute.status, DisputeStatus::Active);
        assert!(dispute.resolution_deadline > dispute.created_at);
        let stake = client.get_dispute_stake(&dispute_id);
        assert_eq!(stake, 100_000i128);
        assert!(dispute_id > 0);
    }

    #[test]
    fn test_vote_on_dispute_by_eligible_researcher() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let disputant = test_address(&env, 100);
        let (_admin, _researcher, report_id) = setup_dispute_env(&env, &client);
        let evidence: soroban_sdk::Vec<BytesN<32>> = soroban_sdk::Vec::new(&env);
        let dispute_id = client.file_dispute(
            &disputant,
            &report_id,
            &String::from_str(&env, "Disputed: vulnerability does not exist"),
            &evidence,
        );
        let verifier = test_address(&env, 200);
        grant_role(&env, &contract_id, &verifier, &Role::Verifier);
        set_reputation(&env, &contract_id, &verifier, 100);
        client.vote_on_dispute(&verifier, &dispute_id, &Vote::Accept);
        let dispute = client.get_dispute(&dispute_id);
        assert_eq!(dispute.votes.get(verifier).unwrap(), Vote::Accept);
    }

    #[test]
    fn test_cannot_vote_with_insufficient_reputation() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let disputant = test_address(&env, 100);
        let (_admin, _researcher, report_id) = setup_dispute_env(&env, &client);
        let evidence: soroban_sdk::Vec<BytesN<32>> = soroban_sdk::Vec::new(&env);
        let dispute_id = client.file_dispute(
            &disputant,
            &report_id,
            &String::from_str(&env, "Disputed"),
            &evidence,
        );
        let low_rep_researcher = test_address(&env, 300);
        set_reputation(&env, &contract_id, &low_rep_researcher, 30);
        assert_eq!(
            client.try_vote_on_dispute(&low_rep_researcher, &dispute_id, &Vote::Accept),
            Err(Ok(ContractError::InsufficientReputation))
        );
    }

    #[test]
    fn test_cannot_vote_twice() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let disputant = test_address(&env, 100);
        let (_admin, _researcher, report_id) = setup_dispute_env(&env, &client);
        let evidence: soroban_sdk::Vec<BytesN<32>> = soroban_sdk::Vec::new(&env);
        let dispute_id = client.file_dispute(
            &disputant,
            &report_id,
            &String::from_str(&env, "Disputed"),
            &evidence,
        );
        let verifier = test_address(&env, 200);
        grant_role(&env, &contract_id, &verifier, &Role::Verifier);
        set_reputation(&env, &contract_id, &verifier, 100);
        client.vote_on_dispute(&verifier, &dispute_id, &Vote::Accept);
        assert_eq!(
            client.try_vote_on_dispute(&verifier, &dispute_id, &Vote::Reject),
            Err(Ok(ContractError::AlreadyVoted))
        );
    }

    #[test]
    fn test_resolve_dispute_after_quorum_dispute_upheld() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let disputant = test_address(&env, 100);
        let (_admin, _researcher, report_id) = setup_dispute_env(&env, &client);
        let evidence: soroban_sdk::Vec<BytesN<32>> = soroban_sdk::Vec::new(&env);
        let dispute_id = client.file_dispute(
            &disputant,
            &report_id,
            &String::from_str(&env, "Disputed: vulnerability is a false positive"),
            &evidence,
        );
        for seed in [201u64, 202, 203] {
            let v = test_address(&env, seed);
            grant_role(&env, &contract_id, &v, &Role::Verifier);
            set_reputation(&env, &contract_id, &v, 100);
            client.vote_on_dispute(&v, &dispute_id, &Vote::Accept);
        }
        assert_eq!(client.resolve_dispute(&dispute_id), DisputeStatus::Resolved);
        let dispute = client.get_dispute(&dispute_id);
        assert_eq!(dispute.status, DisputeStatus::Resolved);
        assert_eq!(
            client.try_get_dispute_stake(&dispute_id),
            Err(Ok(ContractError::DisputeNotFound))
        );
    }

    #[test]
    fn test_resolve_dispute_dismissed_stake_forfeited() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let disputant = test_address(&env, 100);
        let (_admin, _researcher, report_id) = setup_dispute_env(&env, &client);
        let initial_bounty_pool = client.get_bounty_pool();
        let evidence: soroban_sdk::Vec<BytesN<32>> = soroban_sdk::Vec::new(&env);
        let dispute_id = client.file_dispute(
            &disputant,
            &report_id,
            &String::from_str(&env, "Disputed: vulnerability does not exist"),
            &evidence,
        );
        for seed in [201u64, 202, 203] {
            let v = test_address(&env, seed);
            grant_role(&env, &contract_id, &v, &Role::Verifier);
            set_reputation(&env, &contract_id, &v, 100);
            client.vote_on_dispute(&v, &dispute_id, &Vote::Reject);
        }
        assert_eq!(
            client.resolve_dispute(&dispute_id),
            DisputeStatus::Dismissed
        );
        assert_eq!(
            client.get_dispute(&dispute_id).status,
            DisputeStatus::Dismissed
        );
        assert_eq!(client.get_bounty_pool(), initial_bounty_pool + 100_000i128);
        assert_eq!(
            client.try_get_dispute_stake(&dispute_id),
            Err(Ok(ContractError::DisputeNotFound))
        );
    }

    #[test]
    fn test_resolve_dispute_after_deadline() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let disputant = test_address(&env, 100);
        let (_admin, _researcher, report_id) = setup_dispute_env(&env, &client);
        let initial_bounty_pool = client.get_bounty_pool();
        let evidence: soroban_sdk::Vec<BytesN<32>> = soroban_sdk::Vec::new(&env);
        let dispute_id = client.file_dispute(
            &disputant,
            &report_id,
            &String::from_str(&env, "Disputed: vulnerability is not valid"),
            &evidence,
        );
        let verifier = test_address(&env, 200);
        grant_role(&env, &contract_id, &verifier, &Role::Verifier);
        set_reputation(&env, &contract_id, &verifier, 100);
        client.vote_on_dispute(&verifier, &dispute_id, &Vote::Accept);
        assert_eq!(
            client.try_resolve_dispute(&dispute_id),
            Err(Ok(ContractError::InvalidDisputeStatus))
        );
        advance_timestamp(&env, DISPUTE_DEADLINE + 1);
        assert_eq!(
            client.resolve_dispute(&dispute_id),
            DisputeStatus::Dismissed
        );
        assert_eq!(client.get_bounty_pool(), initial_bounty_pool + 100_000i128);
    }

    #[test]
    fn test_cannot_resolve_already_resolved_dispute() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let disputant = test_address(&env, 100);
        let (_admin, _researcher, report_id) = setup_dispute_env(&env, &client);
        let evidence: soroban_sdk::Vec<BytesN<32>> = soroban_sdk::Vec::new(&env);
        let dispute_id = client.file_dispute(
            &disputant,
            &report_id,
            &String::from_str(&env, "Disputed"),
            &evidence,
        );
        for seed in [201u64, 202, 203] {
            let v = test_address(&env, seed);
            grant_role(&env, &contract_id, &v, &Role::Verifier);
            set_reputation(&env, &contract_id, &v, 100);
            client.vote_on_dispute(&v, &dispute_id, &Vote::Reject);
        }
        assert_eq!(
            client.resolve_dispute(&dispute_id),
            DisputeStatus::Dismissed
        );
        assert_eq!(
            client.try_resolve_dispute(&dispute_id),
            Err(Ok(ContractError::DisputeAlreadyResolved))
        );
    }

    #[test]
    fn test_file_dispute_on_nonexistent_report() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let disputant = test_address(&env, 100);
        let _ = setup_dispute_env(&env, &client);
        let evidence: soroban_sdk::Vec<BytesN<32>> = soroban_sdk::Vec::new(&env);
        assert_eq!(
            client.try_file_dispute(
                &disputant,
                &99999,
                &String::from_str(&env, "Dispute on invalid report"),
                &evidence
            ),
            Err(Ok(ContractError::NotFound))
        );
    }

    #[test]
    fn test_vote_with_abstain() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let disputant = test_address(&env, 100);
        let (_admin, _researcher, report_id) = setup_dispute_env(&env, &client);
        let evidence: soroban_sdk::Vec<BytesN<32>> = soroban_sdk::Vec::new(&env);
        let dispute_id = client.file_dispute(
            &disputant,
            &report_id,
            &String::from_str(&env, "Disputed"),
            &evidence,
        );
        for seed in [201u64, 202, 203] {
            let v = test_address(&env, seed);
            grant_role(&env, &contract_id, &v, &Role::Verifier);
            set_reputation(&env, &contract_id, &v, 100);
            client.vote_on_dispute(&v, &dispute_id, &Vote::Abstain);
        }
        assert_eq!(
            client.try_resolve_dispute(&dispute_id),
            Err(Ok(ContractError::InvalidDisputeStatus))
        );
        advance_timestamp(&env, DISPUTE_DEADLINE + 1);
        assert_eq!(
            client.resolve_dispute(&dispute_id),
            DisputeStatus::Dismissed
        );
    }

    #[test]
    fn test_dispute_with_evidence_hashes() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let disputant = test_address(&env, 100);
        let (_admin, _researcher, report_id) = setup_dispute_env(&env, &client);
        let mut evidence: soroban_sdk::Vec<BytesN<32>> = soroban_sdk::Vec::new(&env);
        evidence.push_back(BytesN::from_array(&env, &[1; 32]));
        evidence.push_back(BytesN::from_array(&env, &[2; 32]));
        let dispute_id = client.file_dispute(
            &disputant,
            &report_id,
            &String::from_str(&env, "Dispute with evidence hashes"),
            &evidence,
        );
        let dispute = client.get_dispute(&dispute_id);
        assert_eq!(dispute.evidence_hashes.len(), 2);
        assert_eq!(
            dispute.evidence_hashes.get(0).unwrap(),
            BytesN::from_array(&env, &[1; 32])
        );
        assert_eq!(
            dispute.evidence_hashes.get(1).unwrap(),
            BytesN::from_array(&env, &[2; 32])
        );
    }

    #[test]
    fn test_file_dispute_on_already_disputed_report() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let disputant1 = test_address(&env, 100);
        let disputant2 = test_address(&env, 101);
        let (_admin, _researcher, report_id) = setup_dispute_env(&env, &client);
        let evidence: soroban_sdk::Vec<BytesN<32>> = soroban_sdk::Vec::new(&env);
        let dispute_id1 = client.file_dispute(
            &disputant1,
            &report_id,
            &String::from_str(&env, "First dispute on this report"),
            &evidence,
        );
        let dispute_id2 = client.file_dispute(
            &disputant2,
            &report_id,
            &String::from_str(&env, "Second dispute on same report"),
            &evidence,
        );
        assert_ne!(dispute_id1, dispute_id2);
        assert_eq!(client.get_dispute(&dispute_id1).report_id, report_id);
        assert_eq!(client.get_dispute(&dispute_id2).report_id, report_id);
    }

    #[test]
    fn test_get_dispute_nonexistent() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let _ = setup_dispute_env(&env, &client);
        assert_eq!(
            client.try_get_dispute(&99999),
            Err(Ok(ContractError::DisputeNotFound))
        );
    }

    // ── Enhanced Upgrade Mechanism Tests ──

    #[test]
    fn test_rollback_within_window() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let upgrade_authority = test_address(&env, 2);
        let new_contract = test_address(&env, 3);

        client.initialize(&admin);
        client.set_upgrade_authority(&admin, &upgrade_authority);

        // Perform a standard upgrade
        client.propose_upgrade(
            &upgrade_authority,
            &new_contract,
            &String::from_str(&env, "2.0.0"),
            &String::from_str(&env, "Upgrade to version 2.0.0 with new features"),
        );
        advance_timestamp(&env, 604800 + 1);
        client.execute_upgrade(&upgrade_authority);
        assert_eq!(client.get_version(), String::from_str(&env, "2.0.0"));

        // Rollback to previous version within window
        client.rollback_upgrade(&admin, &String::from_str(&env, "1.0.0"));
        assert_eq!(client.get_version(), String::from_str(&env, "1.0.0"));
    }

    #[test]
    fn test_rollback_after_window_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let upgrade_authority = test_address(&env, 2);
        let new_contract = test_address(&env, 3);

        client.initialize(&admin);
        client.set_upgrade_authority(&admin, &upgrade_authority);

        client.propose_upgrade(
            &upgrade_authority,
            &new_contract,
            &String::from_str(&env, "2.0.0"),
            &String::from_str(&env, "Upgrade to version 2.0.0"),
        );
        advance_timestamp(&env, 604800 + 1);
        client.execute_upgrade(&upgrade_authority);

        // Advance past 30-day rollback window
        advance_timestamp(&env, 31 * 24 * 60 * 60);

        assert_eq!(
            client.try_rollback_upgrade(&admin, &String::from_str(&env, "1.0.0")),
            Err(Ok(ContractError::InvalidInput))
        );
    }

    #[test]
    fn test_migration_event_emitted() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let upgrade_authority = test_address(&env, 2);
        let new_contract = test_address(&env, 3);

        client.initialize(&admin);
        client.set_upgrade_authority(&admin, &upgrade_authority);

        client.propose_upgrade(
            &upgrade_authority,
            &new_contract,
            &String::from_str(&env, "2.0.0"),
            &String::from_str(&env, "Upgrade with migration events"),
        );
        advance_timestamp(&env, 604800 + 1);
        client.execute_upgrade(&upgrade_authority);

        // Verify migration status is set
        let (migrated_addr, _ts) = client.get_migration_status().unwrap();
        assert_eq!(migrated_addr, new_contract);

        // Verify version was updated
        assert_eq!(client.get_version(), String::from_str(&env, "2.0.0"));

        // Upgrade history should have the entry
        assert_eq!(client.get_upgrade_history().len(), 1);
    }

    #[test]
    fn test_emergency_upgrade_multi_sig_quorum() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let approver1 = test_address(&env, 2);
        let approver2 = test_address(&env, 3);
        let approver3 = test_address(&env, 4);
        let new_contract = test_address(&env, 5);

        client.initialize(&admin);
        grant_role(&env, &contract_id, &approver1, &Role::SuperAdmin);
        grant_role(&env, &contract_id, &approver2, &Role::SuperAdmin);
        grant_role(&env, &contract_id, &approver3, &Role::SuperAdmin);

        // 2/3 of 5 = 3 (integer math), so 2 approvals is insufficient
        let proposal_id = client.propose_emergency_upgrade(
            &approver1,
            &new_contract,
            &String::from_str(&env, "1.0.1"),
            &String::from_str(
                &env,
                "Critical security patch for a severe vulnerability that must be fixed immediately",
            ),
            &5,
            &0,
        );

        // Only 2 approvals — not enough for 2/3 of 5 (need 3)
        client.approve_emerg_upgrade(&approver1, &proposal_id);
        client.approve_emerg_upgrade(&approver2, &proposal_id);

        // Wait for execution delay to pass
        advance_timestamp(&env, 3601);

        assert_eq!(
            client.try_execute_emergency_upgrade(&approver1, &proposal_id),
            Err(Ok(ContractError::InsufficientPermissions))
        );

        // Third approval reaches 2/3 quorum
        client.approve_emerg_upgrade(&approver3, &proposal_id);
        client.execute_emergency_upgrade(&approver1, &proposal_id);

        assert_eq!(client.get_upgrade_history().len(), 1);
    }

    #[test]
    fn test_cancel_upgrade_by_emergency_actions_holder() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SecurityScannerContract);
        let client = SecurityScannerContractClient::new(&env, &contract_id);

        let admin = test_address(&env, 1);
        let upgrade_authority = test_address(&env, 2);
        let emergency_holder = test_address(&env, 3);
        let new_contract = test_address(&env, 4);

        client.initialize(&admin);
        client.set_upgrade_authority(&admin, &upgrade_authority);
        grant_role(&env, &contract_id, &emergency_holder, &Role::SuperAdmin);

        // Propose upgrade
        client.propose_upgrade(
            &upgrade_authority,
            &new_contract,
            &String::from_str(&env, "2.0.0"),
            &String::from_str(&env, "Standard upgrade"),
        );

        // EmergencyActions holder can cancel
        client.cancel_upgrade(&emergency_holder);

        // Pending upgrade should be cleared
        assert!(client.try_get_pending_upgrade().is_err());
    }
}
