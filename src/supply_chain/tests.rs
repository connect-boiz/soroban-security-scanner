//! End-to-end integration tests: scan an inventory, generate an SBOM, verify
//! integrity, triage patches per policy, and alert on new vulnerabilities.

use super::*;

fn inventory() -> DependencyInventory {
    let mut inv = DependencyInventory::new();
    inv.add(
        Dependency::new("serde", Version::new(1, 0, 200), Ecosystem::Cargo)
            .with_license("MIT")
            .with_checksum(sha256_hex(b"serde-artifact")),
    );
    inv.add(
        Dependency::new("vuln-lib", Version::new(1, 1, 0), Ecosystem::Cargo)
            .with_license("Apache-2.0"),
    );
    inv.add(
        Dependency::new("gpl-lib", Version::new(0, 9, 0), Ecosystem::Npm)
            .transitive()
            .with_license("GPL-3.0"),
    );
    inv
}

fn database() -> AdvisoryDatabase {
    let mut db = AdvisoryDatabase::new();
    db.add(Advisory {
        id: "RUSTSEC-2024-0042".to_string(),
        ecosystem: Ecosystem::Cargo,
        package: "vuln-lib".to_string(),
        range: VersionRange::below(Version::new(1, 2, 0)),
        severity: VulnSeverity::High,
        fixed_version: Some(Version::new(1, 2, 0)),
        summary: "auth bypass in vuln-lib".to_string(),
    });
    db
}

#[test]
fn end_to_end_scan_sbom_and_patches() {
    let guard = SupplyChainGuard::new(inventory(), database(), DependencyPolicy::default());

    // 100% scan coverage with one High finding.
    let report = guard.scan();
    assert!(report.is_full_coverage());
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.worst_severity(), Some(VulnSeverity::High));

    // SBOM lists every component with purls.
    let sbom = guard.sbom();
    assert_eq!(sbom.len(), 3);
    assert!(sbom.to_json().contains("pkg:cargo/vuln-lib@1.1.0"));

    // A patch is proposed (1.1.0 -> 1.2.0, minor → auto-approve).
    let patches = guard.propose_patches();
    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].proposal.to, Version::new(1, 2, 0));
    assert!(patches[0].decision.is_auto());
}

#[test]
fn integrity_detects_tampered_artifact() {
    let inv = inventory();
    let serde = inv.find(Ecosystem::Cargo, "serde").unwrap();
    // The pinned checksum verifies the real artifact and rejects a swap.
    assert_eq!(
        verify_integrity(serde, b"serde-artifact"),
        IntegrityStatus::Verified
    );
    assert_eq!(
        verify_integrity(serde, b"evil-artifact"),
        IntegrityStatus::Mismatch
    );

    // The unpinned dependency is reported, not silently trusted.
    let vuln_lib = inv.find(Ecosystem::Cargo, "vuln-lib").unwrap();
    assert_eq!(
        verify_integrity(vuln_lib, b"anything"),
        IntegrityStatus::Unpinned
    );
}

#[test]
fn policy_blocks_release_on_vuln_and_license() {
    let guard = SupplyChainGuard::new(inventory(), database(), DependencyPolicy::default());
    let posture = guard.posture();
    assert_eq!(posture.total_dependencies, 3);
    assert_eq!(posture.scan_coverage, 1.0);
    // High vuln blocks release at the default threshold...
    assert!(!posture.release_ready);
    // ...and the GPL-3.0 transitive dep is a license violation.
    assert!(posture.license_violations >= 1);
}

#[test]
fn newly_disclosed_vulnerability_alerts_once() {
    let mut guard = SupplyChainGuard::new(inventory(), database(), DependencyPolicy::default());
    // First pass alerts on the existing advisory.
    let first = guard.check_new_vulnerabilities();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].advisory_id, "RUSTSEC-2024-0042");
    // Re-running yields no repeat noise.
    assert!(guard.check_new_vulnerabilities().is_empty());
}

#[test]
fn high_risk_package_patch_requires_approval() {
    let mut inv = DependencyInventory::new();
    inv.add(
        Dependency::new("openssl-sys", Version::new(0, 9, 0), Ecosystem::Cargo).with_license("MIT"),
    );
    let mut db = AdvisoryDatabase::new();
    db.add(Advisory {
        id: "RUSTSEC-2024-0099".to_string(),
        ecosystem: Ecosystem::Cargo,
        package: "openssl-sys".to_string(),
        range: VersionRange::below(Version::new(0, 9, 1)),
        severity: VulnSeverity::Critical,
        fixed_version: Some(Version::new(0, 9, 1)),
        summary: "memory corruption".to_string(),
    });
    let policy = DependencyPolicy {
        high_risk_packages: vec!["openssl-sys".to_string()],
        ..DependencyPolicy::default()
    };
    let guard = SupplyChainGuard::new(inv, db, policy);

    let patches = guard.propose_patches();
    assert_eq!(patches.len(), 1);
    // Even a patch-level bump of a high-risk package needs human approval.
    assert!(!patches[0].decision.is_auto());
}

#[test]
fn clean_release_ready_inventory() {
    let mut inv = DependencyInventory::new();
    inv.add(
        Dependency::new("serde", Version::new(1, 0, 200), Ecosystem::Cargo).with_license("MIT"),
    );
    inv.add(
        Dependency::new("anyhow", Version::new(1, 0, 80), Ecosystem::Cargo)
            .with_license("Apache-2.0"),
    );
    let guard = SupplyChainGuard::new(inv, AdvisoryDatabase::new(), DependencyPolicy::default());
    let posture = guard.posture();
    assert!(posture.release_ready);
    assert_eq!(posture.vulnerabilities, 0);
    assert_eq!(posture.license_violations, 0);
}
