//! Security and Invariant Scanners for Stellar Smart Contracts
//!
//! This crate provides comprehensive security analysis tools for Stellar Soroban contracts,
//! including vulnerability detection, invariant checking, and best practices enforcement.

// === Core types that compile cleanly ===

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub file_path: String,
    pub vulnerabilities: Vec<String>,
    pub invariant_violations: Vec<String>,
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
        (0, 0, 0)
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

// === Clean modules (no feature gate needed) ===
// The api_versioning module compiles cleanly without the broken-modules feature.
pub mod api_versioning;

// Multi-factor authentication for administrative access (issue #328).
// Self-contained and compiles cleanly under default features.
pub mod admin_mfa;

// === Broken modules gated behind feature flag ===
// Each module has pre-existing compilation errors (borrow checker violations,
// missing trait impls, type mismatches, unresolved imports) that are being
// fixed incrementally. Enable "broken-modules" feature to include them.

#[cfg(feature = "broken-modules")]
pub mod address_filter;
#[cfg(feature = "broken-modules")]
pub mod analysis;
#[cfg(feature = "broken-modules")]
pub mod audit_proof_of_scan;
#[cfg(feature = "broken-modules")]
pub mod batch_operations;
#[cfg(feature = "broken-modules")]
pub mod config;
#[cfg(feature = "broken-modules")]
pub mod database;
#[cfg(feature = "broken-modules")]
pub mod differential_fuzzing;
#[cfg(feature = "broken-modules")]
pub mod emergency_stop;
#[cfg(feature = "broken-modules")]
pub mod escrow;
#[cfg(feature = "broken-modules")]
pub mod event_logging;
#[cfg(feature = "broken-modules")]
pub mod gas_limits;
#[cfg(feature = "broken-modules")]
pub mod invariants;
#[cfg(feature = "broken-modules")]
pub mod kubernetes;
#[cfg(feature = "broken-modules")]
pub mod multisig;
#[cfg(feature = "broken-modules")]
pub mod notification_service;
#[cfg(feature = "broken-modules")]
pub mod rate_limiting;
#[cfg(feature = "broken-modules")]
pub mod report;
#[cfg(feature = "broken-modules")]
pub mod scanner_registry;
#[cfg(feature = "broken-modules")]
pub mod scanners;
#[cfg(feature = "broken-modules")]
pub mod secure_id_generation;
#[cfg(feature = "broken-modules")]
pub mod security_analyzer;
#[cfg(feature = "broken-modules")]
pub mod session;
#[cfg(feature = "broken-modules")]
pub mod time_travel_debugger;
#[cfg(feature = "broken-modules")]
pub mod wallet;

#[cfg(feature = "broken-modules")]
#[cfg(test)]
mod multisig_tests;

#[cfg(feature = "broken-modules")]
pub use analysis::AnalysisResult;
#[cfg(feature = "broken-modules")]
pub use audit_proof_of_scan::{
    AuditProofOfScan, CertificateStatus, RiskScore, SecurityCertificate,
};
#[cfg(feature = "broken-modules")]
pub use batch_operations::{
    BatchEscrowReleaseRequest, BatchOperationResult, BatchOperationStatus, BatchOperationSummary,
    BatchOperations, BatchVerificationRequest,
};
#[cfg(feature = "broken-modules")]
pub use config::ScannerConfig;
#[cfg(feature = "broken-modules")]
pub use differential_fuzzing::{
    DifferentialFuzzer, DifferentialFuzzingConfig, DifferentialFuzzingReport, DiscrepancyDetector,
    ExecutionResult, NonDeterministicBehavior, SdkVersion, TestInput,
};
#[cfg(feature = "broken-modules")]
pub use invariants::InvariantRule;
#[cfg(feature = "broken-modules")]
pub use kubernetes::{K8sScanManager, ScanAutoScaler, ScanPodConfig};
#[cfg(feature = "broken-modules")]
pub use multisig::{
    AggregatedSignatures, CreateProposalRequest, InMemoryMultiSigStore, MultiSigError,
    MultiSigProposal, MultiSigService, MultiSigSigner, MultiSigStore, ProposalStatus,
    SignatureEntry, SignerDecision, SignerSpec, SubmitSignatureRequest,
};
#[cfg(feature = "broken-modules")]
pub use notification_service::{
    DeliveryStatus, DeliveryTracker, InMemoryBackend, NotificationChannel, NotificationMessage,
    NotificationPriority, NotificationProvider, NotificationResult, NotificationService,
    NotificationServiceTrait, NotificationTemplate, Recipient, StorageBackend, TemplateManager,
};
#[cfg(feature = "broken-modules")]
pub use rate_limiting::{
    EndpointRateLimit, RateLimitConfig, RateLimitContext, RateLimitMiddleware, RateLimitPolicy,
    RateLimitResult, RateLimitStats, RateLimitStorage, RateLimitTier, RateLimitViolation,
    RateLimitWindow, RateLimiter,
};
#[cfg(feature = "broken-modules")]
pub use report::{ReportFormat, SecurityReport};
#[cfg(feature = "broken-modules")]
pub use scanner_registry::{ScannerRegistry, ScannerVersion, VersionStatus};
#[cfg(feature = "broken-modules")]
pub use scanners::{InvariantScanner, SecurityScanner};
#[cfg(feature = "broken-modules")]
pub use session::stateless::{
    ExternalSessionStore, InMemorySessionStore, SessionClaims, SessionError, SessionStoreRecord,
    StatelessSessionManager,
};
#[cfg(feature = "broken-modules")]
pub use time_travel_debugger::{
    CacheStats, ContractState, ForkedState, LedgerSnapshot, TestResult, TimeTravelConfig,
    TimeTravelDebugger, UpgradeSimulationResult,
};
#[cfg(feature = "broken-modules")]
pub use wallet::{
    CreateWalletRequest, ImportWalletRequest, InMemoryWalletStore, RestoreWalletRequest, Wallet,
    WalletBalance, WalletError, WalletExport, WalletService, WalletStatus, WalletStore,
    WalletSyncRecord, WalletType,
};
