//! Decentralized Audit "Proof of Scan" Contract
//! 
//! This contract allows the scanner to issue "Security Certificates" on-chain 
//! after a contract passes all security invariants. Certificates are non-transferable 
//! SBTs (Soulbound Tokens) that provide proof of security clearance.

use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Env, Symbol, panic_with_error, 
    Map, Vec, String, u64, i128
};

// Contract state keys
const ADMIN: Symbol = Symbol::short("ADMIN");
const SCANNER_PUBLIC_KEY: Symbol = Symbol::short("SCANNER");
const CERTIFICATE_COUNTER: Symbol = Symbol::short("CERT_C");
const CERTIFICATES: Symbol = Symbol::short("CERTS");
const CONTRACT_CERTIFICATES: Symbol = Symbol::short("CONTRACT_CERTS");
const CLEANUP_TIMESTAMP: Symbol = Symbol::short("CLEANUP_TS");
const VALIDITY_PERIOD: u64 = 30 * 24 * 60 * 60; // 30 days in seconds
const MAX_CERTIFICATES_PER_CONTRACT: u64 = 10; // Limit history per contract
const CERTIFICATE_RETENTION_DAYS: u64 = 90; // Keep certificates for 90 days

// Certificate status
#[derive(Clone, Debug, PartialEq, Eq, contracttype)]
pub enum CertificateStatus {
    Active,
    Revoked,
    Expired,
}

// Risk score levels
#[derive(Clone, Debug, PartialEq, Eq, contracttype)]
pub enum RiskScore {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskScore {
    pub fn as_number(&self) -> u8 {
        match self {
            RiskScore::Low => 1,
            RiskScore::Medium => 2,
            RiskScore::High => 3,
            RiskScore::Critical => 4,
        }
    }
    
    pub fn is_acceptable(&self) -> bool {
        match self {
            RiskScore::Low | RiskScore::Medium => true,
            RiskScore::High | RiskScore::Critical => false,
        }
    }
}

// Security report structure
#[derive(Clone, Debug, PartialEq, Eq, contracttype)]
pub struct SecurityReport {
    pub contract_id: Address,
    pub timestamp: u64,
    pub risk_score: RiskScore,
    pub vulnerabilities_found: u64,
    pub invariants_passed: u64,
    pub invariants_failed: u64,
    pub scan_duration: u64, // Scan duration in seconds
    pub scanner_version: String,
    pub ipfs_cid: String, // IPFS CID for full detailed report
}

// Security certificate structure (SBT - Soulbound Token)
#[derive(Clone, Debug, PartialEq, Eq, contracttype)]
pub struct SecurityCertificate {
    pub certificate_id: u64,
    pub contract_id: Address,
    pub report: SecurityReport,
    pub status: CertificateStatus,
    pub issued_at: u64,
    pub expires_at: u64,
    pub issued_by: Address, // Scanner public key
    pub revoked_at: Option<u64>,
    pub revoke_reason: Option<String>,
}

// Event topics
const CERTIFICATE_MINTED: Symbol = Symbol::short("CERT_MINT");
const CERTIFICATE_REVOKED: Symbol = Symbol::short("CERT_REV");

// Contract errors
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AuditError {
    NotAuthorized = 1,
    CertificateNotFound = 2,
    CertificateExpired = 3,
    CertificateRevoked = 4,
    InvalidRiskScore = 5,
    InvalidIPFSCID = 6,
    AlreadyCertified = 7,
    NotCertified = 8,
    InvalidContract = 9,
    TransferNotAllowed = 10,
    ScannerNotAuthorized = 11,
    InvalidValidityPeriod = 12,
}

// Contract implementation
#[contract]
pub struct AuditProofOfScan;

#[contractimpl]
impl AuditProofOfScan {
    /// Initialize the contract with admin and scanner public key
    /// 
    /// # Arguments
    /// * `admin` - The contract administrator
    /// * `scanner_public_key` - The scanner's authorized public key
    pub fn initialize(env: Env, admin: Address, scanner_public_key: Address) {
        // Check if already initialized
        if env.storage().instance().has(&ADMIN) {
            panic_with_error!(&env, AuditError::AlreadyCertified);
        }

        // Validate addresses
        if admin == Address::default() || scanner_public_key == Address::default() {
            panic_with_error!(&env, AuditError::InvalidContract);
        }

        // Set admin and scanner
        env.storage().instance().set(&ADMIN, &admin);
        env.storage().instance().set(&SCANNER_PUBLIC_KEY, &scanner_public_key);
        
        // Initialize certificate counter
        env.storage().instance().set(&CERTIFICATE_COUNTER, &0u64);
        
        // Initialize empty storage maps
        env.storage().instance().set(&CERTIFICATES, &Map::<u64, SecurityCertificate>::new(&env));
        env.storage().instance().set(&CONTRACT_CERTIFICATES, &Map::<Address, u64>::new(&env));
        env.storage().instance().set(&CLEANUP_TIMESTAMP, &env.ledger().timestamp());
    }

