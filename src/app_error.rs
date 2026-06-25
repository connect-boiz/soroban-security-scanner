//! Unified error handling for the Soroban Security Scanner Axum API.
//!
//! All handler functions return `Result<T, AppError>`.  `AppError` implements
//! `IntoResponse` so Axum automatically serialises errors to a consistent JSON
//! envelope:
//!
//! ```json
//! {
//!   "error": {
//!     "code": "NOT_FOUND",
//!     "message": "resource not found",
//!     "correlation_id": "01HX9YZ"
//!   }
//! }
//! ```

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Error envelope
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Debug, Serialize)]
struct ErrorDetail {
    code:           &'static str,
    message:        String,
    correlation_id: String,
}

// ---------------------------------------------------------------------------
// AppError
// ---------------------------------------------------------------------------

/// Application-level error type.  Variants map to HTTP status codes.
#[derive(Debug, Error)]
pub enum AppError {
    /// 404 — the requested resource does not exist.
    #[error("resource not found")]
    NotFound,

    /// 400 — the request payload failed validation.
    #[error("validation error: {0}")]
    ValidationError(String),

    /// 401 — the caller is not authenticated.
    #[error("authentication required")]
    Unauthorized,

    /// 403 — the caller is authenticated but lacks permission.
    #[error("permission denied")]
    Forbidden,

    /// 429 — the caller has exceeded their rate limit.
    #[error("rate limit exceeded")]
    RateLimited,

    /// 500 — an unexpected internal error occurred.
    /// The `String` is logged server-side but **not** returned to the caller.
    #[error("internal error")]
    InternalError(String),
}

impl AppError {
    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            Self::NotFound             => (StatusCode::NOT_FOUND,                  "NOT_FOUND"),
            Self::ValidationError(_)   => (StatusCode::BAD_REQUEST,                "VALIDATION_ERROR"),
            Self::Unauthorized         => (StatusCode::UNAUTHORIZED,               "UNAUTHORIZED"),
            Self::Forbidden            => (StatusCode::FORBIDDEN,                  "FORBIDDEN"),
            Self::RateLimited          => (StatusCode::TOO_MANY_REQUESTS,          "RATE_LIMITED"),
            Self::InternalError(_)     => (StatusCode::INTERNAL_SERVER_ERROR,      "INTERNAL_ERROR"),
        }
    }

    /// Human-readable message safe to return to the caller.
    /// Internal errors use a generic string to avoid leaking implementation details.
    fn safe_message(&self) -> String {
        match self {
            Self::InternalError(_) => "an internal error occurred".to_owned(),
            other                  => other.to_string(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();

        // Log internal errors server-side with the root cause.
        if let Self::InternalError(ref cause) = self {
            tracing::error!(error.kind = "InternalError", cause = %cause, "unhandled error");
        }

        let body = ErrorBody {
            error: ErrorDetail {
                code,
                message:        self.safe_message(),
                correlation_id: Uuid::new_v4().to_string(),
            },
        };

        (status, Json(body)).into_response()
    }
}

// Convenience: convert anyhow errors to InternalError
impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        Self::InternalError(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    fn status(e: AppError) -> StatusCode {
        let (s, _) = e.status_and_code();
        s
    }

    #[test]
    fn not_found_returns_404()         { assert_eq!(status(AppError::NotFound),                         StatusCode::NOT_FOUND); }
    #[test]
    fn validation_returns_400()        { assert_eq!(status(AppError::ValidationError("x".into())),     StatusCode::BAD_REQUEST); }
    #[test]
    fn unauthorized_returns_401()      { assert_eq!(status(AppError::Unauthorized),                    StatusCode::UNAUTHORIZED); }
    #[test]
    fn forbidden_returns_403()         { assert_eq!(status(AppError::Forbidden),                       StatusCode::FORBIDDEN); }
    #[test]
    fn rate_limited_returns_429()      { assert_eq!(status(AppError::RateLimited),                     StatusCode::TOO_MANY_REQUESTS); }
    #[test]
    fn internal_returns_500()          { assert_eq!(status(AppError::InternalError("boom".into())),   StatusCode::INTERNAL_SERVER_ERROR); }
    #[test]
    fn internal_error_message_is_generic() {
        let e = AppError::InternalError("secret db password".into());
        assert!(!e.safe_message().contains("secret"));
    }
}
