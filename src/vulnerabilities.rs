//! Vulnerability detection patterns for Stellar smart contracts

use crate::Severity;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VulnerabilityType {
    // Access Control Vulnerabilities
    MissingAccessControl,
    WeakAccessControl,
    UnauthorizedMint,
    UnauthorizedBurn,
    
    // Token Economics Vulnerabilities
    InfiniteMint,
    InflationBug,
    Reentrancy,
    IntegerOverflow,
    IntegerUnderflow,
    
    // Logic Vulnerabilities
    FrozenFunds,
    BrokenInvariant,
    RaceCondition,
    FrontRunningSusceptibility,
    
    // Stellar Specific Vulnerabilities
    InsufficientFeeBump,
    InvalidTimeBounds,
    WeakSignatureVerification,
    StellarAssetManipulation,
    
    // Smart Contract Best Practices
    UninitializedStorage,
    MissingEventEmission,
    PoorErrorHandling,
    HardcodedValues,
    
    // Security Best Practices
    LackOfInputValidation,
    DenialOfService,
    InformationLeakage,
    CentralizationRisk,
}

impl VulnerabilityType {
    pub fn description(&self) -> &'static str {
        match self {
            VulnerabilityType::MissingAccessControl => "Critical functions lack proper access control mechanisms",
            VulnerabilityType::WeakAccessControl => "Access control can be bypassed through alternative execution paths",
            VulnerabilityType::UnauthorizedMint => "Function allows unauthorized token minting",
            VulnerabilityType::UnauthorizedBurn => "Function allows unauthorized token burning",
            
            VulnerabilityType::InfiniteMint => "Token supply can be inflated without limits",
            VulnerabilityType::InflationBug => "Token supply can be increased unexpectedly",
            VulnerabilityType::Reentrancy => "Contract vulnerable to reentrancy attacks",
            VulnerabilityType::IntegerOverflow => "Integer operations may overflow",
            VulnerabilityType::IntegerUnderflow => "Integer operations may underflow",
            
            VulnerabilityType::FrozenFunds => "Assets can become permanently locked",
            VulnerabilityType::BrokenInvariant => "Contract invariants can be violated",
            VulnerabilityType::RaceCondition => "Race condition in contract logic",
            VulnerabilityType::FrontRunningSusceptibility => "Operations vulnerable to front-running",
            
            VulnerabilityType::InsufficientFeeBump => "Contract doesn't handle fee bumps properly",
            VulnerabilityType::InvalidTimeBounds => "Time bounds validation is missing or weak",
            VulnerabilityType::WeakSignatureVerification => "Signature verification can be bypassed",
            VulnerabilityType::StellarAssetManipulation => "Stellar asset operations can be manipulated",
            
            VulnerabilityType::UninitializedStorage => "Storage variables used before initialization",
            VulnerabilityType::MissingEventEmission => "Critical state changes don't emit events",
            VulnerabilityType::PoorErrorHandling => "Error handling is insufficient or misleading",
            VulnerabilityType::HardcodedValues => "Sensitive values are hardcoded",
            
            VulnerabilityType::LackOfInputValidation => "Inputs are not properly validated",
            VulnerabilityType::DenialOfService => "Contract vulnerable to DoS attacks",
            VulnerabilityType::InformationLeakage => "Sensitive information is exposed",
            VulnerabilityType::CentralizationRisk => "Excessive centralization in contract logic",
        }
    }

    pub fn severity(&self) -> Severity {
        match self {
            VulnerabilityType::MissingAccessControl => Severity::Critical,
            VulnerabilityType::InfiniteMint => Severity::Critical,
            VulnerabilityType::UnauthorizedMint => Severity::Critical,
            VulnerabilityType::UnauthorizedBurn => Severity::Critical,
            
            VulnerabilityType::WeakAccessControl => Severity::High,
            VulnerabilityType::Reentrancy => Severity::High,
            VulnerabilityType::FrozenFunds => Severity::High,
            VulnerabilityType::BrokenInvariant => Severity::High,
            VulnerabilityType::StellarAssetManipulation => Severity::High,
            
            VulnerabilityType::InflationBug => Severity::Medium,
            VulnerabilityType::IntegerOverflow => Severity::Medium,
            VulnerabilityType::IntegerUnderflow => Severity::Medium,
            VulnerabilityType::RaceCondition => Severity::Medium,
            VulnerabilityType::FrontRunningSusceptibility => Severity::Medium,
            VulnerabilityType::InsufficientFeeBump => Severity::Medium,
            VulnerabilityType::InvalidTimeBounds => Severity::Medium,
            VulnerabilityType::WeakSignatureVerification => Severity::Medium,
            
            VulnerabilityType::UninitializedStorage => Severity::Low,
            VulnerabilityType::MissingEventEmission => Severity::Low,
            VulnerabilityType::PoorErrorHandling => Severity::Low,
            VulnerabilityType::HardcodedValues => Severity::Low,
            VulnerabilityType::LackOfInputValidation => Severity::Low,
            VulnerabilityType::DenialOfService => Severity::Medium,
            VulnerabilityType::InformationLeakage => Severity::Low,
            VulnerabilityType::CentralizationRisk => Severity::Medium,
        }
    }

    pub fn recommendation(&self) -> &'static str {
        match self {
            VulnerabilityType::MissingAccessControl => "Implement proper access control using require_auth() or custom authorization logic",
            VulnerabilityType::WeakAccessControl => "Review and strengthen access control mechanisms, consider multiple authorization layers",
            VulnerabilityType::UnauthorizedMint => "Add proper authorization checks before minting operations",
            VulnerabilityType::UnauthorizedBurn => "Add proper authorization checks before burning operations",
            
            VulnerabilityType::InfiniteMint => "Implement supply limits and proper minting controls",
            VulnerabilityType::InflationBug => "Add supply caps and validate all supply changes",
            VulnerabilityType::Reentrancy => "Use checks-effects-interactions pattern and implement reentrancy guards",
            VulnerabilityType::IntegerOverflow => "Use safe arithmetic operations or add overflow checks",
            VulnerabilityType::IntegerUnderflow => "Use safe arithmetic operations or add underflow checks",
            
            VulnerabilityType::FrozenFunds => "Ensure all asset flows have proper exit paths",
            VulnerabilityType::BrokenInvariant => "Define and enforce critical invariants explicitly",
            VulnerabilityType::RaceCondition => "Implement proper locking mechanisms or atomic operations",
            VulnerabilityType::FrontRunningSusceptibility => "Add commit-reveal schemes or time-based protections",
            
            VulnerabilityType::InsufficientFeeBump => "Implement proper fee handling and bump mechanisms",
            VulnerabilityType::InvalidTimeBounds => "Add comprehensive time bounds validation",
            VulnerabilityType::WeakSignatureVerification => "Strengthen signature verification with proper validation",
            VulnerabilityType::StellarAssetManipulation => "Implement proper asset validation and authorization",
            
            VulnerabilityType::UninitializedStorage => "Initialize all storage variables before use",
            VulnerabilityType::MissingEventEmission => "Add events for all critical state changes",
            VulnerabilityType::PoorErrorHandling => "Implement comprehensive error handling with clear messages",
            VulnerabilityType::HardcodedValues => "Use configuration or environment variables instead of hardcoded values",
            
            VulnerabilityType::LackOfInputValidation => "Add comprehensive input validation for all external inputs",
            VulnerabilityType::DenialOfService => "Implement rate limiting and resource management",
            VulnerabilityType::InformationLeakage => "Remove sensitive information from public interfaces",
            VulnerabilityType::CentralizationRisk => "Implement decentralized governance or multi-sig controls",
        }
    }
}

impl fmt::Display for VulnerabilityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