    /// Mint a security certificate (scanner only)
    /// 
    /// # Arguments
    /// * `contract_id` - The contract being certified
    /// * `report` - Security report with scan results
    /// * `validity_days` - Custom validity period (optional, defaults to 30 days)
    /// 
    /// # Events
    /// Emits CERTIFICATE_MINTED event with certificate details
    pub fn mint_certificate(
        env: Env,
        contract_id: Address,
        report: SecurityReport,
        validity_days: Option<u64>,
    ) -> u64 {
        // Check scanner authorization
        let scanner_key = Self::get_scanner_public_key(&env);
        scanner_key.require_auth();

        // Validate contract address
        if contract_id == Address::default() {
            panic_with_error!(&env, AuditError::InvalidContract);
        }

        // Validate IPFS CID
        Self::validate_ipfs_cid(&report.ipfs_cid);

        // Validate risk score acceptability
        if !report.risk_score.is_acceptable() {
            panic_with_error!(&env, AuditError::InvalidRiskScore);
        }

        // Check if contract already has active certificate
        let contract_certs = Self::get_contract_certificates(&env);
        if let Some(&existing_cert_id) = contract_certs.get(&contract_id) {
            let existing_cert = Self::get_certificate(&env, existing_cert_id);
            if existing_cert.status == CertificateStatus::Active && !Self::is_expired(&env, existing_cert) {
                panic_with_error!(&env, AuditError::AlreadyCertified);
            }
        }

        // Calculate validity period
        let validity_period = validity_days.unwrap_or(30) * 24 * 60 * 60; // Convert days to seconds
        if validity_period == 0 || validity_period > 365 * 24 * 60 * 60 { // Max 1 year
            panic_with_error!(&env, AuditError::InvalidValidityPeriod);
        }

        // Generate certificate ID
        let mut counter = env.storage().instance().get(&CERTIFICATE_COUNTER).unwrap_or(0u64);
        counter += 1;
        let certificate_id = counter;

        // Create certificate
        let now = env.ledger().timestamp();
        let certificate = SecurityCertificate {
            certificate_id,
            contract_id,
            report: report.clone(),
            status: CertificateStatus::Active,
            issued_at: now,
            expires_at: now + validity_period,
            issued_by: scanner_key,
            revoked_at: None,
            revoke_reason: None,
        };

        // Store certificate
        let mut certificates = Self::get_certificates(&env);
        certificates.set(certificate_id, certificate.clone());
        env.storage().instance().set(&CERTIFICATES, &certificates);

        // Update contract certificates mapping
        let mut contract_certs = Self::get_contract_certificates(&env);
        contract_certs.set(contract_id, certificate_id);
        env.storage().instance().set(&CONTRACT_CERTIFICATES, &contract_certs);

        // Update counter
        env.storage().instance().set(&CERTIFICATE_COUNTER, &counter);

        // Emit event
        env.events().publish(
            (CERTIFICATE_MINTED, certificate_id),
            (contract_id, report.risk_score.as_number(), now, now + validity_period),
        );

        certificate_id
    }

