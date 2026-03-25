//! Command-line interface for the Stellar Security Scanner

use clap::{Parser, Subcommand};
use colored::*;
use stellar_security_scanner::{scanners::{SecurityScanner, InvariantScanner}, analysis::AnalysisResult, report::{SecurityReport, ReportFormat}, config::ScannerConfig, kubernetes::{K8sScanManager, ScanPodConfig, ScanAutoScaler}, time_travel_debugger::{TimeTravelDebugger, TimeTravelConfig, ForkedState, TestResult}};
use std::path::PathBuf;
use std::time::{Instant, Duration};
use anyhow::Result;
use uuid;

#[derive(Parser)]
#[command(name = "stellar-scanner")]
#[command(about = "Security and invariant scanner for Stellar smart contracts")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan for security vulnerabilities
    Security {
        /// Path to scan (default: current directory)
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
        
        /// Output format (console, json, html, markdown)
        #[arg(short, long, default_value = "console")]
        format: String,
        
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,
        
        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Scan for invariant violations
    Invariants {
        /// Path to scan (default: current directory)
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
        
        /// Output format (console, json, html, markdown)
        #[arg(short, long, default_value = "console")]
        format: String,
        
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,
        
        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Run comprehensive scan (security + invariants)
    Scan {
        /// Path to scan (default: current directory)
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
        
        /// Output format (console, json, html, markdown)
        #[arg(short, long, default_value = "console")]
        format: String,
        
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,
        
        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Generate default configuration file
    Init {
        /// Configuration file path
        #[arg(short, long, default_value = "stellar-scanner.toml")]
        path: PathBuf,
    },
    
    /// List available vulnerability checks
    ListChecks {
        /// Filter by severity (critical, high, medium, low)
        #[arg(short, long)]
        severity: Option<String>,
    },
    
    /// List available invariant rules
    ListInvariants {
        /// Filter by severity (critical, high, medium, low)
        #[arg(short, long)]
        severity: Option<String>,
    },
    
    /// Run scan in isolated Kubernetes pod
    K8sScan {
        /// Path to scan (default: current directory)
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
        
        /// Output format (console, json, html, markdown)
        #[arg(short, long, default_value = "console")]
        format: String,
        
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,
        
        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
        
        /// CPU limit per scan pod
        #[arg(long, default_value = "1000m")]
        cpu_limit: String,
        
        /// Memory limit per scan pod
        #[arg(long, default_value = "2Gi")]
        memory_limit: String,
        
        /// Scan timeout in seconds
        #[arg(long, default_value = "600")]
        timeout: u64,
    },
    
    /// Manage Kubernetes scan operations
    K8sManage {
        #[command(subcommand)]
        action: K8sAction,
    },
    
    /// Time Travel Debugger - Fork and test against historical ledger states
    TimeTravel {
        #[command(subcommand)]
        action: TimeTravelAction,
    },
}

#[derive(Subcommand)]
enum K8sAction {
    /// List all active scans
    List,
    /// Cleanup stuck scans older than specified minutes
    Cleanup {
        /// Age in minutes for cleanup
        #[arg(short, long, default_value = "30")]
        age_minutes: u64,
    },
    /// Show current load metrics
    Status,
}

#[derive(Subcommand)]
enum TimeTravelAction {
    /// Fork the network at a specific ledger sequence
    Fork {
        /// Ledger sequence to fork at
        #[arg(short, long)]
        ledger_sequence: u32,
        
        /// Stellar RPC URL
        #[arg(long, default_value = "https://mainnet.stellar.rpc")]
        rpc_url: String,
        
        /// Network passphrase
        #[arg(long, default_value = "Public Global Stellar Network ; September 2015")]
        network_passphrase: String,
        
        /// Cache size for contract states
        #[arg(long, default_value = "10000")]
        cache_size: usize,
    },
    
    /// Test a contract against a forked state
    Test {
        /// Contract ID to test
        #[arg(short, long)]
        contract_id: String,
        
        /// Ledger sequence to test against
        #[arg(short, long)]
        ledger_sequence: u32,
        
        /// Stellar RPC URL
        #[arg(long, default_value = "https://mainnet.stellar.rpc")]
        rpc_url: String,
        
        /// Output format (console, json)
        #[arg(short, long, default_value = "console")]
        format: String,
        
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// Simulate contract upgrade compatibility
    Upgrade {
        /// Contract ID to upgrade
        #[arg(short, long)]
        contract_id: String,
        
        /// Path to new WASM file
        #[arg(short, long)]
        wasm_file: PathBuf,
        
        /// Ledger sequence to test against
        #[arg(short, long)]
        ledger_sequence: u32,
        
        /// Stellar RPC URL
        #[arg(long, default_value = "https://mainnet.stellar.rpc")]
        rpc_url: String,
        
        /// Output format (console, json)
        #[arg(short, long, default_value = "console")]
        format: String,
        
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// Find orphaned state entries
    Orphaned {
        /// Contract ID to analyze
        #[arg(short, long)]
        contract_id: String,
        
        /// Path to new WASM file
        #[arg(short, long)]
        wasm_file: PathBuf,
        
        /// Ledger sequence to test against
        #[arg(short, long)]
        ledger_sequence: u32,
        
        /// Stellar RPC URL
        #[arg(long, default_value = "https://mainnet.stellar.rpc")]
        rpc_url: String,
        
        /// Output format (console, json)
        #[arg(short, long, default_value = "console")]
        format: String,
        
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// Show cache statistics
    CacheStats {
        /// Stellar RPC URL
        #[arg(long, default_value = "https://mainnet.stellar.rpc")]
        rpc_url: String,
    },
    
    /// Clear all caches
    ClearCache {
        /// Stellar RPC URL
        #[arg(long, default_value = "https://mainnet.stellar.rpc")]
        rpc_url: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Security { path, format, output, config, verbose } => {
            run_security_scan(path, format, output, config, verbose)
        }
        Commands::Invariants { path, format, output, config, verbose } => {
            run_invariant_scan(path, format, output, config, verbose)
        }
        Commands::Scan { path, format, output, config, verbose } => {
            run_comprehensive_scan(path, format, output, config, verbose)
        }
        Commands::Init { path } => {
            generate_config(path)
        }
        Commands::ListChecks { severity } => {
            list_vulnerability_checks(severity)
        }
        Commands::ListInvariants { severity } => {
            list_invariant_rules(severity)
        }
        Commands::K8sScan { path, format, output, config, verbose, cpu_limit, memory_limit, timeout } => {
            run_k8s_scan(path, format, output, config, verbose, cpu_limit, memory_limit, timeout)
        }
        Commands::K8sManage { action } => {
            run_k8s_management(action)
        }
        Commands::TimeTravel { action } => {
            run_time_travel_action(action)
        }
    }
}

fn run_security_scan(path: PathBuf, format: String, output: Option<PathBuf>, config_path: Option<PathBuf>, verbose: bool) -> Result<()> {
    println!("{}", "🔍 Starting Stellar Security Scan".bold().cyan());
    
    let config = load_config(config_path)?;
    let scanner = SecurityScanner::new()?;
    
    if verbose {
        println!("Scanning path: {}", path.display());
        println!("Output format: {}", format);
        if let Some(ref output_path) = output {
            println!("Output file: {}", output_path.display());
        }
    }
    
    let start_time = Instant::now();
    let results = scanner.scan_directory(&path)?;
    let scan_duration = start_time.elapsed().as_millis() as u64;
    
    if verbose {
        println!("Scanned {} files in {}ms", results.len(), scan_duration);
    }
    
    // Combine results for analysis
    let analysis = AnalysisResult::new(results, scan_duration);
    
    // Generate report
    let report_format = parse_report_format(&format)?;
    let report = SecurityReport::new(report_format);
    report.generate(&analysis, output.as_deref())?;
    
    // Exit with error code if critical issues found
    if analysis.risk_score.risk_level == stellar_security_scanner::analysis::RiskLevel::Critical {
        std::process::exit(1);
    }
    
    Ok(())
}

fn run_invariant_scan(path: PathBuf, format: String, output: Option<PathBuf>, config_path: Option<PathBuf>, verbose: bool) -> Result<()> {
    println!("{}", "🔒 Starting Stellar Invariant Scan".bold().cyan());
    
    let config = load_config(config_path)?;
    let scanner = InvariantScanner::new()?;
    
    if verbose {
        println!("Scanning path: {}", path.display());
        println!("Output format: {}", format);
        if let Some(ref output_path) = output {
            println!("Output file: {}", output_path.display());
        }
    }
    
    let start_time = Instant::now();
    let results = scanner.scan_directory(&path)?;
    let scan_duration = start_time.elapsed().as_millis() as u64;
    
    if verbose {
        println!("Scanned {} files in {}ms", results.len(), scan_duration);
    }
    
    // Combine results for analysis
    let analysis = AnalysisResult::new(results, scan_duration);
    
    // Generate report
    let report_format = parse_report_format(&format)?;
    let report = SecurityReport::new(report_format);
    report.generate(&analysis, output.as_deref())?;
    
    Ok(())
}

fn run_comprehensive_scan(path: PathBuf, format: String, output: Option<PathBuf>, config_path: Option<PathBuf>, verbose: bool) -> Result<()> {
    println!("{}", "🚀 Starting Comprehensive Stellar Security & Invariant Scan".bold().cyan());
    
    let config = load_config(config_path)?;
    let security_scanner = SecurityScanner::new()?;
    let invariant_scanner = InvariantScanner::new()?;
    
    if verbose {
        println!("Scanning path: {}", path.display());
        println!("Output format: {}", format);
        if let Some(ref output_path) = output {
            println!("Output file: {}", output_path.display());
        }
    }
    
    let start_time = Instant::now();
    
    // Run both scans
    let security_results = security_scanner.scan_directory(&path)?;
    let invariant_results = invariant_scanner.scan_directory(&path)?;
    
    let scan_duration = start_time.elapsed().as_millis() as u64;
    
    if verbose {
        println!("Security scan: {} files", security_results.len());
        println!("Invariant scan: {} files", invariant_results.len());
        println!("Total scan time: {}ms", scan_duration);
    }
    
    // Combine results
    let mut combined_results = security_results;
    for invariant_result in invariant_results {
        if let Some(security_result) = combined_results.iter_mut().find(|r| r.file_path == invariant_result.file_path) {
            security_result.invariant_violations.extend(invariant_result.invariant_violations);
            security_result.recommendations.extend(invariant_result.recommendations);
        } else {
            combined_results.push(invariant_result);
        }
    }
    
    let analysis = AnalysisResult::new(combined_results, scan_duration);
    
    // Generate report
    let report_format = parse_report_format(&format)?;
    let report = SecurityReport::new(report_format);
    report.generate(&analysis, output.as_deref())?;
    
    // Exit with error code if critical issues found
    if analysis.risk_score.risk_level == stellar_security_scanner::analysis::RiskLevel::Critical {
        std::process::exit(1);
    }
    
    Ok(())
}

fn generate_config(path: PathBuf) -> Result<()> {
    println!("📝 Generating default configuration file: {}", path.display());
    
    let config = ScannerConfig::default();
    config.save_to_file(&path)?;
    
    println!("✅ Configuration file generated successfully!");
    println!("Edit the file to customize your scanning preferences.");
    
    Ok(())
}

fn list_vulnerability_checks(severity_filter: Option<String>) -> Result<()> {
    println!("{}", "🚨 Available Vulnerability Checks".bold().red());
    println!("{}", "═".repeat(50).red());
    
    use stellar_security_scanner::vulnerabilities::VulnerabilityType;
    
    for vuln in [
        VulnerabilityType::MissingAccessControl,
        VulnerabilityType::WeakAccessControl,
        VulnerabilityType::UnauthorizedMint,
        VulnerabilityType::UnauthorizedBurn,
        VulnerabilityType::InfiniteMint,
        VulnerabilityType::InflationBug,
        VulnerabilityType::Reentrancy,
        VulnerabilityType::IntegerOverflow,
        VulnerabilityType::IntegerUnderflow,
        VulnerabilityType::FrozenFunds,
        VulnerabilityType::BrokenInvariant,
        VulnerabilityType::RaceCondition,
        VulnerabilityType::FrontRunningSusceptibility,
        VulnerabilityType::InsufficientFeeBump,
        VulnerabilityType::InvalidTimeBounds,
        VulnerabilityType::WeakSignatureVerification,
        VulnerabilityType::StellarAssetManipulation,
        VulnerabilityType::UninitializedStorage,
        VulnerabilityType::MissingEventEmission,
        VulnerabilityType::PoorErrorHandling,
        VulnerabilityType::HardcodedValues,
        VulnerabilityType::LackOfInputValidation,
        VulnerabilityType::DenialOfService,
        VulnerabilityType::InformationLeakage,
        VulnerabilityType::CentralizationRisk,
    ] {
        let vuln_severity = vuln.severity().as_str();
        
        if let Some(ref filter) = severity_filter {
            if filter.to_lowercase() != vuln_severity.to_lowercase() {
                continue;
            }
        }
        
        let severity_color = match vuln.severity() {
            stellar_security_scanner::Severity::Critical => "red",
            stellar_security_scanner::Severity::High => "yellow",
            stellar_security_scanner::Severity::Medium => "blue",
            stellar_security_scanner::Severity::Low => "white",
        };
        
        println!("\n{} [{}]", 
            vuln.to_string().bold().color(severity_color),
            vuln_severity.color(severity_color).bold()
        );
        println!("  {}", vuln.description());
        println!("  💄 {}", vuln.recommendation().italic());
    }
    
    Ok(())
}

fn list_invariant_rules(severity_filter: Option<String>) -> Result<()> {
    println!("{}", "🔒 Available Invariant Rules".bold().yellow());
    println!("{}", "═".repeat(50).yellow());
    
    use stellar_security_scanner::invariants::InvariantRule;
    
    for invariant in [
        InvariantRule::TotalSupplyConsistency,
        InvariantRule::BalanceNonNegative,
        InvariantRule::TransferConservation,
        InvariantRule::MintSupplyIncrease,
        InvariantRule::BurnSupplyDecrease,
        InvariantRule::AdminAuthorization,
        InvariantRule::OwnershipConsistency,
        InvariantRule::PermissionIntegrity,
        InvariantRule::SumOfBalancesEqualsSupply,
        InvariantRule::NoNegativeBalances,
        InvariantRule::OverflowProtection,
        InvariantRule::StateTransitionValidity,
        InvariantRule::EventStateConsistency,
        InvariantRule::TimestampMonotonicity,
        InvariantRule::NoFreeMoney,
        InvariantRule::ConservationOfValue,
        InvariantRule::FairDistribution,
        InvariantRule::StellarAssetIntegrity,
        InvariantRule::AccountStateConsistency,
        InvariantRule::SequenceNumberIntegrity,
        InvariantRule::FeeConservation,
    ] {
        let inv_severity = invariant.severity().as_str();
        
        if let Some(ref filter) = severity_filter {
            if filter.to_lowercase() != inv_severity.to_lowercase() {
                continue;
            }
        }
        
        let severity_color = match invariant.severity() {
            stellar_security_scanner::Severity::Critical => "red",
            stellar_security_scanner::Severity::High => "yellow",
            stellar_security_scanner::Severity::Medium => "blue",
            stellar_security_scanner::Severity::Low => "white",
        };
        
        println!("\n{} [{}]", 
            invariant.to_string().bold().color(severity_color),
            inv_severity.color(severity_color).bold()
        );
        println!("  {}", invariant.description());
        println!("  💄 {}", invariant.recommendation().italic());
    }
    
    Ok(())
}

fn load_config(config_path: Option<PathBuf>) -> Result<ScannerConfig> {
    match config_path {
        Some(path) => {
            if path.exists() {
                ScannerConfig::load_from_file(&path)
            } else {
                println!("⚠️  Config file not found, using defaults");
                Ok(ScannerConfig::default())
            }
        }
        None => {
            // Try to find default config files
            let default_paths = [
                PathBuf::from("stellar-scanner.toml"),
                PathBuf::from("stellar-scanner.json"),
                PathBuf::from(".stellar-scanner.toml"),
                PathBuf::from(".stellar-scanner.json"),
            ];
            
            for path in &default_paths {
                if path.exists() {
                    return ScannerConfig::load_from_file(path);
                }
            }
            
            Ok(ScannerConfig::default())
        }
    }
}

fn parse_report_format(format: &str) -> Result<ReportFormat> {
    match format.to_lowercase().as_str() {
        "console" => Ok(ReportFormat::Console),
        "json" => Ok(ReportFormat::Json),
        "html" => Ok(ReportFormat::Html),
        "markdown" | "md" => Ok(ReportFormat::Markdown),
        _ => anyhow::bail!("Invalid output format: {}. Use: console, json, html, markdown", format),
    }
}

fn run_k8s_scan(
    path: PathBuf,
    format: String,
    output: Option<PathBuf>,
    config_path: Option<PathBuf>,
    verbose: bool,
    cpu_limit: String,
    memory_limit: String,
    timeout: u64,
) -> Result<()> {
    println!("{}", "🚀 Starting Kubernetes Isolated Scan".bold().cyan());
    
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        // Load configuration
        let config = load_config(config_path)?;
        
        // Setup Kubernetes scan configuration
        let scan_config = ScanPodConfig {
            cpu_limit,
            memory_limit,
            timeout: Duration::from_secs(timeout),
            ..Default::default()
        };
        
        if verbose {
            println!("📊 Scan Configuration:");
            println!("  CPU Limit: {}", scan_config.cpu_limit);
            println!("  Memory Limit: {}", scan_config.memory_limit);
            println!("  Timeout: {} seconds", timeout);
            println!("  Path: {}", path.display());
        }
        
        // Create Kubernetes manager
        let manager = K8sScanManager::new(scan_config).await?;
        
        // Read contract code
        let contract_code = std::fs::read(&path)?;
        
        // Generate unique scan ID
        let scan_id = uuid::Uuid::new_v4().to_string();
        
        println!("🔍 Starting scan with ID: {}", scan_id);
        
        // Execute scan in isolated pod
        let start_time = std::time::Instant::now();
        let result = manager.execute_scan(&scan_id, &config, &contract_code).await?;
        let duration = start_time.elapsed();
        
        // Generate and output report
        let report = SecurityReport::new(result);
        let report_format = parse_report_format(&format)?;
        
        match output {
            Some(output_path) => {
                report.save_to_file(&output_path, report_format)?;
                println!("✅ Scan completed in {:.2}s", duration.as_secs_f64());
                println!("📄 Report saved to: {}", output_path.display());
            }
            None => {
                report.print(report_format)?;
                println!("✅ Scan completed in {:.2}s", duration.as_secs_f64());
            }
        }
        
        Ok::<(), anyhow::Error>(())
    })
}

fn run_k8s_management(action: K8sAction) -> Result<()> {
    println!("{}", "⚙️  Kubernetes Scan Management".bold().cyan());
    
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let manager = K8sScanManager::new(Default::default()).await?;
        
        match action {
            K8sAction::List => {
                println!("📋 Active Scans:");
                let active_scans = manager.list_active_scans().await?;
                
                if active_scans.is_empty() {
                    println!("  No active scans found.");
                } else {
                    for scan_id in active_scans {
                        println!("  🔄 {}", scan_id);
                    }
                }
            }
            K8sAction::Cleanup { age_minutes } => {
                println!("🧹 Cleaning up scans older than {} minutes...", age_minutes);
                let cleaned_count = manager.cleanup_stuck_scans(Duration::from_secs(age_minutes * 60)).await?;
                println!("✅ Cleaned up {} stuck scans", cleaned_count);
            }
            K8sAction::Status => {
                println!("📊 System Status:");
                
                let active_scans = manager.list_active_scans().await?;
                let auto_scaler = ScanAutoScaler::new(manager, 10); // Default max concurrent
                let (current, max) = auto_scaler.get_load_metrics();
                
                println!("  Active Scans: {}", active_scans.len());
                println!("  Current Load: {}/{}", current, max);
                println!("  System Health: {}", if current < max { "✅ Healthy" } else { "⚠️  At Capacity" });
            }
        }
        
        Ok::<(), anyhow::Error>(())
    })
}

fn run_time_travel_action(action: TimeTravelAction) -> Result<()> {
    println!("{}", "⏰ Time Travel Debugger".bold().cyan());
    
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        match action {
            TimeTravelAction::Fork { ledger_sequence, rpc_url, network_passphrase, cache_size } => {
                println!("🔗 Forking network at ledger sequence {}", ledger_sequence);
                
                let config = TimeTravelConfig {
                    rpc_url,
                    network_passphrase,
                    cache_size,
                    ..Default::default()
                };
                
                let debugger = TimeTravelDebugger::new(config).await?;
                let forked_state = debugger.fork_at_ledger(ledger_sequence).await?;
                
                println!("✅ Successfully forked at ledger {}", forked_state.ledger_sequence());
                println!("📅 Ledger timestamp: {}", forked_state.ledger_snapshot.close_time);
                println!("⏱️  Fork created: {:.2}s ago", forked_state.age().as_secs_f64());
            }
            
            TimeTravelAction::Test { contract_id, ledger_sequence, rpc_url, format, output } => {
                println!("🧪 Testing contract {} against ledger {}", contract_id, ledger_sequence);
                
                let config = TimeTravelConfig {
                    rpc_url,
                    ..Default::default()
                };
                
                let debugger = TimeTravelDebugger::new(config).await?;
                let forked_state = debugger.fork_at_ledger(ledger_sequence).await?;
                let test_result = forked_state.test_contract(&contract_id).await?;
                
                println!("📊 Test Results:");
                println!("  Contract: {}", test_result.contract_id);
                println!("  Ledger: {}", test_result.ledger_sequence);
                println!("  Status: {}", if test_result.passed { "✅ PASSED" } else { "❌ FAILED" });
                println!("  Duration: {:.2}s", test_result.execution_time.as_secs_f64());
                
                if !test_result.issues.is_empty() {
                    println!("  Issues:");
                    for issue in test_result.issues {
                        println!("    ⚠️  {}", issue);
                    }
                }
                
                // Output to file if requested
                if let Some(output_path) = output {
                    let json_output = serde_json::to_string_pretty(&test_result)?;
                    std::fs::write(&output_path, json_output)?;
                    println!("📄 Results saved to: {}", output_path.display());
                }
            }
            
            TimeTravelAction::Upgrade { contract_id, wasm_file, ledger_sequence, rpc_url, format, output } => {
                println!("🔄 Simulating upgrade for contract {} at ledger {}", contract_id, ledger_sequence);
                
                let config = TimeTravelConfig {
                    rpc_url,
                    ..Default::default()
                };
                
                let debugger = TimeTravelDebugger::new(config).await?;
                let new_wasm = std::fs::read(&wasm_file)?;
                
                let upgrade_result = debugger.simulate_contract_upgrade(&contract_id, &new_wasm, ledger_sequence).await?;
                
                println!("📊 Upgrade Simulation Results:");
                println!("  Compatible: {}", if upgrade_result.is_compatible { "✅ YES" } else { "❌ NO" });
                
                if !upgrade_result.compatibility_issues.is_empty() {
                    println!("  Compatibility Issues:");
                    for issue in upgrade_result.compatibility_issues {
                        println!("    ❌ {}", issue);
                    }
                }
                
                if !upgrade_result.orphaned_entries.is_empty() {
                    println!("  Orphaned Entries:");
                    for entry in upgrade_result.orphaned_entries {
                        println!("    ⚠️  {}", entry);
                    }
                }
                
                if !upgrade_result.warnings.is_empty() {
                    println!("  Warnings:");
                    for warning in upgrade_result.warnings {
                        println!("    ⚠️  {}", warning);
                    }
                }
                
                // Output to file if requested
                if let Some(output_path) = output {
                    let json_output = serde_json::to_string_pretty(&upgrade_result)?;
                    std::fs::write(&output_path, json_output)?;
                    println!("📄 Results saved to: {}", output_path.display());
                }
            }
            
            TimeTravelAction::Orphaned { contract_id, wasm_file, ledger_sequence, rpc_url, format, output } => {
                println!("🔍 Analyzing orphaned state for contract {} at ledger {}", contract_id, ledger_sequence);
                
                let config = TimeTravelConfig {
                    rpc_url,
                    ..Default::default()
                };
                
                let debugger = TimeTravelDebugger::new(config).await?;
                let new_wasm = std::fs::read(&wasm_file)?;
                
                let orphaned_entries = debugger.get_orphaned_state(&contract_id, ledger_sequence, &new_wasm).await?;
                
                println!("📊 Orphaned State Analysis:");
                println!("  Total orphaned entries: {}", orphaned_entries.len());
                
                for entry in &orphaned_entries {
                    println!("    🔗 {}", entry);
                }
                
                // Output to file if requested
                if let Some(output_path) = output {
                    let json_output = serde_json::to_string_pretty(&orphaned_entries)?;
                    std::fs::write(&output_path, json_output)?;
                    println!("📄 Results saved to: {}", output_path.display());
                }
            }
            
            TimeTravelAction::CacheStats { rpc_url } => {
                println!("📊 Cache Statistics");
                
                let config = TimeTravelConfig {
                    rpc_url,
                    ..Default::default()
                };
                
                let debugger = TimeTravelDebugger::new(config).await?;
                let stats = debugger.get_cache_stats().await;
                
                println!("  Contract states cached: {}", stats.contract_states_cached);
                println!("  Ledgers cached: {}", stats.ledgers_cached);
                println!("  Max contract states: {}", stats.max_contract_states);
                println!("  Max ledgers: {}", stats.max_ledgers);
            }
            
            TimeTravelAction::ClearCache { rpc_url } => {
                println!("🧹 Clearing caches...");
                
                let config = TimeTravelConfig {
                    rpc_url,
                    ..Default::default()
                };
                
                let debugger = TimeTravelDebugger::new(config).await?;
                debugger.clear_caches().await;
                
                println!("✅ All caches cleared");
            }
        }
        
        Ok::<(), anyhow::Error>(())
    })
}
