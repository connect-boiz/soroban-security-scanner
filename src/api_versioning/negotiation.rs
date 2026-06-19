//! API version negotiation via Accept headers
//!
//! Supports content negotiation using custom media types:
//! - `application/vnd.soroban.v1+json`
//! - `application/vnd.soroban.v2+json`
//!
//! Falls back to URL-based versioning when no Accept header is present.

use crate::api_versioning::deprecation::VersionRegistry;
use crate::api_versioning::version::{ApiVersion, VersionLifecycle};
use axum::http::HeaderMap;
use std::sync::Arc;

/// Handles API version negotiation from HTTP headers
#[derive(Debug, Clone)]
pub struct VersionNegotiator {
    registry: Arc<VersionRegistry>,
}

impl VersionNegotiator {
    /// Create a new VersionNegotiator
    pub fn new(registry: Arc<VersionRegistry>) -> Self {
        Self { registry }
    }

    /// Try to determine the requested API version from Accept headers
    ///
    /// Supports:
    /// - `Accept: application/vnd.soroban.v1+json` → V1
    /// - `Accept: application/vnd.soroban.v2+json` → V2
    /// - Multiple Accept values (takes the first matching one)
    /// - Falls back to None if no version header found
    pub fn negotiate_version(&self, headers: &HeaderMap) -> Option<ApiVersion> {
        let accept_header = headers
            .get("accept")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        // Look for our custom media types
        for version in ApiVersion::all() {
            let media_type = version.media_type();
            if accept_header.contains(&media_type) {
                return Some(version);
            }
        }

        // Also check X-API-Version header as a simpler alternative
        if let Some(version_str) = headers.get("x-api-version").and_then(|h| h.to_str().ok()) {
            if let Ok(version) = version_str.parse::<ApiVersion>() {
                return Some(version);
            }
        }

        None
    }

    /// Extract version from URL path (e.g., "/api/v1/users" → V1)
    pub fn version_from_path(path: &str) -> Option<ApiVersion> {
        for version in ApiVersion::all() {
            let prefix = format!("/api/{}/", version.as_path());
            let prefix_no_trailing = format!("/api/{}", version.as_path());

            if path.starts_with(&prefix) || path == prefix_no_trailing {
                return Some(version);
            }
        }
        None
    }

    /// Determine the effective API version for a request
    /// Priority: URL path > Accept header > default current version
    pub fn determine_version(&self, path: &str, headers: &HeaderMap) -> ApiVersion {
        // 1. Try URL path first (explicit versioning)
        if let Some(version) = Self::version_from_path(path) {
            return version;
        }

        // 2. Try Accept header negotiation
        if let Some(version) = self.negotiate_version(headers) {
            return version;
        }

        // 3. Default to current stable version
        ApiVersion::current()
    }

    /// Check if the requested version is deprecated and get deprecation info
    pub fn get_deprecation_headers(&self, version: ApiVersion) -> Vec<(&'static str, String)> {
        let mut headers = Vec::new();

        if let Some(info) = self.registry.get_version(version) {
            if info.lifecycle == VersionLifecycle::Deprecated {
                headers.push(("X-API-Deprecated", "true".to_string()));

                if let Some(sunset) = info.sunset_date {
                    headers.push(("X-API-Sunset", sunset.to_rfc3339()));
                }

                headers.push((
                    "X-API-Deprecation-Message",
                    format!(
                        "API version {} is deprecated. Please migrate to {}.",
                        version.as_path(),
                        ApiVersion::current().as_path()
                    ),
                ));
            } else if info.lifecycle == VersionLifecycle::Sunset {
                headers.push(("X-API-Sunset", "true".to_string()));
            }
        }

        headers
    }

    /// Validate that a requested version is being served
    pub fn validate_version(&self, version: ApiVersion) -> Result<(), VersionError> {
        match self.registry.get_version(version) {
            Some(info) => {
                match info.lifecycle {
                    VersionLifecycle::Sunset => Err(VersionError::Sunset {
                        version,
                        migration: format!(
                            "Please migrate to API {}: /api/{}",
                            ApiVersion::current().as_path(),
                            ApiVersion::current().as_path()
                        ),
                    }),
                    VersionLifecycle::Deprecated => {
                        // Deprecated is still served, but with warnings
                        Ok(())
                    }
                    _ => Ok(()),
                }
            }
            None => Err(VersionError::NotFound { version }),
        }
    }
}

/// Errors returned during version negotiation
#[derive(Debug, Clone)]
pub enum VersionError {
    /// Requested version does not exist
    NotFound { version: ApiVersion },
    /// Requested version has been sunset
    Sunset {
        version: ApiVersion,
        migration: String,
    },
    /// Multiple conflicting versions specified
    Ambiguous {
        path_version: ApiVersion,
        header_version: ApiVersion,
    },
}

