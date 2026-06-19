//! Security and Invariant Scanners for Stellar Smart Contracts
//!
//! This crate provides comprehensive security analysis tools for Stellar Soroban contracts,
//! including vulnerability detection, invariant checking, and best practices enforcement.

pub mod address_filter;
pub mod analysis;
pub mod audit_proof_of_scan;
pub mod batch_operations;
pub mod config;
pub mod database;
pub mod differential_fuzzing;
pub mod emergency_stop;
pub mod escrow;
pub mod event_logging;
pub mod gas_limits;
pub mod invariants;
pub mod kubernetes;
pub mod multisig;
pub mod notification_service;
pub mod rate_limiting;
pub mod report;
pub mod scanner_registry;
pub mod scanners;
pub mod secure_id_generation;
pub mod security_analyzer;
pub mod session;
pub mod time_travel_debugger;
pub mod wallet;

#[cfg(test)]
mod multisig_tests;

pub use analysis::AnalysisResult;
pub use audit_proof_of_scan::{
    AuditProofOfScan, CertificateStatus, RiskScore, SecurityCertificate,
};
pub use batch_operations::{
    BatchEscrowReleaseRequest, BatchOperationResult, BatchOperationStatus, BatchOperationSummary,
    BatchOperations, BatchVerificationRequest,
};
pub use config::ScannerConfig;
pub use differential_fuzzing::{
    DifferentialFuzzer, DifferentialFuzzingConfig, DifferentialFuzzingReport, DiscrepancyDetector,
    ExecutionResult, NonDeterministicBehavior, SdkVersion, TestInput,
};
pub use invariants::InvariantRule;
pub use kubernetes::{K8sScanManager, ScanAutoScaler, ScanPodConfig};
pub use multisig::{
    AggregatedSignatures, CreateProposalRequest, InMemoryMultiSigStore, MultiSigError,
    MultiSigProposal, MultiSigService, MultiSigSigner, MultiSigStore, ProposalStatus,
    SignatureEntry, SignerDecision, SignerSpec, SubmitSignatureRequest,
};
pub use notification_service::{
    DeliveryStatus, DeliveryTracker, InMemoryBackend, NotificationChannel, NotificationMessage,
    NotificationPriority, NotificationProvider, NotificationResult, NotificationService,
    NotificationServiceTrait, NotificationTemplate, Recipient, StorageBackend, TemplateManager,
};
pub use rate_limiting::{
    EndpointRateLimit, RateLimitConfig, RateLimitContext, RateLimitMiddleware, RateLimitPolicy,
    RateLimitResult, RateLimitStats, RateLimitStorage, RateLimitTier, RateLimitViolation,
    RateLimitWindow, RateLimiter,
};
pub use report::{ReportFormat, SecurityReport};
pub use scanner_registry::{ScannerRegistry, ScannerVersion, VersionStatus};
pub use scanners::{InvariantScanner, SecurityScanner};
pub use session::stateless::{
    ExternalSessionStore, InMemorySessionStore, SessionClaims, SessionError, SessionStoreRecord,
    StatelessSessionManager,
};
pub use time_travel_debugger::{
    CacheStats, ContractState, ForkedState, LedgerSnapshot, TestResult, TimeTravelConfig,
    TimeTravelDebugger, UpgradeSimulationResult,
};
pub use vulnerabilities::VulnerabilityType;
pub use wallet::{
    CreateWalletRequest, ImportWalletRequest, InMemoryWalletStore, RestoreWalletRequest, Wallet,
    WalletBalance, WalletError, WalletExport, WalletService, WalletStatus, WalletStore,
    WalletSyncRecord, WalletType,
};

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub file_path: String,
    pub vulnerabilities: Vec<VulnerabilityType>,
    pub invariant_violations: Vec<InvariantRule>,
    pub recommendations: Vec<String>,
}

impl ScanResult {
    pub fn new(file_path: String) -> Self {
        Self {
            file_path,
            vulnerabilities: Vec::new(),
            invariant_violations: Vec::new(),
            recommendations: Vec::new(),
        }
    }

    pub fn has_issues(&self) -> bool {
        !self.vulnerabilities.is_empty() || !self.invariant_violations.is_empty()
    }

    pub fn severity_count(&self) -> (usize, usize, usize) {
        let mut critical = 0;
        let mut high = 0;
        let mut medium = 0;

        for vuln in &self.vulnerabilities {
            match vuln.severity() {
                Severity::Critical => critical += 1,
                Severity::High => high += 1,
                Severity::Medium => medium += 1,
                Severity::Low => {}
            }
        }

        (critical, high, medium)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Critical => "CRITICAL",
            Severity::High => "HIGH",
            Severity::Medium => "MEDIUM",
            Severity::Low => "LOW",
        }
    }
}
