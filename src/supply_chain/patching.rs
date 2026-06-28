//! Automated security patching with approval workflows.
//!
//! Turns a vulnerability finding into a [`PatchProposal`] (bump to the fixed
//! version) and decides, per policy, whether it can be auto-merged or needs
//! human approval — major-version bumps and high-risk packages always require a
//! reviewer; low-risk patch/minor security fixes can flow automatically.

use crate::supply_chain::policy::DependencyPolicy;
use crate::supply_chain::version::{update_kind, UpdateKind, Version};
use crate::supply_chain::vuln::{VulnFinding, VulnSeverity};
use serde::{Deserialize, Serialize};

/// The approval decision for a patch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalDecision {
    /// May be applied automatically.
    AutoApprove,
    /// Requires human approval, with a reason.
    RequireApproval(String),
}

impl ApprovalDecision {
    /// Whether this decision allows automatic application.
    pub fn is_auto(&self) -> bool {
        matches!(self, ApprovalDecision::AutoApprove)
    }
}

/// A proposed dependency update to remediate a vulnerability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchProposal {
    /// Package name.
    pub package: String,
    /// Current version.
    pub from: Version,
    /// Target (fixed) version.
    pub to: Version,
    /// The update kind (patch/minor/major).
    pub kind: UpdateKind,
    /// Advisory ids this patch resolves.
    pub fixes: Vec<String>,
    /// Highest severity being remediated.
    pub severity: VulnSeverity,
}

impl PatchProposal {
    /// Builds a proposal from a finding, if the advisory has a fixed version
    /// newer than the current one.
    pub fn from_finding(finding: &VulnFinding) -> Option<Self> {
        let to = finding.advisory.fixed_version?;
        let from = finding.dependency.version;
        let kind = update_kind(&from, &to);
        if kind == UpdateKind::None {
            return None;
        }
        Some(Self {
            package: finding.dependency.name.clone(),
            from,
            to,
            kind,
            fixes: vec![finding.advisory.id.clone()],
            severity: finding.advisory.severity,
        })
    }

    /// Decides whether this proposal can be auto-applied under `policy`.
    pub fn approval(&self, policy: &DependencyPolicy) -> ApprovalDecision {
        if policy.is_high_risk(&self.package) {
            return ApprovalDecision::RequireApproval(format!(
                "{} is a high-risk package",
                self.package
            ));
        }
        match self.kind {
            UpdateKind::Major => ApprovalDecision::RequireApproval(
                "major version bump may contain breaking changes".to_string(),
            ),
            UpdateKind::Patch | UpdateKind::Minor => ApprovalDecision::AutoApprove,
            UpdateKind::None => ApprovalDecision::RequireApproval("no version change".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supply_chain::inventory::{Dependency, Ecosystem};
    use crate::supply_chain::version::VersionRange;
    use crate::supply_chain::vuln::Advisory;

    fn finding(dep_version: Version, fixed: Version, pkg: &str) -> VulnFinding {
        VulnFinding {
            dependency: Dependency::new(pkg, dep_version, Ecosystem::Cargo),
            advisory: Advisory {
                id: "ADV-1".to_string(),
                ecosystem: Ecosystem::Cargo,
                package: pkg.to_string(),
                range: VersionRange::below(fixed),
                severity: VulnSeverity::High,
                fixed_version: Some(fixed),
                summary: "bug".to_string(),
            },
        }
    }

    #[test]
    fn patch_update_auto_approves() {
        let f = finding(Version::new(1, 2, 0), Version::new(1, 2, 5), "lib");
        let p = PatchProposal::from_finding(&f).unwrap();
        assert_eq!(p.kind, UpdateKind::Patch);
        assert!(p.approval(&DependencyPolicy::default()).is_auto());
    }

    #[test]
    fn major_update_requires_approval() {
        let f = finding(Version::new(1, 9, 0), Version::new(2, 0, 0), "lib");
        let p = PatchProposal::from_finding(&f).unwrap();
        assert_eq!(p.kind, UpdateKind::Major);
        assert!(!p.approval(&DependencyPolicy::default()).is_auto());
    }

    #[test]
    fn high_risk_package_requires_approval_even_for_patch() {
        let f = finding(Version::new(1, 0, 0), Version::new(1, 0, 1), "openssl-sys");
        let p = PatchProposal::from_finding(&f).unwrap();
        let policy = DependencyPolicy {
            high_risk_packages: vec!["openssl-sys".to_string()],
            ..DependencyPolicy::default()
        };
        match p.approval(&policy) {
            ApprovalDecision::RequireApproval(r) => assert!(r.contains("high-risk")),
            ApprovalDecision::AutoApprove => panic!("should require approval"),
        }
    }

    #[test]
    fn no_fixed_version_yields_no_proposal() {
        let mut f = finding(Version::new(1, 0, 0), Version::new(1, 0, 1), "lib");
        f.advisory.fixed_version = None;
        assert!(PatchProposal::from_finding(&f).is_none());
    }
}
