//! Vulnerability detection patterns for Stellar smart contracts
//!
//! This module defines the vulnerability classification system used by the scanner.
//! Vulnerabilities are categorized by type and severity level for risk assessment and reporting.
//!
//! # Vulnerability Classification
//!
//! Vulnerabilities are organized into five categories:
//! 1. **Access Control**: Missing or weak authorization mechanisms
//! 2. **Token Economics**: Mint/burn logic, reentrancy, overflow/underflow
//! 3. **Logic**: Frozen funds, broken invariants, race conditions
//! 4. **Stellar-Specific**: Fee bumps, time bounds, signature verification
//! 5. **Best Practices**: Event emission, input validation, error handling
//!
//! # Severity Levels
//!
//! - **Critical**: Direct loss of funds or complete contract failure
//! - **High**: Significant compromise of contract functionality or security
//! - **Medium**: Moderate issues requiring mitigation
//! - **Low**: Best practice violations or minor issues
//!
//! # Security Considerations
//!
//! - All severity classifications assume worst-case exploitation
//! - Developers must validate recommendations in their specific context
//! - Absence of a finding does not guarantee the code is secure
//! - Manual code review is still required; this is a detection aid only
//!
//! # Audit Trail
//!
//! Each finding includes vulnerability type, severity, description, and remediation guidance
//! to enable comprehensive security auditing and tracking.

use crate::Severity;
use std::fmt;

/// Classification of security vulnerabilities found in smart contracts.
///
/// Each variant represents a distinct vulnerability pattern that can compromise
/// contract security or violate security best practices.
///
/// # Severity Context
///
/// Severity levels are assigned based on potential impact:
/// - **Critical**: Could result in immediate loss of funds or contract compromise
/// - **High**: Significant issues affecting core contract security
/// - **Medium**: Issues that require mitigation but may not be immediately exploitable
/// - **Low**: Best practice violations or quality issues
///
/// # Usage
///
/// This enum is used by the analyzer to classify detected vulnerabilities.
/// Each variant has an associated description, severity level, and remediation recommendation.
/// This enables developers to understand and fix identified issues.
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
    InsufficientBalance,
    BalanceUnderflow,
    BalanceOverflow,
    TransferWithoutBalanceCheck,
    
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
    /// Returns a human-readable description of the vulnerability.
    ///
    /// Provides technical context about what the vulnerability is and why it's problematic.
    /// The description is static and does not include context-specific details from the scan.
    ///
    /// # Return Value
    ///
    /// A static string describing the vulnerability type. The description explains:
    /// - What the vulnerability is
    /// - Why it's a security concern
    /// - The potential impact on the contract
    ///
    /// # Security Notes
    ///
    /// - Descriptions are educational but may not cover all attack vectors
    /// - Context-specific variants of the vulnerability may not be covered
    /// - Developers should always review the specific code location identified in the scan
    ///
    /// # Example
    /// ```ignore
    /// match vuln_type {
    ///     VulnerabilityType::MissingAccessControl => {
    ///         println!(\"Issue: {}\", vuln_type.description());
    ///         println!(\"Recommendation: {}\", vuln_type.recommendation());
    ///     }
    ///     _ => {}
    /// }
    /// ```
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
            VulnerabilityType::InsufficientBalance => "Transfer operations don't validate sufficient balance",
            VulnerabilityType::BalanceUnderflow => "Balance operations may cause underflow",
            VulnerabilityType::BalanceOverflow => "Balance operations may cause overflow",
            VulnerabilityType::TransferWithoutBalanceCheck => "Transfer executed without balance verification",
            
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

    /// Returns the severity level of this vulnerability type.
    ///
    /// Severity is assigned based on potential impact to contract security and user funds.
    /// This is used for prioritization and risk assessment.
    ///
    /// # Severity Levels
    ///
    /// - **Critical**: Direct threat to funds; immediate remediation required
    /// - **High**: Significant security issue; remediation required before production
    /// - **Medium**: Important issue that should be addressed; may be exploitable
    /// - **Low**: Best practice violation or minor issue; should be fixed but not critical
    ///
    /// # Security Notes
    ///
    /// - Severity is a general classification; context matters for actual risk assessment
    /// - Critical-rated issues may not be critical in specific contract contexts
    /// - Low-rated issues can compound to create higher-risk situations
    /// - Always perform manual review to validate severity in your specific context
    ///
    /// # Audit Trail
    ///
    /// Severity levels are used for:
    /// - Risk scoring (sum of all vulnerability scores)
    /// - Alerting and prioritization
    /// - SLA tracking and compliance reporting
    ///
    /// # Example
    /// ```ignore
    /// let severity = VulnerabilityType::MissingAccessControl.severity();
    /// println!(\"Severity: {}\", severity); // Output: Critical
    /// ```
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
            VulnerabilityType::InsufficientBalance => Severity::Critical,
            VulnerabilityType::BalanceUnderflow => Severity::Critical,
            VulnerabilityType::BalanceOverflow => Severity::High,
            VulnerabilityType::TransferWithoutBalanceCheck => Severity::Critical,
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

    /// Returns remediation guidance for this vulnerability type.
    ///
    /// Provides practical recommendations for fixing the identified vulnerability.
    /// The recommendation is static and general; developers must adapt it to their context.
    ///
    /// # Return Value
    ///
    /// A static string with specific remediation steps:
    /// - What to implement or change
    /// - Security patterns to use
    /// - Best practices to follow
    ///
    /// # Security Notes
    ///
    /// - Recommendations are generic; may need context-specific adjustments
    /// - Implementing the recommendation does not guarantee security
    /// - Always validate that the fix doesn't introduce new issues
    /// - Consider security review before deploying fixes
    ///
    /// # Implementation Guidance
    ///
    /// When implementing recommendations:
    /// 1. Understand the vulnerability type and its specific context
    /// 2. Review the recommendation in detail
    /// 3. Implement the fix in your specific context
    /// 4. Add tests to verify the fix works
    /// 5. Conduct security review before deployment
    ///
    /// # Example
    /// ```ignore
    /// let recommendation = VulnerabilityType::MissingAccessControl.recommendation();
    /// println!(\"Fix: {}\", recommendation);
    /// // Output: Implement proper access control using require_auth() or custom authorization logic
    /// ```
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
            VulnerabilityType::InsufficientBalance => "Add balance validation before all transfer operations",
            VulnerabilityType::BalanceUnderflow => "Implement underflow protection for balance operations",
            VulnerabilityType::BalanceOverflow => "Implement overflow protection for balance operations",
            VulnerabilityType::TransferWithoutBalanceCheck => "Add balance checks before executing transfers",
            
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
