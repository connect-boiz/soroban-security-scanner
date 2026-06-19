//! API version definitions and lifecycle management
//!
//! Implements semantic API versioning with lifecycle states and
//! backward compatibility tracking.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Represents an API version (e.g., v1, v2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ApiVersion {
    V1,
    V2,
    V3,
    V4,
    V5,
}

impl ApiVersion {
    /// Returns the string representation used in URL paths
    pub fn as_path(&self) -> &'static str {
        match self {
            ApiVersion::V1 => "v1",
            ApiVersion::V2 => "v2",
            ApiVersion::V3 => "v3",
            ApiVersion::V4 => "v4",
            ApiVersion::V5 => "v5",
        }
    }

    /// Returns the full URL prefix for this version
    pub fn url_prefix(&self) -> String {
        format!("/api/{}", self.as_path())
    }

    /// Returns the media type for Accept header negotiation
    pub fn media_type(&self) -> String {
        format!("application/vnd.soroban.{}+json", self.as_path())
    }

    /// Returns the numeric version
    pub fn number(&self) -> u8 {
        match self {
            ApiVersion::V1 => 1,
            ApiVersion::V2 => 2,
            ApiVersion::V3 => 3,
            ApiVersion::V4 => 4,
            ApiVersion::V5 => 5,
        }
    }

    /// Returns the current stable version
    pub fn current() -> Self {
        ApiVersion::V1
    }

    /// Returns all available versions
    pub fn all() -> Vec<ApiVersion> {
        vec![
            ApiVersion::V1,
            ApiVersion::V2,
            ApiVersion::V3,
            ApiVersion::V4,
            ApiVersion::V5,
        ]
    }

    /// Returns the next version
    pub fn next(&self) -> Option<ApiVersion> {
        match self {
            ApiVersion::V1 => Some(ApiVersion::V2),
            ApiVersion::V2 => Some(ApiVersion::V3),
            ApiVersion::V3 => Some(ApiVersion::V4),
            ApiVersion::V4 => Some(ApiVersion::V5),
            ApiVersion::V5 => None,
        }
    }
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_path())
    }
}

impl FromStr for ApiVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "v1" | "1" => Ok(ApiVersion::V1),
            "v2" | "2" => Ok(ApiVersion::V2),
            "v3" | "3" => Ok(ApiVersion::V3),
            "v4" | "4" => Ok(ApiVersion::V4),
            "v5" | "5" => Ok(ApiVersion::V5),
            _ => Err(format!("Unknown API version: {}", s)),
        }
    }
}

/// Lifecycle phase of an API version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionLifecycle {
    /// Under active development, may have breaking changes
    Alpha,
    /// Feature complete, stabilizing for release
    Beta,
    /// Production-ready, no breaking changes allowed
    Stable,
    /// Still available but clients should migrate away
    Deprecated,
    /// No longer served, returns 410 Gone
    Sunset,
}

impl VersionLifecycle {
    pub fn as_str(&self) -> &'static str {
        match self {
            VersionLifecycle::Alpha => "alpha",
            VersionLifecycle::Beta => "beta",
            VersionLifecycle::Stable => "stable",
            VersionLifecycle::Deprecated => "deprecated",
            VersionLifecycle::Sunset => "sunset",
        }
    }

    /// Whether this lifecycle phase allows breaking changes
    pub fn allows_breaking_changes(&self) -> bool {
        matches!(self, VersionLifecycle::Alpha | VersionLifecycle::Beta)
    }

    /// Whether endpoints in this lifecycle are served
    pub fn is_served(&self) -> bool {
        !matches!(self, VersionLifecycle::Sunset)
    }
}

impl fmt::Display for VersionLifecycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Full version information including lifecycle metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: ApiVersion,
    pub lifecycle: VersionLifecycle,
    pub release_date: DateTime<Utc>,
    pub deprecation_date: Option<DateTime<Utc>>,
    pub sunset_date: Option<DateTime<Utc>>,
    pub description: String,
    pub breaking_changes: Vec<String>,
    pub non_breaking_changes: Vec<String>,
}

impl VersionInfo {
    /// Creates a new VersionInfo in stable state
    pub fn new_stable(version: ApiVersion, description: &str) -> Self {
        Self {
            version,
            lifecycle: VersionLifecycle::Stable,
            release_date: Utc::now(),
            deprecation_date: None,
            sunset_date: None,
            description: description.to_string(),
            breaking_changes: Vec::new(),
            non_breaking_changes: Vec::new(),
        }
    }

