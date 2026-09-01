#!/usr/bin/env python3
"""
Decentralized Audit "Proof of Scan" Usage Examples

This script demonstrates how to interact with the Audit Proof of Scan contract
for various use cases including certificate issuance, verification, and monitoring.
"""

import hashlib
import json
import time
import requests
from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass
from enum import Enum

class RiskScore(Enum):
    LOW = "Low"
    MEDIUM = "Medium"
    HIGH = "High"
    CRITICAL = "Critical"

class CertificateStatus(Enum):
    ACTIVE = "Active"
    REVOKED = "Revoked"
    EXPIRED = "Expired"

@dataclass
class SecurityReport:
    contract_id: str
    timestamp: int
    risk_score: RiskScore
    vulnerabilities_found: int
    invariants_passed: int
    invariants_failed: int
    scan_duration: int
    scanner_version: str
    ipfs_cid: str

@dataclass
class SecurityCertificate:
    certificate_id: int
    contract_id: str
    report: SecurityReport
    status: CertificateStatus
    issued_at: int
    expires_at: int
    issued_by: str
    revoked_at: Optional[int]
    revoke_reason: Optional[str]

class AuditProofOfScanClient:
    """Client for interacting with the Audit Proof of Scan contract"""
    
    def __init__(self, contract_id: str, network: str = "testnet"):
        self.contract_id = contract_id
        self.network = network
        self.ipfs_gateway = "https://ipfs.io/ipfs/"
        # Initialize Soroban SDK client here
        # self.client = SorobanClient(network)
    
    def upload_report_to_ipfs(self, report_data: Dict) -> str:
        """Upload detailed security report to IPFS"""
        try:
            # In a real implementation, this would use an IPFS client
            # For now, we'll simulate the upload
            report_json = json.dumps(report_data, indent=2)
            report_hash = hashlib.sha256(report_json.encode()).hexdigest()
            
            # Simulate IPFS CID generation
            cid = f"Qm{report_hash[:46]}"  # Mock CID format
            
            print(f"📤 Uploaded report to IPFS: {cid}")
            print(f"🔗 Access at: {self.ipfs_gateway}{cid}")
            
            return cid
        except Exception as e:
            print(f"❌ Failed to upload to IPFS: {e}")
            raise
    
    def create_security_report(self, contract_id: str, scan_results: Dict) -> SecurityReport:
        """Create a security report from scan results"""
        # Determine risk score
        risk_score = self._calculate_risk_score(scan_results)
        
        # Upload detailed report to IPFS
        detailed_report = {
            "contract_id": contract_id,
            "scan_timestamp": int(time.time()),
            "scanner_version": "1.2.0",
            "scan_results": scan_results,
            "detailed_findings": scan_results.get("detailed_findings", []),
            "recommendations": scan_results.get("recommendations", []),
            "scan_metadata": {
                "duration_seconds": scan_results.get("duration", 0),
                "invariants_tested": scan_results.get("invariants_tested", 0),
                "gas_used": scan_results.get("gas_used", 0)
            }
        }
        
        ipfs_cid = self.upload_report_to_ipfs(detailed_report)
        
        return SecurityReport(
            contract_id=contract_id,
            timestamp=int(time.time()),
            risk_score=risk_score,
            vulnerabilities_found=len(scan_results.get("vulnerabilities", [])),
            invariants_passed=scan_results.get("invariants_passed", 0),
            invariants_failed=scan_results.get("invariants_failed", 0),
            scan_duration=scan_results.get("duration", 0),
            scanner_version="1.2.0",
            ipfs_cid=ipfs_cid
        )
    
    def mint_certificate(self, report: SecurityReport, validity_days: int = 30) -> int:
        """Mint a security certificate"""
        # Validate risk score acceptability
        if report.risk_score in [RiskScore.HIGH, RiskScore.CRITICAL]:
            raise ValueError(f"Cannot issue certificate for {risk_score.value} risk")
        
        # Call contract to mint certificate
        # certificate_id = self.client.call_contract(
        #     self.contract_id,
        #     "mint_certificate",
        #     [report.contract_id, report, validity_days]
        # )
        
        # Mock implementation
        certificate_id = int(hashlib.sha256(f"{report.contract_id}{time.time()}".encode()).hexdigest()[:8], 16)
        
        print(f"🏆 Certificate minted successfully!")
        print(f"📋 Certificate ID: {certificate_id}")
        print(f"🎯 Contract: {report.contract_id}")
        print(f"⚡ Risk Score: {report.risk_score.value}")
        print(f"📅 Valid for: {validity_days} days")
        print(f"🔗 Report: {self.ipfs_gateway}{report.ipfs_cid}")
        
        return certificate_id
    
    def verify_contract_cleared(self, contract_id: str) -> bool:
        """Check if a contract is security cleared"""
        # is_cleared = self.client.call_contract(self.contract_id, "is_contract_cleared", [contract_id])
        # return is_cleared
        
        # Mock implementation
        print(f"🔍 Verifying contract: {contract_id}")
        print("✅ Contract is security cleared")
        return True
    
    def get_certificate(self, contract_id: str) -> SecurityCertificate:
        """Get certificate details for a contract"""
        # certificate_data = self.client.call_contract(self.contract_id, "get_contract_certificate", [contract_id])
        # return SecurityCertificate(**certificate_data)
        
        # Mock implementation
        return SecurityCertificate(
            certificate_id=12345,
            contract_id=contract_id,
            report=SecurityReport(
                contract_id=contract_id,
                timestamp=int(time.time()) - 86400, # 1 day ago
                risk_score=RiskScore.LOW,
                vulnerabilities_found=0,
                invariants_passed=15,
                invariants_failed=0,
                scan_duration=120,
                scanner_version="1.2.0",
                ipfs_cid="QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG"
            ),
            status=CertificateStatus.ACTIVE,
            issued_at=int(time.time()) - 86400,
            expires_at=int(time.time()) + (30 * 86400 - 86400),
            issued_by="GD123...scanner",
            revoked_at=None,
            revoke_reason=None
        )
    
    def revoke_certificate(self, certificate_id: int, reason: str) -> bool:
        """Revoke a certificate"""
        # success = self.client.call_contract(
        #     self.contract_id,
        #     "revoke_certificate",
        #     [certificate_id, reason],
        #     private_key=admin_private_key
        # )
        # return success
        
        # Mock implementation
        print(f"⚠️ Certificate {certificate_id} revoked")
        print(f"📝 Reason: {reason}")
        return True
    
    def get_certificate_stats(self) -> Tuple[int, int, int, int]:
        """Get certificate statistics"""
        # stats = self.client.call_contract(self.contract_id, "get_certificate_stats", [])
        # return stats
        
        # Mock implementation
        return (150, 120, 20, 10)  # (total, active, revoked, expired)
    
    def _calculate_risk_score(self, scan_results: Dict) -> RiskScore:
        """Calculate risk score from scan results"""
        vulnerabilities = scan_results.get("vulnerabilities", [])
        invariants_failed = scan_results.get("invariants_failed", 0)
        
        critical_vulns = len([v for v in vulnerabilities if v.get("severity") == "Critical"])
        high_vulns = len([v for v in vulnerabilities if v.get("severity") == "High"])
        
        if critical_vulns > 0 or invariants_failed > 5:
            return RiskScore.CRITICAL
        elif high_vulns > 2 or invariants_failed > 2:
            return RiskScore.HIGH
        elif high_vulns > 0 or invariants_failed > 0:
            return RiskScore.MEDIUM
        else:
            return RiskScore.LOW

