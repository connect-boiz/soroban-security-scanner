//! Address Filtering and Management
//!
//! Provides whitelist/blacklist support for addresses, allowing users to:
//! - Whitelist trusted addresses to skip during scanning
//! - Blacklist known malicious addresses
//! - Configure address lists per project or globally
//! - Import/export address lists in multiple formats

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::fs;
use anyhow::{Result, anyhow};
use regex::Regex;
use chrono::{DateTime, Utc};

/// Supported address formats on Stellar/Soroban
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AddressFormat {
    /// Stellar Classic Address (G...)
    StellarClassic,
    /// Stellar Contract Address (C...)
    StellarContract,
    /// Soroban Contract Address
    SorobanContract,
    /// SHA256 Hash
    Sha256Hash,
    /// Public Key
    PublicKey,
    /// Secret Key (masked)
    SecretKey,
    /// Generic (any format)
    Generic,
}

/// Address category for classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AddressCategory {
    /// Known safe/trusted addresses
    Trusted,
    /// Known malicious addresses
    Malicious,
    /// Test/development addresses
    Test,
    /// Exchange addresses
    Exchange,
    /// Contract deployer addresses
    Deployer,
    /// Protocol treasury addresses
    Treasury,
    /// User/customer addresses
    User,
    /// Smart contract addresses
    Contract,
    /// Multi-signature wallet addresses
    MultiSig,
    /// Other/unknown category
    Other(String),
}

/// Address entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressEntry {
    /// The address value
    pub address: String,
    /// Address format type
    pub format: AddressFormat,
    /// Address category
    pub category: AddressCategory,
    /// Description of the address
    pub description: String,
    /// Source of this entry (e.g., "manual", "contract_scan", "import")
    pub source: String,
    /// Tags for custom classification
    pub tags: Vec<String>,
    /// When this entry was added
    pub added_at: DateTime<Utc>,
    /// Optional expiration date
    pub expires_at: Option<DateTime<Utc>>,
    /// Whether this entry is active
    pub active: bool,
    /// Optional metadata
    pub metadata: HashMap<String, String>,
}

impl AddressEntry {
    /// Create a new address entry
    pub fn new(
        address: String,
        format: AddressFormat,
        category: AddressCategory,
        description: String,
    ) -> Self {
        Self {
            address,
            format,
            category,
            description,
            source: "manual".to_string(),
            tags: Vec::new(),
            added_at: Utc::now(),
            expires_at: None,
            active: true,
            metadata: HashMap::new(),
        }
    }

    /// Check if this entry is currently valid (not expired and active)
    pub fn is_valid(&self) -> bool {
        self.active && self.expires_at.map_or(true, |exp| exp > Utc::now())
    }

    /// Set expiration date
    pub fn set_expiration(&mut self, expires_at: DateTime<Utc>) {
        self.expires_at = Some(expires_at);
    }

    /// Add a tag to this entry
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Remove a tag from this entry
    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
    }

    /// Update metadata
    pub fn update_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
}

/// Address filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressFilterConfig {
    /// Whether to enable address filtering
    pub enabled: bool,
    /// Whitelist of addresses to always allow
    pub whitelist_paths: Vec<PathBuf>,
    /// Blacklist of addresses to always block
    pub blacklist_paths: Vec<PathBuf>,
    /// Default action for addresses not in lists
    pub default_action: FilterAction,
    /// Whether to log filtered addresses
    pub log_filtered: bool,
    /// Whether to validate Stellar addresses
    pub validate_stellar_addresses: bool,
    /// Categories to whitelist automatically
    pub auto_whitelist_categories: Vec<AddressCategory>,
    /// Categories to blacklist automatically
    pub auto_blacklist_categories: Vec<AddressCategory>,
}

impl Default for AddressFilterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            whitelist_paths: vec![],
            blacklist_paths: vec![],
            default_action: FilterAction::Skip,
            log_filtered: true,
            validate_stellar_addresses: true,
            auto_whitelist_categories: vec![AddressCategory::Test],
            auto_blacklist_categories: vec![AddressCategory::Malicious],
        }
    }
}

/// Action to take when an address matches a filter
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FilterAction {
    /// Allow/skip scanning this address
    Allow,
    /// Block/flag this address
    Block,
    /// Skip this address entirely
    Skip,
    /// Require manual review
    Review,
}

/// Result of address filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterResult {
    /// The address that was checked
    pub address: String,
    /// The action taken
    pub action: FilterAction,
    /// The list type that matched (whitelist/blacklist)
    pub list_type: ListType,
    /// The matching entry (if any)
    pub matching_entry: Option<AddressEntry>,
    /// Timestamp of the check
    pub checked_at: DateTime<Utc>,
}

