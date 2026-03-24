# Scanner Registry & Versioning Contract

## Overview

The Scanner Registry contract serves as the authoritative source of truth for "Certified" scanner versions and vulnerability database hashes in the Soroban Security Scanner ecosystem. It provides integrity checking, version management, and security controls for scanner deployments.

## Features

### 🔐 **Security & Integrity**
- **SHA-256 Hash Verification**: Stores cryptographic hashes of scanner WASM binaries
- **Vulnerability Database Hashing**: Maintains integrity of vulnerability databases
- **Admin-Only Controls**: Only authorized addresses can manage versions
- **Version Status Tracking**: Active, Deprecated, Insecure, and Beta status

### 📋 **Version Management**
- **Semantic Versioning**: Validates version format (x.y.z)
- **Version History**: Complete audit trail of all registered versions
- **Automatic Latest Tracking**: Maintains pointer to latest active version
- **Deprecation System**: Mark old versions as deprecated or insecure

### 🔔 **Event System**
- **Registration Events**: Emitted when new versions are registered
- **Deprecation Events**: Emitted when versions are deprecated
- **Security Events**: Emitted when versions are marked insecure

### 🛠 **CI/CD Integration**
- **get_latest()**: For CI/CD tools to verify official versions
- **verify_latest_wasm()**: Quick hash verification
- **get_active_versions()**: List all currently supported versions

## Contract Architecture

### Data Structures

#### `ScannerVersion`
```rust
pub struct ScannerVersion {
    pub version: String,                    // Semantic version (e.g., "1.2.3")
    pub wasm_hash: BytesN<32>,              // SHA-256 hash of WASM binary
    pub vulnerability_db_hash: BytesN<32>,  // SHA-256 hash of vulnerability DB
    pub status: VersionStatus,              // Active, Deprecated, Insecure, Beta
    pub registered_at: u64,                 // Registration timestamp
    pub registered_by: Address,             // Admin who registered
    pub changelog: String,                  // Version changelog
    pub min_stellar_protocol: u64,          // Minimum Stellar protocol version
}
```

#### `VersionStatus`
```rust
pub enum VersionStatus {
    Active,     // Current supported version
    Deprecated, // Old but not dangerous
    Insecure,   // Has security vulnerabilities
    Beta,       // Testing version
}
```

### Storage Layout

| Key | Type | Description |
|-----|------|-------------|
| `ADMIN` | `Address` | Contract administrator |
| `VERSION_COUNTER` | `u64` | Total versions registered |
| `VERSIONS` | `Map<String, ScannerVersion>` | All version data |
| `LATEST_VERSION` | `String` | Latest active version |

## API Reference

### Admin Functions

#### `initialize(admin: Address)`
Initializes the contract with an administrator address.

**Requirements:**
- Contract must not be already initialized

**Events:** None

#### `register_version(version, wasm_hash, vulnerability_db_hash, changelog, min_stellar_protocol)`
Registers a new scanner version.

**Requirements:**
- Caller must be admin
- Version format must be valid (x.y.z)
- Version must not already exist

**Events:**
- `VERSION_REGISTERED`: version, admin, wasm_hash, vulnerability_db_hash, timestamp

#### `deprecate_version(version, reason)`
Deprecates a version (marks it as outdated but not dangerous).

**Requirements:**
- Caller must be admin
- Version must exist
- Cannot deprecate the latest version

**Events:**
- `VERSION_DEPRECATED`: version, admin, reason, timestamp

#### `mark_insecure(version, security_issue)`
Marks a version as having security vulnerabilities.

**Requirements:**
- Caller must be admin
- Version must exist

**Side Effects:**
- Updates latest version if marking current latest as insecure

**Events:**
- `VERSION_DEPRECATED`: version, admin, "SECURITY: {issue}", timestamp

#### `update_vulnerability_db(version, new_db_hash)`
Updates the vulnerability database hash for a specific version.

**Requirements:**
- Caller must be admin
- Version must exist

#### `transfer_admin(new_admin)`
Transfers administrative rights to a new address.

**Requirements:**
- Caller must be current admin

### Query Functions

#### `get_latest() -> ScannerVersion`
Returns the latest active scanner version.

**Panics:** If no versions exist

#### `get_version(version: String) -> ScannerVersion`
Returns a specific version by version string.

**Panics:** If version doesn't exist

#### `get_all_versions() -> Map<String, ScannerVersion>`
Returns all registered versions (admin use).

#### `get_active_versions() -> Vec<String>`
Returns list of all active version strings.

#### `get_registry_stats() -> (u64, u64, u64, u64)`
Returns registry statistics:
- Total versions
- Active versions  
- Deprecated versions
- Insecure versions

