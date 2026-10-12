//! Security and Invariant Scanners for Stellar Smart Contracts
//!
//! This crate provides comprehensive security analysis tools for Stellar Soroban contracts,
//! including vulnerability detection, invariant checking, and best practices enforcement.

#![recursion_limit = "2048"]

// === Core types that compile cleanly ===

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub file_path: String,
    pub vulnerabilities: Vec<crate::vulnerabilities::VulnerabilityType>,
    pub invariant_violations: Vec<crate::invariants::InvariantRule>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
pub mod error_handler;
pub mod observability;

// Comprehensive audit trail for security-critical operations (#326). Compiles
// cleanly with no feature gate so its tests run under default `cargo test`.
pub mod audit_trail;
pub use audit_trail::{
    ActorContext, AuditAction, AuditCategory, AuditConfig, AuditEvent, AuditEventBuilder,
    AuditOutcome, AuditQuery, AuditSeverity, AuditTrail, ChainVerification,
    SuspiciousActivityAlert, UserRole,
};

// Input sanitization & validation for contract-code uploads (issue #330).
// Self-contained and compiles cleanly under default features.
pub mod upload_sanitization;

// Scan access control & ownership verification (issue #329).
// Self-contained RBAC, ownership checks, scan sharing, and audit logging
// to prevent Insecure Direct Object Reference (IDOR) vulnerabilities.
pub mod scan_access_control;
pub use scan_access_control::{
    ScanAccessAction, ScanAccessControl, ScanAccessControlConfig, ScanAccessError,
    ScanAccessLogEntry, ScanAccessMetadata, ScanAccessRole, ScanOwnershipGuard, ScanRecord,
    ScanSeveritySummary, ScanStatus, ShareScanRequest, ShareScanResponse,
};

#[cfg(test)]
mod scan_access_control_tests;

// Secure database connection pooling, TLS, monitoring and replica routing
// (issue #331). Self-contained and compiles cleanly under default features.
pub mod db_pool;

// Protocol-Level Invariant Verification (Issue #449)
// Self-contained protocol analysis engine for multi-contract invariant verification.
// Compiles cleanly without any feature gates.
pub mod protocol_analysis;

// Real-time security monitoring, alerting, SIEM and incident response
// (issue #333). Self-contained and compiles cleanly under default features.
pub mod security_monitoring;
pub use security_monitoring::{
    AlertDispatcher, AlertMessage, AlertRouting, AnomalyConfig, AnomalyDetector, AnomalyScore,
    BaselineConfig, BaselineLearner, BaselineStatistics, BaselineStatus, ChannelKind, Component,
    DashboardSnapshot, DetectionConfig, DispatchResult, EventKind, Finding, Incident,
    IncidentStatus, Priority, ProcessOutcome, ResetBaselineResponse, RuleEngine, SecurityEvent,
    SecurityMonitor, SecurityMonitoringApiState, SecuritySeverity,
};

// Authentication & authorization (JWT, sessions, rate limiting, account
// lockout, OAuth, security headers). Self-contained and compiles cleanly
// under default features.
pub mod auth;
pub use auth::{
    AccountLockoutService, AuthContext, AuthMiddleware, AuthMiddlewareConfig, AuthServices,
    CspBuilder, InMemoryLockoutStore, InMemoryRateLimitStore, InMemorySessionStore, JwtClaims,
    JwtError, JwtService, LockoutConfig, LockoutError, LockoutStatus, OAuthError, OAuthProvider,
    OAuthService, OAuthUserInfo, PasswordConfig, PasswordError, PasswordService, PasswordStrength,
    RateLimitConfig, RateLimitError, RateLimitService, RateLimitStatus, RevocationError,
    RevokedToken, SecurityHeadersConfig, SecurityHeadersMiddleware, SessionData, SessionError,
    SessionManager, SessionStore, TokenRevocationList,
};