impl std::fmt::Display for VersionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionError::NotFound { version } => {
                write!(f, "API version {} not found", version.as_path())
            }
            VersionError::Sunset { version, migration } => {
                write!(
                    f,
                    "API version {} has been sunset. {}",
                    version.as_path(),
                    migration
                )
            }
            VersionError::Ambiguous {
                path_version,
                header_version,
            } => {
                write!(
                    f,
                    "Ambiguous version request: path specifies {} but Accept header specifies {}",
                    path_version.as_path(),
                    header_version.as_path(),
                )
            }
        }
    }
}

impl std::error::Error for VersionError {}

/// Axum middleware for version negotiation
pub async fn version_negotiation_middleware(
    headers: HeaderMap,
    path: &str,
) -> Result<ApiVersion, VersionError> {
    let negotiator = VersionNegotiator::new(Arc::new(VersionRegistry::default()));

    let path_version = VersionNegotiator::version_from_path(path);
    let header_version = negotiator.negotiate_version(&headers);

    // Check for ambiguity
    match (path_version, header_version) {
        (Some(pv), Some(hv)) if pv != hv => {
            return Err(VersionError::Ambiguous {
                path_version: pv,
                header_version: hv,
            });
        }
        (Some(pv), _) => {
            negotiator.validate_version(pv)?;
            return Ok(pv);
        }
        (None, Some(hv)) => {
            negotiator.validate_version(hv)?;
            return Ok(hv);
        }
        (None, None) => {
            // Default to current version
            let current = ApiVersion::current();
            negotiator.validate_version(current)?;
            return Ok(current);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_from_path() {
        assert_eq!(
            VersionNegotiator::version_from_path("/api/v1/users"),
            Some(ApiVersion::V1)
        );
        assert_eq!(
            VersionNegotiator::version_from_path("/api/v2/transactions"),
            Some(ApiVersion::V2)
        );
        assert_eq!(
            VersionNegotiator::version_from_path("/api/v1"),
            Some(ApiVersion::V1)
        );
        assert_eq!(VersionNegotiator::version_from_path("/auth/login"), None);
        assert_eq!(VersionNegotiator::version_from_path("/health"), None);
    }

    #[test]
    fn test_negotiate_version_from_accept_header() {
        let registry = Arc::new(VersionRegistry::default());
        let negotiator = VersionNegotiator::new(registry);

        let mut headers = HeaderMap::new();
        headers.insert("accept", "application/vnd.soroban.v1+json".parse().unwrap());
        assert_eq!(negotiator.negotiate_version(&headers), Some(ApiVersion::V1));

        let mut headers = HeaderMap::new();
        headers.insert("accept", "application/vnd.soroban.v2+json".parse().unwrap());
        assert_eq!(negotiator.negotiate_version(&headers), Some(ApiVersion::V2));
    }

    #[test]
    fn test_negotiate_version_from_x_api_version_header() {
        let registry = Arc::new(VersionRegistry::default());
        let negotiator = VersionNegotiator::new(registry);

        let mut headers = HeaderMap::new();
        headers.insert("x-api-version", "v1".parse().unwrap());
        assert_eq!(negotiator.negotiate_version(&headers), Some(ApiVersion::V1));

        let mut headers = HeaderMap::new();
        headers.insert("x-api-version", "2".parse().unwrap());
        assert_eq!(negotiator.negotiate_version(&headers), Some(ApiVersion::V2));
    }

    #[test]
    fn test_negotiate_version_no_version_header() {
        let registry = Arc::new(VersionRegistry::default());
        let negotiator = VersionNegotiator::new(registry);
        let headers = HeaderMap::new();
        assert_eq!(negotiator.negotiate_version(&headers), None);
    }

    #[test]
    fn test_determine_version_priority() {
        let registry = Arc::new(VersionRegistry::default());
        let negotiator = VersionNegotiator::new(registry);

        // URL path takes priority
        let mut headers = HeaderMap::new();
        headers.insert("accept", "application/vnd.soroban.v2+json".parse().unwrap());
        assert_eq!(
            negotiator.determine_version("/api/v1/users", &headers),
            ApiVersion::V1
        );

        // Header used when no URL version
        let empty_headers = HeaderMap::new();
        assert_eq!(
            negotiator.determine_version("/some/other/path", &headers),
            ApiVersion::V2
        );

        // Default when nothing specified
        assert_eq!(
            negotiator.determine_version("/some/other/path", &empty_headers),
            ApiVersion::V1
        );
    }
}