#### `verify_latest_wasm(wasm_hash: BytesN<32>) -> bool`
Verifies if a hash matches the latest version.

#### `verify_version_wasm(version: String, wasm_hash: BytesN<32>) -> bool`
Verifies if a hash matches a specific version.

#### `get_latest_vulnerability_db_hash() -> BytesN<32>`
Returns the vulnerability database hash of the latest version.

## Usage Examples

### CI/CD Integration

```rust
// Verify current binary matches latest registered version
let current_hash = calculate_wasm_hash();
let is_valid = scanner_registry::verify_latest_wasm(env, current_hash);

if !is_valid {
    panic!("Scanner binary is not the official latest version!");
}
```

### Version Registration

```rust
// Register new version as admin
scanner_registry::register_version(
    env,
    String::from_str(&env, "1.2.0"),
    wasm_hash,
    vuln_db_hash,
    String::from_str(&env, "Added new vulnerability patterns"),
    20, // Minimum Stellar protocol
);
```

### Security Response

```rust
// Mark version as insecure if vulnerability found
scanner_registry::mark_insecure(
    env,
    String::from_str(&env, "1.1.0"),
    String::from_str(&env, "Critical buffer overflow in contract analysis"),
);
```

## Security Considerations

### **Admin Security**
- Admin address controls all version management
- Consider using multi-sig for admin address
- Regular admin key rotation recommended

### **Hash Security**
- SHA-256 provides strong cryptographic guarantees
- Hashes stored immutably on-chain
- No way to tamper with version integrity

### **Version Lifecycle**
1. **Beta**: Testing phase, limited deployment
2. **Active**: Production-ready, fully supported
3. **Deprecated**: Outdated but safe to use
4. **Insecure**: Has known vulnerabilities, should not be used

### **Access Control**
- All state-changing functions require admin authorization
- Query functions are publicly accessible
- Events provide transparency for all changes

## Deployment

### **Prerequisites**
- Soroban CLI installed
- Admin address determined
- Initial version hash calculated

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
  --arg <ADMIN_ADDRESS>
```

4. **Register First Version**
```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --function register_version \
  --arg "1.0.0" \
  --arg <WASM_HASH> \
  --arg <VULN_DB_HASH> \
  --arg "Initial release" \
  --arg 20
```

## Testing

The contract includes comprehensive tests covering:

- ✅ Initialization and admin controls
- ✅ Version registration and validation
- ✅ Hash verification functionality
- ✅ Deprecation and security marking
- ✅ Event emission
- ✅ Authorization checks
- ✅ Edge cases and error conditions

Run tests:
```bash
cargo test scanner_registry_tests
```

## Integration Points

### **Scanner Binary Integration**
- Calculate SHA-256 hash of WASM binary
- Verify against registry before execution
- Check for deprecation/insecure status

### **CI/CD Pipeline Integration**
- Verify binary integrity in build pipeline
- Check for newer versions
- Alert on deprecated/insecure versions

### **Frontend Integration**
- Display version status to users
- Show upgrade recommendations
- Provide security warnings

## Monitoring

### **Event Monitoring**
Monitor for these critical events:
- `VERSION_REGISTERED`: New version available
- `VERSION_DEPRECATED`: Version deprecated
- `VERSION_DEPRECATED` with "SECURITY": Version compromised

### **Health Checks**
- Registry should always have a latest version
- No version should be both Active and Insecure
- Version counter should increment properly

## Future Enhancements

### **Proposed Features**
- **Multi-Admin Support**: Multiple authorized addresses
- **Automatic Updates**: Auto-deprecate old versions
- **Version Dependencies**: Track compatibility between versions
- **Audit Log**: Enhanced audit trail with IP tracking
- **Rate Limiting**: Prevent spam version registrations

### **Integration Opportunities**
- **Package Managers**: Integration with cargo/npm registries
- **Security Scanners**: Automated vulnerability scanning
- **Marketplace**: Bounty integration for version testing

## Troubleshooting

### **Common Issues**

**"NotAuthorized" Error**
- Verify caller is the admin address
- Check if admin rights were transferred

**"VersionNotFound" Error**
- Verify version string format (x.y.z)
- Check if version was properly registered

**"CannotDeprecateLatest" Error**
- Register a newer version first
- Then deprecate the old version

### **Debug Commands**

```bash
# Check contract admin
soroban contract read --id <CONTRACT_ID> --key ADMIN

# Check latest version
soroban contract read --id <CONTRACT_ID> --key LATEST_VERSION

# Get version count
soroban contract read --id <CONTRACT_ID> --key VERSION_COUNTER
```

## License

This contract is part of the Soroban Security Scanner project and is licensed under the MIT License.