    /// Creates a new VersionInfo in alpha state
    pub fn new_alpha(version: ApiVersion, description: &str) -> Self {
        Self {
            version,
            lifecycle: VersionLifecycle::Alpha,
            release_date: Utc::now(),
            deprecation_date: None,
            sunset_date: None,
            description: description.to_string(),
            breaking_changes: Vec::new(),
            non_breaking_changes: Vec::new(),
        }
    }
    /// Transition this version to deprecated status
    /// Sets sunset date to 6 months from now (minimum notice period).
    /// Uses a single `now` snapshot so the policy validation in
    /// `deprecate_version` cannot observe clock drift between two
    /// `Utc::now()` calls.
    pub fn deprecate(&mut self) {
        let now = Utc::now();
        self.lifecycle = VersionLifecycle::Deprecated;
        self.deprecation_date = Some(now);
        self.sunset_date = Some(now + Duration::days(180)); // 6 months
    }

    /// Transition this version to deprecated with a custom sunset date.
    /// Uses a single `now` snapshot for the same reason as `deprecate()`.
    pub fn deprecate_with_sunset(&mut self, sunset: DateTime<Utc>) {
        let now = Utc::now();
        let min_sunset = now + Duration::days(180);
        self.lifecycle = VersionLifecycle::Deprecated;
        self.deprecation_date = Some(now);
        // Ensure at least 6-month notice
        self.sunset_date = Some(if sunset > min_sunset {
            sunset
        } else {
            min_sunset
        });
    }

    /// Transition this version to sunset (no longer served)
    pub fn sunset(&mut self) {
        self.lifecycle = VersionLifecycle::Sunset;
    }

    /// Returns days until sunset, if deprecated
    pub fn days_until_sunset(&self) -> Option<i64> {
        self.sunset_date
            .map(|sunset| (sunset - Utc::now()).num_days())
    }

    /// Whether clients should receive deprecation warnings
    pub fn should_warn(&self) -> bool {
        self.lifecycle == VersionLifecycle::Deprecated
    }

    /// Add a breaking change entry
    pub fn add_breaking_change(&mut self, change: &str) {
        self.breaking_changes.push(change.to_string());
    }

    /// Add a non-breaking change entry
    pub fn add_non_breaking_change(&mut self, change: &str) {
        self.non_breaking_changes.push(change.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_version_paths() {
        assert_eq!(ApiVersion::V1.as_path(), "v1");
        assert_eq!(ApiVersion::V1.url_prefix(), "/api/v1");
        assert_eq!(ApiVersion::V2.as_path(), "v2");
        assert_eq!(ApiVersion::V2.url_prefix(), "/api/v2");
    }

    #[test]
    fn test_api_version_media_type() {
        assert_eq!(
            ApiVersion::V1.media_type(),
            "application/vnd.soroban.v1+json"
        );
    }

    #[test]
    fn test_api_version_from_str() {
        assert_eq!("v1".parse::<ApiVersion>().unwrap(), ApiVersion::V1);
        assert_eq!("V2".parse::<ApiVersion>().unwrap(), ApiVersion::V2);
        assert_eq!("3".parse::<ApiVersion>().unwrap(), ApiVersion::V3);
        assert!("v99".parse::<ApiVersion>().is_err());
    }

    #[test]
    fn test_api_version_next() {
        assert_eq!(ApiVersion::V1.next(), Some(ApiVersion::V2));
        assert_eq!(ApiVersion::V2.next(), Some(ApiVersion::V3));
        assert_eq!(ApiVersion::V5.next(), None);
    }

    #[test]
    fn test_lifecycle_allows_breaking_changes() {
        assert!(VersionLifecycle::Alpha.allows_breaking_changes());
        assert!(VersionLifecycle::Beta.allows_breaking_changes());
        assert!(!VersionLifecycle::Stable.allows_breaking_changes());
        assert!(!VersionLifecycle::Deprecated.allows_breaking_changes());
    }

    #[test]
    fn test_lifecycle_is_served() {
        assert!(VersionLifecycle::Alpha.is_served());
        assert!(VersionLifecycle::Beta.is_served());
        assert!(VersionLifecycle::Stable.is_served());
        assert!(VersionLifecycle::Deprecated.is_served());
        assert!(!VersionLifecycle::Sunset.is_served());
    }

    #[test]
    fn test_version_info_deprecation_min_six_months() {
        let mut info = VersionInfo::new_stable(ApiVersion::V1, "Initial version");
        info.deprecate();
        assert_eq!(info.lifecycle, VersionLifecycle::Deprecated);
        assert!(info.days_until_sunset().unwrap() >= 179); // ~6 months
        assert!(info.should_warn());
    }

    #[test]
    fn test_version_info_deprecate_with_custom_sunset() {
        let mut info = VersionInfo::new_stable(ApiVersion::V1, "Initial version");
        let short_sunset = Utc::now() + Duration::days(30);
        info.deprecate_with_sunset(short_sunset);
        // Must enforce minimum 6-month notice
        assert!(info.days_until_sunset().unwrap() >= 179);
    }
}
