//! Canonical registry of all API endpoints for 100% security test coverage.

use serde::{Deserialize, Serialize};

/// HTTP methods supported by the API surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
        }
    }
}

/// Authentication requirement for an endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndpointAuth {
    Public,
    Authenticated,
    Admin,
    RateLimited,
}

/// A single API endpoint definition used by the security suite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    pub path: &'static str,
    pub method: HttpMethod,
    pub auth: EndpointAuth,
    pub description: &'static str,
    pub critical_workflow: bool,
}

/// Registry of every API endpoint in the platform.
#[derive(Debug, Clone)]
pub struct EndpointRegistry {
    endpoints: Vec<ApiEndpoint>,
}

impl EndpointRegistry {
    /// Build the full endpoint catalog (versioned API, auth, transactions, monitoring).
    pub fn full_catalog() -> Self {
        let endpoints = vec![
            // Versioned API meta
            ApiEndpoint {
                path: "/api",
                method: HttpMethod::Get,
                auth: EndpointAuth::Public,
                description: "API service info",
                critical_workflow: false,
            },
            ApiEndpoint {
                path: "/api/versions",
                method: HttpMethod::Get,
                auth: EndpointAuth::Public,
                description: "List API versions",
                critical_workflow: false,
            },
            ApiEndpoint {
                path: "/api/v1/changelog.md",
                method: HttpMethod::Get,
                auth: EndpointAuth::Public,
                description: "V1 changelog (markdown)",
                critical_workflow: false,
            },
            ApiEndpoint {
                path: "/api/v1/changelog.json",
                method: HttpMethod::Get,
                auth: EndpointAuth::Public,
                description: "V1 changelog (JSON)",
                critical_workflow: false,
            },
            // Auth server — public
            ApiEndpoint {
                path: "/health",
                method: HttpMethod::Get,
                auth: EndpointAuth::Public,
                description: "Health check",
                critical_workflow: false,
            },
            ApiEndpoint {
                path: "/auth/login",
                method: HttpMethod::Post,
                auth: EndpointAuth::Public,
                description: "User login",
                critical_workflow: true,
            },
            ApiEndpoint {
                path: "/auth/register",
                method: HttpMethod::Post,
                auth: EndpointAuth::Public,
                description: "User registration",
                critical_workflow: true,
            },
            ApiEndpoint {
                path: "/auth/forgot-password",
                method: HttpMethod::Post,
                auth: EndpointAuth::Public,
                description: "Password reset request",
                critical_workflow: true,
            },
            ApiEndpoint {
                path: "/auth/reset-password",
                method: HttpMethod::Post,
                auth: EndpointAuth::Public,
                description: "Password reset completion",
                critical_workflow: true,
            },
            ApiEndpoint {
                path: "/auth/status/:email",
                method: HttpMethod::Get,
                auth: EndpointAuth::Public,
                description: "Account lockout status",
                critical_workflow: false,
            },
            // Auth server — protected
            ApiEndpoint {
                path: "/api/profile",
                method: HttpMethod::Get,
                auth: EndpointAuth::Authenticated,
                description: "User profile",
                critical_workflow: true,
            },
            ApiEndpoint {
                path: "/api/admin/users",
                method: HttpMethod::Get,
                auth: EndpointAuth::Admin,
                description: "Admin user listing",
                critical_workflow: true,
            },
            ApiEndpoint {
                path: "/api/admin/sessions",
                method: HttpMethod::Get,
                auth: EndpointAuth::Admin,
                description: "Admin session listing",
                critical_workflow: true,
            },
            ApiEndpoint {
                path: "/api/admin/stats",
                method: HttpMethod::Get,
                auth: EndpointAuth::Admin,
                description: "Admin statistics",
                critical_workflow: true,
            },
            ApiEndpoint {
                path: "/api/limited",
                method: HttpMethod::Get,
                auth: EndpointAuth::RateLimited,
                description: "Rate-limited endpoint",
                critical_workflow: false,
            },
            // Transaction engine
            ApiEndpoint {
                path: "/transactions",
                method: HttpMethod::Post,
                auth: EndpointAuth::Authenticated,
                description: "Create transaction",
                critical_workflow: true,
            },
            ApiEndpoint {
                path: "/transactions",
                method: HttpMethod::Get,
                auth: EndpointAuth::Authenticated,
                description: "List transactions",
                critical_workflow: true,
            },
            ApiEndpoint {
                path: "/transactions/:id",
                method: HttpMethod::Get,
                auth: EndpointAuth::Authenticated,
                description: "Get transaction by ID",
                critical_workflow: true,
            },
            ApiEndpoint {
                path: "/transactions/:id/retry",
                method: HttpMethod::Post,
                auth: EndpointAuth::Authenticated,
                description: "Retry failed transaction",
                critical_workflow: true,
            },
            ApiEndpoint {
                path: "/transactions/:id/cancel",
                method: HttpMethod::Post,
                auth: EndpointAuth::Authenticated,
                description: "Cancel pending transaction",
                critical_workflow: true,
            },
            // Queue
            ApiEndpoint {
                path: "/queue/stats",
                method: HttpMethod::Get,
                auth: EndpointAuth::Authenticated,
                description: "Queue statistics",
                critical_workflow: false,
            },
            ApiEndpoint {
                path: "/queue/retryable",
                method: HttpMethod::Get,
                auth: EndpointAuth::Authenticated,
                description: "List retryable transactions",
                critical_workflow: false,
            },
            ApiEndpoint {
                path: "/queue/cleanup",
                method: HttpMethod::Post,
                auth: EndpointAuth::Admin,
                description: "Queue cleanup",
                critical_workflow: false,
            },
            // Monitoring
            ApiEndpoint {
                path: "/monitoring/snapshot",
                method: HttpMethod::Get,
                auth: EndpointAuth::Authenticated,
                description: "Monitoring snapshot",
                critical_workflow: false,
            },
            ApiEndpoint {
                path: "/monitoring/history",
                method: HttpMethod::Get,
                auth: EndpointAuth::Authenticated,
                description: "Monitoring history",
                critical_workflow: false,
            },
            ApiEndpoint {
                path: "/monitoring/alerts",
                method: HttpMethod::Get,
                auth: EndpointAuth::Authenticated,
                description: "Active alerts",
                critical_workflow: false,
            },
            ApiEndpoint {
                path: "/monitoring/health",
                method: HttpMethod::Get,
                auth: EndpointAuth::Public,
                description: "Monitoring health",
                critical_workflow: false,
            },
            ApiEndpoint {
                path: "/monitoring/dashboard",
                method: HttpMethod::Get,
                auth: EndpointAuth::Authenticated,
                description: "Monitoring dashboard",
                critical_workflow: false,
            },
            // Metrics
            ApiEndpoint {
                path: "/metrics/processors",
                method: HttpMethod::Get,
                auth: EndpointAuth::Authenticated,
                description: "Processor metrics",
                critical_workflow: false,
            },
            ApiEndpoint {
                path: "/metrics/retries",
                method: HttpMethod::Get,
                auth: EndpointAuth::Authenticated,
                description: "Retry metrics",
                critical_workflow: false,
            },
            ApiEndpoint {
                path: "/metrics/transactions",
                method: HttpMethod::Get,
                auth: EndpointAuth::Authenticated,
                description: "Transaction metrics",
                critical_workflow: false,
            },
            // State / backup
            ApiEndpoint {
                path: "/state/export",
                method: HttpMethod::Post,
                auth: EndpointAuth::Admin,
                description: "Export state backup",
                critical_workflow: true,
            },
            ApiEndpoint {
                path: "/state/import",
                method: HttpMethod::Post,
                auth: EndpointAuth::Admin,
                description: "Import state backup",
                critical_workflow: true,
            },
            ApiEndpoint {
                path: "/state/save",
                method: HttpMethod::Post,
                auth: EndpointAuth::Admin,
                description: "Persist state to disk",
                critical_workflow: true,
            },
            // Scanner API (rate-limited)
            ApiEndpoint {
                path: "/api/scan",
                method: HttpMethod::Post,
                auth: EndpointAuth::RateLimited,
                description: "Submit contract scan",
                critical_workflow: true,
            },
            ApiEndpoint {
                path: "/api/status",
                method: HttpMethod::Get,
                auth: EndpointAuth::Authenticated,
                description: "Scan job status",
                critical_workflow: false,
            },
        ];
        Self { endpoints }
    }

