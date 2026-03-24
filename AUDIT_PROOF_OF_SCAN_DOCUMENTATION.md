# Decentralized Audit "Proof of Scan" Contract

## Overview

The Decentralized Audit "Proof of Scan" contract enables the Soroban Security Scanner to issue non-transferable "Security Certificates" on-chain after a contract passes all security invariants. These certificates serve as cryptographic proof that a DeFi protocol has been thoroughly audited and cleared for deployment.

## Features

### 🔐 **Security Certificates (SBTs)**
- **Non-transferable**: Soulbound Token logic prevents certificate transfer
- **Cryptographically verifiable**: On-chain proof of security clearance
- **Time-limited validity**: Automatic expiration requiring re-scanning
- **Revocable**: Immediate revocation if vulnerabilities are discovered

### 📋 **Security Reporting**
- **Comprehensive reports**: Contract ID, timestamp, risk scores
- **IPFS integration**: Full detailed reports stored on IPFS
- **Risk scoring**: Low, Medium, High, Critical risk levels
- **Invariant tracking**: Passed/failed invariants and scan metrics

### 🔍 **Third-Party Verification**
- **Public verification**: Anyone can query certificate status
- **Contract clearance**: Simple API to check if protocol is "Cleared"
- **Historical tracking**: Complete audit trail of all certificates
- **Real-time monitoring**: Event-driven updates for certificate changes

### ⚖️ **Governance & Control**
- **Scanner authorization**: Only authorized scanner can issue certificates
- **Admin oversight**: Admin can revoke certificates and manage settings
- **Flexible validity periods**: Customizable certificate lifetimes
- **Audit transparency**: All actions emit on-chain events

## Contract Architecture

### Data Structures

#### `SecurityReport`
```rust
pub struct SecurityReport {
    pub contract_id: Address,              // Contract being audited
    pub timestamp: u64,                    // Scan timestamp
    pub risk_score: RiskScore,             // Overall risk assessment
    pub vulnerabilities_found: u64,        // Number of vulnerabilities found
    pub invariants_passed: u64,            // Invariants that passed
    pub invariants_failed: u64,            // Invariants that failed
    pub scan_duration: u64,                // Scan duration in seconds
    pub scanner_version: String,           // Scanner version used
    pub ipfs_cid: String,                  // IPFS CID for full report
}
```

#### `SecurityCertificate`
```rust
pub struct SecurityCertificate {
    pub certificate_id: u64,                // Unique certificate identifier
    pub contract_id: Address,              // Contract being certified
    pub report: SecurityReport,            // Complete security report
    pub status: CertificateStatus,          // Active, Revoked, Expired
    pub issued_at: u64,                     // Issuance timestamp
    pub expires_at: u64,                    // Expiration timestamp
    pub issued_by: Address,                 // Scanner that issued it
    pub revoked_at: Option<u64>,            // Revocation timestamp
    pub revoke_reason: Option<String>,      // Reason for revocation
}
```

#### `RiskScore`
```rust
pub enum RiskScore {
    Low,        // Acceptable risk (1)
    Medium,     // Acceptable risk (2)
    High,       // Unacceptable risk (3)
    Critical,   // Unacceptable risk (4)
}
```

#### `CertificateStatus`
```rust
pub enum CertificateStatus {
    Active,     // Currently valid certificate
    Revoked,    // Revoked due to issues
    Expired,    // Past validity period
}
```

### Storage Layout

| Key | Type | Description |
|-----|------|-------------|
| `ADMIN` | `Address` | Contract administrator |
| `SCANNER_PUBLIC_KEY` | `Address` | Authorized scanner public key |
| `CERTIFICATE_COUNTER` | `u64` | Total certificates issued |
| `CERTIFICATES` | `Map<u64, SecurityCertificate>` | All certificate data |
| `CONTRACT_CERTIFICATES` | `Map<Address, u64>` | Contract → Latest certificate mapping |

## API Reference

### Scanner Functions

#### `mint_certificate(contract_id, report, validity_days)`
Issues a new security certificate for a contract.

**Requirements:**
- Caller must be authorized scanner
- Contract must not have active certificate
- Risk score must be acceptable (Low/Medium)
- Valid IPFS CID required

**Arguments:**
- `contract_id: Address` - Contract being certified
- `report: SecurityReport` - Complete security scan results
- `validity_days: Option<u64>` - Custom validity period (1-365 days)

**Returns:**
- `u64` - Certificate ID

**Events:**
- `CERTIFICATE_MINTED`: certificate_id, contract_id, risk_score, issued_at, expires_at

#### `revoke_certificate(certificate_id, reason)`
Revokes an existing certificate (admin or scanner only).

**Requirements:**
- Caller must be admin or scanner
- Certificate must exist and not already be revoked

**Arguments:**
- `certificate_id: u64` - Certificate to revoke
- `reason: String` - Reason for revocation

**Events:**
- `CERTIFICATE_REVOKED`: certificate_id, contract_id, reason, timestamp

### Query Functions

