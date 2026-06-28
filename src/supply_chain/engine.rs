//! The supply-chain security guard.
//!
//! The top-level façade over the dependency inventory: it runs vulnerability
//! scans, generates the SBOM, proposes (and triages) security patches per
//! policy, checks licenses, and reports overall posture — the single object an
//! operator or CI step drives.

use crate::supply_chain::alerting::{VulnAlert, VulnAlerter};
use crate::supply_chain::inventory::DependencyInventory;
use crate::supply_chain::patching::{ApprovalDecision, PatchProposal};
use crate::supply_chain::policy::{DependencyPolicy, PolicyViolation};
use crate::supply_chain::sbom::Sbom;
use crate::supply_chain::vuln::{scan, AdvisoryDatabase, ScanReport, VulnSeverity};
use serde::{Deserialize, Serialize};

/// A triaged patch: the proposal plus its approval decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriagedPatch {
    /// The proposed update.
    pub proposal: PatchProposal,
    /// Whether it can auto-merge or needs review.
    pub decision: ApprovalDecision,
}

/// Overall supply-chain posture report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostureReport {
    /// Total dependencies.
    pub total_dependencies: usize,
    /// Scan coverage fraction (target 1.0).
    pub scan_coverage: f64,
    /// Number of vulnerability findings.
    pub vulnerabilities: usize,
    /// Worst severity found, if any.
    pub worst_severity: Option<VulnSeverity>,
    /// License policy violations.
    pub license_violations: usize,
    /// Whether the posture is release-ready (full coverage, no release-blocking
    /// vulns, no license violations).
    pub release_ready: bool,
}

/// The supply-chain guard over an inventory + advisory DB + policy.
pub struct SupplyChainGuard {
    inventory: DependencyInventory,
    db: AdvisoryDatabase,
    policy: DependencyPolicy,
    alerter: VulnAlerter,
}

impl SupplyChainGuard {
    /// Creates a guard.
    pub fn new(
        inventory: DependencyInventory,
        db: AdvisoryDatabase,
        policy: DependencyPolicy,
    ) -> Self {
        Self {
            inventory,
            db,
            policy,
            alerter: VulnAlerter::new(),
        }
    }

    /// The dependency inventory.
    pub fn inventory(&self) -> &DependencyInventory {
        &self.inventory
    }

    /// Runs a vulnerability scan.
    pub fn scan(&self) -> ScanReport {
        scan(&self.inventory, &self.db)
    }

    /// Generates the SBOM.
    pub fn sbom(&self) -> Sbom {
        Sbom::from_inventory(&self.inventory)
    }

    /// Proposes and triages security patches for every fixable finding.
    pub fn propose_patches(&self) -> Vec<TriagedPatch> {
        self.scan()
            .findings
            .iter()
            .filter_map(|f| {
                PatchProposal::from_finding(f).map(|proposal| {
                    let decision = proposal.approval(&self.policy);
                    TriagedPatch { proposal, decision }
                })
            })
            .collect()
    }

    /// Checks every dependency's license against policy.
    pub fn license_violations(&self) -> Vec<PolicyViolation> {
        self.inventory
            .dependencies
            .iter()
            .filter_map(|d| self.policy.check_license(d))
            .collect()
    }

    /// Re-checks the advisory feed and returns alerts for newly-disclosed
    /// vulnerabilities affecting the inventory.
    pub fn check_new_vulnerabilities(&mut self) -> Vec<VulnAlert> {
        self.alerter.check(&self.inventory, &self.db)
    }

    /// Computes the overall posture report.
    pub fn posture(&self) -> PostureReport {
        let report = self.scan();
        let worst = report.worst_severity();
        let license_violations = self.license_violations().len();
        let release_blocking = worst
            .map(|s| self.policy.blocks_release(s))
            .unwrap_or(false);
        PostureReport {
            total_dependencies: self.inventory.len(),
            scan_coverage: report.coverage(),
            vulnerabilities: report.findings.len(),
            worst_severity: worst,
            license_violations,
            release_ready: report.is_full_coverage()
                && !release_blocking
                && license_violations == 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supply_chain::inventory::{Dependency, Ecosystem};
    use crate::supply_chain::version::{Version, VersionRange};
    use crate::supply_chain::vuln::Advisory;

    fn vulnerable_advisory() -> Advisory {
        Advisory {
            id: "RUSTSEC-2024-0009".to_string(),
            ecosystem: Ecosystem::Cargo,
            package: "vuln-lib".to_string(),
            range: VersionRange::below(Version::new(1, 4, 0)),
            severity: VulnSeverity::Critical,
            fixed_version: Some(Version::new(1, 4, 0)),
            summary: "critical RCE".to_string(),
        }
    }

    fn guard_with_vuln() -> SupplyChainGuard {
        let mut inv = DependencyInventory::new();
        inv.add(
            Dependency::new("vuln-lib", Version::new(1, 3, 0), Ecosystem::Cargo)
                .with_license("MIT"),
        );
        inv.add(
            Dependency::new("serde", Version::new(1, 0, 0), Ecosystem::Cargo).with_license("MIT"),
        );
        let mut db = AdvisoryDatabase::new();
        db.add(vulnerable_advisory());
        SupplyChainGuard::new(inv, db, DependencyPolicy::default())
    }

    #[test]
    fn scan_and_sbom_cover_inventory() {
        let g = guard_with_vuln();
        assert!(g.scan().is_full_coverage());
        assert_eq!(g.sbom().len(), 2);
    }

    #[test]
    fn proposes_patch_for_vulnerability() {
        let g = guard_with_vuln();
        let patches = g.propose_patches();
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].proposal.to, Version::new(1, 4, 0));
        // 1.3.0 -> 1.4.0 is a minor bump → auto-approve.
        assert!(patches[0].decision.is_auto());
    }

    #[test]
    fn critical_vuln_blocks_release() {
        let g = guard_with_vuln();
        let posture = g.posture();
        assert_eq!(posture.scan_coverage, 1.0);
        assert_eq!(posture.worst_severity, Some(VulnSeverity::Critical));
        assert!(!posture.release_ready);
    }

    #[test]
    fn clean_inventory_is_release_ready() {
        let mut inv = DependencyInventory::new();
        inv.add(
            Dependency::new("serde", Version::new(1, 0, 0), Ecosystem::Cargo).with_license("MIT"),
        );
        let g = SupplyChainGuard::new(inv, AdvisoryDatabase::new(), DependencyPolicy::default());
        let posture = g.posture();
        assert!(posture.release_ready);
        assert_eq!(posture.vulnerabilities, 0);
    }

    #[test]
    fn license_violation_blocks_release() {
        let mut inv = DependencyInventory::new();
        inv.add(
            Dependency::new("gpl-lib", Version::new(1, 0, 0), Ecosystem::Cargo)
                .with_license("GPL-3.0"),
        );
        let g = SupplyChainGuard::new(inv, AdvisoryDatabase::new(), DependencyPolicy::default());
        let posture = g.posture();
        assert_eq!(posture.license_violations, 1);
        assert!(!posture.release_ready);
    }

    #[test]
    fn alerts_on_new_vulnerability() {
        let mut g = guard_with_vuln();
        // First check alerts on the existing critical advisory.
        assert_eq!(g.check_new_vulnerabilities().len(), 1);
        // Re-check: already known, no new alerts.
        assert!(g.check_new_vulnerabilities().is_empty());
    }
}
