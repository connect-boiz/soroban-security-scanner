//! End-to-end integration tests for the upload-sanitization subsystem,
//! combining the validation pipeline with rate limiting, quarantine and
//! progress tracking as they would be wired in an endpoint handler.

use super::*;
use chrono::DateTime;
use uuid::Uuid;

fn valid_wasm() -> Vec<u8> {
    let mut m = vec![0x00, 0x61, 0x73, 0x6d];
    m.extend_from_slice(&1u32.to_le_bytes());
    m.push(1); // type section
    m.push(1); // length
    m.push(0); // payload
    m
}

#[test]
fn full_accept_flow_with_rate_limit() {
    let scanner = SignatureScanner::new();
    let limiter = UploadRateLimiter::new(UploadRateConfig::default());
    let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

    // Rate gate passes, then the pipeline accepts the WASM.
    assert!(limiter
        .check_and_record("user-1", "203.0.113.1", now)
        .is_ok());
    let wasm = valid_wasm();
    let req = UploadRequest {
        bytes: &wasm,
        declared_content_type: "application/wasm",
        filename: "contract.wasm",
        tier: UploadTier::Paid,
    };
    assert!(validate_upload(&req, &scanner).is_accepted());
}

#[test]
fn malicious_upload_is_rejected_and_can_be_quarantined_for_forensics() {
    let scanner = SignatureScanner::new();
    let mut quarantine = Quarantine::new();
    let uploader = Uuid::new_v4();

    let req = UploadRequest {
        bytes: b"fn go() { let _ = \"/dev/tcp/10.0.0.1/4444\"; }",
        declared_content_type: "text/x-rust",
        filename: "payload.rs",
        tier: UploadTier::Free,
    };

    match validate_upload(&req, &scanner) {
        UploadVerdict::Rejected { stage, reason } => {
            assert_eq!(stage, "deep-inspection");
            // Operator retains the artifact for forensic review.
            let id = quarantine.add(req.bytes, "payload.rs", uploader, reason, 1000);
            assert_eq!(quarantine.pending_count(), 1);
            assert!(quarantine.purge(id));
            assert!(quarantine.get(id).unwrap().bytes.is_none());
        }
        other => panic!("expected rejection, got {other:?}"),
    }
}

#[test]
fn suspicious_upload_routes_to_quarantine_for_review() {
    let scanner = SignatureScanner::new();
    let mut quarantine = Quarantine::new();
    let uploader = Uuid::new_v4();

    let req = UploadRequest {
        bytes: b"let cfg = include_str!(\"../../config/secrets.toml\");",
        declared_content_type: "text/x-rust",
        filename: "loader.rs",
        tier: UploadTier::Free,
    };

    match validate_upload(&req, &scanner) {
        UploadVerdict::Quarantined { reason, filename } => {
            let id = quarantine.add(req.bytes, filename, uploader, reason, 1000);
            // A reviewer clears the false positive and recovers the bytes.
            let released = quarantine.release(id).unwrap();
            assert_eq!(released, req.bytes);
        }
        other => panic!("expected quarantine, got {other:?}"),
    }
}

#[test]
fn tiers_gate_large_uploads_consistently() {
    let scanner = SignatureScanner::new();
    // ~5 MB of realistic multi-line source (short lines avoid the
    // overlong-line obfuscation heuristic).
    let line = b"pub fn helper_value() -> i32 { 42 }\n";
    let mut payload = Vec::with_capacity(5 * 1024 * 1024 + line.len());
    while payload.len() < 5 * 1024 * 1024 {
        payload.extend_from_slice(line);
    }

    // Free tier (1 MB) rejects.
    let free = UploadRequest {
        bytes: &payload,
        declared_content_type: "text/plain",
        filename: "big.rs",
        tier: UploadTier::Free,
    };
    assert!(matches!(
        validate_upload(&free, &scanner),
        UploadVerdict::Rejected { .. }
    ));

    // Paid tier (10 MB) accepts the same content.
    let paid = UploadRequest {
        bytes: &payload,
        declared_content_type: "text/plain",
        filename: "big.rs",
        tier: UploadTier::Paid,
    };
    assert!(validate_upload(&paid, &scanner).is_accepted());
}

#[test]
fn upload_progress_completes_then_pipeline_runs() {
    // Simulate streaming a WASM upload to completion before validation.
    let wasm = valid_wasm();
    let mut progress = UploadProgress::start(wasm.len() as u64, 1000, 30);
    let mut t = 1000;
    for chunk in wasm.chunks(3) {
        t += 1;
        progress.record_chunk(chunk.len() as u64, t);
    }
    assert!(progress.is_complete());

    let scanner = SignatureScanner::new();
    let req = UploadRequest {
        bytes: &wasm,
        declared_content_type: "application/wasm",
        filename: "contract.wasm",
        tier: UploadTier::Free,
    };
    assert!(validate_upload(&req, &scanner).is_accepted());
}

#[test]
fn rate_limit_blocks_flood_from_one_ip() {
    let limiter = UploadRateLimiter::new(UploadRateConfig {
        per_user_hourly: 1000,
        per_ip_hourly: 5,
    });
    let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
    for i in 0..5 {
        assert!(limiter
            .check_and_record(&format!("user-{i}"), "198.51.100.7", now)
            .is_ok());
    }
    assert_eq!(
        limiter.check_and_record("user-x", "198.51.100.7", now),
        Err(RateScope::Ip)
    );
}