// Analysis, reporting and scanner modules required by the CLI and the
// integration test suite. Wired under default features; the remaining
// modules stay behind the `broken-modules` feature gate.
pub mod analysis;
pub mod config;
pub mod emergency_stop;
pub mod event_logging;
pub mod gas_limits;
pub mod incremental_scan;
pub mod invariants;
pub mod report;
pub mod scanners;
pub mod secure_id_generation;
pub mod severity_scoring;
pub mod vulnerabilities;

// === Broken modules gated behind feature flag ===
// Each module has pre-existing compilation errors (borrow checker violations,
// missing trait impls, type mismatches, unresolved imports) that are being
// fixed incrementally. Enable "broken-modules" feature to include them.

#[cfg(feature = "broken-modules")]
pub mod audit_proof_of_scan;
#[cfg(feature = "broken-modules")]
pub mod batch_operations;
#[cfg(feature = "broken-modules")]
pub mod database;
#[cfg(feature = "broken-modules")]
pub mod differential_fuzzing;
#[cfg(feature = "broken-modules")]
pub mod escrow;
#[cfg(feature = "broken-modules")]
pub mod kubernetes;
#[cfg(feature = "broken-modules")]
pub mod multisig;
#[cfg(feature = "broken-modules")]
pub mod notification_service;
#[cfg(feature = "broken-modules")]
pub mod rate_limiting;
#[cfg(feature = "broken-modules")]
pub mod scanner_registry;
#[cfg(feature = "broken-modules")]
pub mod security_analyzer;
#[cfg(feature = "broken-modules")]
pub mod session;
#[cfg(feature = "broken-modules")]
pub mod storage_safety;
#[cfg(feature = "broken-modules")]
pub mod time_travel_debugger;
#[cfg(feature = "broken-modules")]
pub mod wallet;

// Economic exploit simulation framework (#444).
// Models DeFi protocols and searches for profitable attack sequences:
// flash loans, oracle manipulation, and MEV detection.
#[cfg(feature = "broken-modules")]
pub mod economic_security;

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
pub use db_pool::{DbPool, DbPoolConfig, PoolMonitor};
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
pub use protocol_analysis::{
    dashboard::ProtocolHealth, manifest::ProtocolManifest, InvariantKind, ProtocolInvariant,
    ProtocolVerificationReport, VerificationStatus,
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
pub use upload_sanitization::{
    SanitizationPipeline, SignatureValidationResult, SorobanContractInterface,
};
#[cfg(feature = "broken-modules")]
pub use wallet::{
    CreateWalletRequest, ImportWalletRequest, InMemoryWalletStore, RestoreWalletRequest, Wallet,
    WalletBalance, WalletError, WalletExport, WalletService, WalletStatus, WalletStore,
    WalletSyncRecord, WalletType,
};

#[cfg(feature = "broken-modules")]
pub use economic_security::{
    attack_agent::{AgentObjective, AttackAgent, AttackCapability},
    defi_primitives::{
        ConstantProductAmm, DeFiPrimitive, DeFiPrimitiveType, LendingPool, LiquidityPool, Oracle,
        OracleType, StakingRewards, StateChange, TwapOracle,
    },
    flash_loan::{FlashLoanAttack, FlashLoanScenario, FlashLoanSimulator},
    mev_detection::{MevDetector, MevOpportunity, MevType, SandwichAttack, TransactionOrder},
    oracle_detection::{OracleManipulationDetector, OracleManipulationScenario, PriceDeviation},
    profitability::{
        ExploitDifficulty, ExploitProfitability, ProfitabilityAnalyzer, ProfitabilityScore,
    },
    report::{AttackSequence, EconomicExploitReport, EconomicExploitSummary, EconomicFinding},
    search_engine::{
        BeamSearch, GeneticAlgorithm, MonteCarloTreeSearch, SearchAlgorithm, SearchConfig,
        SearchResult, TransactionSequence,
    },
};
