//! Vulnerability alerting for newly-discovered dependency issues.
//!
//! Tracks which advisory ids have already been seen, so a refreshed advisory
//! feed only alerts on *newly-disclosed* vulnerabilities that affect the
//! current inventory — avoiding repeat noise for already-known issues.

use crate::supply_chain::inventory::DependencyInventory;
use crate::supply_chain::vuln::{scan, AdvisoryDatabase, VulnFinding};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// An alert for a newly-discovered vulnerability affecting the inventory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VulnAlert {
    /// Advisory id.
    pub advisory_id: String,
    /// Affected package.
    pub package: String,
    /// Severity.
    pub severity: String,
    /// Summary.
    pub summary: String,
}

impl VulnAlert {
    fn from_finding(f: &VulnFinding) -> Self {
        Self {
            advisory_id: f.advisory.id.clone(),
            package: f.advisory.package.clone(),
            severity: f.advisory.severity.as_str().to_string(),
            summary: f.advisory.summary.clone(),
        }
    }
}

/// Tracks seen advisories to emit alerts only for new ones.
#[derive(Debug, Clone, Default)]
pub struct VulnAlerter {
    seen: HashSet<String>,
}

impl VulnAlerter {
    /// Creates an alerter with no history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Re-scans the inventory against the (possibly updated) database and
    /// returns alerts only for advisories not previously seen.
    pub fn check(
        &mut self,
        inventory: &DependencyInventory,
        db: &AdvisoryDatabase,
    ) -> Vec<VulnAlert> {
        let report = scan(inventory, db);
        let mut alerts = Vec::new();
        for finding in &report.findings {
            if self.seen.insert(finding.advisory.id.clone()) {
                alerts.push(VulnAlert::from_finding(finding));
            }
        }
        alerts
    }

    /// Number of distinct advisories alerted on so far.
    pub fn known_count(&self) -> usize {
        self.seen.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supply_chain::inventory::{Dependency, Ecosystem};
    use crate::supply_chain::version::{Version, VersionRange};
    use crate::supply_chain::vuln::{Advisory, VulnSeverity};

    fn inventory() -> DependencyInventory {
        let mut inv = DependencyInventory::new();
        inv.add(Dependency::new(
            "lib",
            Version::new(1, 0, 0),
            Ecosystem::Cargo,
        ));
        inv
    }

    fn advisory(id: &str) -> Advisory {
        Advisory {
            id: id.to_string(),
            ecosystem: Ecosystem::Cargo,
            package: "lib".to_string(),
            range: VersionRange::below(Version::new(2, 0, 0)),
            severity: VulnSeverity::High,
            fixed_version: Some(Version::new(2, 0, 0)),
            summary: "issue".to_string(),
        }
    }

    #[test]
    fn first_scan_alerts_then_silences_known() {
        let inv = inventory();
        let mut db = AdvisoryDatabase::new();
        db.add(advisory("ADV-1"));
        let mut alerter = VulnAlerter::new();

        // First check alerts on the known-affecting advisory.
        let first = alerter.check(&inv, &db);
        assert_eq!(first.len(), 1);
        // Re-checking the same db produces no new alerts.
        assert!(alerter.check(&inv, &db).is_empty());
    }

    #[test]
    fn newly_disclosed_advisory_alerts() {
        let inv = inventory();
        let mut db = AdvisoryDatabase::new();
        db.add(advisory("ADV-1"));
        let mut alerter = VulnAlerter::new();
        alerter.check(&inv, &db);

        // A new advisory is disclosed → one fresh alert.
        db.add(advisory("ADV-2"));
        let new_alerts = alerter.check(&inv, &db);
        assert_eq!(new_alerts.len(), 1);
        assert_eq!(new_alerts[0].advisory_id, "ADV-2");
        assert_eq!(alerter.known_count(), 2);
    }
}
