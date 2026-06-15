# Soroban Security Scanner — Issue Tracker

> **30 issues** organized by priority, type, and area.  
> Last updated: June 15, 2026

---

## 🔴 Critical Priority (6 issues)

| # | Issue | Type | Area | Milestone | Est. Effort |
|---|-------|------|------|-----------|-------------|
| 14 | [Secure ID Generation] DefaultHasher Used Instead of Cryptographic Hash | `security` `crypto` | Security/Crypto | MVP | 2–3d |
| 21 | [Audit Proof] Certificate Revocation List Not Implemented | `security` `compliance` | Audit | MVP | 2–3d |
| 24 | [Auth] OAuth2 State Parameter Not Validated (CSRF) | `security` `bug` | Authentication | MVP | 1–2d |
| 26 | [Contract] Timelock Bypass in Emergency Upgrade | `security` `contract` | Contracts | MVP | 3–4d |
| 29 | [Bounty] Payout Without Escrow Balance Check | `bug` `financial` | Bounty Marketplace | MVP | 2d |
| 5 | [Multi-Sig] Signer Weight Thresholds Not Validated | `bug` `security` | Multi-Sig | MVP | 3–4d |

---

## 🟠 High Priority (13 issues)

| # | Issue | Type | Area | Milestone | Est. Effort |
|---|-------|------|------|-----------|-------------|
| 1 | [Frontend] Incomplete Error Boundary Coverage | `bug` `ux` | Frontend | Quality | 3–4d |
| 2 | [Scanner] Access Control False Positives for Internal Helpers | `bug` `scanner` | Core Scanner | MVP | 3–5d |
| 3 | [Auth] Account Lockout Notification Missing | `enhancement` `ux` | Authentication | Quality | 2–3d |
| 6 | [Notifications] Delivery Status Not Persisted | `bug` `data-loss` | Notifications | MVP | 3–4d |
| 7 | [Rate Limiting] Ignores Reverse Proxy Headers | `bug` `infra` | Rate Limiting | MVP | 2–3d |
| 8 | [Time Travel] Forked State Incompatible with Upgraded WASM | `bug` `tool` | Time Travel | MVP | 4–5d |
| 9 | [Fuzzing] Input Generator Misses Composite Edge Cases | `bug` `scanner` | Differential Fuzzing | MVP | 3–4d |
| 11 | [Emergency Stop] Stuck Scans Not Detected | `bug` `reliability` | Core Scanner | MVP | 3–4d |
| 23 | [Offline] Cached Data Inaccessible When Offline | `bug` `ux` | Offline | MVP | 3–4d |
| 27 | [Fuzzing] Cross-Contract Simulator Ignores Recursive Calls | `bug` `scanner` | Differential Fuzzing | MVP | 4–5d |
| 28 | [Frontend] WebSocket Subscriptions Lost on Reconnect | `bug` `ux` | Frontend | MVP | 2–3d |

---

## 🟡 Medium Priority (10 issues)

| # | Issue | Type | Area | Milestone | Est. Effort |
|---|-------|------|------|-----------|-------------|
| 4 | [Wallet] Ledger Import Fails Silently on Timeout | `enhancement` `ux` | Wallet | Quality | 2–3d |
| 10 | [Batch] Gas Estimation Not Available Before Execution | `enhancement` `feature` | Batch Operations | Quality | 2–3d |
| 12 | [Events] No Export or Query API for Compliance | `enhancement` `compliance` | Event Logging | Quality | 4–5d |
| 13 | [Gas] No Dynamic Adjustment Based on History | `enhancement` `performance` | Gas Limits | Quality | 3–4d |
| 15 | [Frontend] Accessibility Violations in Results Table | `bug` `a11y` | Frontend | Quality | 3–4d |
| 16 | [Performance] Unused Translations in Bundle (1.2MB) | `performance` `optimization` | Frontend | Quality | 2–3d |
| 17 | [Database] Missing Index on `transactions` | `performance` `database` | Database | Quality | 1–2d |
| 18 | [CI/CD] Accessibility Tests Not Running in Pipeline | `process` `ci` | CI/CD | Quality | 1–2d |
| 19 | [Multi-Sig] Wizard Lacks Real-Time Address Validation | `enhancement` `ux` | Multi-Sig | Quality | 2–3d |
| 20 | [Registry] No Semantic Version Comparison Logic | `enhancement` `feature` | Scanner Registry | Quality | 2–3d |
| 22 | [i18n] Error Messages Not Internationalized | `enhancement` `l10n` | i18n | Quality | 3–4d |
| 25 | [Performance] Skeleton Flash on Fast Connections | `bug` `ux` | Frontend | Quality | 1–2d |

---

## 🟢 Low Priority (1 issue)

| # | Issue | Type | Area | Milestone | Est. Effort |
|---|-------|------|------|-----------|-------------|
| 30 | [Docs] No Differential Fuzzing Documentation | `docs` | Documentation | Quality | 2–3d |

---

## View by Area

### Frontend / UI / UX — 7 issues
1, 15, 16, 19, 23, 25, 28

### Core Scanner / Detection — 5 issues
2, 7, 10, 11, 20

### Authentication / Authorization — 2 issues
3, 24

### Wallet / Multi-Sig — 3 issues
4, 5, 19

### Smart Contracts — 2 issues
26, 29

### Security / Cryptography — 2 issues
14, 21

### Performance / Infrastructure — 4 issues
16, 17, 18, 25

### Data / Persistence — 2 issues
6, 12

### i18n / L10n — 1 issue
22

### Documentation — 1 issue
30

### Differential Fuzzing — 2 issues
9, 27

### Gas / Events — 2 issues
10, 13

### Offline / Connectivity — 2 issues
23, 28

---

## View by Type

| Type | Count | Issues |
|------|-------|--------|
| 🐛 Bug | 13 | 1, 2, 5, 6, 7, 8, 9, 11, 15, 23, 25, 27, 28 |
| 🔒 Security | 5 | 14, 21, 24, 26, 29 |
| 🚀 Enhancement | 7 | 3, 4, 10, 12, 13, 19, 22 |
| ⚡ Performance | 2 | 16, 17 |
| 📋 Process | 1 | 18 |
| 📄 Documentation | 1 | 30 |
| ♿ Accessibility | 1 | 15 (also bug) |

---

## Burndown Progress

```
MVP Hardening:     ████████░░░░░░░░░░░░  [14/14 complete]
Quality & Perf:    ░░░░░░░░░░░░░░░░░░░░  [0/16 complete]
Total:             ░░░░░░░░░░░░░░░░░░░░  [0/30 complete]
```

---

*See [ROADMAP.md](./ROADMAP.md) for milestone details and [.github/issues/](./.github/issues/) for individual issue files.*
