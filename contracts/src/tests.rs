#[cfg(test)]
mod tests {
    use soroban_sdk::{symbol_short, Address, BytesN, Env, String};
    use crate::{
        ContractError, SecurityScannerContract, UpgradeRequest, UpgradeHistory,
        ADMIN, CONTRACT_VERSION, UPGRADE_AUTHORITY, UPGRADE_DELAY, PENDING_UPGRADE
    };

    #[test]
    fn test_upgrade_mechanism() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let upgrade_authority = Address::generate(&env);
        let new_contract = Address::generate(&env);
        
        // Initialize contract
        SecurityScannerContract::initialize(env.clone(), admin.clone()).unwrap();
        
        // Set upgrade authority
        SecurityScannerContract::set_upgrade_authority(
            env.clone(), 
            admin.clone(), 
            upgrade_authority.clone()
        ).unwrap();
        
        // Test version
        assert_eq!(SecurityScannerContract::get_version(env.clone()), "1.0.0");
        
        // Propose upgrade
        SecurityScannerContract::propose_upgrade(
            env.clone(),
            upgrade_authority.clone(),
            new_contract.clone(),
            String::from_str(&env, "2.0.0"),
            String::from_str(&env, "Security improvements and new features"),
        ).unwrap();
        
        // Check pending upgrade
        let pending = SecurityScannerContract::get_pending_upgrade(env.clone()).unwrap();
        assert_eq!(pending.new_contract_address, new_contract);
        assert_eq!(pending.version, String::from_str(&env, "2.0.0"));
        assert_eq!(pending.proposed_by, upgrade_authority);
        
        // Try to execute before delay (should fail)
        let result = SecurityScannerContract::execute_upgrade(env.clone(), upgrade_authority.clone());
        assert_eq!(result, Err(ContractError::UpgradeNotReady));
        
        // Fast forward time
        env.ledger().set_timestamp(env.ledger().timestamp() + 604800 + 1);
        
        // Execute upgrade
        SecurityScannerContract::execute_upgrade(env.clone(), upgrade_authority.clone()).unwrap();
        
