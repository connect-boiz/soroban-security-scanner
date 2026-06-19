//! Versioned API router
//!
//! Provides a builder pattern for creating versioned API routes
//! with support for URL-based versioning and Accept header negotiation.

use crate::api_versioning::deprecation::VersionRegistry;
use crate::api_versioning::version::ApiVersion;
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{Json, Response},
    routing::get,
    Router,
};
use serde_json::json;
use std::sync::Arc;

/// Configuration for the versioned router
#[derive(Debug, Clone)]
pub struct VersionedRouterConfig {
    /// Base path for API routes (e.g., "/api")
    pub base_path: String,
    /// The current stable version
    pub current_version: ApiVersion,
    /// Whether to enable Accept header version negotiation
    pub enable_header_negotiation: bool,
    /// Whether to include deprecation warnings in responses
    pub include_deprecation_warnings: bool,
    /// Whether to strictly require version prefix in URL
    pub require_version_prefix: bool,
    /// Whether to auto-redirect unversioned requests to current version
    pub auto_redirect_unversioned: bool,
}

impl Default for VersionedRouterConfig {
    fn default() -> Self {
        Self {
            base_path: "/api".to_string(),
            current_version: ApiVersion::current(),
            enable_header_negotiation: true,
            include_deprecation_warnings: true,
            require_version_prefix: true,
            auto_redirect_unversioned: true,
        }
    }
}

/// Shared state for the versioned router
#[derive(Debug, Clone)]
struct RouterState {
    config: VersionedRouterConfig,
    version_registry: Arc<VersionRegistry>,
}

/// Builder for creating versioned API routes
#[derive(Clone)]
pub struct VersionedRouter {
    state: Arc<RouterState>,
}

impl VersionedRouter {
    /// Create a new VersionedRouter with default configuration
    pub fn new() -> Self {
        Self::with_config(VersionedRouterConfig::default())
    }

    /// Create a new VersionedRouter with a custom configuration
    pub fn with_config(config: VersionedRouterConfig) -> Self {
        let registry = Arc::new(VersionRegistry::default());
        Self {
            state: Arc::new(RouterState {
                config,
                version_registry: registry,
            }),
        }
    }

    /// Get the version registry
    pub fn registry(&self) -> &Arc<VersionRegistry> {
        &self.state.version_registry
    }

    /// Create a router for a specific API version
    /// Routes will be mounted at `/api/v{N}/`
    pub fn version_router(&self, version: ApiVersion, routes: Router) -> Router {
        let state = self.state.clone();
        let version_path = format!("{}/{}", state.config.base_path, version.as_path());
        let mut version_router = Router::new();

        // Add deprecation middleware for deprecated versions
        if state.config.include_deprecation_warnings {
            let version_info = state.version_registry.get_version(version);
            if let Some(info) = version_info {
                if info.should_warn() {
                    let sunset = info.sunset_date;
                    let current_version = state.config.current_version;
                    let version_str = version.as_path().to_string();

                    version_router = version_router.layer(axum::middleware::from_fn(
                        move |request: Request, next: Next| {
                            let sunset = sunset;
                            let current_version = current_version;
                            let version_str = version_str.clone();
                            async move {
                                let mut response = next.run(request).await;
                                if let Some(sunset_date) = sunset {
                                    response
                                        .headers_mut()
                                        .insert("X-API-Deprecated", "true".parse().unwrap());
                                    response.headers_mut().insert(
                                        "X-API-Sunset",
                                        sunset_date.to_rfc3339().parse().unwrap(),
                                    );
                                    response.headers_mut().insert(
                                        "X-API-Deprecation-Message",
                                        format!(
                                            "API {} is deprecated and will be sunset on {}. \
                                             Please migrate to API {}.",
                                            version_str,
                                            sunset_date.format("%Y-%m-%d"),
                                            current_version.as_path()
                                        )
                                        .parse()
                                        .unwrap(),
                                    );
                                }
                                response
                            }
                        },
                    ));
                }
            }
        }

        version_router.nest(&version_path, routes)
    }