def scanner_integration_example():
    """Example of scanner integration"""
    print("=== Scanner Integration Example ===")
    
    client = AuditProofOfScanClient("GD123...contract")
    
    # Simulate scan results
    scan_results = {
        "vulnerabilities": [],
        "invariants_passed": 15,
        "invariants_failed": 0,
        "duration": 120,  # 2 minutes
        "detailed_findings": [
            {
                "type": "invariant_check",
                "name": "balance_conservation",
                "status": "passed",
                "details": "All balance transfers properly conserve total supply"
            }
        ],
        "recommendations": [
            "Consider implementing additional access controls",
            "Add comprehensive event logging"
        ]
    }
    
    # Create security report
    contract_id = "GD456...defi_protocol"
    report = client.create_security_report(contract_id, scan_results)
    
    # Mint certificate
    certificate_id = client.mint_certificate(report, validity_days=30)
    
    print(f"✅ Certificate {certificate_id} issued for {contract_id}")

def defi_protocol_verification_example():
    """Example of DeFi protocol verification"""
    print("\n=== DeFi Protocol Verification Example ===")
    
    client = AuditProofOfScanClient("GD123...contract")
    
    # List of DeFi protocols to verify
    protocols = [
        "GD456...defi_protocol_1",
        "GD789...defi_protocol_2",
        "GDABC...defi_protocol_3",
    ]
    
    safe_protocols = []
    unsafe_protocols = []
    
    for protocol in protocols:
        is_safe = client.verify_contract_cleared(protocol)
        
        if is_safe:
            safe_protocols.append(protocol)
            try:
                cert = client.get_certificate(protocol)
                print(f"✅ {protocol[:10]}... - SAFE (Risk: {cert.report.risk_score.value})")
            except:
                print(f"✅ {protocol[:10]}... - SAFE (Certificate details unavailable)")
        else:
            unsafe_protocols.append(protocol)
            print(f"⚠️ {protocol[:10]}... - NOT CLEARED")
    
    print(f"\n📊 Summary: {len(safe_protocols)} safe, {len(unsafe_protocols)} unsafe")

