//! Scanner Registry & Versioning Smart Contract
//! 
//! This contract acts as the source of truth for "Certified" scanner versions 
//! and vulnerability database hashes. It provides integrity checking and 
//! version management for the Soroban Security Scanner ecosystem.

use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Env, Symbol, panic_with_error, 
    Map, Vec, BytesN, String, u64, i128
};

// Contract state keys
const ADMIN: Symbol = Symbol::short("ADMIN");
const VERSION_COUNTER: Symbol = Symbol::short("VER_CTR");
const VERSIONS: Symbol = Symbol::short("VERSIONS");
const LATEST_VERSION: Symbol = Symbol::short("LATEST");
const VULNERABILITY_DB_HASHES: Symbol = Symbol::short("VULN_DB");

// Version status
#[derive(Clone, Debug, PartialEq, Eq, contracttype)]
pub enum VersionStatus {
    Active,
    Deprecated,
    Insecure,
    Beta,
}

// Scanner version information
#[derive(Clone, Debug, PartialEq, Eq, contracttype)]
pub struct ScannerVersion {
    pub version: String,
    pub wasm_hash: BytesN<32>, // SHA-256 hash of WASM binary
    pub vulnerability_db_hash: BytesN<32>, // SHA-256 hash of vulnerability database
    pub status: VersionStatus,
    pub registered_at: u64, // Unix timestamp
    pub registered_by: Address,
    pub changelog: String,
    pub min_stellar_protocol: u64,
}

// Event topics
const VERSION_REGISTERED: Symbol = Symbol::short("VER_REG");
const VERSION_DEPRECATED: Symbol = Symbol::short("VER_DEP");

// Contract errors
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RegistryError {
    NotAuthorized = 1,
    VersionNotFound = 2,
    InvalidVersionFormat = 3,
    InvalidHash = 4,
    AlreadyExists = 5,
    CannotDeprecateLatest = 6,
    InvalidTimestamp = 7,
}

// Contract implementation
#[contract]
pub struct ScannerRegistry;

#[contractimpl]
impl ScannerRegistry {
    /// Initialize the registry with an admin address
    /// 
    /// # Arguments
    /// * `admin` - The address that can manage versions
    pub fn initialize(env: Env, admin: Address) {
        // Check if already initialized
        if env.storage().instance().has(&ADMIN) {
            panic_with_error!(&env, RegistryError::AlreadyExists);
        }

        // Set admin
        env.storage().instance().set(&ADMIN, &admin);
        
        // Initialize version counter
        env.storage().instance().set(&VERSION_COUNTER, &0u64);
        
        // Initialize empty versions map
        env.storage().instance().set(&VERSIONS, &Map::<String, ScannerVersion>::new(&env));
    }

    /// Register a new scanner version (admin only)
    /// 
    /// # Arguments
    /// * `version` - Semantic version string (e.g., "1.2.3")
    /// * `wasm_hash` - SHA-256 hash of the WASM binary
    /// * `vulnerability_db_hash` - SHA-256 hash of vulnerability database
    /// * `changelog` - Version changelog
    /// * `min_stellar_protocol` - Minimum Stellar protocol version required
    /// 
    /// # Events
    /// Emits VERSION_REGISTERED event with version details
    pub fn register_version(
        env: Env,
        version: String,
        wasm_hash: BytesN<32>,
        vulnerability_db_hash: BytesN<32>,
        changelog: String,
        min_stellar_protocol: u64,
    ) {
        // Check authorization
        let admin = Self::get_admin(&env);
        admin.require_auth();

        // Validate version format (basic semantic versioning)
        Self::validate_version_format(&version);

        // Check if version already exists
        let versions = Self::get_versions(&env);
        if versions.contains_key(&version) {
            panic_with_error!(&env, RegistryError::AlreadyExists);
        }

        // Create new version record
        let new_version = ScannerVersion {
            version: version.clone(),
            wasm_hash,
            vulnerability_db_hash,
            status: VersionStatus::Active,
            registered_at: env.ledger().timestamp(),
            registered_by: admin,
            changelog,
            min_stellar_protocol,
        };

        // Add to versions map
        let mut updated_versions = versions;
        updated_versions.set(version.clone(), new_version);
        env.storage().instance().set(&VERSIONS, &updated_versions);

        // Update version counter
        let mut counter = env.storage().instance().get(&VERSION_COUNTER).unwrap_or(0u64);
        counter += 1;
        env.storage().instance().set(&VERSION_COUNTER, &counter);

        // Set as latest version
        env.storage().instance().set(&LATEST_VERSION, &version);

        // Emit event
        env.events().publish(
            (VERSION_REGISTERED, version.clone()),
            (admin, wasm_hash, vulnerability_db_hash, env.ledger().timestamp()),
        );
    }