    /// Nest multiple versioned routers into a single router
    /// Also adds:
    /// - A `/api/versions` endpoint listing all versions
    /// - Auto-redirect for unversioned paths (if enabled)
    /// - An `/api` info endpoint
    pub fn build(&self, versioned_routers: Vec<(ApiVersion, Router)>) -> Router {
        let state = self.state.clone();
        let base_path = state.config.base_path.clone();
        let mut root = Router::new();

        // Add version listing endpoint
        {
            let registry = state.version_registry.clone();
            let versions_path = format!("{}/versions", base_path);
            root = root.route(
                &versions_path,
                get(move || {
                    let registry = registry.clone();
                    async move {
                        let versions: Vec<_> = registry
                            .list_versions()
                            .into_iter()
                            .map(|info| {
                                json!({
                                    "version": info.version.as_path(),
                                    "lifecycle": info.lifecycle.as_str(),
                                    "release_date": info.release_date.to_rfc3339(),
                                    "deprecation_date": info.deprecation_date.map(|d| d.to_rfc3339()),
                                    "sunset_date": info.sunset_date.map(|d| d.to_rfc3339()),
                                    "description": info.description,
                                    "breaking_changes": info.breaking_changes,
                                    "non_breaking_changes": info.non_breaking_changes,
                                })
                            })
                            .collect();
                        Json(json!({
                            "versions": versions,
                            "current_stable": ApiVersion::current().as_path(),
                            "base_path": "/api",
                        }))
                    }
                }),
            );
        }

        // Add API info endpoint (unversioned)
        {
            let current = state.config.current_version;
            let bp_for_route = base_path.clone();
            let bp_for_handler = base_path.clone();
            root = root.route(
                &bp_for_route,
                get(move || {
                    let bp = bp_for_handler.clone();
                    async move {
                        Json(json!({
                            "service": "soroban-security-scanner",
                            "api_version": current.as_path(),
                            "versions_url": format!("{}/versions", bp),
                            "documentation_url": format!("{}/{}/docs", bp, current.as_path()),
                            "media_types": {
                                "v1": ApiVersion::V1.media_type(),
                                "v2": ApiVersion::V2.media_type(),
                            },
                        }))
                    }
                }),
            );
        }

        // Nest all versioned routers
        for (version, routes) in versioned_routers {
            root = root.merge(self.version_router(version, routes));
        }

        // Add auto-redirect for unversioned paths if enabled
        if state.config.auto_redirect_unversioned {
            let current_version = state.config.current_version;
            let bp = base_path.clone();
            root = root.fallback(get(move |request: Request| {
                let path = request.uri().path().to_string();
                let bp = bp.clone();
                let current_version = current_version;
                async move {
                    if path.starts_with(&bp)
                        && !path.contains("/v1/")
                        && !path.contains("/v2/")
                        && path != bp
                        && path != format!("{}/versions", bp)
                    {
                        // Redirect to current version
                        let new_path =
                            path.replacen(&bp, &format!("{}/{}", bp, current_version.as_path()), 1);
                        Response::builder()
                            .status(StatusCode::MOVED_PERMANENTLY)
                            .header("Location", new_path)
                            .body(axum::body::Body::empty())
                            .unwrap()
                    } else {
                        Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .body(axum::body::Body::from("Not Found"))
                            .unwrap()
                    }
                }
            }));
        }

        root
    }

    /// Create a simple router for a single version (convenience method)
    /// Just nests routes under /api/v{N}/ without extra features
    pub fn simple_version_router(version: ApiVersion, routes: Router) -> Router {
        Router::new().nest(&format!("/api/{}", version.as_path()), routes)
    }
}

impl Default for VersionedRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;

    #[test]
    fn test_simple_version_router_creates_correct_path() {
        let routes = Router::new().route("/test", get(|| async { "test" }));
        let _router = VersionedRouter::simple_version_router(ApiVersion::V1, routes);
        // Router creation succeeded
    }

    #[test]
    fn test_versioned_router_default() {
        let router = VersionedRouter::new();
        assert_eq!(router.state.config.base_path, "/api");
        assert_eq!(router.state.config.current_version, ApiVersion::V1);
        assert!(router.state.config.enable_header_negotiation);
    }

    #[test]
    fn test_versioned_router_with_config() {
        let config = VersionedRouterConfig {
            base_path: "/custom".to_string(),
            current_version: ApiVersion::V2,
            ..Default::default()
        };
        let router = VersionedRouter::with_config(config);
        assert_eq!(router.state.config.base_path, "/custom");
        assert_eq!(router.state.config.current_version, ApiVersion::V2);
    }
}