def security_incident_response_example():
    """Example of security incident response"""
    print("\n=== Security Incident Response Example ===")
    
    client = AuditProofOfScanClient("GD123...contract")
    
    # Simulate discovering a vulnerability
    contract_id = "GD456...defi_protocol"
    
    try:
        cert = client.get_certificate(contract_id)
        print(f"🔍 Found active certificate {cert.certificate_id} for {contract_id}")
        
        # Revoke certificate due to vulnerability
        reason = "Critical reentrancy vulnerability discovered in withdrawal function"
        success = client.revoke_certificate(cert.certificate_id, reason)
        
        if success:
            print(f"⚠️ Certificate revoked immediately")
            print(f"📢 Security alert sent to all users")
            print(f"🔄 Protocol marked as unsafe")
            
            # Verify it's now unsafe
            is_safe = client.verify_contract_cleared(contract_id)
            print(f"🔍 Verification result: {'SAFE' if is_safe else 'UNSAFE'}")
            
    except Exception as e:
        print(f"❌ Error during incident response: {e}")

def certificate_monitoring_example():
    """Example of certificate monitoring"""
    print("\n=== Certificate Monitoring Example ===")
    
    client = AuditProofOfScanClient("GD123...contract")
    
    # Get certificate statistics
    total, active, revoked, expired = client.get_certificate_stats()
    
    print(f"📊 Certificate Statistics:")
    print(f"   Total Issued: {total}")
    print(f"   Currently Active: {active}")
    print(f"   Revoked: {revoked}")
    print(f"   Expired: {expired}")
    
    # Calculate health metrics
    if total > 0:
        active_rate = (active / total) * 100
        revocation_rate = (revoked / total) * 100
        
        print(f"\n🏥 Health Metrics:")
        print(f"   Active Rate: {active_rate:.1f}%")
        print(f"   Revocation Rate: {revocation_rate:.1f}%")
        
        if revocation_rate > 10:
            print("⚠️ High revocation rate detected - review scanner accuracy")
        elif active_rate > 80:
            print("✅ Healthy certificate ecosystem")
        else:
            print("📋 Monitor certificate expiration rates")

