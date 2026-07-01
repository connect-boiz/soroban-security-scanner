# Scan Result Access Control Policies

> **Purpose:** Prevent Insecure Direct Object Reference (IDOR) vulnerabilities by enforcing strict ownership verification, role-based access control (RBAC), and comprehensive audit logging for all scan result access.

---

## 1. Overview

The Soroban Security Scanner stores sensitive scan results containing:
- Proprietary smart contract source code
- Vulnerability details that could be exploited
- Confidential security assessments
- Bounty and financial information

Unauthorized access to another user's scan results constitutes a critical security breach. This document defines the access control policies implemented to prevent such breaches.

---

## 2. Core Security Principles

### 2.1 Ownership Verification (Primary Defense)

**Every** access to a scan result MUST verify that the requesting user either:
1. **Owns** the scan result (created it), OR
2. Has been **explicitly granted access** via the sharing mechanism, OR
3. Holds a **privileged role** (Admin, Auditor) that permits cross-user access, OR
4. The scan has been **explicitly marked as public** by its owner

This verification is performed by `ScanAccessControl::verify_scan_access()` and must be called at the start of every endpoint that retrieves, modifies, or exports scan results.

### 2.2 UUID-Based Identifiers (Enumeration Prevention)

All scan identifiers use **UUID v4** (cryptographically random, 122 bits of entropy) instead of sequential integers. This prevents attackers from:
- Enumerating scan results by incrementing IDs
- Discovering the total number of scans in the system
- Inferring scan creation patterns

Sequential identifiers (`u64` counters) are **prohibited** for any scan result identifier exposed to users.

### 2.3 Defense in Depth

Multiple layers of protection ensure that even if one layer fails, others still prevent unauthorized access:
1. **Authentication** (JWT/session) — verifies user identity
2. **Ownership verification** — checks user owns the resource
3. **RBAC** — role-based fallback for privileged users
4. **Audit logging** — records all access attempts for forensic analysis

---

## 3. Role-Based Access Control (RBAC)

| Role | View Own Scans | View Others' Scans | Share Scans | Modify/Delete Scans | View Access Logs |
|------|:---:|:---:|:---:|:---:|:---:|
| **Admin** | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Auditor** | ✅ | ✅ | ❌ | ❌ | ✅ |
| **Security Researcher** | ✅ | ❌* | ✅ | ✅ (own only) | ✅ (own only) |
| **Developer** | ✅ | ❌* | ❌ | ✅ (own only) | ✅ (own only) |
| **User** | ✅ | ❌* | ❌ | ❌ | ❌ |

*Unless the scan has been explicitly shared with them or is marked public.*

### 3.1 Role Definitions

- **Admin:** Full system access. Can view, modify, delete, and share all scan results. Typically held by platform operators and security leads.
- **Auditor:** Read-only access to all scan results. Cannot modify or share. Used for compliance reviews and external security assessments.
- **Security Researcher:** Can create, view, and share their own scans. Can access scans shared with them. The default role for security researchers using the platform.
- **Developer:** Can create and view their own scans. Cannot share with others. The default role for developers scanning their own contracts.
- **User:** Most restricted. Can only view explicitly shared or public scan results.

### 3.2 Role Assignment

Roles are assigned during user registration and stored in JWT claims. Role changes require admin approval and are logged in the audit trail.

---

## 4. Scan Sharing Mechanism

### 4.1 Sharing Rules

- Only the scan **owner** (or Admin) can share a scan
- Sharing requires an explicit recipient user ID
- Shares have an optional **expiration time** (default: 30 days, max: 1 year)
- A scan cannot be shared with the owner themselves
- Maximum 50 recipients per scan (configurable)

### 4.2 Revocation

- Owners can revoke sharing at any time
- Revocation takes effect immediately — no grace period
- Revoked users lose access to the scan immediately

### 4.3 Public Scans

- Owner must explicitly mark a scan as **public**
- Public scans are accessible to any authenticated user
- Public scans still require authentication (no anonymous access)
- The public flag is prominently displayed in the UI

