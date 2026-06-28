//! Minimal semantic versioning: parsing, ordering and OSV-style ranges.
//!
//! Just enough semver to match advisory affected-ranges and classify updates,
//! without pulling in an external crate. Pre-release/build metadata is parsed
//! and ignored for range matching (release versions only).

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// A semantic version (major.minor.patch).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version {
    /// Major component.
    pub major: u64,
    /// Minor component.
    pub minor: u64,
    /// Patch component.
    pub patch: u64,
}

impl Version {
    /// Constructs a version.
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parses `"1.2.3"` (with optional leading `v` and `-pre`/`+build` suffix
    /// which are ignored). Missing minor/patch default to 0.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().trim_start_matches('v');
        // Drop pre-release / build metadata.
        let core = s.split(['-', '+']).next().unwrap_or(s);
        let mut parts = core.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next().unwrap_or("0").parse().ok()?;
        let patch = parts.next().unwrap_or("0").parse().ok()?;
        if parts.next().is_some() {
            return None; // too many components
        }
        Some(Self {
            major,
            minor,
            patch,
        })
    }

    /// Renders as `major.minor.patch`.
    pub fn to_string_semver(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}
impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// An OSV-style affected range: `[introduced, fixed)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionRange {
    /// First affected version (inclusive); `None` means "from 0".
    pub introduced: Option<Version>,
    /// First fixed version (exclusive); `None` means "no fix / all later".
    pub fixed: Option<Version>,
}

impl VersionRange {
    /// A range affecting everything below `fixed`.
    pub fn below(fixed: Version) -> Self {
        Self {
            introduced: None,
            fixed: Some(fixed),
        }
    }

    /// A bounded `[introduced, fixed)` range.
    pub fn between(introduced: Version, fixed: Version) -> Self {
        Self {
            introduced: Some(introduced),
            fixed: Some(fixed),
        }
    }

    /// Whether `v` falls within the affected range.
    pub fn contains(&self, v: &Version) -> bool {
        let after_intro = self.introduced.map(|i| *v >= i).unwrap_or(true);
        let before_fixed = self.fixed.map(|f| *v < f).unwrap_or(true);
        after_intro && before_fixed
    }
}

/// The kind of an update between two versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateKind {
    /// No change or a downgrade.
    None,
    /// Patch-level update (z).
    Patch,
    /// Minor update (y).
    Minor,
    /// Major update (x) — potentially breaking.
    Major,
}

/// Classifies the update from `from` to `to`.
pub fn update_kind(from: &Version, to: &Version) -> UpdateKind {
    if to <= from {
        UpdateKind::None
    } else if to.major != from.major {
        UpdateKind::Major
    } else if to.minor != from.minor {
        UpdateKind::Minor
    } else {
        UpdateKind::Patch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing() {
        assert_eq!(Version::parse("1.2.3"), Some(Version::new(1, 2, 3)));
        assert_eq!(Version::parse("v2.0"), Some(Version::new(2, 0, 0)));
        assert_eq!(Version::parse("1.2.3-rc1"), Some(Version::new(1, 2, 3)));
        assert_eq!(Version::parse("1.2.3+build"), Some(Version::new(1, 2, 3)));
        assert_eq!(Version::parse("1.2.3.4"), None);
        assert_eq!(Version::parse("abc"), None);
    }

    #[test]
    fn ordering() {
        assert!(Version::new(1, 2, 0) < Version::new(1, 10, 0));
        assert!(Version::new(2, 0, 0) > Version::new(1, 99, 99));
        assert_eq!(Version::new(1, 0, 0), Version::new(1, 0, 0));
    }

    #[test]
    fn range_contains() {
        let r = VersionRange::below(Version::new(1, 5, 0));
        assert!(r.contains(&Version::new(1, 4, 9)));
        assert!(!r.contains(&Version::new(1, 5, 0))); // fixed is exclusive
        let b = VersionRange::between(Version::new(1, 2, 0), Version::new(1, 4, 0));
        assert!(!b.contains(&Version::new(1, 1, 0)));
        assert!(b.contains(&Version::new(1, 3, 0)));
        assert!(!b.contains(&Version::new(1, 4, 0)));
    }

    #[test]
    fn update_classification() {
        assert_eq!(
            update_kind(&Version::new(1, 2, 3), &Version::new(1, 2, 4)),
            UpdateKind::Patch
        );
        assert_eq!(
            update_kind(&Version::new(1, 2, 3), &Version::new(1, 3, 0)),
            UpdateKind::Minor
        );
        assert_eq!(
            update_kind(&Version::new(1, 2, 3), &Version::new(2, 0, 0)),
            UpdateKind::Major
        );
        assert_eq!(
            update_kind(&Version::new(1, 2, 3), &Version::new(1, 2, 3)),
            UpdateKind::None
        );
        assert_eq!(
            update_kind(&Version::new(2, 0, 0), &Version::new(1, 0, 0)),
            UpdateKind::None
        );
    }
}
