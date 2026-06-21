# API Security Testing Guide

This document describes the comprehensive API security testing framework,
test cases, procedures, and schedules for the Soroban Security Scanner platform.

**Issue:** [#348 — Missing Comprehensive API Security Testing](https://github.com/connect-boiz/soroban-security-scanner/issues/348)

## Overview

The `src/api_security/` module provides:

- **SecurityTestSuite** — automated checks across auth, authz, fuzzing, and business logic
- **FuzzingEngine** — input validation and error-handling regression tests
- **RegressionTestSuite** — historical vulnerability regression tests
- **SecurityCoverageReport** — 100% endpoint coverage with quality gates
- **SecurityDefectTracker** — defect tracking with remediation workflows

## Quick Start

### Run the full security suite locally

```bash
./scripts/api_security_scan.sh
```

### Run Rust tests only

```bash
cargo test --lib api_security
cargo test --test api_security_tests
```

### Run Node.js fuzz tests

```bash
npm test -- --testPathPattern=api-security-fuzz
```

### Run with OWASP ZAP baseline scan

```bash
# Start the API server first, then:
ZAP_TARGET_URL=http://localhost:3000 ./scripts/api_security_scan.sh
```

## Test Categories

### 1. Authentication Testing

| Test Case | Endpoint | Expected |
|-----------|----------|----------|
| Missing JWT on protected route | `/api/profile` | 401 Unauthorized |
| Expired JWT token | `/api/profile` | 401 Unauthorized |
| Invalid JWT signature | `/api/profile` | 401 Unauthorized |
| JWT none-algorithm bypass | `/api/admin/users` | 401 Unauthorized |
| Brute-force lockout | `/auth/login` | 429 after threshold |

### 2. Authorization Testing

| Test Case | Endpoint | Expected |
|-----------|----------|----------|
| Non-admin access to admin routes | `/api/admin/users` | 403 Forbidden |
| User accessing another user's transaction | `/transactions/:id` | 403 Forbidden |
| Anonymous state export | `/state/export` | 403 Forbidden |

### 3. Input Validation / Fuzzing

| Category | Payload Type | Expected |
|----------|-------------|----------|
| SQL injection | `admin' OR '1'='1` | Rejected |
| XSS | `<script>alert(1)</script>` | Rejected |
| Path traversal | `../../etc/passwd` | Rejected |
| Oversized payload | >32KB JSON body | Rejected (413) |
| Null bytes | `\x00` in email | Rejected |
| JSON bomb | Deeply nested arrays | Rejected |

### 4. Business Logic Testing (Critical Workflows)

| Workflow | Endpoints | Checks |
|----------|-----------|--------|
| User registration → login → profile | `/auth/register`, `/auth/login`, `/api/profile` | Token issued, profile accessible |
| Transaction lifecycle | `POST /transactions`, `GET /transactions/:id`, `POST .../cancel` | State transitions valid |
| Scan submission | `POST /api/scan`, `GET /api/status` | Rate limit enforced |
| State backup/restore | `POST /state/export`, `POST /state/import` | Admin-only, data integrity |

### 5. Historical Vulnerability Regression

| ID | Vulnerability | Mitigation |
|----|--------------|------------|
| REG-001 | Time-based attack | time-based-attack-detector |
| REG-002 | Reentrancy | security_analyzer |
| REG-003 | JWT none-algorithm bypass | auth middleware whitelist |
| REG-004 | Rate limit bypass | trusted proxy config |
| REG-005 | Missing CSP | frontend middleware |
| REG-006 | IDOR on transactions | ownership check |
| REG-007 | SQL injection | parameterized queries |
| REG-008 | Session fixation | token rotation on login |

## Security Test Coverage

The framework maintains **100% endpoint coverage** via `EndpointRegistry::full_catalog()`.
The quality gate requires:

- **Endpoint coverage:** 100%
- **Critical workflow coverage:** 100%
- **High-severity blocking:** CI fails on high/critical findings

Reports are written to `target/api-security-report.md` and `target/api-security-coverage.md`.

## CI/CD Integration

The `api-security` job in `.github/workflows/ci.yml`:

1. Runs `cargo test --lib api_security`
2. Runs `cargo test --test api_security_tests`
3. Runs Node.js fuzz tests
4. **Blocks merge** on high-severity failures or coverage gate failure
5. Uploads security report as artifact

Optional: set `ZAP_TARGET_URL` in CI to enable OWASP ZAP baseline scanning.

## Penetration Testing Schedule

| Activity | Frequency | Tool | Scope |
|----------|-----------|------|-------|
| Automated security suite | Every CI run | SecurityTestSuite | All endpoints |
| OWASP ZAP baseline | Every CI run (when URL set) | OWASP ZAP | Running API instance |
| Full penetration test | **Quarterly** | OWASP ZAP + manual review | All API endpoints |
| Security regression | Every CI run | RegressionTestSuite | Historical vulns |

Next quarterly assessment is tracked by `PenTestSchedule::quarterly()`.

## Security Defect Tracking

Defects are tracked via `SecurityDefectTracker` with this workflow:

```
Open → InRemediation → Resolved → Verified
```

| Severity | CI Blocking |
|----------|-------------|
| Critical | Yes |
| High | Yes |
| Medium | No |
| Low | No |

## Burp Suite Integration

For manual penetration testing with Burp Suite:

1. Configure Burp proxy to intercept traffic to the API
2. Import the endpoint catalog from `EndpointRegistry::full_catalog()`
3. Run active scan against authenticated session
4. Log findings in `SecurityDefectTracker`

## File Reference

| File | Purpose |
|------|---------|
| `src/api_security/suite.rs` | Main security test suite |
| `src/api_security/endpoints.rs` | Endpoint registry (100% coverage) |
| `src/api_security/fuzzing.rs` | Input fuzzing engine |
| `src/api_security/regression.rs` | Historical vulnerability tests |
| `src/api_security/coverage.rs` | Coverage reporting and gates |
| `src/api_security/defect_tracking.rs` | Defect tracking workflow |
| `scripts/api_security_scan.sh` | CI entry point |
| `tests/api_security_tests.rs` | Integration tests |
| `tests/api-security-fuzz.test.js` | Node fuzz tests |
