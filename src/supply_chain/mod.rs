//! Dependency vulnerability management and supply-chain security (issue #342).
//!
//! A self-contained toolkit for securing third-party dependencies: an inventory
//! with lifecycle tracking, vulnerability scanning against an advisory feed,
//! SBOM generation, artifact integrity verification, automated security
//! patching with approval workflows, a dependency security policy, and alerting
//! on newly-disclosed vulnerabilities.
//!
//! # Acceptance-criteria mapping
//!
//! | Requirement | Where |
//! |-------------|-------|
//! | Automated dependency vulnerability scanning | [`vuln::scan`], [`vuln::AdvisoryDatabase`] |
//! | Automated patching with approval workflows | [`patching::PatchProposal`], [`patching::ApprovalDecision`] |
//! | Software Bill of Materials (SBOM) | [`sbom::Sbom`] |
//! | Dependency integrity verification (checksums) | [`integrity::verify`] |
//! | Dependency inventory + lifecycle management | [`inventory::DependencyInventory`] |
//! | Vulnerability alerting for new issues | [`alerting::VulnAlerter`] |
//! | Dependency security policy (high-risk, licenses) | [`policy::DependencyPolicy`] |
//! | 100% dependency coverage in scanning | [`vuln::ScanReport::is_full_coverage`] |
//! | Comprehensive supply-chain testing | per-module tests + [`tests`] |
//!
//! Supply-chain controls such as signed commits and branch protection are
//! repository/CI configuration; this module provides the in-process scanning,
//! SBOM, integrity and policy machinery they complement.
//!
//! # Example
//!
//! ```
//! use soroban_security_scanner::supply_chain::*;
//!
//! let mut inventory = DependencyInventory::new();
//! inventory.add(Dependency::new("serde", Version::new(1, 0, 200), Ecosystem::Cargo).with_license("MIT"));
//! let guard = SupplyChainGuard::new(inventory, AdvisoryDatabase::new(), DependencyPolicy::default());
//! assert!(guard.scan().is_full_coverage());
//! assert!(guard.posture().release_ready);
//! ```

pub mod alerting;
pub mod engine;
pub mod integrity;
pub mod inventory;
pub mod patching;
pub mod policy;
pub mod sbom;
pub mod version;
pub mod vuln;

#[cfg(test)]
mod tests;

pub use alerting::{VulnAlert, VulnAlerter};
pub use engine::{PostureReport, SupplyChainGuard, TriagedPatch};
pub use integrity::{sha256_hex, verify as verify_integrity, IntegrityStatus, IntegritySummary};
pub use inventory::{Dependency, DependencyInventory, Ecosystem, Lifecycle};
pub use patching::{ApprovalDecision, PatchProposal};
pub use policy::{DependencyPolicy, PolicyViolation};
pub use sbom::{Sbom, SbomComponent};
pub use version::{update_kind, UpdateKind, Version, VersionRange};
pub use vuln::{scan, Advisory, AdvisoryDatabase, ScanReport, VulnFinding, VulnSeverity};
