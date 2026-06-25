//! Comprehensive error handling without information disclosure.
//!
//! Wraps all error types into sanitised responses that log full
//! context server-side while returning only safe, actionable
//! messages to callers.

use crate::app_error::AppError;
use std::panic;
use tracing::{error, warn};

/// Install a global panic hook that logs panics as structured errors
/// instead of printing to stderr, preventing stack-trace leakage.
pub fn install_panic_hook() {
    panic::set_hook(Box::new(|info| {
        let location = info.location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown".into());
        let message = info.payload()
            .downcast_ref::<&str>().copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
            .unwrap_or("unknown panic");
        error!(location = %location, message = %message, "PANIC");
    }));
}

/// Sanitise an anyhow error chain for API callers.
/// Logs the full chain internally, returns only the outermost message.
pub fn sanitise_error(err: &anyhow::Error) -> AppError {
    // Log full chain for internal observability
    let chain: Vec<String> = err.chain().map(|e| e.to_string()).collect();
    error!(error_chain = ?chain, "internal error");
    AppError::InternalError(err.to_string())
}

/// Convert common OS / IO errors to typed AppError variants.
pub fn map_io_error(err: std::io::Error) -> AppError {
    use std::io::ErrorKind::*;
    match err.kind() {
        NotFound        => AppError::NotFound,
        PermissionDenied => AppError::Forbidden,
        TimedOut        => AppError::InternalError("operation timed out".into()),
        _               => AppError::InternalError(format!("io error: {}", err.kind())),
    }
}

/// Log a security event (auth failure, rate limit, forbidden) with context.
pub fn log_security_event(
    event_type: &str,
    principal:  Option<&str>,
    ip:         Option<&str>,
    detail:     &str,
) {
    warn!(
        event     = %event_type,
        principal = %principal.unwrap_or("anonymous"),
        ip        = %ip.unwrap_or("unknown"),
        detail    = %detail,
        "security_event"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn io_not_found_maps_to_app_not_found() {
        let e = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        assert!(matches!(map_io_error(e), AppError::NotFound));
    }
    #[test]
    fn io_permission_maps_to_forbidden() {
        let e = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        assert!(matches!(map_io_error(e), AppError::Forbidden));
    }
}
