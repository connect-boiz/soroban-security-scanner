use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Env, Symbol, panic_with_error, 
    Map, Vec, String, u64, i128, testutils::{Address as TestAddress, Ledger as TestLedger}
};

use crate::audit_proof_of_scan::{
    AuditProofOfScan, SecurityCertificate, SecurityReport, CertificateStatus, RiskScore, AuditError,
    ADMIN, SCANNER_PUBLIC_KEY, CERTIFICATE_COUNTER, CERTIFICATES, CONTRACT_CERTIFICATES,
    CERTIFICATE_MINTED, CERTIFICATE_REVOKED
};

#[contract]
pub struct TestAuditProofOfScan;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_env() -> (Env, Address, Address) {
        let env = Env::default();
        let admin = Address::random(&env);
        let scanner = Address::random(&env);
        (env, admin, scanner)
    }

    fn setup_contract(env: &Env, admin: Address, scanner: Address) {
        AuditProofOfScan::initialize(env.clone(), admin, scanner);
    }

    fn create_test_security_report(env: &Env, contract_id: Address, risk_score: RiskScore) -> SecurityReport {
        SecurityReport {
            contract_id,
            timestamp: env.ledger().timestamp(),
            risk_score,
            vulnerabilities_found: 0,
            invariants_passed: 10,
            invariants_failed: 0,
            scan_duration: 300, // 5 minutes
            scanner_version: String::from_str(env, "1.0.0"),
            ipfs_cid: String::from_str(env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG"),
        }
    }

    #[test]
    fn test_initialize_contract() {
        let (env, admin, scanner) = create_test_env();
        
        // Test successful initialization
        AuditProofOfScan::initialize(env.clone(), admin, scanner);
        
        // Verify admin and scanner are set
        let stored_admin = env.storage().instance().get(&ADMIN).unwrap();
        let stored_scanner = env.storage().instance().get(&SCANNER_PUBLIC_KEY).unwrap();
        assert_eq!(stored_admin, admin);
        assert_eq!(stored_scanner, scanner);
        
        // Verify certificate counter is initialized
        let counter = env.storage().instance().get(&CERTIFICATE_COUNTER).unwrap();
        assert_eq!(counter, 0u64);
    }

    #[test]
    fn test_initialize_already_initialized() {
        let (env, admin, scanner) = create_test_env();
        
        // Initialize once
        AuditProofOfScan::initialize(env.clone(), admin, scanner);
        
        // Try to initialize again - should panic
        let result = std::panic::catch_unwind(|| {
            AuditProofOfScan::initialize(env.clone(), admin, scanner);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_initialize_invalid_addresses() {
        let (env, admin, scanner) = create_test_env();
        let default_addr = Address::default();
        
        // Test with default admin
        let result = std::panic::catch_unwind(|| {
            AuditProofOfScan::initialize(env.clone(), default_addr, scanner);
        });
        assert!(result.is_err());
        
        // Test with default scanner
        let result = std::panic::catch_unwind(|| {
            AuditProofOfScan::initialize(env.clone(), admin, default_addr);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_mint_certificate_success() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);
        
        let contract_id = Address::random(&env);
        let report = create_test_security_report(&env, contract_id, RiskScore::Low);
        
        // Mint certificate as scanner
        let cert_id = AuditProofOfScan::mint_certificate(
            env.clone(),
            contract_id,
            report.clone(),
            None, // Use default validity period
        );

        // Verify certificate was created
        let certificate = AuditProofOfScan::get_certificate_by_id(env.clone(), cert_id);
        assert_eq!(certificate.certificate_id, cert_id);
        assert_eq!(certificate.contract_id, contract_id);
        assert_eq!(certificate.status, CertificateStatus::Active);
        assert_eq!(certificate.issued_by, scanner);
        assert_eq!(certificate.report.risk_score, RiskScore::Low);

        // Verify contract is cleared
        assert!(AuditProofOfScan::is_contract_cleared(env.clone(), contract_id));
    }

    #[test]
    fn test_mint_certificate_unauthorized() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);
        
        let unauthorized = Address::random(&env);
        let contract_id = Address::random(&env);
        let report = create_test_security_report(&env, contract_id, RiskScore::Low);

        // Try to mint certificate as non-scanner - should panic
        let result = std::panic::catch_unwind(|| {
            AuditProofOfScan::mint_certificate(
                env.clone(),
                contract_id,
                report,
                None,
            );
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_mint_certificate_invalid_risk_score() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);
        
        let contract_id = Address::random(&env);
        let report = create_test_security_report(&env, contract_id, RiskScore::High); // High risk not acceptable

        // Try to mint certificate with unacceptable risk score - should panic
        let result = std::panic::catch_unwind(|| {
            AuditProofOfScan::mint_certificate(
                env.clone(),
                contract_id,
                report,
                None,
            );
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_mint_certificate_already_certified() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);
        
        let contract_id = Address::random(&env);
        let report = create_test_security_report(&env, contract_id, RiskScore::Low);

        // Mint first certificate
        AuditProofOfScan::mint_certificate(
            env.clone(),
            contract_id,
            report.clone(),
            None,
        );

        // Try to mint second certificate for same contract - should panic
        let result = std::panic::catch_unwind(|| {
            AuditProofOfScan::mint_certificate(
                env.clone(),
                contract_id,
                report,
                None,
            );
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_mint_certificate_invalid_ipfs_cid() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);
        
        let contract_id = Address::random(&env);
        let mut report = create_test_security_report(&env, contract_id, RiskScore::Low);
        report.ipfs_cid = String::from_str(&env, "invalid_cid");

        // Try to mint certificate with invalid IPFS CID - should panic
        let result = std::panic::catch_unwind(|| {
            AuditProofOfScan::mint_certificate(
                env.clone(),
                contract_id,
                report,
                None,
            );
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_mint_certificate_custom_validity_period() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);
        
        let contract_id = Address::random(&env);
        let report = create_test_security_report(&env, contract_id, RiskScore::Low);
        
        // Mint certificate with custom validity period (60 days)
        let cert_id = AuditProofOfScan::mint_certificate(
            env.clone(),
            contract_id,
            report.clone(),
            Some(60),
        );

        let certificate = AuditProofOfScan::get_certificate_by_id(env.clone(), cert_id);
        let expected_expiry = certificate.issued_at + (60 * 24 * 60 * 60);
        assert_eq!(certificate.expires_at, expected_expiry);
    }

    #[test]
    fn test_mint_certificate_invalid_validity_period() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);
        
        let contract_id = Address::random(&env);
        let report = create_test_security_report(&env, contract_id, RiskScore::Low);

        // Try to mint with zero validity period - should panic
        let result = std::panic::catch_unwind(|| {
            AuditProofOfScan::mint_certificate(
                env.clone(),
                contract_id,
                report.clone(),
                Some(0),
            );
        });
        assert!(result.is_err());

        // Try to mint with too long validity period - should panic
        let result = std::panic::catch_unwind(|| {
            AuditProofOfScan::mint_certificate(
                env.clone(),
                contract_id,
                report,
                Some(400), // More than 1 year
            );
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_revoke_certificate_success() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);
        
        let contract_id = Address::random(&env);
        let report = create_test_security_report(&env, contract_id, RiskScore::Low);

        // Mint certificate
        let cert_id = AuditProofOfScan::mint_certificate(
            env.clone(),
            contract_id,
            report,
            None,
        );

        // Revoke certificate as admin
        let reason = String::from_str(&env, "Security vulnerability discovered");
        AuditProofOfScan::revoke_certificate(env.clone(), cert_id, reason.clone());

        // Verify certificate is revoked
        let certificate = AuditProofOfScan::get_certificate_by_id(env.clone(), cert_id);
        assert_eq!(certificate.status, CertificateStatus::Revoked);
        assert_eq!(certificate.revoke_reason, Some(reason));

        // Verify contract is no longer cleared
        assert!(!AuditProofOfScan::is_contract_cleared(env.clone(), contract_id));
    }

    #[test]
    fn test_revoke_certificate_unauthorized() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);
        
        let contract_id = Address::random(&env);
        let report = create_test_security_report(&env, contract_id, RiskScore::Low);

        // Mint certificate
        let cert_id = AuditProofOfScan::mint_certificate(
            env.clone(),
            contract_id,
            report,
            None,
        );

        // Try to revoke certificate as unauthorized user - should panic
        let reason = String::from_str(&env, "Unauthorized revocation");
        let result = std::panic::catch_unwind(|| {
            AuditProofOfScan::revoke_certificate(env.clone(), cert_id, reason);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_revoke_certificate_already_revoked() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);
        
        let contract_id = Address::random(&env);
        let report = create_test_security_report(&env, contract_id, RiskScore::Low);

        // Mint certificate
        let cert_id = AuditProofOfScan::mint_certificate(
            env.clone(),
            contract_id,
            report,
            None,
        );

        // Revoke certificate once
        let reason = String::from_str(&env, "First revocation");
        AuditProofOfScan::revoke_certificate(env.clone(), cert_id, reason);

        // Try to revoke again - should panic
        let reason2 = String::from_str(&env, "Second revocation");
        let result = std::panic::catch_unwind(|| {
            AuditProofOfScan::revoke_certificate(env.clone(), cert_id, reason2);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_is_contract_cleared() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);
        
        let contract_id = Address::random(&env);
        let report = create_test_security_report(&env, contract_id, RiskScore::Low);

        // Contract should not be cleared initially
        assert!(!AuditProofOfScan::is_contract_cleared(env.clone(), contract_id));

        // Mint certificate
        AuditProofOfScan::mint_certificate(
            env.clone(),
            contract_id,
            report,
            None,
        );

        // Contract should now be cleared
        assert!(AuditProofOfScan::is_contract_cleared(env.clone(), contract_id));
    }

    #[test]
    fn test_get_contract_certificate() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);
        
        let contract_id = Address::random(&env);
        let report = create_test_security_report(&env, contract_id, RiskScore::Low);

        // Mint certificate
        let cert_id = AuditProofOfScan::mint_certificate(
            env.clone(),
            contract_id,
            report.clone(),
            None,
        );

        // Get certificate for contract
        let certificate = AuditProofOfScan::get_contract_certificate(env.clone(), contract_id);
        assert_eq!(certificate.certificate_id, cert_id);
        assert_eq!(certificate.contract_id, contract_id);
        assert_eq!(certificate.report.risk_score, RiskScore::Low);
    }

    #[test]
    fn test_get_contract_certificate_not_found() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);
        
        let contract_id = Address::random(&env);

        // Try to get certificate for non-certified contract - should panic
        let result = std::panic::catch_unwind(|| {
            AuditProofOfScan::get_contract_certificate(env.clone(), contract_id);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_get_certificate_by_id_not_found() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);

        // Try to get non-existent certificate - should panic
        let result = std::panic::catch_unwind(|| {
            AuditProofOfScan::get_certificate_by_id(env.clone(), 999);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_get_contract_certificate_history() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);
        
        let contract_id = Address::random(&env);
        let report1 = create_test_security_report(&env, contract_id, RiskScore::Low);
        let report2 = create_test_security_report(&env, contract_id, RiskScore::Medium);

        // Mint first certificate
        let cert_id1 = AuditProofOfScan::mint_certificate(
            env.clone(),
            contract_id,
            report1,
            None,
        );

        // Revoke first certificate
        let reason = String::from_str(&env, "Test revocation");
        AuditProofOfScan::revoke_certificate(env.clone(), cert_id1, reason);

        // Mint second certificate
        let cert_id2 = AuditProofOfScan::mint_certificate(
            env.clone(),
            contract_id,
            report2,
            Some(15), // 15 days validity
        );

        // Get certificate history
        let history = AuditProofOfScan::get_contract_certificate_history(env.clone(), contract_id);
        assert_eq!(history.len(), 2);

        // Verify both certificates are in history
        let cert_ids: Vec<u64> = history.iter().map(|cert| cert.certificate_id).collect();
        assert!(cert_ids.contains(&cert_id1));
        assert!(cert_ids.contains(&cert_id2));
    }

    #[test]
    fn test_get_certificate_stats() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);
        
        let contract1 = Address::random(&env);
        let contract2 = Address::random(&env);
        let contract3 = Address::random(&env);
        
        let report_low = create_test_security_report(&env, contract1, RiskScore::Low);
        let report_medium = create_test_security_report(&env, contract2, RiskScore::Medium);
        let report_critical = create_test_security_report(&env, contract3, RiskScore::Critical);

        // Mint two certificates (low and medium risk)
        let cert_id1 = AuditProofOfScan::mint_certificate(
            env.clone(),
            contract1,
            report_low,
            None,
        );

        let cert_id2 = AuditProofOfScan::mint_certificate(
            env.clone(),
            contract2,
            report_medium,
            None,
        );

        // Try to mint with critical risk (should fail, but we'll test stats with 2 certs)
        let result = std::panic::catch_unwind(|| {
            AuditProofOfScan::mint_certificate(
                env.clone(),
                contract3,
                report_critical,
                None,
            );
        });
        assert!(result.is_err());

        // Check initial stats
        let (total, active, revoked, expired) = AuditProofOfScan::get_certificate_stats(env.clone());
        assert_eq!(total, 2);
        assert_eq!(active, 2);
        assert_eq!(revoked, 0);
        assert_eq!(expired, 0);

        // Revoke one certificate
        let reason = String::from_str(&env, "Test revocation");
        AuditProofOfScan::revoke_certificate(env.clone(), cert_id1, reason);

        // Check updated stats
        let (total, active, revoked, expired) = AuditProofOfScan::get_certificate_stats(env.clone());
        assert_eq!(total, 2);
        assert_eq!(active, 1);
        assert_eq!(revoked, 1);
        assert_eq!(expired, 0);
    }

    #[test]
    fn test_transfer_admin() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);
        
        let new_admin = Address::random(&env);

        // Transfer admin rights
        AuditProofOfScan::transfer_admin(env.clone(), new_admin);

        // Verify new admin is set
        let stored_admin = env.storage().instance().get(&ADMIN).unwrap();
        assert_eq!(stored_admin, new_admin);
    }

    #[test]
    fn test_update_scanner_public_key() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);
        
        let new_scanner = Address::random(&env);

        // Update scanner public key
        AuditProofOfScan::update_scanner_public_key(env.clone(), new_scanner);

        // Verify new scanner is set
        let stored_scanner = env.storage().instance().get(&SCANNER_PUBLIC_KEY).unwrap();
        assert_eq!(stored_scanner, new_scanner);
    }

    #[test]
    fn test_transfer_certificate_not_allowed() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);
        
        let contract_id = Address::random(&env);
        let report = create_test_security_report(&env, contract_id, RiskScore::Low);

        // Mint certificate
        let cert_id = AuditProofOfScan::mint_certificate(
            env.clone(),
            contract_id,
            report,
            None,
        );

        // Try to transfer certificate - should always fail (SBT logic)
        let recipient = Address::random(&env);
        let result = std::panic::catch_unwind(|| {
            AuditProofOfScan::transfer_certificate(env.clone(), cert_id, recipient);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_risk_score_acceptability() {
        assert!(RiskScore::Low.is_acceptable());
        assert!(RiskScore::Medium.is_acceptable());
        assert!(!RiskScore::High.is_acceptable());
        assert!(!RiskScore::Critical.is_acceptable());
    }

    #[test]
    fn test_risk_score_as_number() {
        assert_eq!(RiskScore::Low.as_number(), 1);
        assert_eq!(RiskScore::Medium.as_number(), 2);
        assert_eq!(RiskScore::High.as_number(), 3);
        assert_eq!(RiskScore::Critical.as_number(), 4);
    }

    #[test]
    fn test_certificate_expiration() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);
        
        let contract_id = Address::random(&env);
        let report = create_test_security_report(&env, contract_id, RiskScore::Low);

        // Mint certificate with 1 day validity
        let cert_id = AuditProofOfScan::mint_certificate(
            env.clone(),
            contract_id,
            report,
            Some(1), // 1 day
        );

        // Contract should be cleared initially
        assert!(AuditProofOfScan::is_contract_cleared(env.clone(), contract_id));

        // Fast-forward time past expiration
        env.ledger().set_timestamp(
            env.ledger().timestamp() + (24 * 60 * 60) + 1 // 1 day + 1 second
        );

        // Contract should no longer be cleared
        assert!(!AuditProofOfScan::is_contract_cleared(env.clone(), contract_id));

        // Certificate should be marked as expired
        let certificate = AuditProofOfScan::get_certificate_by_id(env.clone(), cert_id);
        assert_eq!(certificate.status, CertificateStatus::Expired);
    }

    #[test]
    fn test_multiple_contracts_certification() {
        let (env, admin, scanner) = create_test_env();
        setup_contract(&env, admin, scanner);
        
        let contract1 = Address::random(&env);
        let contract2 = Address::random(&env);
        let contract3 = Address::random(&env);
        
        let report1 = create_test_security_report(&env, contract1, RiskScore::Low);
        let report2 = create_test_security_report(&env, contract2, RiskScore::Medium);
        let report3 = create_test_security_report(&env, contract3, RiskScore::Low);

        // Certify multiple contracts
        let cert_id1 = AuditProofOfScan::mint_certificate(
            env.clone(),
            contract1,
            report1,
            None,
        );

        let cert_id2 = AuditProofOfScan::mint_certificate(
            env.clone(),
            contract2,
            report2,
            None,
        );

        let cert_id3 = AuditProofOfScan::mint_certificate(
            env.clone(),
            contract3,
            report3,
            Some(90), // 90 days
        );

        // Verify all contracts are cleared
        assert!(AuditProofOfScan::is_contract_cleared(env.clone(), contract1));
        assert!(AuditProofOfScan::is_contract_cleared(env.clone(), contract2));
        assert!(AuditProofOfScan::is_contract_cleared(env.clone(), contract3));

        // Verify certificate details
        let cert1 = AuditProofOfScan::get_contract_certificate(env.clone(), contract1);
        let cert2 = AuditProofOfScan::get_contract_certificate(env.clone(), contract2);
        let cert3 = AuditProofOfScan::get_contract_certificate(env.clone(), contract3);

        assert_eq!(cert1.certificate_id, cert_id1);
        assert_eq!(cert2.certificate_id, cert_id2);
        assert_eq!(cert3.certificate_id, cert_id3);
        assert_eq!(cert1.report.risk_score, RiskScore::Low);
        assert_eq!(cert2.report.risk_score, RiskScore::Medium);
        assert_eq!(cert3.report.risk_score, RiskScore::Low);
    }
}
