//! Comprehensive API Security Testing Framework
//!
//! Provides automated security testing, penetration test scheduling,
//! regression suites, fuzzing, coverage reporting, and defect tracking
//! for all API endpoints. Implements issue #348 acceptance criteria.

pub mod acceptance;
pub mod coverage;
pub mod defect_tracking;
pub mod endpoints;
pub mod fuzzing;
pub mod regression;
pub mod scenarios;
pub mod suite;

pub use coverage::{CoverageGate, SecurityCoverageReport};
pub use defect_tracking::{DefectSeverity, DefectStatus, SecurityDefect, SecurityDefectTracker};
pub use endpoints::{ApiEndpoint, EndpointAuth, EndpointRegistry, HttpMethod};
pub use fuzzing::{FuzzCase, FuzzResult, FuzzingEngine};
pub use regression::{HistoricalVulnerability, RegressionTestSuite};
pub use suite::{SecurityCheckResult, SecurityReport, SecurityTestSuite};