    pub fn all(&self) -> &[ApiEndpoint] {
        &self.endpoints
    }

    pub fn count(&self) -> usize {
        self.endpoints.len()
    }

    pub fn critical_workflows(&self) -> Vec<&ApiEndpoint> {
        self.endpoints
            .iter()
            .filter(|e| e.critical_workflow)
            .collect()
    }

    pub fn by_auth(&self, auth: EndpointAuth) -> Vec<&ApiEndpoint> {
        self.endpoints.iter().filter(|e| e.auth == auth).collect()
    }
}

impl Default for EndpointRegistry {
    fn default() -> Self {
        Self::full_catalog()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_catalog_covers_all_endpoints() {
        let registry = EndpointRegistry::full_catalog();
        assert!(
            registry.count() >= 30,
            "expected comprehensive endpoint catalog, got {}",
            registry.count()
        );
    }

    #[test]
    fn admin_endpoints_require_admin_auth() {
        let registry = EndpointRegistry::full_catalog();
        let admin_paths = ["/api/admin/users", "/state/export", "/queue/cleanup"];
        for path in admin_paths {
            let ep = registry
                .all()
                .iter()
                .find(|e| e.path == path)
                .unwrap_or_else(|| panic!("missing endpoint {path}"));
            assert_eq!(ep.auth, EndpointAuth::Admin, "{path} must require admin");
        }
    }
}
