//! Dependency integrity verification.
//!
//! Verifies that downloaded dependency artifacts match their pinned SHA-256
//! checksums — defending against tampered packages and registry/supply-chain
//! compromise. A dependency with no pinned checksum is reported as unverifiable
//! rather than silently trusted.

use crate::supply_chain::inventory::Dependency;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Result of verifying one dependency artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntegrityStatus {
    /// Checksum matched the pinned value.
    Verified,
    /// Checksum did not match — the artifact has been tampered with.
    Mismatch,
    /// No pinned checksum to verify against.
    Unpinned,
}

/// Computes the SHA-256 hex digest of artifact bytes.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Verifies an artifact's bytes against a dependency's pinned checksum.
pub fn verify(dep: &Dependency, artifact: &[u8]) -> IntegrityStatus {
    match &dep.checksum {
        None => IntegrityStatus::Unpinned,
        Some(expected) => {
            let actual = sha256_hex(artifact);
            if constant_time_eq(actual.as_bytes(), expected.as_bytes()) {
                IntegrityStatus::Verified
            } else {
                IntegrityStatus::Mismatch
            }
        }
    }
}

/// A summary of verifying a whole set of dependencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct IntegritySummary {
    /// Verified artifacts.
    pub verified: usize,
    /// Tampered artifacts.
    pub mismatched: usize,
    /// Artifacts without a pinned checksum.
    pub unpinned: usize,
}

impl IntegritySummary {
    /// Records a status into the summary.
    pub fn record(&mut self, status: IntegrityStatus) {
        match status {
            IntegrityStatus::Verified => self.verified += 1,
            IntegrityStatus::Mismatch => self.mismatched += 1,
            IntegrityStatus::Unpinned => self.unpinned += 1,
        }
    }

    /// Whether every artifact verified cleanly (none mismatched or unpinned).
    pub fn all_verified(&self) -> bool {
        self.mismatched == 0 && self.unpinned == 0
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supply_chain::inventory::Ecosystem;
    use crate::supply_chain::version::Version;

    fn dep_with_checksum(artifact: &[u8]) -> Dependency {
        Dependency::new("pkg", Version::new(1, 0, 0), Ecosystem::Cargo)
            .with_checksum(sha256_hex(artifact))
    }

    #[test]
    fn matching_artifact_verifies() {
        let artifact = b"the real package bytes";
        let dep = dep_with_checksum(artifact);
        assert_eq!(verify(&dep, artifact), IntegrityStatus::Verified);
    }

    #[test]
    fn tampered_artifact_detected() {
        let dep = dep_with_checksum(b"original bytes");
        assert_eq!(verify(&dep, b"malicious bytes"), IntegrityStatus::Mismatch);
    }

    #[test]
    fn unpinned_dependency_reported() {
        let dep = Dependency::new("pkg", Version::new(1, 0, 0), Ecosystem::Cargo);
        assert_eq!(verify(&dep, b"anything"), IntegrityStatus::Unpinned);
    }

    #[test]
    fn summary_tracks_outcomes() {
        let mut s = IntegritySummary::default();
        s.record(IntegrityStatus::Verified);
        s.record(IntegrityStatus::Mismatch);
        s.record(IntegrityStatus::Unpinned);
        assert_eq!(s.verified, 1);
        assert_eq!(s.mismatched, 1);
        assert_eq!(s.unpinned, 1);
        assert!(!s.all_verified());
    }

    #[test]
    fn known_sha256_vector() {
        // SHA-256("abc")
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