    /// Revoke a certificate (admin or scanner only)
    /// 
    /// # Arguments
    /// * `certificate_id` - The certificate to revoke
    /// * `reason` - Reason for revocation
    /// 
    /// # Events
    /// Emits CERTIFICATE_REVOKED event
    pub fn revoke_certificate(env: Env, certificate_id: u64, reason: String) {
        // Check authorization (admin or scanner)
        let admin = Self::get_admin(&env);
        let scanner = Self::get_scanner_public_key(&env);
        
        let caller = env.current_contract_address();
        if caller != admin && caller != scanner {
            panic_with_error!(&env, AuditError::NotAuthorized);
        }

        // Get certificate
        let mut certificate = Self::get_certificate(&env, certificate_id);

        // Check if already revoked
        if certificate.status == CertificateStatus::Revoked {
            panic_with_error!(&env, AuditError::CertificateRevoked);
        }

        // Revoke certificate
        certificate.status = CertificateStatus::Revoked;
        certificate.revoked_at = Some(env.ledger().timestamp());
        certificate.revoke_reason = Some(reason.clone());

        // Update certificate
        let mut certificates = Self::get_certificates(&env);
        certificates.set(certificate_id, certificate.clone());
        env.storage().instance().set(&CERTIFICATES, &certificates);

        // Emit event
        env.events().publish(
            (CERTIFICATE_REVOKED, certificate_id),
            (certificate.contract_id, reason, env.ledger().timestamp()),
        );
    }

    /// Check if a contract is "Cleared" (has active certificate)
    /// 
    /// # Arguments
    /// * `contract_id` - The contract to check
    /// 
    /// # Returns
    /// Boolean indicating if the contract is cleared
    pub fn is_contract_cleared(env: Env, contract_id: Address) -> bool {
        let contract_certs = Self::get_contract_certificates(&env);
        
        if let Some(&cert_id) = contract_certs.get(&contract_id) {
            let certificate = Self::get_certificate(&env, cert_id);
            certificate.status == CertificateStatus::Active && !Self::is_expired(&env, certificate)
        } else {
            false
        }
    }

    /// Get certificate details for a contract
    /// 
    /// # Arguments
    /// * `contract_id` - The contract to query
    /// 
    /// # Returns
    /// SecurityCertificate if found and active
    pub fn get_contract_certificate(env: Env, contract_id: Address) -> SecurityCertificate {
        let contract_certs = Self::get_contract_certificates(&env);
        
        if let Some(&cert_id) = contract_certs.get(&contract_id) {
            let certificate = Self::get_certificate(&env, cert_id);
            
            // Check if certificate is still valid
            if certificate.status == CertificateStatus::Active && !Self::is_expired(&env, certificate) {
                return certificate;
            } else if Self::is_expired(&env, certificate) {
                // Auto-expire certificate
                Self::expire_certificate(&env, cert_id);
            }
        }
        
        panic_with_error!(&env, AuditError::NotCertified);
    }

    /// Get certificate by ID
    /// 
    /// # Arguments
    /// * `certificate_id` - The certificate ID
    /// 
    /// # Returns
    /// SecurityCertificate details
    pub fn get_certificate_by_id(env: Env, certificate_id: u64) -> SecurityCertificate {
        Self::get_certificate(&env, certificate_id)
    }

    /// Get all certificates for a contract (including historical)
    /// 
    /// # Arguments
    /// * `contract_id` - The contract to query
    /// 
    /// # Returns
    /// Vector of all certificates for the contract
    pub fn get_contract_certificate_history(env: Env, contract_id: Address) -> Vec<SecurityCertificate> {
        let mut history = Vec::new(&env);
        
        // For now, we'll search through all certificates
        // In a production system, you might want a more efficient indexing
        let certificates = Self::get_certificates(&env);
        for (_, cert) in certificates.iter() {
            if cert.contract_id == contract_id {
                history.push_back(cert);
            }
        }
        
        history
    }

