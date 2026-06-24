# Security Testing Procedures and Policies

## Overview

This document outlines the comprehensive security testing strategy for the Soroban Security Scanner platform. All code changes must pass through defined security gates before reaching production.

## Security Testing Framework

### 1. SAST (Static Application Security Testing)

**Tools Used:**
- **CodeQL** - GitHub's semantic code analysis engine (JS/TS, Rust)
- **Semgrep** - Pattern-based SAST with OWASP Top 10 coverage
- **ESLint Security Plugins** - JavaScript/TypeScript security linting
- **Gitleaks** - Secret and credential detection

**Scope:** All source code in `src/`, `frontend/`, `component-library/`, `contracts/`

**Trigger:** Every push and pull request

**Rules:**
- Critical findings: Block deployment, must be fixed
- High findings: Block deployment if >5 findings
- Secrets: Block PR merge immediately

### 2. DAST (Dynamic Application Security Testing)

**Tools Used:**
- **OWASP ZAP** - Full active and baseline scanning

**Scope:** Staging environment (`staging.soroban-security-scanner.io`)

**Trigger:** Weekly scheduled scan, deployment to staging, manual dispatch

**Rules:**
- High-severity findings: Block deployment
- Medium-severity findings: Warn if >10

### 3. SCA (Software Composition Analysis)

**Tools Used:**
- **npm audit** - Node.js dependency scanning
- **cargo audit** - Rust dependency scanning
- **Snyk** - Comprehensive dependency vulnerability analysis

**Scope:** All `package.json`, `Cargo.toml` dependencies

**Trigger:** Every push and pull request, weekly full scan

**Rules:**
- Critical dependencies: Block deployment
- High dependencies: Warn if >5, require approval

### 4. Container Security Scanning

**Tools Used:**
- **Trivy** - Filesystem and container image vulnerability scanning

**Scope:** All Docker images, filesystem, IaC configurations

**Trigger:** Every push, weekly full scan

**Rules:**
- CRITICAL/HIGH vulnerabilities in images: Block deployment

### 5. IaC Security Scanning

**Tools Used:**
- **Checkov** - Terraform, K8s, Dockerfile misconfiguration scanning
- **KubeSec** - Kubernetes manifest security analysis

**Scope:** `infrastructure/`, `Dockerfile*`, `docker-compose*.yml`

**Trigger:** Every push with IaC changes

**Rules:**
- High-severity misconfigurations: Block deployment

### 6. Secret Detection

**Tools Used:**
- **Gitleaks** - Git history and filesystem scanning for secrets

**Scope:** Full repository including git history

**Trigger:** Every push and pull request

**Rules:**
- Any secret found: Block PR merge, branch auto-removal

## Security Gates

### PR Merge Gate
Required checks before merge:
- [x] SAST (CodeQL, Semgrep, ESLint Security) - no critical findings
- [x] SCA (npm audit, cargo audit) - no critical vulnerabilities
- [x] Secret Detection - no secrets found
- [x] IaC Security - no high-severity misconfigurations
- [x] Security Regression Tests - no regressions

### Deployment Gate
Required checks before deployment:
- [x] All PR merge gates passed
- [x] DAST scan passed (staging only)
- [x] Container scan passed (no CRITICAL/HIGH)
- [x] Security approval (if critical/high findings exist)
- [x] Manual approval (if risk threshold exceeded)

## Policy Enforcement

### Vulnerability Blocking Rules

| Severity | Category | Action | Auto-Remediate |
|----------|----------|--------|----------------|
| Critical | Secrets | Block PR merge | No |
| Critical | SAST | Block deployment | No |
| Critical | SCA | Block deployment | No |
| Critical | DAST | Block deployment | No |
| High | SAST | Block if >5 | No |
| High | SCA | Require approval | No |
| High | DAST | Block deployment | No |
| Medium | Any | Notify team | Yes (auto-fix) |

### Remediation Timelines

| Severity | Time to Fix | Auto-Escalation |
|----------|-------------|-----------------|
| Critical | 24 hours | 12 hours |
| High | 72 hours | 48 hours |
| Medium | 7 days | N/A |

## Metrics and Reporting

### Weekly Security Metrics
- Vulnerability counts by severity (trending)
- Scan coverage percentage (target: 100%)
- Mean time to remediation (MTTR)
- Security gate pass/fail rate
- Top vulnerability types

### Dashboards
- **GitHub Security Tab**: Code scanning alerts, secret scanning, Dependabot
- **SARIF Reports**: Uploaded to GitHub for each tool
- **Custom Metrics**: Stored in `security/reports/metrics/`

## Security Regression Testing

Regression tests verify that previously fixed vulnerabilities are not reintroduced:

```
npm test -- --testPathPattern=security
cargo test -- --test vulnerability_detection
```

### Test Categories
1. **Known vulnerability patterns** - Ensure detectors identify OWASP Top 10
2. **False positive regression** - Previously FP-prone patterns remain clean
3. **Fix verification** - Previously fixed issues stay fixed

## Coverage Requirements

| Test Type | Coverage Target | Verification Method |
|-----------|----------------|-------------------|
| SAST | 100% of code | All source files scanned |
| SCA | 100% of dependencies | All manifests scanned |
| DAST | 80% of endpoints | Staging environment |
| Container | 100% of images | Every build scanned |
| IaC | 100% of manifests | All TF/K8s files scanned |

## Procedures

### For Developers
1. Run `npm run lint:security` before committing
2. Fix all security findings locally
3. Push changes and verify security CI passes
4. If security gate blocks deployment, remediate findings
5. Request security review for override (if needed)

### For Security Team
1. Review weekly security metrics
2. Triage new vulnerability findings
3. Approve/deny security overrides
4. Update security policies as needed
5. Conduct quarterly penetration testing

## Compliance

This security framework aligns with:
- OWASP Application Security Verification Standard (ASVS)
- SOC 2 Type II security criteria
- GDPR Article 32 security requirements
- NIST Cybersecurity Framework
- CWE/SANS Top 25 Most Dangerous Software Errors
