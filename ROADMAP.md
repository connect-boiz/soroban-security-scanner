# Soroban Security Scanner — Roadmap

> **Last updated:** June 15, 2026

This roadmap organizes the 30 open issues into three milestones with estimated timelines, dependencies, and success criteria for each.

---

## Milestone 1: MVP Hardening 🚀

**Target:** Q3 2026 (July–September)  
**Theme:** Fix critical security vulnerabilities and broken core functionality

This milestone addresses issues that pose security risks, cause data loss, or break fundamental platform capabilities. These should be resolved before any new feature development.

### Security Fixes

| # | Issue | Priority | Est. Effort |
|---|-------|----------|-------------|
| 14 | DefaultHasher Used Instead of Cryptographic Hash | **Critical** | 2–3 days |
| 24 | OAuth2 State Parameter Not Validated (CSRF) | **Critical** | 1–2 days |
| 21 | Certificate Revocation List Not Implemented | **Critical** | 2–3 days |
| 26 | Timelock Bypass in Emergency Upgrade | **Critical** | 3–4 days |
| 29 | Bounty Payout Without Escrow Balance Check | **Critical** | 2 days |
| 5 | Multi-Sig Signer Weight Thresholds Not Validated | **High** | 3–4 days |

### Core Functionality Fixes

| # | Issue | Priority | Est. Effort |
|---|-------|----------|-------------|
| 2 | Access Control False Positives for Internal Helpers | **High** | 3–5 days |
| 11 | Stuck Scans Not Automatically Detected | **High** | 3–4 days |
| 8 | Time Travel State Incompatibility with Upgraded WASM | **High** | 4–5 days |
| 9 | Fuzzing Input Generator Misses Composite Edge Cases | **High** | 3–4 days |
| 27 | Cross-Contract Simulator Ignores Recursive Calls | **High** | 4–5 days |

### Infrastructure Fixes

| # | Issue | Priority | Est. Effort |
|---|-------|----------|-------------|
| 7 | Rate Limiting Ignores Reverse Proxy Headers | **High** | 2–3 days |
| 6 | Notification Delivery Status Not Persisted | **High** | 3–4 days |
| 23 | Offline Cached Data Inaccessible | **High** | 3–4 days |
| 28 | WebSocket Subscriptions Lost on Reconnect | **High** | 2–3 days |

### Dependencies

- Issue 14 (crypto) must be completed before any new auth features
- Issue 24 (OAuth2) blocks social login reliability
- Issue 5 (multi-sig) should precede Issue 19 (wizard validation)
- Issue 2 (access control) is a prerequisite for any scanner accuracy improvements

### Success Criteria

- [ ] Zero critical-severity security vulnerabilities
- [ ] All core scanner features produce reliable results (false positive rate < 5%)
- [ ] No data loss scenarios on service restart or network interruption
- [ ] Rate limiting works correctly behind production proxy infrastructure

---

## Milestone 2: Quality & Performance 🌟

**Target:** Q4 2026 (October–December)  
**Theme:** UX polish, performance optimization, accessibility compliance, and documentation

### UX & Accessibility

| # | Issue | Priority | Est. Effort |
|---|-------|----------|-------------|
| 1 | Incomplete Error Boundary Coverage | **High** | 3–4 days |
| 3 | Account Lockout Notification Missing | **High** | 2–3 days |
| 15 | Accessibility Violations in Scan Results Table | **Medium** | 3–4 days |
| 19 | Multi-Sig Wizard Lacks Real-Time Address Validation | **Medium** | 2–3 days |
| 4 | Ledger Import Fails Silently on Timeout | **Medium** | 2–3 days |
| 25 | Skeleton Components Flash on Fast Connections | **Medium** | 1–2 days |

### Performance

| # | Issue | Priority | Est. Effort |
|---|-------|----------|-------------|
| 16 | Unused Translations in Bundle (1.2MB) | **Medium** | 2–3 days |
| 17 | Missing Database Index on `transactions` | **Medium** | 1–2 days |
| 13 | Gas Estimation Not Adaptive | **Medium** | 3–4 days |
| 10 | Batch Operations Lack Gas Estimation | **Medium** | 2–3 days |

### Process & Compliance

| # | Issue | Priority | Est. Effort |
|---|-------|----------|-------------|
| 18 | Accessibility Tests Not Running in CI | **Medium** | 1–2 days |
| 12 | Event Logging Lacks Export & Query API | **Medium** | 4–5 days |
| 22 | Error Messages Not Internationalized | **Medium** | 3–4 days |
| 20 | Scanner Registry Lacks Semantic Versioning | **Medium** | 2–3 days |

### Documentation

| # | Issue | Priority | Est. Effort |
|---|-------|----------|-------------|
| 30 | No Differential Fuzzing Documentation | **Low** | 2–3 days |

### Dependencies

- Issue 18 (CI tests) should precede Issue 15 (a11y fixes) to catch regressions
- Issue 16 (bundle size) is a prerequisite for Lighthouse CI threshold updates
- Issue 12 (event export) depends on Issue 6 (persistence) from Milestone 1

### Success Criteria

- [ ] WCAG 2.1 AA compliance with zero critical/serious axe-core violations
- [ ] Lighthouse Performance score ≥ 90 (mobile)
- [ ] Lighthouse Accessibility score ≥ 95
- [ ] Dashboard queries complete in under 100ms for 100k+ transactions
- [ ] All CI pipelines pass — unit, integration, e2e, and accessibility tests

---

## Milestone 3: Advanced Features & Ecosystem 🎯

**Target:** Q1 2027 (January–March)  
**Theme:** Differentiated capabilities, platform maturity, and developer ecosystem

### Planned Initiatives

| Initiative | Related Issues | Est. Effort |
|------------|----------------|-------------|
| **Adaptive Gas Optimization Engine** | 10, 13 | 2–3 weeks |
| **Comprehensive Fuzzing Suite** | 9, 27 + new issues | 3–4 weeks |
| **Enterprise Compliance Pack** | 12, 21, 30 | 2–3 weeks |
| **Internationalization Completion** | 16, 22 | 2 weeks |
| **Developer SDK & API** | 20, 30 | 3–4 weeks |

### Dependencies

- All Milestone 1 and Milestone 2 issues must be resolved
- External: Community feedback and usage data from Milestone 2 release

### Success Criteria

- [ ] Gas optimization reduces user costs by average 15%
- [ ] Fuzzing suite detects reentrancy patterns up to 5 levels deep
- [ ] SOC 2 Type II audit readiness (tamper-evident logs, certificate management)
- [ ] 100% i18n coverage for top 10 locales
- [ ] Public API documentation and SDK published

---

## Prioritization Summary

| Severity | Count | Milestone |
|----------|-------|-----------|
| 🔴 Critical | 6 | MVP Hardening |
| 🟠 High | 13 | MVP Hardening (9), Quality (4) |
| 🟡 Medium | 10 | Quality & Performance |
| 🟢 Low | 1 | Quality & Performance |

## How to Contribute

1. Pick an issue from the current milestone
2. Assign yourself on GitHub
3. Follow the issue template's acceptance criteria
4. Submit a PR referencing the issue number

---

*See [ISSUE_TRACKER.md](./ISSUE_TRACKER.md) for the full kanban-style view of all 30 issues.*
