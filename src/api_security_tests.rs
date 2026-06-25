//! API security test helpers and penetration test automation.
//!
//! Provides typed test cases for common API security vulnerabilities:
//! - Authentication bypass
//! - Authorization / IDOR
//! - Injection payloads
//! - Rate limit bypass
//! - XXE and SSRF vectors

use serde::{Deserialize, Serialize};

/// A security test case definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityTestCase {
    pub id:          &'static str,
    pub category:    VulnCategory,
    pub description: &'static str,
    pub method:      &'static str,
    pub path:        &'static str,
    pub payload:     Option<&'static str>,
    pub expected_status: u16,
    pub should_fail: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VulnCategory {
    AuthBypass, Idor, SqlInjection, CommandInjection,
    RateLimitBypass, Ssrf, Xxe, PathTraversal,
}

/// Standard OWASP API security test suite.
pub fn standard_test_suite() -> Vec<SecurityTestCase> {
    vec![
        SecurityTestCase {
            id: "AUTH-001", category: VulnCategory::AuthBypass,
            description: "Access protected endpoint without JWT",
            method: "GET", path: "/v1/vulnerabilities",
            payload: None, expected_status: 401, should_fail: false,
        },
        SecurityTestCase {
            id: "AUTH-002", category: VulnCategory::AuthBypass,
            description: "Access protected endpoint with expired JWT",
            method: "GET", path: "/v1/scan/any",
            payload: None, expected_status: 401, should_fail: false,
        },
        SecurityTestCase {
            id: "IDOR-001", category: VulnCategory::Idor,
            description: "Access scan result belonging to another user",
            method: "GET", path: "/v1/scan/other-user-scan-id",
            payload: None, expected_status: 403, should_fail: false,
        },
        SecurityTestCase {
            id: "INJ-001", category: VulnCategory::SqlInjection,
            description: "SQL injection in scan ID parameter",
            method: "GET", path: "/v1/scan/1' OR '1'='1",
            payload: None, expected_status: 400, should_fail: false,
        },
        SecurityTestCase {
            id: "PATH-001", category: VulnCategory::PathTraversal,
            description: "Path traversal in file upload filename",
            method: "POST", path: "/v1/upload",
            payload: Some(r#"{"filename": "../../etc/passwd", "content": "x"}"#),
            expected_status: 400, should_fail: false,
        },
        SecurityTestCase {
            id: "RATE-001", category: VulnCategory::RateLimitBypass,
            description: "Exceed scan submission rate limit",
            method: "POST", path: "/v1/scan",
            payload: None, expected_status: 429, should_fail: false,
        },
    ]
}

/// Check if a test result matches expectations.
pub fn assert_test_case(tc: &SecurityTestCase, actual_status: u16) -> Result<(), String> {
    if actual_status == tc.expected_status {
        Ok(())
    } else {
        Err(format!(
            "[{}] {}: expected {} got {}",
            tc.id, tc.description, tc.expected_status, actual_status
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn suite_is_non_empty() { assert!(!standard_test_suite().is_empty()); }
    #[test] fn auth_test_expects_401() {
        let tc = standard_test_suite().into_iter().find(|t| t.id == "AUTH-001").unwrap();
        assert_eq!(tc.expected_status, 401);
    }
    #[test] fn idor_test_expects_403() {
        let tc = standard_test_suite().into_iter().find(|t| t.id == "IDOR-001").unwrap();
        assert_eq!(tc.expected_status, 403);
    }
    #[test] fn correct_status_passes() {
        let tc = &standard_test_suite()[0];
        assert!(assert_test_case(tc, 401).is_ok());
    }
    #[test] fn wrong_status_fails() {
        let tc = &standard_test_suite()[0];
        assert!(assert_test_case(tc, 200).is_err());
    }
}