---

## 5. Audit Logging

### 5.1 What Is Logged

Every scan access attempt records:
- `scan_id` — which scan was accessed
- `user_id` — who attempted access
- `role` — the user's role at time of access
- `timestamp` — when the access occurred
- `ip_address` — the user's IP address
- `action` — what action was attempted (View, Download, Share, Delete, Verify, Export)
- `success` — whether access was granted or denied
- `failure_reason` — reason for denial (if applicable)

### 5.2 Access Log Retention

- Logs are retained for the lifetime of the scan record
- Maximum 1,000 log entries per scan (oldest entries are rotated)
- Access logs are only visible to the scan owner, Admin, and Auditor roles

### 5.3 Integration

Access events are emitted via `EventLogger` with the `ScanResultAccess` and `ScanResultShare` critical operation types for centralized monitoring and SIEM integration.

---

## 6. Implementation Guidelines

### 6.1 Endpoint Checklist

Every scan result endpoint MUST:

```rust
// 1. Extract authenticated user from request
let user_id = get_user_id(&request)?;
let role = ScanAccessRole::from_str(&get_auth_context(&request)?.role);

// 2. Verify scan access (IDOR prevention)
access_control.verify_scan_access(&scan_id, &user_id, &role)?;

// 3. Log the access attempt
access_control.log_access(
    &scan_id, &user_id, &role,
    ip_address, ScanAccessAction::View,
    true, None,
)?;

// 4. Retrieve and return the scan result
let scan = access_control.get_scan_record(&scan_id)?;
```

### 6.2 Error Responses

Access denials should return **HTTP 404 Not Found** (not 403 Forbidden) to prevent information leakage about the existence of scan IDs. This follows the principle of not revealing whether a resource exists.

```rust
// Map ScanAccessError::AccessDenied to 404 in route handlers
Err(ScanAccessError::AccessDenied { .. }) => StatusCode::NOT_FOUND,
Err(ScanAccessError::ScanNotFound(_)) => StatusCode::NOT_FOUND,
```

### 6.3 Performance

- Access control checks MUST complete in **<20ms** (target from acceptance criteria)
- UUID comparison is O(1)
- RBAC checks are constant-time
- In-memory scan registry provides sub-millisecond lookups

---

## 7. Security Testing

### 7.1 Automated Tests

The following test categories are enforced in CI:

1. **Ownership tests:** Verify that non-owners cannot access, modify, or delete other users' scans
2. **Enumeration tests:** Verify that UUIDs prevent sequential ID guessing
3. **RBAC tests:** Verify that each role has correct permissions
4. **Sharing tests:** Verify sharing, revocation, and expiration
5. **Audit log tests:** Verify all access attempts are logged
6. **Concurrent access tests:** Verify thread safety

### 7.2 CI/CD Integration

The `.github/workflows/security-idor-tests.yml` workflow runs on every push and PR that modifies access control code.

### 7.3 Manual Penetration Testing

Periodic manual penetration testing should verify:
- IDOR attacks across all scan-related endpoints
- JWT token manipulation for role escalation
- Rate limiting bypass for scan enumeration
- Race conditions in sharing/revocation

---

## 8. Related Documentation

- [Authentication Service](./AUTHENTICATION_SERVICE_COMPLETE.md)
- [Rate Limiting](./RATE_LIMITING.md)
- [Audit Trail](./AUDIT_TRAIL.md)
- [Event Logging System](../src/event_logging.rs)
- [Security Headers](./SECURITY_HEADERS_COMMIT.md)

---

## 9. Changelog

| Date | Change | Author |
|------|--------|--------|
| 2026-06-28 | Initial access control policies (Issue #329) | Emmanuel-Ugochukwu1 |

---

*For questions or security concerns about these policies, open an issue at [github.com/connect-boiz/soroban-security-scanner/issues](https://github.com/connect-boiz/soroban-security-scanner/issues)*
