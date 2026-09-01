use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Env, Symbol, panic_with_error, 
    Map, Vec, BytesN, String, u64, i128, testutils::{Address as TestAddress, Ledger as TestLedger}
};

use crate::scanner_registry::{
    ScannerRegistry, ScannerVersion, VersionStatus, RegistryError,
    ADMIN, VERSION_COUNTER, VERSIONS, LATEST_VERSION, VERSION_REGISTERED, VERSION_DEPRECATED
};

#[contract]
pub struct TestScannerRegistry;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_env() -> (Env, Address) {
        let env = Env::default();
        let admin = Address::random(&env);
        (env, admin)
    }

    fn setup_contract(env: &Env, admin: Address) {
        ScannerRegistry::initialize(env.clone(), admin);
    }

    fn create_test_wasm_hash() -> BytesN<32> {
        BytesN::from_array(&[0u8; 32])
    }

    fn create_test_db_hash() -> BytesN<32> {
        BytesN::from_array(&[1u8; 32])
    }

    #[test]
    fn test_initialize_contract() {
        let (env, admin) = create_test_env();
        
        // Test successful initialization
        ScannerRegistry::initialize(env.clone(), admin);
        
        // Verify admin is set
        let stored_admin = env.storage().instance().get(&ADMIN).unwrap();
        assert_eq!(stored_admin, admin);
        
        // Verify version counter is initialized
        let counter = env.storage().instance().get(&VERSION_COUNTER).unwrap();
        assert_eq!(counter, 0u64);
    }

    #[test]
    fn test_initialize_already_initialized() {
        let (env, admin) = create_test_env();
        
        // Initialize once
        ScannerRegistry::initialize(env.clone(), admin);
        
        // Try to initialize again - should panic
        let result = std::panic::catch_unwind(|| {
            ScannerRegistry::initialize(env.clone(), admin);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_register_version_success() {
        let (env, admin) = create_test_env();
        setup_contract(&env, admin);
        
        let version = String::from_str(&env, "1.0.0");
        let wasm_hash = create_test_wasm_hash();
        let db_hash = create_test_db_hash();
        let changelog = String::from_str(&env, "Initial release");
        let min_protocol = 20u64;

        // Register version as admin
        ScannerRegistry::register_version(
            env.clone(),
            version.clone(),
            wasm_hash,
            db_hash,
            changelog.clone(),
            min_protocol,
        );

        // Verify version was stored
        let stored_version = ScannerRegistry::get_version(env.clone(), version.clone());
        assert_eq!(stored_version.version, version);
        assert_eq!(stored_version.wasm_hash, wasm_hash);
        assert_eq!(stored_version.vulnerability_db_hash, db_hash);
        assert_eq!(stored_version.status, VersionStatus::Active);
        assert_eq!(stored_version.registered_by, admin);
        assert_eq!(stored_version.changelog, changelog);
        assert_eq!(stored_version.min_stellar_protocol, min_protocol);

        // Verify it's set as latest
        let latest = ScannerRegistry::get_latest(env.clone());
        assert_eq!(latest.version, version);
    }

    #[test]
    fn test_register_version_unauthorized() {
        let (env, admin) = create_test_env();
        setup_contract(&env, admin);
        
        let unauthorized = Address::random(&env);
        let version = String::from_str(&env, "1.0.0");
        let wasm_hash = create_test_wasm_hash();
        let db_hash = create_test_db_hash();
        let changelog = String::from_str(&env, "Initial release");
        let min_protocol = 20u64;

        // Try to register version as non-admin - should panic
        let result = std::panic::catch_unwind(|| {
            ScannerRegistry::register_version(
                env.clone(),
                version,
                wasm_hash,
                db_hash,
                changelog,
                min_protocol,
            );
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_register_version_already_exists() {
        let (env, admin) = create_test_env();
        setup_contract(&env, admin);
        
        let version = String::from_str(&env, "1.0.0");
        let wasm_hash = create_test_wasm_hash();
        let db_hash = create_test_db_hash();
        let changelog = String::from_str(&env, "Initial release");
        let min_protocol = 20u64;

        // Register version first time
        ScannerRegistry::register_version(
            env.clone(),
            version.clone(),
            wasm_hash,
            db_hash,
            changelog.clone(),
            min_protocol,
        );

        // Try to register same version again - should panic
        let result = std::panic::catch_unwind(|| {
            ScannerRegistry::register_version(
                env.clone(),
                version,
                wasm_hash,
                db_hash,
                changelog,
                min_protocol,
            );
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_get_version_not_found() {
        let (env, admin) = create_test_env();
        setup_contract(&env, admin);
        
        let version = String::from_str(&env, "1.0.0");

        // Try to get non-existent version - should panic
        let result = std::panic::catch_unwind(|| {
            ScannerRegistry::get_version(env.clone(), version);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_get_latest_no_versions() {
        let (env, admin) = create_test_env();
        setup_contract(&env, admin);

        // Try to get latest when no versions exist - should panic
        let result = std::panic::catch_unwind(|| {
            ScannerRegistry::get_latest(env.clone());
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_deprecate_version_success() {
        let (env, admin) = create_test_env();
        setup_contract(&env, admin);
        
        let version1 = String::from_str(&env, "1.0.0");
        let version2 = String::from_str(&env, "1.1.0");
        let wasm_hash = create_test_wasm_hash();
        let db_hash = create_test_db_hash();
        let changelog = String::from_str(&env, "Initial release");
        let min_protocol = 20u64;

        // Register two versions
        ScannerRegistry::register_version(
            env.clone(),
            version1.clone(),
            wasm_hash,
            db_hash,
            changelog.clone(),
            min_protocol,
        );

        ScannerRegistry::register_version(
            env.clone(),
            version2.clone(),
            wasm_hash,
            db_hash,
            changelog.clone(),
            min_protocol,
        );

        // Deprecate the first version
        let reason = String::from_str(&env, "Old version");
        ScannerRegistry::deprecate_version(env.clone(), version1.clone(), reason.clone());

        // Verify version is deprecated
        let stored_version = ScannerRegistry::get_version(env.clone(), version1.clone());
        assert_eq!(stored_version.status, VersionStatus::Deprecated);
        assert!(stored_version.changelog.contains("DEPRECATED"));
    }

    #[test]
    fn test_deprecate_latest_version() {
        let (env, admin) = create_test_env();
        setup_contract(&env, admin);
        
        let version = String::from_str(&env, "1.0.0");
        let wasm_hash = create_test_wasm_hash();
        let db_hash = create_test_db_hash();
        let changelog = String::from_str(&env, "Initial release");
        let min_protocol = 20u64;

        // Register version
        ScannerRegistry::register_version(
            env.clone(),
            version.clone(),
            wasm_hash,
            db_hash,
            changelog,
            min_protocol,
        );

        // Try to deprecate latest version - should panic
        let reason = String::from_str(&env, "Test deprecation");
        let result = std::panic::catch_unwind(|| {
            ScannerRegistry::deprecate_version(env.clone(), version, reason);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_mark_insecure() {
        let (env, admin) = create_test_env();
        setup_contract(&env, admin);
        
        let version1 = String::from_str(&env, "1.0.0");
        let version2 = String::from_str(&env, "1.1.0");
        let wasm_hash = create_test_wasm_hash();
        let db_hash = create_test_db_hash();
        let changelog = String::from_str(&env, "Initial release");
        let min_protocol = 20u64;

        // Register two versions
        ScannerRegistry::register_version(
            env.clone(),
            version1.clone(),
            wasm_hash,
            db_hash,
            changelog.clone(),
            min_protocol,
        );

        ScannerRegistry::register_version(
            env.clone(),
            version2.clone(),
            wasm_hash,
            db_hash,
            changelog.clone(),
            min_protocol,
        );

        // Mark first version as insecure
        let security_issue = String::from_str(&env, "Critical vulnerability found");
        ScannerRegistry::mark_insecure(env.clone(), version1.clone(), security_issue.clone());

        // Verify version is marked as insecure
        let stored_version = ScannerRegistry::get_version(env.clone(), version1.clone());
        assert_eq!(stored_version.status, VersionStatus::Insecure);
        assert!(stored_version.changelog.contains("INSECURE"));

        // Verify latest version changed to second version
        let latest = ScannerRegistry::get_latest(env.clone());
        assert_eq!(latest.version, version2);
    }

    #[test]
    fn test_verify_latest_wasm() {
        let (env, admin) = create_test_env();
        setup_contract(&env, admin);
        
        let version = String::from_str(&env, "1.0.0");
        let wasm_hash = create_test_wasm_hash();
        let db_hash = create_test_db_hash();
        let changelog = String::from_str(&env, "Initial release");
        let min_protocol = 20u64;

        // Register version
        ScannerRegistry::register_version(
            env.clone(),
            version,
            wasm_hash,
            db_hash,
            changelog,
            min_protocol,
        );

        // Verify correct hash
        assert!(ScannerRegistry::verify_latest_wasm(env.clone(), wasm_hash));

        // Verify incorrect hash
        let wrong_hash = BytesN::from_array(&[2u8; 32]);
        assert!(!ScannerRegistry::verify_latest_wasm(env.clone(), wrong_hash));
    }

    #[test]
    fn test_verify_version_wasm() {
        let (env, admin) = create_test_env();
        setup_contract(&env, admin);
        
        let version = String::from_str(&env, "1.0.0");
        let wasm_hash = create_test_wasm_hash();
        let db_hash = create_test_db_hash();
        let changelog = String::from_str(&env, "Initial release");
        let min_protocol = 20u64;

        // Register version
        ScannerRegistry::register_version(
            env.clone(),
            version.clone(),
            wasm_hash,
            db_hash,
            changelog,
            min_protocol,
        );

        // Verify correct hash
        assert!(ScannerRegistry::verify_version_wasm(env.clone(), version.clone(), wasm_hash));

        // Verify incorrect hash
        let wrong_hash = BytesN::from_array(&[2u8; 32]);
        assert!(!ScannerRegistry::verify_version_wasm(env.clone(), version, wrong_hash));
    }

    #[test]
    fn test_get_active_versions() {
        let (env, admin) = create_test_env();
        setup_contract(&env, admin);
        
        let version1 = String::from_str(&env, "1.0.0");
        let version2 = String::from_str(&env, "1.1.0");
        let version3 = String::from_str(&env, "1.2.0");
        let wasm_hash = create_test_wasm_hash();
        let db_hash = create_test_db_hash();
        let changelog = String::from_str(&env, "Initial release");
        let min_protocol = 20u64;

        // Register three versions
        ScannerRegistry::register_version(
            env.clone(),
            version1.clone(),
            wasm_hash,
            db_hash,
            changelog.clone(),
            min_protocol,
        );

        ScannerRegistry::register_version(
            env.clone(),
            version2.clone(),
            wasm_hash,
            db_hash,
            changelog.clone(),
            min_protocol,
        );

        ScannerRegistry::register_version(
            env.clone(),
            version3.clone(),
            wasm_hash,
            db_hash,
            changelog.clone(),
            min_protocol,
        );

        // All should be active initially
        let active_versions = ScannerRegistry::get_active_versions(env.clone());
        assert_eq!(active_versions.len(), 3);

        // Deprecate one version
        let reason = String::from_str(&env, "Old version");
        ScannerRegistry::deprecate_version(env.clone(), version1.clone(), reason);

        // Should have 2 active versions
        let active_versions = ScannerRegistry::get_active_versions(env.clone());
        assert_eq!(active_versions.len(), 2);

        // Mark one as insecure
        let security_issue = String::from_str(&env, "Security issue");
        ScannerRegistry::mark_insecure(env.clone(), version2.clone(), security_issue);

        // Should have 1 active version
        let active_versions = ScannerRegistry::get_active_versions(env.clone());
        assert_eq!(active_versions.len(), 1);
        assert_eq!(active_versions.get(0).unwrap(), version3);
    }

    #[test]
    fn test_get_registry_stats() {
        let (env, admin) = create_test_env();
        setup_contract(&env, admin);
        
        let version1 = String::from_str(&env, "1.0.0");
        let version2 = String::from_str(&env, "1.1.0");
        let version3 = String::from_str(&env, "1.2.0");
        let wasm_hash = create_test_wasm_hash();
        let db_hash = create_test_db_hash();
        let changelog = String::from_str(&env, "Initial release");
        let min_protocol = 20u64;

        // Register three versions
        ScannerRegistry::register_version(
            env.clone(),
            version1,
            wasm_hash,
            db_hash,
            changelog.clone(),
            min_protocol,
        );

        ScannerRegistry::register_version(
            env.clone(),
            version2,
            wasm_hash,
            db_hash,
            changelog.clone(),
            min_protocol,
        );

        ScannerRegistry::register_version(
            env.clone(),
            version3,
            wasm_hash,
            db_hash,
            changelog.clone(),
            min_protocol,
        );

        // Check initial stats
        let (total, active, deprecated, insecure) = ScannerRegistry::get_registry_stats(env.clone());
        assert_eq!(total, 3);
        assert_eq!(active, 3);
        assert_eq!(deprecated, 0);
        assert_eq!(insecure, 0);

        // Deprecate one version
        let version_to_deprecate = String::from_str(&env, "1.0.0");
        let reason = String::from_str(&env, "Old version");
        ScannerRegistry::deprecate_version(env.clone(), version_to_deprecate, reason);

        let (total, active, deprecated, insecure) = ScannerRegistry::get_registry_stats(env.clone());
        assert_eq!(total, 3);
        assert_eq!(active, 2);
        assert_eq!(deprecated, 1);
        assert_eq!(insecure, 0);

        // Mark one as insecure
        let version_to_mark_insecure = String::from_str(&env, "1.1.0");
        let security_issue = String::from_str(&env, "Security issue");
        ScannerRegistry::mark_insecure(env.clone(), version_to_mark_insecure, security_issue);

        let (total, active, deprecated, insecure) = ScannerRegistry::get_registry_stats(env.clone());
        assert_eq!(total, 3);
        assert_eq!(active, 1);
        assert_eq!(deprecated, 1);
        assert_eq!(insecure, 1);
    }

    #[test]
    fn test_transfer_admin() {
        let (env, admin) = create_test_env();
        setup_contract(&env, admin);
        
        let new_admin = Address::random(&env);

        // Transfer admin rights
        ScannerRegistry::transfer_admin(env.clone(), new_admin);

        // Verify new admin is set
        let stored_admin = env.storage().instance().get(&ADMIN).unwrap();
        assert_eq!(stored_admin, new_admin);

        // Old admin should no longer be able to register versions
        let version = String::from_str(&env, "1.0.0");
        let wasm_hash = create_test_wasm_hash();
        let db_hash = create_test_db_hash();
        let changelog = String::from_str(&env, "Initial release");
        let min_protocol = 20u64;

        let result = std::panic::catch_unwind(|| {
            ScannerRegistry::register_version(
                env.clone(),
                version,
                wasm_hash,
                db_hash,
                changelog,
                min_protocol,
            );
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_update_vulnerability_db() {
        let (env, admin) = create_test_env();
        setup_contract(&env, admin);
        
        let version = String::from_str(&env, "1.0.0");
        let wasm_hash = create_test_wasm_hash();
        let db_hash = create_test_db_hash();
        let changelog = String::from_str(&env, "Initial release");
        let min_protocol = 20u64;

        // Register version
        ScannerRegistry::register_version(
            env.clone(),
            version.clone(),
            wasm_hash,
            db_hash,
            changelog,
            min_protocol,
        );

        // Update vulnerability database hash
        let new_db_hash = BytesN::from_array(&[3u8; 32]);
        ScannerRegistry::update_vulnerability_db(env.clone(), version.clone(), new_db_hash);

        // Verify hash was updated
        let stored_version = ScannerRegistry::get_version(env.clone(), version);
        assert_eq!(stored_version.vulnerability_db_hash, new_db_hash);
    }

    #[test]
    fn test_get_latest_vulnerability_db_hash() {
        let (env, admin) = create_test_env();
        setup_contract(&env, admin);
        
        let version = String::from_str(&env, "1.0.0");
        let wasm_hash = create_test_wasm_hash();
        let db_hash = create_test_db_hash();
        let changelog = String::from_str(&env, "Initial release");
        let min_protocol = 20u64;

        // Register version
        ScannerRegistry::register_version(
            env.clone(),
            version,
            wasm_hash,
            db_hash,
            changelog,
            min_protocol,
        );

        // Get latest vulnerability DB hash
        let latest_db_hash = ScannerRegistry::get_latest_vulnerability_db_hash(env.clone());
        assert_eq!(latest_db_hash, db_hash);
    }

    #[test]
    fn test_version_format_validation() {
        let (env, admin) = create_test_env();
        setup_contract(&env, admin);
        
        let wasm_hash = create_test_wasm_hash();
        let db_hash = create_test_db_hash();
        let changelog = String::from_str(&env, "Initial release");
        let min_protocol = 20u64;

        // Test invalid version formats
        let invalid_versions = vec![
            String::from_str(&env, "1.0"), // Missing patch version
            String::from_str(&env, "1.0.0.0"), // Too many parts
            String::from_str(&env, "1..0"), // Empty part
            String::from_str(&env, "v1.0.0"), // Prefix not allowed
        ];

        for invalid_version in invalid_versions {
            let result = std::panic::catch_unwind(|| {
                ScannerRegistry::register_version(
                    env.clone(),
                    invalid_version,
                    wasm_hash,
                    db_hash,
                    changelog.clone(),
                    min_protocol,
                );
            });
            assert!(result.is_err());
        }

        // Test valid version format
        let valid_version = String::from_str(&env, "1.2.3");
        let result = std::panic::catch_unwind(|| {
            ScannerRegistry::register_version(
                env.clone(),
                valid_version,
                wasm_hash,
                db_hash,
                changelog,
                min_protocol,
            );
        });
        assert!(result.is_ok());
    }
}