    /// Get the latest scanner version
    /// 
    /// # Returns
    /// ScannerVersion of the latest active version
    pub fn get_latest(env: Env) -> ScannerVersion {
        let latest_version = env.storage()
            .instance()
            .get(&LATEST_VERSION)
            .unwrap_or_else(|| panic_with_error!(&env, RegistryError::VersionNotFound));

        let versions = Self::get_versions(&env);
        versions.get(latest_version)
            .unwrap_or_else(|| panic_with_error!(&env, RegistryError::VersionNotFound))
    }

    /// Get a specific version by version string
    /// 
    /// # Arguments
    /// * `version` - Version string to retrieve
    /// 
    /// # Returns
    /// ScannerVersion for the requested version
    pub fn get_version(env: Env, version: String) -> ScannerVersion {
        let versions = Self::get_versions(&env);
        versions.get(version)
            .unwrap_or_else(|| panic_with_error!(&env, RegistryError::VersionNotFound))
    }

    /// Get all versions (for admin use)
    /// 
    /// # Returns
    /// Map of all versions
    pub fn get_all_versions(env: Env) -> Map<String, ScannerVersion> {
        Self::get_versions(&env)
    }

    /// Deprecate a version (admin only)
    /// 
    /// # Arguments
    /// * `version` - Version to deprecate
    /// * `reason` - Reason for deprecation
    /// 
    /// # Events
    /// Emits VERSION_DEPRECATED event
    pub fn deprecate_version(env: Env, version: String, reason: String) {
        // Check authorization
        let admin = Self::get_admin(&env);
        admin.require_auth();

        // Cannot deprecate the latest version
        let latest_version = env.storage()
            .instance()
            .get(&LATEST_VERSION)
            .unwrap_or_else(|| panic_with_error!(&env, RegistryError::VersionNotFound));

        if version == latest_version {
            panic_with_error!(&env, RegistryError::CannotDeprecateLatest);
        }

        // Get and update version
        let mut versions = Self::get_versions(&env);
        let mut scanner_version = versions.get(version.clone())
            .unwrap_or_else(|| panic_with_error!(&env, RegistryError::VersionNotFound));

        scanner_version.status = VersionStatus::Deprecated;
        scanner_version.changelog = format!("DEPRECATED: {} - {}", reason, scanner_version.changelog);

        versions.set(version.clone(), scanner_version);
        env.storage().instance().set(&VERSIONS, &versions);

        // Emit event
        env.events().publish(
            (VERSION_DEPRECATED, version.clone()),
            (admin, reason, env.ledger().timestamp()),
        );
    }

    /// Mark a version as insecure (admin only)
    /// 
    /// # Arguments
    /// * `version` - Version to mark as insecure
    /// * `security_issue` - Description of the security issue
    pub fn mark_insecure(env: Env, version: String, security_issue: String) {
        // Check authorization
        let admin = Self::get_admin(&env);
        admin.require_auth();

        // Get and update version
        let mut versions = Self::get_versions(&env);
        let mut scanner_version = versions.get(version.clone())
            .unwrap_or_else(|| panic_with_error!(&env, RegistryError::VersionNotFound));

        scanner_version.status = VersionStatus::Insecure;
        scanner_version.changelog = format!("INSECURE: {} - {}", security_issue, scanner_version.changelog);

        versions.set(version.clone(), scanner_version);
        env.storage().instance().set(&VERSIONS, &versions);

        // If this was the latest version, we need to set a new latest
        let latest_version = env.storage()
            .instance()
            .get(&LATEST_VERSION)
            .unwrap_or_else(|| panic_with_error!(&env, RegistryError::VersionNotFound));

        if version == latest_version {
            // Find the most recent active version
            let mut latest_active = String::new();
            let mut latest_timestamp = 0u64;

            for (ver, info) in versions.iter() {
                if info.status == VersionStatus::Active && info.registered_at > latest_timestamp {
                    latest_active = ver;
                    latest_timestamp = info.registered_at;
                }
            }

            if !latest_active.is_empty() {
                env.storage().instance().set(&LATEST_VERSION, &latest_active);
            }
        }

        // Emit deprecation event with security context
        env.events().publish(
            (VERSION_DEPRECATED, version.clone()),
            (admin, format!("SECURITY: {}", security_issue), env.ledger().timestamp()),
        );
    }