        // Check upgrade history
        let history = SecurityScannerContract::get_upgrade_history(env.clone());
        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).unwrap().from_version, "1.0.0");
        assert_eq!(history.get(0).unwrap().to_version, "2.0.0");
        
        // No pending upgrade should exist
        assert_eq!(
            SecurityScannerContract::get_pending_upgrade(env.clone()),
            Err(ContractError::NotFound)
        );
    }

    #[test]
    fn test_emergency_upgrade() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let new_contract = Address::generate(&env);
        
        // Initialize contract
        SecurityScannerContract::initialize(env.clone(), admin.clone()).unwrap();
        
        // Emergency upgrade (no delay)
        SecurityScannerContract::emergency_upgrade(
            env.clone(),
            admin.clone(),
            new_contract.clone(),
            String::from_str(&env, "1.0.1"),
            String::from_str(&env, "Critical security patch"),
        ).unwrap();
        
        // Check upgrade history
        let history = SecurityScannerContract::get_upgrade_history(env.clone());
        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).unwrap().from_version, "1.0.0");
        assert_eq!(history.get(0).unwrap().to_version, "1.0.1");
        assert!(history.get(0).unwrap().new_contract == new_contract);
    }

    #[test]
    fn test_upgrade_authority_controls() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let unauthorized_user = Address::generate(&env);
        let new_contract = Address::generate(&env);
        
        // Initialize contract
        SecurityScannerContract::initialize(env.clone(), admin.clone()).unwrap();
        
        // Try to propose upgrade as unauthorized user (should fail)
        let result = SecurityScannerContract::propose_upgrade(
            env.clone(),
            unauthorized_user.clone(),
            new_contract.clone(),
            String::from_str(&env, "2.0.0"),
            String::from_str(&env, "Unauthorized upgrade"),
        );
        assert_eq!(result, Err(ContractError::Unauthorized));
        
        // Set upgrade authority to different address
        let new_authority = Address::generate(&env);
        SecurityScannerContract::set_upgrade_authority(
            env.clone,
            admin.clone(),
            new_authority.clone()
        ).unwrap();
        
        // Now admin should not be able to propose upgrade
        let result = SecurityScannerContract::propose_upgrade(
            env.clone(),
            admin.clone(),
            new_contract.clone(),
            String::from_str(&env, "2.0.0"),
            String::from_str(&env, "Admin attempt after authority change"),
        );
        assert_eq!(result, Err(ContractError::Unauthorized));
        
        // But new authority should be able to
        SecurityScannerContract::propose_upgrade(
            env.clone(),
            new_authority.clone(),
            new_contract.clone(),
            String::from_str(&env, "2.0.0"),
            String::from_str(&env, "Authorized upgrade"),
        ).unwrap();
    }

    #[test]
    fn test_upgrade_delay_configuration() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        
        // Initialize contract
        SecurityScannerContract::initialize(env.clone(), admin.clone()).unwrap();
        
        // Test default delay (7 days)
        let default_delay: u64 = env.storage().instance().get(&UPGRADE_DELAY).unwrap();
        assert_eq!(default_delay, 604800);
        
        // Set new delay (3 days)
        SecurityScannerContract::set_upgrade_delay(
            env.clone(),
            admin.clone(),
            259200, // 3 days
        ).unwrap();
        
        let new_delay: u64 = env.storage().instance().get(&UPGRADE_DELAY).unwrap();
        assert_eq!(new_delay, 259200);
        
        // Try to set delay below minimum (should fail)
        let result = SecurityScannerContract::set_upgrade_delay(
            env.clone(),
            admin.clone(),
            3600, // 1 hour - below 24 hour minimum
        );
        assert_eq!(result, Err(ContractError::InvalidInput));
    }

    #[test]
    fn test_concurrent_upgrade_prevention() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let upgrade_authority = Address::generate(&env);
        let new_contract1 = Address::generate(&env);
        let new_contract2 = Address::generate(&env);
        
        // Initialize contract
        SecurityScannerContract::initialize(env.clone(), admin.clone()).unwrap();
        SecurityScannerContract::set_upgrade_authority(
            env.clone(), 
            admin.clone(), 
            upgrade_authority.clone()
        ).unwrap();
        
        // Propose first upgrade
        SecurityScannerContract::propose_upgrade(
            env.clone(),
            upgrade_authority.clone(),
            new_contract1.clone(),
            String::from_str(&env, "2.0.0"),
            String::from_str(&env, "First upgrade"),
        ).unwrap();
        
        // Try to propose second upgrade (should fail)
        let result = SecurityScannerContract::propose_upgrade(
            env.clone(),
            upgrade_authority.clone(),
            new_contract2.clone(),
            String::from_str(&env, "3.0.0"),
            String::from_str(&env, "Second upgrade"),
        );
        assert_eq!(result, Err(ContractError::UpgradeInProgress));
        
        // Cancel first upgrade
        SecurityScannerContract::cancel_upgrade(env.clone(), upgrade_authority.clone()).unwrap();
        
        // Now should be able to propose second upgrade
        SecurityScannerContract::propose_upgrade(
            env.clone(),
            upgrade_authority.clone(),
            new_contract2.clone(),
            String::from_str(&env, "3.0.0"),
            String::from_str(&env, "Second upgrade"),
        ).unwrap();
    }

    #[test]
    fn test_state_migration_tracking() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let upgrade_authority = Address::generate(&env);
        let new_contract = Address::generate(&env);
        
        // Initialize contract
        SecurityScannerContract::initialize(env.clone(), admin.clone()).unwrap();
        SecurityScannerContract::set_upgrade_authority(
            env.clone(), 
            admin.clone(), 
            upgrade_authority.clone()
        ).unwrap();
        
        // No migration status initially
        assert_eq!(SecurityScannerContract::get_migration_status(env.clone()), None);
        
        // Fast forward time and execute upgrade
        env.ledger().set_timestamp(env.ledger().timestamp() + 604800 + 1);
        
        SecurityScannerContract::propose_upgrade(
            env.clone(),
            upgrade_authority.clone(),
            new_contract.clone(),
            String::from_str(&env, "2.0.0"),
            String::from_str(&env, "Test migration"),
        ).unwrap();
        
        SecurityScannerContract::execute_upgrade(env.clone(), upgrade_authority.clone()).unwrap();
        
        // Check migration status
        let migration_status = SecurityScannerContract::get_migration_status(env.clone());
        assert!(migration_status.is_some());
        let (contract_addr, timestamp) = migration_status.unwrap();
        assert_eq!(contract_addr, new_contract);
        assert!(timestamp > 0);
    }
}