/// Type of list that matched
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ListType {
    /// Matched whitelist
    Whitelist,
    /// Matched blacklist
    Blacklist,
    /// No match found
    None,
}

/// Main address filter manager
#[derive(Debug, Clone)]
pub struct AddressFilter {
    /// Whitelisted addresses
    whitelist: HashSet<String>,
    /// Blacklisted addresses
    blacklist: HashSet<String>,
    /// Full address entries with metadata
    entries: HashMap<String, AddressEntry>,
    /// Filter configuration
    config: AddressFilterConfig,
    /// Address patterns for regex matching
    patterns: Vec<(Regex, FilterAction)>,
}

impl AddressFilter {
    /// Create a new address filter with default config
    pub fn new() -> Self {
        Self {
            whitelist: HashSet::new(),
            blacklist: HashSet::new(),
            entries: HashMap::new(),
            config: AddressFilterConfig::default(),
            patterns: Vec::new(),
        }
    }

    /// Create a new address filter with custom config
    pub fn with_config(config: AddressFilterConfig) -> Self {
        Self {
            whitelist: HashSet::new(),
            blacklist: HashSet::new(),
            entries: HashMap::new(),
            config,
            patterns: Vec::new(),
        }
    }

    /// Add an address to the whitelist
    pub fn add_to_whitelist(&mut self, entry: AddressEntry) -> Result<()> {
        if !entry.is_valid() {
            return Err(anyhow!("Cannot add expired address to whitelist"));
        }
        self.whitelist.insert(entry.address.clone());
        self.entries.insert(entry.address.clone(), entry);
        Ok(())
    }

    /// Add an address to the blacklist
    pub fn add_to_blacklist(&mut self, entry: AddressEntry) -> Result<()> {
        if !entry.is_valid() {
            return Err(anyhow!("Cannot add expired address to blacklist"));
        }
        self.blacklist.insert(entry.address.clone());
        self.entries.insert(entry.address.clone(), entry);
        Ok(())
    }

    /// Remove an address from all lists
    pub fn remove_address(&mut self, address: &str) -> bool {
        let removed = self.whitelist.remove(address) | self.blacklist.remove(address);
        if removed {
            self.entries.remove(address);
        }
        removed
    }

    /// Check if an address is whitelisted
    pub fn is_whitelisted(&self, address: &str) -> bool {
        self.whitelist.contains(address)
    }

    /// Check if an address is blacklisted
    pub fn is_blacklisted(&self, address: &str) -> bool {
        self.blacklist.contains(address)
    }

    /// Get address entry by address
    pub fn get_entry(&self, address: &str) -> Option<&AddressEntry> {
        self.entries.get(address)
    }

    /// Add a regex pattern for address matching
    pub fn add_pattern(&mut self, pattern: &str, action: FilterAction) -> Result<()> {
        let regex = Regex::new(pattern)?;
        self.patterns.push((regex, action));
        Ok(())
    }

    /// Filter a single address
    pub fn filter_address(&self, address: &str) -> FilterResult {
        // Check blacklist first (higher priority)
        if self.blacklist.contains(address) {
            return FilterResult {
                address: address.to_string(),
                action: FilterAction::Block,
                list_type: ListType::Blacklist,
                matching_entry: self.entries.get(address).cloned(),
                checked_at: Utc::now(),
            };
        }

        // Check whitelist
        if self.whitelist.contains(address) {
            return FilterResult {
                address: address.to_string(),
                action: FilterAction::Allow,
                list_type: ListType::Whitelist,
                matching_entry: self.entries.get(address).cloned(),
                checked_at: Utc::now(),
            };
        }

        // Check regex patterns
        for (pattern, action) in &self.patterns {
            if pattern.is_match(address) {
                return FilterResult {
                    address: address.to_string(),
                    action: action.clone(),
                    list_type: ListType::None,
                    matching_entry: None,
                    checked_at: Utc::now(),
                };
            }
        }

        // Default action
        FilterResult {
            address: address.to_string(),
            action: self.config.default_action.clone(),
            list_type: ListType::None,
            matching_entry: None,
            checked_at: Utc::now(),
        }
    }

    /// Filter multiple addresses
    pub fn filter_addresses(&self, addresses: &[String]) -> Vec<FilterResult> {
        addresses.iter()
            .map(|addr| self.filter_address(addr))
            .collect()
    }

    /// Validate a Stellar address format
    pub fn validate_stellar_address(&self, address: &str) -> bool {
        if !self.config.validate_stellar_addresses {
            return true;
        }
        
        // Stellar addresses start with 'G' and are 56 chars
        let re = Regex::new(r"^G[0-9A-Z]{55}$").unwrap();
        re.is_match(address)
    }