    /// Get certificate statistics
    /// 
    /// # Returns
    /// Tuple of (total_certificates, active_certificates, revoked_certificates, expired_certificates)
    pub fn get_certificate_stats(env: Env) -> (u64, u64, u64, u64) {
        let certificates = Self::get_certificates(&env);
        let mut total = 0u64;
        let mut active = 0u64;
        let mut revoked = 0u64;
        let mut expired = 0u64;

        for (_, cert) in certificates.iter() {
            total += 1;
            
            match cert.status {
                CertificateStatus::Active => {
                    if Self::is_expired(&env, cert) {
                        expired += 1;
                    } else {
                        active += 1;
                    }
                },
                CertificateStatus::Revoked => revoked += 1,
                CertificateStatus::Expired => expired += 1,
            }
        }

        (total, active, revoked, expired)
    }

    /// Update scanner public key (admin only)
    /// 
    /// # Arguments
    /// * `new_scanner_public_key` - New scanner public key
    pub fn update_scanner_public_key(env: Env, new_scanner_public_key: Address) {
        let admin = Self::get_admin(&env);
        admin.require_auth();

        if new_scanner_public_key == Address::default() {
            panic_with_error!(&env, AuditError::InvalidContract);
        }

        env.storage().instance().set(&SCANNER_PUBLIC_KEY, &new_scanner_public_key);
    }

    /// Transfer admin rights (admin only)
    /// 
    /// # Arguments
    /// * `new_admin` - Address of the new admin
    pub fn transfer_admin(env: Env, new_admin: Address) {
        let admin = Self::get_admin(&env);
        admin.require_auth();

        if new_admin == Address::default() {
            panic_with_error!(&env, AuditError::InvalidContract);
        }

        env.storage().instance().set(&ADMIN, &new_admin);
    }

    /// Attempt to transfer certificate (should always fail - SBT logic)
    /// 
    /// # Arguments
    /// * `certificate_id` - The certificate to "transfer"
    /// * `to` - The recipient address
    /// 
    /// # Note
    /// This function always panics as certificates are non-transferable
    pub fn transfer_certificate(env: Env, certificate_id: u64, to: Address) {
        // SBTs are non-transferable
        panic_with_error!(&env, AuditError::TransferNotAllowed);
    }

    // Helper functions

    fn get_admin(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&ADMIN)
            .unwrap_or_else(|| panic_with_error!(env, AuditError::NotAuthorized))
    }

