#!/usr/bin/env python3
"""
Scanner Registry Usage Examples

This script demonstrates how to interact with the Scanner Registry contract
for various use cases including CI/CD integration, version management, and security monitoring.
"""

import hashlib
import subprocess
import sys
from pathlib import Path
from typing import Optional, Tuple

class ScannerRegistryClient:
    """Client for interacting with the Scanner Registry contract"""
    
    def __init__(self, contract_id: str, network: str = "testnet"):
        self.contract_id = contract_id
        self.network = network
        # Initialize Soroban SDK client here
        # self.client = SorobanClient(network)
    
    def calculate_wasm_hash(self, wasm_path: str) -> str:
        """Calculate SHA-256 hash of WASM binary"""
        with open(wasm_path, 'rb') as f:
            wasm_bytes = f.read()
        return hashlib.sha256(wasm_bytes).hexdigest()
    
    def verify_current_version(self, wasm_path: str) -> bool:
        """Verify current WASM binary matches latest registered version"""
        current_hash = self.calculate_wasm_hash(wasm_path)
        
        # Call contract to verify
        # result = self.client.call_contract(
        #     self.contract_id,
        #     "verify_latest_wasm",
        #     [current_hash]
        # )
        # return result
        
        # Mock implementation for example
        print(f"Verifying WASM hash: {current_hash}")
        return True
    
    def get_latest_version(self) -> dict:
        """Get latest scanner version information"""
        # result = self.client.call_contract(self.contract_id, "get_latest", [])
        # return result
        
        # Mock implementation
        return {
            "version": "1.2.0",
            "wasm_hash": "abc123...",
            "vulnerability_db_hash": "def456...",
            "status": "Active",
            "registered_at": 1640995200,
            "min_stellar_protocol": 20
        }
    
    def register_new_version(self, version: str, wasm_path: str, 
                          vuln_db_hash: str, changelog: str, 
                          min_protocol: int, admin_private_key: str) -> bool:
        """Register a new scanner version (admin only)"""
        wasm_hash = self.calculate_wasm_hash(wasm_path)
        
        # Call contract to register version
        # result = self.client.call_contract(
        #     self.contract_id,
        #     "register_version",
        #     [version, wasm_hash, vuln_db_hash, changelog, min_protocol],
        #     private_key=admin_private_key
        # )
        # return result
        
        # Mock implementation
        print(f"Registering version {version}")
        print(f"WASM Hash: {wasm_hash}")
        print(f"Vulnerability DB Hash: {vuln_db_hash}")
        print(f"Changelog: {changelog}")
        return True

def ci_cd_integration_example():
    """Example of CI/CD pipeline integration"""
    print("=== CI/CD Integration Example ===")
    
    # Initialize client
    registry = ScannerRegistryClient("GD123...", "testnet")
    
    # Path to current scanner binary
    wasm_path = "target/wasm32-unknown-unknown/release/soroban_security_scanner.wasm"
    
    # Verify current binary is official latest version
    if not registry.verify_current_version(wasm_path):
        print("❌ Scanner binary is not the official latest version!")
        sys.exit(1)
    
    print("✅ Scanner binary verified as official latest version")
    
    # Get latest version info for build metadata
    latest = registry.get_latest_version()
    print(f"📋 Latest version: {latest['version']}")
    print(f"🔧 Minimum Stellar protocol: {latest['min_stellar_protocol']}")

def version_management_example():
    """Example of version management operations"""
    print("\n=== Version Management Example ===")
    
    registry = ScannerRegistryClient("GD123...", "testnet")
    
    # Register new version
    wasm_path = "target/wasm32-unknown-unknown/release/soroban_security_scanner.wasm"
    vuln_db_hash = hashlib.sha256(b"vulnerability_database_content").hexdigest()
    
    success = registry.register_new_version(
        version="1.3.0",
        wasm_path=wasm_path,
        vuln_db_hash=vuln_db_hash,
        changelog="Added support for new vulnerability patterns and improved performance",
        min_protocol=21,
        admin_private_key="private_key_here"
    )
    
    if success:
        print("✅ Version 1.3.0 registered successfully")
    else:
        print("❌ Failed to register version")