#### `is_contract_cleared(contract_id) -> bool`
Checks if a contract has a valid, active certificate.

**Returns:**
- `bool` - True if contract is cleared, false otherwise

#### `get_contract_certificate(contract_id) -> SecurityCertificate`
Returns the active certificate for a contract.

**Panics:** If no active certificate exists

#### `get_certificate_by_id(certificate_id) -> SecurityCertificate`
Returns certificate details by ID.

**Panics:** If certificate doesn't exist

#### `get_contract_certificate_history(contract_id) -> Vec<SecurityCertificate>`
Returns all certificates for a contract (including historical).

#### `get_certificate_stats() -> (u64, u64, u64, u64)`
Returns certificate statistics:
- Total certificates issued
- Currently active certificates
- Revoked certificates
- Expired certificates

### Admin Functions

#### `update_scanner_public_key(new_scanner_public_key)`
Updates the authorized scanner public key.

**Requirements:**
- Caller must be admin

#### `transfer_admin(new_admin)`
Transfers administrative rights to a new address.

**Requirements:**
- Caller must be current admin

### SBT (Soulbound Token) Functions

#### `transfer_certificate(certificate_id, to)`
**Always fails** - certificates are non-transferable SBTs.

**Panics:** Always throws `TransferNotAllowed` error

## Usage Examples

### Scanner Integration

```rust
// After completing security scan
let security_report = SecurityReport {
    contract_id: contract_address,
    timestamp: env.ledger().timestamp(),
    risk_score: RiskScore::Low,
    vulnerabilities_found: 0,
    invariants_passed: 15,
    invariants_failed: 0,
    scan_duration: 120, // 2 minutes
    scanner_version: String::from_str(&env, "1.2.0"),
    ipfs_cid: String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG"),
};

// Issue certificate
let certificate_id = audit_proof_of_scan::mint_certificate(
    env,
    contract_address,
    security_report,
    Some(30), // 30 days validity
);
```

### Third-Party Verification

```rust
// Check if DeFi protocol is cleared
let is_safe = audit_proof_of_scan::is_contract_cleared(env, defi_protocol_address);

if is_safe {
    println!("✅ Protocol is security cleared");
    // Allow interaction with protocol
} else {
    println!("⚠️ Protocol is not security cleared");
    // Show warning or block interaction
}

// Get certificate details
let certificate = audit_proof_of_scan::get_contract_certificate(env, defi_protocol_address);
println!("Risk Score: {:?}", certificate.report.risk_score);
println!("Expires: {}", certificate.expires_at);
println!("Full Report: https://ipfs.io/ipfs/{}", certificate.report.ipfs_cid);
```

### Security Incident Response

```rust
// Vulnerability discovered - revoke certificate
let reason = String::from_str(&env, "Critical reentrancy vulnerability discovered");
audit_proof_of_scan::revoke_certificate(env, certificate_id, reason);

// Protocol is now marked as unsafe
let is_safe = audit_proof_of_scan::is_contract_cleared(env, defi_protocol_address);
assert!(!is_safe); // Should be false
```

## Security Considerations

### **Certificate Security**
- **Non-transferable**: SBT logic prevents certificate transfer or sale
- **Cryptographic binding**: Certificates bound to specific contract addresses
- **Time-limited validity**: Automatic expiration prevents stale certificates
- **Immediate revocation**: Quick response to discovered vulnerabilities

### **Scanner Authorization**
- **Public key verification**: Only authorized scanner can issue certificates
- **Admin oversight**: Admin can revoke certificates and update scanner keys
- **Audit trail**: All certificate changes emit on-chain events

### **Risk Assessment**
- **Strict risk thresholds**: Only Low/Medium risk certificates issued
- **Comprehensive reporting**: Full scan metrics and vulnerability counts
- **IPFS integration**: Detailed reports stored off-chain but referenced on-chain

### **Data Integrity**
- **Immutable storage**: Certificate data stored immutably on-chain
- **IPFS verification**: CID validation ensures report integrity
- **Timestamp verification**: Ledger timestamps prevent backdating

## Deployment

### **Prerequisites**
- Soroban CLI installed
- Admin address determined
- Scanner public key configured
- IPFS node for report storage

### **Deployment Steps**

1. **Build Contract**
```bash
cargo build --target wasm32-unknown-unknown --release
```

2. **Deploy Contract**
```bash
soroban contract deploy --wasm target/wasm32-unknown-unknown/release/soroban_security_scanner.wasm
```

3. **Initialize Contract**
```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --function initialize \
  --arg <ADMIN_ADDRESS> \
  --arg <SCANNER_PUBLIC_KEY>
```

### **Scanner Configuration**

```bash
# Update scanner public key if needed
soroban contract invoke \
  --id <CONTRACT_ID> \
  --function update_scanner_public_key \
  --arg <NEW_SCANNER_ADDRESS>
```

## IPFS Integration

### **Report Upload Process**

1. **Generate Security Report**
```json
{
  "contract_id": "GD123...",
  "scan_results": {
    "vulnerabilities": [],
    "invariants": {
      "passed": 15,
      "failed": 0
    },
    "risk_score": "Low"
  },
  "detailed_analysis": "...",
  "recommendations": "..."
}
```

