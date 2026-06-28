//! Software Bill of Materials (SBOM) generation.
//!
//! Produces a CycloneDX-style component list from the dependency inventory,
//! serializable to JSON for distribution and ingestion by SBOM tooling.

use crate::supply_chain::inventory::DependencyInventory;
use serde::{Deserialize, Serialize};

/// One SBOM component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SbomComponent {
    /// Component name.
    pub name: String,
    /// Resolved version.
    pub version: String,
    /// Ecosystem / package type.
    pub ecosystem: String,
    /// SPDX license id, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Content checksum (sha256 hex), if pinned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    /// Whether it is a direct dependency.
    pub direct: bool,
    /// Package URL (purl), e.g. `pkg:cargo/serde@1.0.200`.
    pub purl: String,
}

/// A Software Bill of Materials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sbom {
    /// SBOM format/spec identifier.
    pub format: String,
    /// The components.
    pub components: Vec<SbomComponent>,
}

impl Sbom {
    /// Generates an SBOM from the inventory.
    pub fn from_inventory(inventory: &DependencyInventory) -> Self {
        let components = inventory
            .dependencies
            .iter()
            .map(|d| {
                let version = d.version.to_string_semver();
                SbomComponent {
                    purl: format!("pkg:{}/{}@{}", d.ecosystem.as_str(), d.name, version),
                    name: d.name.clone(),
                    version,
                    ecosystem: d.ecosystem.as_str().to_string(),
                    license: d.license.clone(),
                    checksum: d.checksum.clone(),
                    direct: d.direct,
                }
            })
            .collect();
        Self {
            format: "CycloneDX-1.5".to_string(),
            components,
        }
    }

    /// Number of components.
    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// Whether the SBOM has no components.
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// Serializes to JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supply_chain::inventory::{Dependency, Ecosystem};
    use crate::supply_chain::version::Version;

    fn inventory() -> DependencyInventory {
        let mut inv = DependencyInventory::new();
        inv.add(
            Dependency::new("serde", Version::new(1, 0, 200), Ecosystem::Cargo)
                .with_license("MIT")
                .with_checksum("abc123"),
        );
        inv.add(Dependency::new("rand", Version::new(0, 8, 5), Ecosystem::Cargo).transitive());
        inv
    }

    #[test]
    fn sbom_covers_every_dependency() {
        let sbom = Sbom::from_inventory(&inventory());
        assert_eq!(sbom.len(), 2);
        assert_eq!(sbom.format, "CycloneDX-1.5");
    }

    #[test]
    fn purl_is_well_formed() {
        let sbom = Sbom::from_inventory(&inventory());
        let serde = sbom.components.iter().find(|c| c.name == "serde").unwrap();
        assert_eq!(serde.purl, "pkg:cargo/serde@1.0.200");
        assert_eq!(serde.license.as_deref(), Some("MIT"));
        assert!(serde.direct);
    }

    #[test]
    fn serializes_to_json() {
        let json = Sbom::from_inventory(&inventory()).to_json();
        assert!(json.contains("CycloneDX-1.5"));
        assert!(json.contains("pkg:cargo/serde@1.0.200"));
        // Round-trips.
        let parsed: Sbom = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 2);
    }
}