    /// Validate a Stellar contract address format
    pub fn validate_contract_address(&self, address: &str) -> bool {
        // Contract addresses start with 'C'
        let re = Regex::new(r"^C[0-9A-Z]{55}$").unwrap();
        re.is_match(address)
    }

    /// Load addresses from a file
    pub fn load_from_file(&self, path: &PathBuf) -> Result<Vec<AddressEntry>> {
        let content = fs::read_to_string(path)?;
        
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let entries: Vec<AddressEntry> = serde_json::from_str(&content)?;
            Ok(entries)
        } else {
            // Assume CSV format
            let mut entries = Vec::new();
            for line in content.lines() {
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() >= 3 {
                    let address = parts[0].trim().to_string();
                    let format = match parts[1].trim() {
                        "stellar" => AddressFormat::StellarClassic,
                        "contract" => AddressFormat::StellarContract,
                        "soroban" => AddressFormat::SorobanContract,
                        "hash" => AddressFormat::Sha256Hash,
                        "pubkey" => AddressFormat::PublicKey,
                        "secret" => AddressFormat::SecretKey,
                        _ => AddressFormat::Generic,
                    };
                    let category = match parts[2].trim() {
                        "trusted" => AddressCategory::Trusted,
                        "malicious" => AddressCategory::Malicious,
                        "test" => AddressCategory::Test,
                        "exchange" => AddressCategory::Exchange,
                        "deployer" => AddressCategory::Deployer,
                        "treasury" => AddressCategory::Treasury,
                        "user" => AddressCategory::User,
                        "contract" => AddressCategory::Contract,
                        "multisig" => AddressCategory::MultiSig,
                        other => AddressCategory::Other(other.to_string()),
                    };
                    let description = if parts.len() > 3 {
                        parts[3].trim().to_string()
                    } else {
                        String::new()
                    };
                    
                    entries.push(AddressEntry::new(address, format, category, description));
                }
            }
            Ok(entries)
        }
    }

    /// Load addresses from multiple files
    pub fn load_from_files(&mut self, paths: &[PathBuf]) -> Result<()> {
        for path in paths {
            let entries = self.load_from_file(path)?;
            for entry in entries {
                if entry.active {
                    match entry.category {
                        AddressCategory::Malicious => {
                            self.add_to_blacklist(entry)?;
                        }
                        _ => {
                            self.add_to_whitelist(entry)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Export addresses to a JSON file
    pub fn export_to_json(&self, path: &PathBuf) -> Result<()> {
        let entries: Vec<&AddressEntry> = self.entries.values().collect();
        let json = serde_json::to_string_pretty(&entries)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Export addresses to a CSV file
    pub fn export_to_csv(&self, path: &PathBuf) -> Result<()> {
        let mut csv = String::new();
        csv.push_str("address,format,category,description,source,tags,added_at,expires_at,active\n");
        
        for entry in self.entries.values() {
            let format_str = match entry.format {
                AddressFormat::StellarClassic => "stellar",
                AddressFormat::StellarContract => "contract",
                AddressFormat::SorobanContract => "soroban",
                AddressFormat::Sha256Hash => "hash",
                AddressFormat::PublicKey => "pubkey",
                AddressFormat::SecretKey => "secret",
                AddressFormat::Generic => "generic",
            };
            
            let category_str = match &entry.category {
                AddressCategory::Trusted => "trusted",
                AddressCategory::Malicious => "malicious",
                AddressCategory::Test => "test",
                AddressCategory::Exchange => "exchange",
                AddressCategory::Deployer => "deployer",
                AddressCategory::Treasury => "treasury",
                AddressCategory::User => "user",
                AddressCategory::Contract => "contract",
                AddressCategory::MultiSig => "multisig",
                AddressCategory::Other(s) => s,
            };
            
            let tags_str = entry.tags.join(";");
            let expires_str = entry.expires_at
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default();
            
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{}\n",
                entry.address,
                format_str,
                category_str,
                entry.description.replace(',', ";"),
                entry.source,
                tags_str,
                entry.added_at.to_rfc3339(),
                expires_str,
                entry.active
            ));
        }
        
        fs::write(path, csv)?;
        Ok(())
    }

    /// Get statistics about the address filter
    pub fn get_stats(&self) -> AddressFilterStats {
        AddressFilterStats {
            total_entries: self.entries.len(),
            whitelisted_count: self.whitelist.len(),
            blacklisted_count: self.blacklist.len(),
            active_count: self.entries.values().filter(|e| e.is_valid()).count(),
            expired_count: self.entries.values().filter(|e| !e.is_valid()).count(),
            patterns_count: self.patterns.len(),
        }
    }

    /// Update address filter configuration
    pub fn update_config(&mut self, config: AddressFilterConfig) {
        self.config = config;
    }

    /// Get current configuration
    pub fn get_config(&self) -> &AddressFilterConfig {
        &self.config
    }

    /// List all addresses with their categories
    pub fn list_addresses(&self) -> Vec<&AddressEntry> {
        self.entries.values().collect()
    }

    /// List addresses by category
    pub fn list_addresses_by_category(&self, category: &AddressCategory) -> Vec<&AddressEntry> {
        self.entries
            .values()
            .filter(|e| &e.category == category)
            .collect()
    }

    /// Check if any address filter is configured
    pub fn has_filters(&self) -> bool {
        !self.whitelist.is_empty() || !self.blacklist.is_empty() || !self.patterns.is_empty()
    }
}

/// Statistics about the address filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressFilterStats {
    /// Total number of address entries
    pub total_entries: usize,
    /// Number of whitelisted addresses
    pub whitelisted_count: usize,
    /// Number of blacklisted addresses
    pub blacklisted_count: usize,
    /// Number of active (non-expired) addresses
    pub active_count: usize,
    /// Number of expired addresses
    pub expired_count: usize,
    /// Number of regex patterns
    pub patterns_count: usize,
}

impl Default for AddressFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_entry(address: &str, category: AddressCategory) -> AddressEntry {
        AddressEntry::new(
            address.to_string(),
            AddressFormat::StellarClassic,
            category,
            "Test entry".to_string(),
        )
    }

    #[test]
    fn test_whitelist_address() {
        let mut filter = AddressFilter::new();
        let entry = create_test_entry("GABC123", AddressCategory::Trusted);
        
        assert!(filter.add_to_whitelist(entry).is_ok());
        assert!(filter.is_whitelisted("GABC123"));
        assert!(!filter.is_blacklisted("GABC123"));
    }

    #[test]
    fn test_blacklist_address() {
        let mut filter = AddressFilter::new();
        let entry = create_test_entry("GMALICIOUS", AddressCategory::Malicious);
        
        assert!(filter.add_to_blacklist(entry).is_ok());
        assert!(filter.is_blacklisted("GMALICIOUS"));
        assert!(!filter.is_whitelisted("GMALICIOUS"));
    }

    #[test]
    fn test_filter_address_whitelisted() {
        let mut filter = AddressFilter::new();
        let entry = create_test_entry("GWHITELIST", AddressCategory::Trusted);
        filter.add_to_whitelist(entry).unwrap();
        
        let result = filter.filter_address("GWHITELIST");
        assert_eq!(result.action, FilterAction::Allow);
        assert_eq!(result.list_type, ListType::Whitelist);
    }

    #[test]
    fn test_filter_address_blacklisted() {
        let mut filter = AddressFilter::new();
        let entry = create_test_entry("GBLACKLIST", AddressCategory::Malicious);
        filter.add_to_blacklist(entry).unwrap();
        
        let result = filter.filter_address("GBLACKLIST");
        assert_eq!(result.action, FilterAction::Block);
        assert_eq!(result.list_type, ListType::Blacklist);
    }

    #[test]
    fn test_filter_address_default() {
        let filter = AddressFilter::new();
        let result = filter.filter_address("GUNKNOWN");
        assert_eq!(result.action, FilterAction::Skip);
        assert_eq!(result.list_type, ListType::None);
    }

    #[test]
    fn test_remove_address() {
        let mut filter = AddressFilter::new();
        let entry = create_test_entry("GREMOVE", AddressCategory::Trusted);
        filter.add_to_whitelist(entry).unwrap();
        
        assert!(filter.is_whitelisted("GREMOVE"));
        assert!(filter.remove_address("GREMOVE"));
        assert!(!filter.is_whitelisted("GREMOVE"));
        assert!(filter.get_entry("GREMOVE").is_none());
    }

    #[test]
    fn test_address_stats() {
        let mut filter = AddressFilter::new();
        filter.add_to_whitelist(create_test_entry("G1", AddressCategory::Trusted)).unwrap();
        filter.add_to_whitelist(create_test_entry("G2", AddressCategory::Trusted)).unwrap();
        filter.add_to_blacklist(create_test_entry("G3", AddressCategory::Malicious)).unwrap();
        
        let stats = filter.get_stats();
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.whitelisted_count, 2);
        assert_eq!(stats.blacklisted_count, 1);
    }

    #[test]