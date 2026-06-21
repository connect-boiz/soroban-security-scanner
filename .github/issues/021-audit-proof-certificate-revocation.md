# Issue 21: [Audit Proof of Scan] Certificate Revocation List Not Implemented, Compromised Certificates Remain Active

## Description

The `AuditProofOfScan` module in `src/audit_proof_of_scan.rs` generates `SecurityCertificate` objects that cryptographically prove that a scan was performed at a specific time with specific results. These certificates can be used for compliance and insurance purposes. However, the module lacks a Certificate Revocation List (CRL) mechanism. If a scanner's private key is compromised, or if a certificate was issued in error (e.g., for the wrong contract), there is no way to revoke that certificate. The `CertificateStatus` enum has variants like `Active`, `Expired`, `Revoked`, but the `Revoked` status is never set anywhere in the codebase — there is no `revoke_certificate(id, reason)` function. This means that compromised certificates continue to be treated as valid in downstream verification and insurance claims, creating a serious security and legal liability.

## Acceptance Criteria

- [ ] Add a `revoke_certificate(certificate_id: &str, reason: RevocationReason, revoked_by: &str)` function to `AuditProofOfScan`
- [ ] Define a `RevocationReason` enum with variants: `KeyCompromise`, `CertificateAuthorityCompromise`, `AffiliationChanged`, `Superseded`, `CessationOfOperation`, `Unspecified`
- [ ] Implement a CRL data structure (in-memory with optional database persistence) that stores revoked certificate IDs and timestamps
- [ ] Add a `verify_certificate_not_revoked(certificate_id)` check that is called before any certificate verification
- [ ] Publish a `crl.json` endpoint at `GET /api/v1/certificates/revocation-list` for external verifiers
- [ ] Write tests covering certificate revocation, double-revocation (should error), and verification of revoked certificates

## Additional Context

Key files: `src/audit_proof_of_scan.rs`, `src/audit_proof_of_scan_tests.rs`, `examples/audit_proof_of_scan_usage.py`.