def security_monitoring_example():
    """Example of security monitoring and response"""
    print("\n=== Security Monitoring Example ===")
    
    registry = ScannerRegistryClient("GD123...", "testnet")
    
    # Simulate security incident response
    print("🚨 Security vulnerability detected in version 1.1.0")
    
    # Mark version as insecure (admin operation)
    # In real implementation, this would call the contract
    print("⚠️ Marking version 1.1.0 as insecure")
    print("📢 Security advisory sent to all users")
    print("🔄 Latest version automatically updated to 1.2.1")

def build_verification_script():
    """Generate a build verification script for CI/CD"""
    script_content = '''#!/bin/bash
# CI/CD Build Verification Script
# Verifies scanner binary integrity against registry

set -e

CONTRACT_ID="GD123..."
NETWORK="testnet"
WASM_PATH="target/wasm32-unknown-unknown/release/soroban_security_scanner.wasm"

echo "🔍 Verifying scanner binary integrity..."

# Calculate current hash
CURRENT_HASH=$(sha256sum $WASM_PATH | cut -d' ' -f1)
echo "Current WASM hash: $CURRENT_HASH"

# Query registry for latest hash
LATEST_HASH=$(soroban contract call \\
    --id $CONTRACT_ID \\
    --function verify_latest_wasm \\
    --arg $CURRENT_HASH \\
    --network $NETWORK)

if [ "$LATEST_HASH" = "true" ]; then
    echo "✅ Scanner binary verified as official latest version"
    exit 0
else
    echo "❌ Scanner binary is NOT the official latest version"
    echo "Please update to the latest version before proceeding"
    exit 1
fi
'''
    
    with open("verify_scanner.sh", "w") as f:
        f.write(script_content)
    
    # Make script executable
    Path("verify_scanner.sh").chmod(0o755)
    print("✅ Build verification script created: verify_scanner.sh")

def deployment_script():
    """Generate deployment script for new versions"""
    script_content = '''#!/bin/bash
# Scanner Registry Deployment Script
# Deploys and initializes the Scanner Registry contract

set -e

CONTRACT_WASM="target/wasm32-unknown-unknown/release/soroban_security_scanner.wasm"
ADMIN_ADDRESS="GD123..."
NETWORK="testnet"

echo "🚀 Deploying Scanner Registry contract..."

# Build contract
echo "🔨 Building contract..."
cargo build --target wasm32-unknown-unknown --release

# Deploy contract
echo "📦 Deploying contract..."
CONTRACT_ID=$(soroban contract deploy \\
    --wasm $CONTRACT_WASM \\
    --network $NETWORK)

echo "✅ Contract deployed: $CONTRACT_ID"

# Initialize contract
echo "🔧 Initializing contract..."
soroban contract invoke \\
    --id $CONTRACT_ID \\
    --function initialize \\
    --arg $ADMIN_ADDRESS \\
    --network $NETWORK

# Register first version
echo "📋 Registering first version..."
WASM_HASH=$(sha256sum $CONTRACT_WASM | cut -d' ' -f1)
VULN_DB_HASH=$(sha256sum vulnerability_database.json | cut -d' ' -f1)

soroban contract invoke \\
    --id $CONTRACT_ID \\
    --function register_version \\
    --arg "1.0.0" \\
    --arg $WASM_HASH \\
    --arg $VULN_DB_HASH \\
    --arg "Initial release of Soroban Security Scanner" \\
    --arg 20 \\
    --network $NETWORK

echo "✅ Scanner Registry deployed and initialized successfully!"
echo "📝 Contract ID: $CONTRACT_ID"
echo "📝 Admin Address: $ADMIN_ADDRESS"
'''
    
    with open("deploy_registry.sh", "w") as f:
        f.write(script_content)
    
    # Make script executable
    Path("deploy_registry.sh").chmod(0o755)
    print("✅ Deployment script created: deploy_registry.sh")

def main():
    """Main function demonstrating various usage patterns"""
    print("Scanner Registry Usage Examples")
    print("=" * 50)
    
    # Run examples
    ci_cd_integration_example()
    version_management_example()
    security_monitoring_example()
    
    # Generate utility scripts
    print("\n=== Generating Utility Scripts ===")
    build_verification_script()
    deployment_script()
    
    print("\n✅ All examples completed successfully!")
    print("\n📚 For more information, see SCANNER_REGISTRY_DOCUMENTATION.md")

if __name__ == "__main__":
    main()