    fn get_scanner_public_key(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&SCANNER_PUBLIC_KEY)
            .unwrap_or_else(|| panic_with_error!(env, AuditError::ScannerNotAuthorized))
    }

    fn get_certificates(env: &Env) -> Map<u64, SecurityCertificate> {
        env.storage()
            .instance()
            .get(&CERTIFICATES)
            .unwrap_or_else(|| Map::new(env))
    }

    fn get_contract_certificates(env: &Env) -> Map<Address, u64> {
        env.storage()
            .instance()
            .get(&CONTRACT_CERTIFICATES)
            .unwrap_or_else(|| Map::new(env))
    }

    fn get_certificate(env: &Env, certificate_id: u64) -> SecurityCertificate {
        let certificates = Self::get_certificates(env);
        certificates.get(certificate_id)
            .unwrap_or_else(|| panic_with_error!(env, AuditError::CertificateNotFound))
    }

    fn is_expired(env: &Env, certificate: &SecurityCertificate) -> bool {
        env.ledger().timestamp() > certificate.expires_at
    }

    fn expire_certificate(env: &Env, certificate_id: u64) {
        let mut certificate = Self::get_certificate(env, certificate_id);
        certificate.status = CertificateStatus::Expired;
        
        let mut certificates = Self::get_certificates(env);
        certificates.set(certificate_id, certificate);
        env.storage().instance().set(&CERTIFICATES, &certificates);
    }

    /// Clean up old and expired certificates to optimize storage
    pub fn cleanup_certificates(env: Env) -> u64 {
        let now = env.ledger().timestamp();
        let retention_seconds = CERTIFICATE_RETENTION_DAYS * 24 * 60 * 60;
        let cutoff_time = now.saturating_sub(retention_seconds);
        
        let mut cleaned_count = 0u64;
        
        // Get all certificates
        let mut certificates = Self::get_certificates(&env);
        let mut contract_certs = Self::get_contract_certificates(&env);
        
        // Find expired and old certificates
        let expired_certificates: Vec<u64> = certificates.iter()
            .filter(|(_, cert)| {
                cert.status == CertificateStatus::Expired || 
                cert.status == CertificateStatus::Revoked ||
                cert.timestamp < cutoff_time
            })
            .map(|(cert_id, _)| cert_id)
            .collect();
        
        // Remove expired/old certificates
        for cert_id in expired_certificates {
            let certificate = certificates.get(cert_id).unwrap();
            
            // Remove from certificates map
            certificates.remove(cert_id);
            
            // Remove from contract certificates mapping if this was the current certificate
            if let Some(&current_cert_id) = contract_certs.get(&certificate.contract_id) {
                if current_cert_id == cert_id {
                    contract_certs.remove(certificate.contract_id);
                }
            }
            
            cleaned_count += 1;
        }
        
        // Enforce maximum certificates per contract
        let mut contract_cert_counts: Map<Address, u64> = Map::new(&env);
        for cert in certificates.iter() {
            let contract_id = cert.1.contract_id;
            let count = contract_cert_counts.get(contract_id).unwrap_or(0) + 1;
            contract_cert_counts.set(contract_id, count);
        }
        
        // Remove excess certificates per contract
        for (contract_id, count) in contract_cert_counts.iter() {
            if count > MAX_CERTIFICATES_PER_CONTRACT {
                // Get all certificates for this contract
                let contract_certs_list: Vec<(u64, &SecurityCertificate)> = certificates.iter()
                    .filter(|(_, cert)| cert.contract_id == contract_id)
                    .collect();
                
                // Sort by timestamp (oldest first)
                let mut sorted_certs = contract_certs_list;
                sorted_certs.sort_by_key(|(_, cert)| cert.timestamp);
                
                // Keep only the most recent ones
                let excess_count = count - MAX_CERTIFICATES_PER_CONTRACT;
                for i in 0..excess_count {
                    let cert_id = sorted_certs[i].0;
                    certificates.remove(cert_id);
                    cleaned_count += 1;
                }
            }
        }
        
        // Update storage
        env.storage().instance().set(&CERTIFICATES, &certificates);
        env.storage().instance().set(&CONTRACT_CERTIFICATES, &contract_certs);
        env.storage().instance().set(&CLEANUP_TIMESTAMP, &now);
        
        cleaned_count
    }

    /// Get certificate storage statistics
    pub fn get_storage_stats(env: Env) -> CertificateStorageStats {
        let certificates = Self::get_certificates(&env);
        let contract_certs = Self::get_contract_certificates(&env);
        
        let mut active_count = 0u64;
        let mut expired_count = 0u64;
        let mut revoked_count = 0u64;
        
        for cert in certificates.iter() {
            match cert.1.status {
                CertificateStatus::Active => active_count += 1,
                CertificateStatus::Expired => expired_count += 1,
                CertificateStatus::Revoked => revoked_count += 1,
            }
        }
        
        CertificateStorageStats {
            total_certificates: certificates.len(),
            active_certificates: active_count,
            expired_certificates: expired_count,
            revoked_certificates: revoked_count,
            contracts_with_certificates: contract_certs.len(),
            last_cleanup: env.storage().instance().get(&CLEANUP_TIMESTAMP).unwrap_or(0),
        }
    }

    fn validate_ipfs_cid(ipfs_cid: &String) {
        // Basic IPFS CID validation
        if ipfs_cid.is_empty() || ipfs_cid.len() < 10 {
            panic_with_error!(soroban_sdk::Env::default(), AuditError::InvalidIPFSCID);
        }
        
        // Check if it starts with "Qm" (CIDv0) or "b" (CIDv1)
        let chars = ipfs_cid.chars().collect::<Vec<char>>();
        if !(chars[0] == 'Q' && chars[1] == 'm') && chars[0] != 'b' {
            panic_with_error!(soroban_sdk::Env::default(), AuditError::InvalidIPFSCID);
        }
    }
}

// Certificate storage statistics
#[derive(Clone, Debug, contracttype)]
pub struct CertificateStorageStats {
    pub total_certificates: u64,
    pub active_certificates: u64,
    pub expired_certificates: u64,
    pub revoked_certificates: u64,
    pub contracts_with_certificates: u64,
    pub last_cleanup: u64,
}
