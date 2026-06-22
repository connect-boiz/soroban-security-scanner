//! API Versioning and Backward Compatibility
//!
//! This module provides a comprehensive API versioning system with:
//! - URL-based versioning (`/api/v1/`, `/api/v2/`)
//! - Accept header version negotiation (`application/vnd.soroban.v1+json`)
//! - Version lifecycle management (alpha, beta, stable, deprecated, sunset)
//! - Deprecation policy with minimum 6-month notice period
//! - Change log with breaking/non-breaking change classification
//! - Backward compatibility testing support
//! - Sunset procedures with automated notifications

pub mod changelog;
pub mod compatibility;
pub mod deprecation;
pub mod negotiation;
pub mod router;
pub mod version;

pub use changelog::{ApiChangeLog, ChangeEntry, ChangeType};
pub use compatibility::{scenarios, CheckResult, CompatibilityReport, CompatibilityTestSuite};
pub use deprecation::{DeprecationPolicy, SunsetProcedures, UrgencyNotification, VersionRegistry};
pub use negotiation::{VersionError, VersionNegotiator};
pub use router::{VersionedRouter, VersionedRouterConfig};
pub use version::{ApiVersion, VersionInfo, VersionLifecycle};
