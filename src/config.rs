//! Configuration management for the security scanner

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::gas_limits::GasLimitConfig as GasLimitConfigType;
use crate::event_logging::EventLoggingConfig as EventLoggingConfigType;
use crate::secure_id_generation::SecureIdConfig as SecureIdConfigType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    pub scan_paths: Vec<PathBuf>,
    pub ignore_paths: Vec<PathBuf>,
    pub vulnerability_checks: VulnerabilityConfig,
    pub invariant_checks: InvariantConfig,
    pub output: OutputConfig,
    pub performance: PerformanceConfig,
    pub emergency_stop: EmergencyStopConfig,
    pub gas_limits: GasLimitConfigType,
    pub event_logging: EventLoggingConfigType,
    pub secure_id_generation: SecureIdConfigType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityConfig {
    pub enabled_checks: Vec<String>,
    pub disabled_checks: Vec<String>,
    pub severity_threshold: String,
    pub check_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantConfig {
    pub enabled_rules: Vec<String>,
    pub disabled_rules: Vec<String>,
    pub custom_rules: Vec<CustomInvariantRule>,
    pub auto_fix: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomInvariantRule {
    pub name: String,
    pub description: String,
    pub pattern: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub format: String,
    pub output_file: Option<PathBuf>,
    pub verbose: bool,
    pub include_recommendations: bool,
    pub include_code_snippets: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub parallel_jobs: usize,
    pub timeout_seconds: u64,
    pub max_file_size_mb: usize,
    pub cache_results: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyStopConfig {
    pub enabled: bool,
    pub stop_on_critical: bool,
    pub save_partial_results: bool,
    pub timeout_seconds: u64,
    pub max_memory_mb: usize,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            scan_paths: vec![PathBuf::from(".")],
            ignore_paths: vec![
                PathBuf::from("target"),
                PathBuf::from("node_modules"),
                PathBuf::from(".git"),
            ],
            vulnerability_checks: VulnerabilityConfig::default(),
            invariant_checks: InvariantConfig::default(),
            output: OutputConfig::default(),
            performance: PerformanceConfig::default(),
            emergency_stop: EmergencyStopConfig::default(),
            gas_limits: GasLimitConfigType::default(),
            event_logging: EventLoggingConfigType::default(),
            secure_id_generation: SecureIdConfigType::default(),
        }
    }
}

impl Default for VulnerabilityConfig {
    fn default() -> Self {
        Self {
            enabled_checks: vec![
                "missing_access_control".to_string(),
                "infinite_mint".to_string(),
                "reentrancy".to_string(),
                "integer_overflow".to_string(),
                "unauthorized_mint".to_string(),
                "unauthorized_burn".to_string(),
            ],
            disabled_checks: vec![],
            severity_threshold: "low".to_string(),
            check_patterns: vec![],
        }
    }
}

impl Default for InvariantConfig {
    fn default() -> Self {
        Self {
            enabled_rules: vec![
                "total_supply_consistency".to_string(),
                "balance_non_negative".to_string(),
                "transfer_conservation".to_string(),
                "no_negative_balances".to_string(),
            ],
            disabled_rules: vec![],
            custom_rules: vec![],
            auto_fix: false,
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            format: "console".to_string(),
            output_file: None,
            verbose: false,
            include_recommendations: true,
            include_code_snippets: false,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            parallel_jobs: num_cpus::get(),
            timeout_seconds: 300,
            max_file_size_mb: 10,
            cache_results: true,
        }
    }
}

impl Default for EmergencyStopConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            stop_on_critical: true,
            save_partial_results: true,
            timeout_seconds: 300,
            max_memory_mb: 1024,
        }
    }
}

impl ScannerConfig {
    pub fn load_from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: ScannerConfig = if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            toml::from_str(&content)?
        } else {
            serde_json::from_str(&content)?
        };
        Ok(config)
    }

    pub fn save_to_file(&self, path: &PathBuf) -> anyhow::Result<()> {
        let content = if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            toml::to_string_pretty(self)?
        } else {
            serde_json::to_string_pretty(self)?
        };
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn should_ignore_path(&self, path: &PathBuf) -> bool {
        self.ignore_paths.iter().any(|ignore| {
            path.starts_with(ignore) || path.file_name() == ignore.file_name()
        })
    }

    pub fn is_vulnerability_enabled(&self, check_name: &str) -> bool {
        self.vulnerability_checks.enabled_checks.contains(&check_name.to_string()) &&
        !self.vulnerability_checks.disabled_checks.contains(&check_name.to_string())
    }

    pub fn is_invariant_enabled(&self, rule_name: &str) -> bool {
        self.invariant_checks.enabled_rules.contains(&rule_name.to_string()) &&
        !self.invariant_checks.disabled_rules.contains(&rule_name.to_string())
    }
}
