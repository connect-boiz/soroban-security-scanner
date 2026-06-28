//! End-to-end integration tests: a secure default response earns A+, CORS
//! validates origins, SRI protects external scripts, and monitoring catches a
//! weakened configuration.

use super::*;

#[test]
fn secure_default_response_is_a_plus() {
    let headers = SecurityHeaders::secure_default("nonce-xyz");
    let report = evaluate(&headers);
    assert_eq!(report.grade, Grade::APlus);
    assert_eq!(report.score, 100);

    // The full header set is present and CSP is strict.
    let pairs = headers.to_pairs();
    let names: Vec<&str> = pairs.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"Content-Security-Policy"));
    assert!(names.contains(&"Strict-Transport-Security"));
    assert!(names.contains(&"Permissions-Policy"));
    assert!(headers.csp.is_strict());
}

#[test]
fn cors_only_reflects_allowlisted_origins() {
    let cors = CorsConfig::default()
        .allow_origin("https://app.example.com")
        .allow_origin("https://admin.example.com");

    assert!(matches!(
        cors.evaluate("https://app.example.com"),
        CorsDecision::Allowed(_)
    ));
    assert_eq!(
        cors.evaluate("https://attacker.example"),
        CorsDecision::Rejected
    );
    // No subdomain/suffix bypass.
    assert_eq!(
        cors.evaluate("https://app.example.com.evil.com"),
        CorsDecision::Rejected
    );
}

#[test]
fn sri_detects_tampered_cdn_script() {
    let original = b"console.log('trusted')";
    let integrity_value = integrity(original, SriAlgorithm::Sha384);
    // The browser would accept the untampered asset...
    assert!(verify(original, &integrity_value));
    // ...and reject a swapped/tampered one.
    assert!(!verify(b"console.log('evil')", &integrity_value));
}

#[test]
fn monitoring_flags_a_weakened_deploy() {
    let monitor = HeaderMonitor::new(Grade::A);

    // A good deploy passes silently.
    assert!(monitor
        .observe(&SecurityHeaders::secure_default("n"))
        .is_none());

    // A regression that drops the CSP to empty is caught and alerted.
    let mut weakened = SecurityHeaders::secure_default("n");
    weakened.csp = ContentSecurityPolicy::new();
    let alert = monitor.observe(&weakened).unwrap();
    assert!(alert.grade < Grade::A);
    assert!(alert.weaknesses.iter().any(|w| w.contains("CSP")));

    let stats = monitor.stats();
    assert_eq!(stats.observed, 2);
    assert_eq!(stats.passing, 1);
    assert_eq!(stats.failing, 1);
}

#[test]
fn csp_nonce_binds_inline_scripts() {
    let headers = SecurityHeaders::secure_default("RANDOM_NONCE");
    let csp = headers.get("Content-Security-Policy").unwrap();
    assert!(csp.contains("script-src 'self' 'nonce-RANDOM_NONCE'"));
    // Inline scripts without the nonce are not permitted (no 'unsafe-inline').
    assert!(!csp.contains("unsafe-inline"));
}

#[test]
fn permissions_policy_disables_dangerous_features() {
    let headers = SecurityHeaders::secure_default("n");
    let pp = headers.get("Permissions-Policy").unwrap();
    assert!(pp.contains("camera=()"));
    assert!(pp.contains("microphone=()"));
    assert!(pp.contains("geolocation=()"));
}
