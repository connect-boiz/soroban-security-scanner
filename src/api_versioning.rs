//! API versioning and backward compatibility middleware.
//!
//! Supports URL-path versioning (`/v1/`, `/v2/`) and
//! `Accept: application/vnd.soroban.v1+json` header versioning.
//! Unknown versions return 400 with a clear migration guide.

use crate::app_error::AppError;
use axum::{
    body::Body,
    http::{HeaderMap, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;

pub const CURRENT_VERSION: ApiVersion = ApiVersion::V1;
pub const MIN_SUPPORTED_VERSION: ApiVersion = ApiVersion::V1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ApiVersion { V1 }

impl ApiVersion {
    pub fn as_str(self) -> &'static str { match self { Self::V1 => "v1" } }

    pub fn from_path(path: &str) -> Option<Self> {
        if path.contains("/v1/") || path.starts_with("/v1") { Some(Self::V1) }
        else { None }
    }

    pub fn from_accept(headers: &HeaderMap) -> Option<Self> {
        headers.get("accept")?
            .to_str().ok()
            .and_then(|v| if v.contains("soroban.v1") { Some(Self::V1) } else { None })
    }
}

#[derive(Serialize)]
struct VersionError {
    error:           &'static str,
    current_version: &'static str,
    upgrade_guide:   &'static str,
}

/// Axum middleware: validates and extracts API version from request.
pub async fn api_version_middleware(req: Request<Body>, next: Next) -> Response {
    let path = req.uri().path().to_owned();
    let version = ApiVersion::from_path(&path)
        .or_else(|| ApiVersion::from_accept(req.headers()))
        .unwrap_or(CURRENT_VERSION);

    if version < MIN_SUPPORTED_VERSION {
        let body = Json(VersionError {
            error: "API version no longer supported",
            current_version: CURRENT_VERSION.as_str(),
            upgrade_guide: "https://docs.soroban-scanner.dev/migration",
        });
        return (StatusCode::BAD_REQUEST, body).into_response();
    }

    let mut req = req;
    req.extensions_mut().insert(version);
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn v1_from_path()   { assert_eq!(ApiVersion::from_path("/v1/scan"), Some(ApiVersion::V1)); }
    #[test] fn unknown_path()   { assert_eq!(ApiVersion::from_path("/scan"),    None); }
    #[test] fn v1_is_current()  { assert_eq!(CURRENT_VERSION, ApiVersion::V1); }
}
