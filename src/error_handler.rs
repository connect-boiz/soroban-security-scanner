//! Unified error handling middleware for the Axum API.
//!
//! Provides a centralized `AppError` enum that every handler can return,
//! ensuring consistent JSON error envelopes, HTTP status codes, and
//! per-request tracing spans.
//!
//! # Example
//! ```ignore
//! use axum::{Json, extract::State};
//! use crate::error_handler::AppError;
//!
//! async fn get_user(State(db): State<DbPool>) -> Result<Json<User>, AppError> {
//!     let user = db.find_user(1).await?.ok_or(AppError::NotFound)?;
//!     Ok(Json(user))
//! }
//! ```

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::fmt;

/// Centralized application error covering all HTTP-facing failure modes.
///
/// Every variant carries the correct HTTP status code and a snake_case
/// error code for the frontend to switch on.
#[derive(Debug)]
pub enum AppError {
    /// Resource not found (HTTP 404).
    NotFound,
    /// Invalid request body, query parameters, or path parameters (HTTP 400).
    ValidationError(String),
    /// Missing or invalid authentication credentials (HTTP 401).
    Unauthorized,
    /// Authenticated but not permitted to access the resource (HTTP 403).
    Forbidden,
    /// Unexpected internal server error (HTTP 500). The contained string
    /// is logged but **not** exposed to the client in production.
    InternalError(String),
    /// Too many requests — rate limit exceeded (HTTP 429).
    RateLimited,
}

impl AppError {
    /// HTTP status code associated with this error variant.
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::ValidationError(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
        }
    }

    /// Machine-readable snake_case error code (e.g. `"validation_error"`).
    pub fn error_code(&self) -> &'static str {
        match self {
            AppError::NotFound => "not_found",
            AppError::ValidationError(_) => "validation_error",
            AppError::Unauthorized => "unauthorized",
            AppError::Forbidden => "forbidden",
            AppError::InternalError(_) => "internal_error",
            AppError::RateLimited => "rate_limited",
        }
    }

    /// Human-readable message. Internal errors never leak their message.
    pub fn message(&self) -> String {
        match self {
            AppError::NotFound => "The requested resource was not found.".to_string(),
            AppError::ValidationError(ref msg) => msg.clone(),
            AppError::Unauthorized => {
                "Authentication is required to access this resource.".to_string()
            }
            AppError::Forbidden => {
                "You do not have permission to access this resource.".to_string()
            }
            AppError::InternalError(_) => "An internal server error occurred.".to_string(),
            AppError::RateLimited => {
                "Too many requests. Please slow down and try again later.".to_string()
            }
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::InternalError(ref detail) => {
                write!(f, "internal error: {}", detail)
            }
            _ => write!(f, "{}", self.message()),
        }
    }
}

impl std::error::Error for AppError {}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        // Matches the IntoResponse JSON envelope: { "error": { "code": ..., "message": ..., "details": ... } }
        let mut state = serializer.serialize_struct("AppError", 1)?;
        state.serialize_field(
            "error",
            &ErrorDetail {
                code: self.error_code().to_string(),
                message: self.message(),
                details: serde_json::Value::Object(Default::default()),
            },
        )?;
        state.end()
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = ErrorBody {
            error: ErrorDetail {
                code: self.error_code().to_string(),
                message: self.message(),
                details: serde_json::Value::Object(Default::default()),
            },
        };

        // Log internal errors on the server side
        if let AppError::InternalError(ref detail) = self {
            tracing::error!(error = %detail, "Internal server error");
        } else {
            tracing::warn!(
                status = status.as_u16(),
                code = self.error_code(),
                "Request error"
            );
        }

        (status, Json(body)).into_response()
    }
}

/// Uniform JSON error envelope returned to clients.
#[derive(Debug, Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    details: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Convenience conversions from common crate types so handlers can use `?`.
// ---------------------------------------------------------------------------

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::InternalError(format!("{:#}", err))
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::InternalError(format!("IO error: {}", err))
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::ValidationError(format!("Invalid JSON: {}", err))
    }
}

// ---------------------------------------------------------------------------
// Tracing middleware factory
// ---------------------------------------------------------------------------

use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

/// Create a [`TraceLayer`] that logs every request with method, path,
/// status code, and duration.
///
/// Add this layer to your Axum router:
/// ```ignore
/// let app = Router::new()
///     .layer(crate::error_handler::request_tracing_layer());
/// ```
pub fn request_tracing_layer() -> impl tower::Layer<axum::Router> + Clone {
    TraceLayer::new_for_http()
        .make_span_with(
            DefaultMakeSpan::new()
                .level(Level::INFO)
                .include_headers(false),
        )
        .on_response(
            DefaultOnResponse::new()
                .level(Level::INFO)
                .include_headers(false)
                .latency_unit(tower_http::LatencyUnit::Millis),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, routing::get, Router};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[test]
    fn test_app_error_status_codes() {
        assert_eq!(AppError::NotFound.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(
            AppError::ValidationError("bad".into()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::Unauthorized.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(AppError::Forbidden.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(
            AppError::InternalError("boom".into()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            AppError::RateLimited.status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    #[test]
    fn test_app_error_codes() {
        assert_eq!(AppError::NotFound.error_code(), "not_found");
        assert_eq!(
            AppError::ValidationError("x".into()).error_code(),
            "validation_error"
        );
        assert_eq!(AppError::Unauthorized.error_code(), "unauthorized");
        assert_eq!(AppError::Forbidden.error_code(), "forbidden");
        assert_eq!(
            AppError::InternalError("x".into()).error_code(),
            "internal_error"
        );
        assert_eq!(AppError::RateLimited.error_code(), "rate_limited");
    }

    #[test]
    fn test_internal_error_does_not_leak_message() {
        let err = AppError::InternalError("secret stack trace".into());
        assert_eq!(err.message(), "An internal server error occurred.");
    }

    #[test]
    fn test_serialize_app_error() {
        let err = AppError::ValidationError("Name is required".into());
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["error"]["code"], "validation_error");
        assert_eq!(json["error"]["message"], "Name is required");
        assert!(json["error"]["details"].is_object());
    }

    #[test]
    fn test_serialize_internal_error_hides_details() {
        let err = AppError::InternalError("secret".into());
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["error"]["code"], "internal_error");
        assert_eq!(
            json["error"]["message"],
            "An internal server error occurred."
        );
    }

    #[tokio::test]
    async fn test_into_response_produces_correct_status() {
        let err = AppError::NotFound;
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed["error"]["code"], "not_found");
    }

    #[tokio::test]
    async fn test_handler_returns_app_error() {
        async fn handler() -> Result<Json<serde_json::Value>, AppError> {
            Err(AppError::Forbidden)
        }

        let app = Router::new().route("/test", get(handler));

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_from_anyhow_error() {
        let anyhow_err = anyhow::anyhow!("something went wrong");
        let app_err: AppError = anyhow_err.into();
        assert!(matches!(app_err, AppError::InternalError(_)));
        assert_eq!(app_err.message(), "An internal server error occurred.");
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let app_err: AppError = io_err.into();
        assert!(matches!(app_err, AppError::InternalError(_)));
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let app_err: AppError = json_err.into();
        assert!(matches!(app_err, AppError::ValidationError(_)));
    }
}
