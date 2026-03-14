//! Command-line interface for the Stellar Security Scanner

use clap::{Parser, Subcommand};
use colored::*;
use stellar_security_scanner::{scanners::{SecurityScanner, InvariantScanner}, analysis::AnalysisResult, report::{SecurityReport, ReportFormat}, config::ScannerConfig};
use std::path::PathBuf;
use std::time::Instant;
use anyhow::Result;

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