def batch_certification_example():
    """Example of batch certification for multiple contracts"""
    print("\n=== Batch Certification Example ===")
    
    client = AuditProofOfScanClient("GD123...contract")
    
    # Simulate scanning multiple contracts
    contracts_to_scan = [
        ("GD456...token_contract", {"vulnerabilities": [], "invariants_passed": 10, "invariants_failed": 0, "duration": 60}),
        ("GD789...amm_contract", {"vulnerabilities": [{"severity": "Low"}], "invariants_passed": 12, "invariants_failed": 1, "duration": 90}),
        ("GDABC...lending_contract", {"vulnerabilities": [], "invariants_passed": 18, "invariants_failed": 0, "duration": 150}),
        ("GDEFI...bridge_contract", {"vulnerabilities": [{"severity": "High"}], "invariants_passed": 8, "invariants_failed": 3, "duration": 120}),
    ]
    
    certificates_issued = []
    certificates_failed = []
    
    for contract_id, scan_results in contracts_to_scan:
        try:
            report = client.create_security_report(contract_id, scan_results)
            cert_id = client.mint_certificate(report, validity_days=30)
            certificates_issued.append((contract_id, cert_id, report.risk_score))
            print(f"✅ {contract_id[:10]}... - Certificate {cert_id} issued ({report.risk_score.value})")
        except ValueError as e:
            certificates_failed.append((contract_id, str(e)))
            print(f"❌ {contract_id[:10]}... - Failed: {e}")
    
    print(f"\n📊 Batch Results:")
    print(f"   Successful: {len(certificates_issued)}")
    print(f"   Failed: {len(certificates_failed)}")
    
    # Show risk distribution
    risk_counts = {}
    for _, _, risk in certificates_issued:
        risk_counts[risk.value] = risk_counts.get(risk.value, 0) + 1
    
    print(f"\n🎯 Risk Distribution:")
    for risk, count in risk_counts.items():
        print(f"   {risk}: {count}")

def frontend_integration_example():
    """Example of frontend integration code"""
    print("\n=== Frontend Integration Example ===")
    
    # JavaScript-like code for frontend integration
    frontend_code = '''
// Frontend integration example
class SecurityCertificateManager {
    constructor(contractAddress) {
        this.contract = new SorobanContract(contractAddress);
    }
    
    async checkProtocolSafety(contractAddress) {
        try {
            const isCleared = await this.contract.is_contract_cleared(contractAddress);
            
            if (isCleared) {
                const certificate = await this.contract.get_contract_certificate(contractAddress);
                return {
                    safe: true,
                    certificate: {
                        id: certificate.certificate_id,
                        riskScore: certificate.report.risk_score,
                        expiresAt: certificate.expires_at,
                        reportLink: `https://ipfs.io/ipfs/${certificate.report.ipfs_cid}`
                    }
                };
            } else {
                return { safe: false, reason: "No valid security certificate" };
            }
        } catch (error) {
            return { safe: false, reason: "Failed to verify certificate" };
        }
    }
    
    displaySafetyBadge(container, safetyInfo) {
        if (safetyInfo.safe) {
            container.innerHTML = `
                <div class="safety-badge safe">
                    ✅ Security Cleared
                    <br>
                    Risk: ${safetyInfo.certificate.riskScore}
                    <br>
                    <a href="${safetyInfo.certificate.reportLink}" target="_blank">
                        📋 View Report
                    </a>
                </div>
            `;
        } else {
            container.innerHTML = `
                <div class="safety-badge unsafe">
                    ⚠️ Not Security Cleared
                    <br>
                    ${safetyInfo.reason}
                </div>
            `;
        }
    }
}

// Usage
const securityManager = new SecurityCertificateManager("GD123...contract");

// Before allowing user interaction
async function checkBeforeInteraction(contractAddress) {
    const safety = await securityManager.checkProtocolSafety(contractAddress);
    
    if (!safety.safe) {
        showWarning("This protocol has not been security cleared");
        return false;
    }
    
    securityManager.displaySafetyBadge(
        document.getElementById("safety-badge"),
        safety
    );
    
    return true;
}
'''
    
    print("📱 Frontend Integration Code:")
    print(frontend_code)

def main():
    """Main function demonstrating various usage patterns"""
    print("Decentralized Audit \"Proof of Scan\" Usage Examples")
    print("=" * 60)
    
    # Run examples
    scanner_integration_example()
    defi_protocol_verification_example()
    security_incident_response_example()
    certificate_monitoring_example()
    batch_certification_example()
    frontend_integration_example()
    
    print("\n✅ All examples completed successfully!")
    print("\n📚 For more information, see AUDIT_PROOF_OF_SCAN_DOCUMENTATION.md")

if __name__ == "__main__":
    main()