2. **Upload to IPFS**
```bash
ipfs add security_report.json
# Returns: QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG
```

3. **Reference in Certificate**
```rust
let report = SecurityReport {
    // ... other fields
    ipfs_cid: String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG"),
};
```

### **Report Retrieval**

```bash
# Fetch full report from IPFS
ipfs cat QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG
```

## Testing

The contract includes comprehensive tests covering:

- ✅ Certificate minting and validation
- ✅ Authorization and access control
- ✅ Risk score acceptability
- ✅ IPFS CID validation
- ✅ Certificate revocation
- ✅ Expiration handling
- ✅ SBT non-transferability
- ✅ Third-party verification
- ✅ Historical tracking
- ✅ Admin functions

Run tests:
```bash
cargo test audit_proof_of_scan_tests
```

## Integration Points

### **DeFi Frontend Integration**
```javascript
// Check protocol safety before allowing interactions
async function checkProtocolSafety(contractAddress) {
    const isSafe = await contract.is_contract_cleared(contractAddress);
    
    if (!isSafe) {
        showWarning("This protocol has not been security cleared");
        disableInteractions();
    } else {
        const certificate = await contract.get_contract_certificate(contractAddress);
        showSafetyBadge(certificate);
    }
}
```

### **Wallet Integration**
```javascript
// Display certificate status in wallet
async function displayCertificateStatus(contractAddress) {
    try {
        const cert = await contract.get_contract_certificate(contractAddress);
        displaySafetyInfo({
            risk: cert.report.risk_score,
            expires: cert.expires_at,
            report: `https://ipfs.io/ipfs/${cert.report.ipfs_cid}`
        });
    } catch (error) {
        displayWarning("No security certificate found");
    }
}
```

### **Scanner Integration**
```python
# Issue certificate after successful scan
def issue_certificate(contract_address, scan_results):
    report = create_security_report(contract_address, scan_results)
    ipfs_cid = upload_to_ipfs(report)
    
    certificate_id = contract.mint_certificate(
        contract_address=contract_address,
        report={
            'contract_id': contract_address,
            'timestamp': int(time.time()),
            'risk_score': map_risk_score(scan_results.risk),
            'vulnerabilities_found': len(scan_results.vulnerabilities),
            'invariants_passed': scan_results.invariants_passed,
            'invariants_failed': scan_results.invariants_failed,
            'scan_duration': scan_results.duration,
            'scanner_version': SCANNER_VERSION,
            'ipfs_cid': ipfs_cid
        },
        validity_days=30
    )
    
    return certificate_id
```

## Monitoring

### **Event Monitoring**
Monitor for these critical events:
- `CERTIFICATE_MINTED`: New certificate issued
- `CERTIFICATE_REVOKED`: Certificate revoked

```bash
# Monitor certificate events
soroban contract events \
  --id <CONTRACT_ID> \
  --topic CERTIFICATE_MINTED \
  --topic CERTIFICATE_REVOKED
```

### **Health Checks**
- Certificate expiration monitoring
- Risk distribution analysis
- Revocation rate tracking

## Future Enhancements

### **Proposed Features**
- **Batch certification**: Issue certificates for multiple contracts
- **Risk-adjusted validity**: Higher risk = shorter validity periods
- **Automatic re-scanning**: Trigger re-scans before expiration
- **Insurance integration**: Link certificates to insurance policies
- **Governance voting**: Community-driven certificate decisions

### **Integration Opportunities**
- **DeFi aggregators**: Certificate-based protocol ranking
- **Insurance protocols**: Premium discounts for certified protocols
- **DAO governance**: Certificate requirements for protocol listings
- **Cross-chain bridges**: Certificate portability across chains

## Troubleshooting

### **Common Issues**

**"NotAuthorized" Error**
- Verify scanner public key is correctly set
- Check if caller is admin or scanner
- Ensure proper signature authentication

**"InvalidRiskScore" Error**
- Risk score must be Low or Medium
- High and Critical risks are unacceptable
- Review scan results and fix issues

**"AlreadyCertified" Error**
- Contract already has active certificate
- Wait for expiration or revoke existing certificate
- Check certificate status

**"TransferNotAllowed" Error**
- This is expected behavior (SBT logic)
- Certificates are intentionally non-transferable
- No action needed

### **Debug Commands**

```bash
# Check contract admin
soroban contract read --id <CONTRACT_ID> --key ADMIN

# Check scanner public key
soroban contract read --id <CONTRACT_ID> --key SCANNER_PUBLIC_KEY

# Check certificate count
soroban contract read --id <CONTRACT_ID> --key CERTIFICATE_COUNTER

# Get certificate for contract
soroban contract call \
  --id <CONTRACT_ID> \
  --function get_contract_certificate \
  --arg <CONTRACT_ADDRESS>
```

## License

This contract is part of the Soroban Security Scanner project and is licensed under the MIT License.