    /// Update vulnerability database hash for a version (admin only)
    /// 
    /// # Arguments
    /// * `version` - Version to update
    /// * `new_db_hash` - New vulnerability database hash
    pub fn update_vulnerability_db(env: Env, version: String, new_db_hash: BytesN<32>) {
        // Check authorization
        let admin = Self::get_admin(&env);
        admin.require_auth();

        // Get and update version
        let mut versions = Self::get_versions(&env);
        let mut scanner_version = versions.get(version.clone())
            .unwrap_or_else(|| panic_with_error!(&env, RegistryError::VersionNotFound));

        scanner_version.vulnerability_db_hash = new_db_hash;
        versions.set(version, scanner_version);
        env.storage().instance().set(&VERSIONS, &versions);
    }

    /// Get vulnerability database hash for latest version
    /// 
    /// # Returns
    /// SHA-256 hash of the latest vulnerability database
    pub fn get_latest_vulnerability_db_hash(env: Env) -> BytesN<32> {
        let latest = Self::get_latest(env);
        latest.vulnerability_db_hash
    }

    /// Verify if a WASM hash matches the latest version
    /// 
    /// # Arguments
    /// * `wasm_hash` - Hash to verify
    /// 
    /// # Returns
    /// Boolean indicating if the hash matches the latest version
    pub fn verify_latest_wasm(env: Env, wasm_hash: BytesN<32>) -> bool {
        let latest = Self::get_latest(env);
        latest.wasm_hash == wasm_hash
    }

    /// Verify if a WASM hash matches a specific version
    /// 
    /// # Arguments
    /// * `version` - Version to check against
    /// * `wasm_hash` - Hash to verify
    /// 
    /// # Returns
    /// Boolean indicating if the hash matches the version
    pub fn verify_version_wasm(env: Env, version: String, wasm_hash: BytesN<32>) -> bool {
        let scanner_version = Self::get_version(env, version);
        scanner_version.wasm_hash == wasm_hash
    }

    /// Get all active versions
    /// 
    /// # Returns
    /// Vector of active version strings
    pub fn get_active_versions(env: Env) -> Vec<String> {
        let versions = Self::get_versions(&env);
        let mut active_versions = Vec::new(&env);

        for (version, info) in versions.iter() {
            if info.status == VersionStatus::Active {
                active_versions.push_back(version);
            }
        }

        active_versions
    }

    /// Get registry statistics
    /// 
    /// # Returns
    /// Tuple of (total_versions, active_versions, deprecated_versions, insecure_versions)
    pub fn get_registry_stats(env: Env) -> (u64, u64, u64, u64) {
        let versions = Self::get_versions(&env);
        let mut total = 0u64;
        let mut active = 0u64;
        let mut deprecated = 0u64;
        let mut insecure = 0u64;

        for (_, info) in versions.iter() {
            total += 1;
            match info.status {
                VersionStatus::Active => active += 1,
                VersionStatus::Deprecated => deprecated += 1,
                VersionStatus::Insecure => insecure += 1,
                VersionStatus::Beta => active += 1, // Count beta as active for stats
            }
        }

        (total, active, deprecated, insecure)
    }

    /// Transfer admin rights (admin only)
    /// 
    /// # Arguments
    /// * `new_admin` - Address of the new admin
    pub fn transfer_admin(env: Env, new_admin: Address) {
        let admin = Self::get_admin(&env);
        admin.require_auth();

        env.storage().instance().set(&ADMIN, &new_admin);
    }

    // Helper functions

    fn get_admin(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&ADMIN)
            .unwrap_or_else(|| panic_with_error!(env, RegistryError::NotAuthorized))
    }

    fn get_versions(env: &Env) -> Map<String, ScannerVersion> {
        env.storage()
            .instance()
            .get(&VERSIONS)
            .unwrap_or_else(|| Map::new(env))
    }

    fn validate_version_format(version: &String) {
        // Basic semantic versioning validation
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            panic_with_error!(soroban_sdk::Env::default(), RegistryError::InvalidVersionFormat);
        }

        for part in parts {
            if part.is_empty() {
                panic_with_error!(soroban_sdk::Env::default(), RegistryError::InvalidVersionFormat);
            }
        }
    }
}
