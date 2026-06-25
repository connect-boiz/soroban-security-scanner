//! API documentation and developer portal helpers.
//!
//! Provides OpenAPI 3.0 metadata structs and a `GET /openapi.json`
//! handler that serves the live spec. All endpoint descriptions,
//! request/response schemas, and error codes are defined here.

use axum::response::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenApiInfo {
    pub title:       &'static str,
    pub description: &'static str,
    pub version:     &'static str,
    pub contact:     ContactInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContactInfo {
    pub name:  &'static str,
    pub email: &'static str,
    pub url:   &'static str,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenApiSpec {
    pub openapi:  &'static str,
    pub info:     OpenApiInfo,
    pub servers:  Vec<ServerInfo>,
    pub tags:     Vec<TagInfo>,
    pub paths:    HashMap<String, PathItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInfo { pub url: String, pub description: String }

#[derive(Debug, Serialize, Deserialize)]
pub struct TagInfo { pub name: &'static str, pub description: &'static str }

#[derive(Debug, Serialize, Deserialize)]
pub struct PathItem {
    pub summary:     String,
    pub description: String,
    pub tags:        Vec<String>,
    pub deprecated:  bool,
}

/// Build the live OpenAPI 3.0 spec for this service.
pub fn build_spec(base_url: &str) -> OpenApiSpec {
    let mut paths = HashMap::new();
    paths.insert("/v1/scan".into(), PathItem {
        summary: "Submit a contract for scanning".into(),
        description: "Upload a Soroban contract and receive a scan_id. Poll /v1/scan/{id} or connect via WebSocket /ws/scan/{id} for results.".into(),
        tags: vec!["scan".into()], deprecated: false,
    });
    paths.insert("/v1/scan/{id}".into(), PathItem {
        summary: "Get scan result".into(),
        description: "Retrieve the completed scan report. Returns 404 while scan is in-progress.".into(),
        tags: vec!["scan".into()], deprecated: false,
    });
    paths.insert("/v1/vulnerabilities".into(), PathItem {
        summary: "List vulnerability reports".into(),
        description: "Returns paginated vulnerability reports submitted by the authenticated principal.".into(),
        tags: vec!["vulnerabilities".into()], deprecated: false,
    });
    paths.insert("/health".into(), PathItem {
        summary: "Health check".into(),
        description: "Liveness and readiness probe. Returns 200 when service is healthy.".into(),
        tags: vec!["infrastructure".into()], deprecated: false,
    });

    OpenApiSpec {
        openapi: "3.0.3",
        info: OpenApiInfo {
            title:       "Soroban Security Scanner API",
            description: "REST API for scanning Soroban smart contracts for security vulnerabilities.",
            version:     env!("CARGO_PKG_VERSION"),
            contact: ContactInfo {
                name: "Security Team", email: "security@soroban-scanner.dev",
                url: "https://docs.soroban-scanner.dev",
            },
        },
        servers: vec![
            ServerInfo { url: base_url.into(), description: "Current environment".into() },
            ServerInfo { url: "https://api.soroban-scanner.dev".into(), description: "Production".into() },
        ],
        tags: vec![
            TagInfo { name: "scan",            description: "Contract scanning operations" },
            TagInfo { name: "vulnerabilities", description: "Vulnerability report management" },
            TagInfo { name: "infrastructure",  description: "Health and metrics" },
        ],
        paths,
    }
}

/// Axum handler: `GET /openapi.json`
pub async fn openapi_handler() -> Json<OpenApiSpec> {
    Json(build_spec("http://localhost:3000"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn spec_has_scan_path()   { assert!(build_spec("http://x").paths.contains_key("/v1/scan")); }
    #[test] fn spec_version_set()     { assert!(!build_spec("http://x").info.version.is_empty()); }
    #[test] fn spec_has_health_path() { assert!(build_spec("http://x").paths.contains_key("/health")); }
}
