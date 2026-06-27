# Comprehensive Audit Trail for Security Operations

Implements issue **#326** — a structured, tamper-evident audit trail for all
security-critical operations on the platform.

## Why

The previous logging only captured ad-hoc error messages: no structured events,
no actor/IP/request context, and no tamper-evident storage. That made it
impossible to attribute, detect, or investigate malicious activity, and it
failed the compliance requirements expected of a platform that handles financial
transactions (bounty payments) and sensitive vulnerability data.

## What was added

| Area | File |
| ---- | ---- |
| Audit trail engine (Rust) | [src/audit_trail.rs](src/audit_trail.rs) |
| Unit tests | [src/audit_trail_tests.rs](src/audit_trail_tests.rs) |
| Integration tests | [tests/audit_trail_integration_tests.rs](tests/audit_trail_integration_tests.rs) |
| Database schema | [migrations/008_add_audit_trail.sql](migrations/008_add_audit_trail.sql) |

## Acceptance criteria coverage

| Criterion | How it is met |
| --------- | ------------- |
| Structured logging for all security-critical operations | `AuditAction` enumerates vulnerability create/update/delete, verify/reject, bounty create/update/payment/cancel, escrow release, and admin role/suspend/config/access actions. |
| Each event includes timestamp, user id, action, resources, IP, user agent, request id, previous/new state | Captured on `AuditEvent`; populated from an `ActorContext` plus the `AuditEventBuilder`. |
| Tamper-evident, write-once / read-many storage | In-memory store appends only; the `security_audit_log` table blocks `UPDATE`/`DELETE` via triggers (the sole exception is the controlled archival flag). |
| 7-year retention with automatic archival | `AuditConfig::retention_period_seconds` defaults to 7 years; `entries_eligible_for_archival()` and the SQL `archive_old_audit_entries()` archive rather than delete. |
| Query API with role-based access control | `AuditTrail::query` requires an admin-class `UserRole`; the `audit_log_readable` SQL view gates reads by session role. |
| Real-time alerting for suspicious patterns | `detect_suspicious_patterns()` and the SQL `detect_suspicious_audit_patterns()` flag a user performing admin actions from multiple IPs in a window. |
| Cryptographic integrity verification | Every entry carries a SHA-256 `entry_hash` chained to `previous_entry_hash`; `verify_chain()` (Rust) and `verify_audit_chain()` (SQL) detect content tampering and chain breaks. |
| 100% coverage of state-changing operations | A dedicated `AuditAction` variant exists for each state-changing class; categories let coverage be reasoned about exhaustively. |
| < 50ms performance per operation | Recording is an in-memory append plus a single SHA-256 hash; the integration suite asserts < 50ms/op over 1,000 operations. |
| Unit and integration tests | 16 unit tests + 7 integration tests covering every criterion. |

## Usage

```rust
use soroban_security_scanner::audit_trail::{
    ActorContext, AuditAction, AuditEventBuilder, AuditQuery, AuditSeverity, AuditTrail, UserRole,
};

let trail = AuditTrail::with_defaults();

// Record a security-critical operation with full request context.
let actor = ActorContext::new("user-123")
    .with_role(UserRole::Admin)
    .with_ip("203.0.113.7")
    .with_user_agent("Mozilla/5.0")
    .with_request_id("req-abc");

trail.record(
    AuditEventBuilder::new(AuditAction::VulnerabilityVerify, actor)
        .description("verified critical reentrancy report")
        .resource("vulnerability", "vuln-42")
        .severity(AuditSeverity::Critical)
        .previous_state("{\"status\":\"reported\"}")
        .new_state("{\"status\":\"verified\"}")
        .build(),
)?;

// Query (admin only).
let recent = trail.query(
    UserRole::Admin,
    &AuditQuery::new().category(AuditAction::VulnerabilityVerify.category()).paginate(0, 50),
)?;

// Integrity + alerting.
assert!(trail.verify_chain()?.intact);
let alerts = trail.detect_suspicious_patterns()?;
```

## Database

Run the migration with the rest of the schema:

```bash
sqlx migrate run --database-url "$DATABASE_URL"
```

`security_audit_log` is append-only (WORM). `UPDATE`/`DELETE` raise an exception;
the only permitted mutation is the archival flag flipped by
`archive_old_audit_entries()` (a `SECURITY DEFINER` function). Reads should go
through the role-gated `audit_log_readable` view.

## Tests

```bash
cargo test --lib audit_trail            # unit tests
cargo test --test audit_trail_integration_tests   # integration tests
```
