# Webhook Notifications & Escrow Contract Implementation

This document describes the implementation of two major features for the Soroban Security Scanner:

## Issue #22: Webhook Notification & Slack Integration

### Features Implemented

#### 1. Notification Settings UI
- **Location**: `frontend/app/webhooks/page.tsx`
- **Features**:
  - Create, read, update, delete webhooks
  - Support for Slack, Discord, and custom webhooks
  - Severity filtering (all, critical, high, medium, low)
  - Enable/disable webhooks
  - Test webhook functionality
  - Real-time statistics dashboard

#### 2. Backend Webhook System
- **Location**: `backend/src/webhook/`
- **Components**:
  - `WebhookService`: Core webhook management logic
  - `WebhookController`: REST API endpoints
  - `NotificationProcessor`: Queue-based webhook delivery
  - `Webhook Entity`: Database schema
  - `NotificationQueue Entity`: Delivery tracking

#### 3. Key Features
- **De-duplication**: Prevents spam for recurring bugs using content hashing
- **Emergency Alerts**: Immediate notifications for critical vulnerabilities
- **Test Webhook**: Built-in testing functionality
- **Signed Payloads**: HMAC-SHA256 signature verification
- **Retry Logic**: Exponential backoff for failed deliveries
- **Statistics**: Comprehensive delivery tracking and analytics

#### 4. Integration Points
- **Scan Processor**: Automatically triggers notifications when scans complete
- **Message Formatting**: Platform-specific formatting for Slack/Discord
- **Deep Linking**: "View Full Report" buttons linking back to the platform

### API Endpoints

```typescript
// Webhook Management
GET    /api/webhooks              // List all webhooks
POST   /api/webhooks              // Create new webhook
GET    /api/webhooks/:id          // Get specific webhook
PUT    /api/webhooks/:id          // Update webhook
DELETE /api/webhooks/:id          // Delete webhook
POST   /api/webhooks/:id/test     // Test webhook

// Statistics
GET    /api/webhooks/stats/overview // Get notification statistics
```

### Webhook Payload Examples

#### Slack Payload
```json
{
  "text": "🚨 Security Scan Alert - 3 vulnerabilities detected",
  "attachments": [{
    "color": "danger",
    "fields": [
      { "title": "Scan ID", "value": "uuid", "short": true },
      { "title": "Risk Score", "value": "85", "short": true },
      { "title": "Critical", "value": "1", "short": true },
      { "title": "High", "value": "2", "short": true }
    ],
    "actions": [{
      "type": "button",
      "text": "View Full Report",
      "url": "https://platform.com/scans/uuid"
    }]
  }]
}
```

#### Discord Payload
```json
{
  "content": "🚨 Security Scan Alert - 3 vulnerabilities detected",
  "embeds": [{
    "title": "Security Scan Results",
    "color": 0xFF0000,
    "fields": [
      { "name": "Scan ID", "value": "uuid", "inline": true },
      { "name": "Risk Score", "value": "85", "inline": true }
    ],
    "url": "https://platform.com/scans/uuid"
  }]
}
```

---

## Issue #19: Escrow & Reward Payout Contract

### Features Implemented

#### 1. Enhanced Smart Contract
- **Location**: `contracts/src/lib.rs`
- **New Structures**:
  - `EscrowEntry`: Bounty payment escrow management
  - `EmergencyAlert`: Critical vulnerability tracking
  - Enhanced `ContractError` types

#### 2. Escrow Functions
```rust
// Create escrow for bounty payment
pub fn create_escrow(
    env: Env,
    depositor: Address,
    beneficiary: Address,
    amount: i128,
    purpose: String,
    lock_duration: u64,
) -> Result<u64, ContractError>

// Release funds to beneficiary
pub fn release_escrow(
    env: Env,
    escrow_id: u64,
    depositor: Address,
    signature: Option<BytesN<32>>,
) -> Result<(), ContractError>

// Refund funds to depositor
pub fn refund_escrow(
    env: Env,
    escrow_id: u64,
    depositor: Address,
) -> Result<(), ContractError>

// Mark escrow conditions as met
pub fn mark_escrow_conditions_met(
    env: Env,
    escrow_id: u64,
    admin: Address,
) -> Result<(), ContractError>
```

#### 3. Emergency Alert System
```rust
// Report emergency vulnerability
pub fn report_emergency_vulnerability(
    env: Env,
    reporter: Address,
    contract_id: BytesN<32>,
    vulnerability_type: String,
    severity: String,
    description: String,
    location: String,
) -> Result<u64, ContractError>

// Verify and trigger immediate reward
pub fn verify_emergency_vulnerability(
    env: Env,
    admin: Address,
    alert_id: u64,
    verified: bool,
) -> Result<(), ContractError>
```

#### 4. Key Features
- **Escrow Locking**: Time-locked funds with condition-based release
- **Emergency Rewards**: Immediate payouts for critical vulnerabilities
- **Reputation Integration**: Automatic reputation updates
- **Fund Management**: Separate pools for regular bounties and emergencies
- **Audit Trail**: Complete transaction history and status tracking

#### 5. Reward Structure
- **Emergency Vulnerabilities**: 10 XLM immediate reward
- **Critical Vulnerabilities**: 5 XLM immediate reward
- **Regular Bounties**: Variable amounts based on severity

### Contract State Management

#### Storage Keys
```rust
const ESCROW: Symbol = Symbol::short("ESCROW");
const EMERGENCY_POOL: Symbol = Symbol::short("EMERG");
```

#### Escrow Status Flow
1. `pending` → Initial state
2. `locked` → Funds deposited and locked
3. `released` → Funds transferred to beneficiary
4. `refunded` → Funds returned to depositor

#### Emergency Alert Flow
1. `pending` → Initial report
2. `verified` → Confirmed by admin, reward triggered
3. `false_positive` → Dismissed as false positive

### Integration Benefits

#### 1. Security Researcher Incentives
- Immediate rewards for critical findings
- Transparent escrow process
- Reputation building system

#### 2. Platform Trust
- Secure fund management
- Clear dispute resolution
- Audit-ready transaction history

#### 3. Operational Efficiency
- Automated reward distribution
- Emergency response capabilities
- Reduced administrative overhead

---

## Database Schema Changes

### Webhook Tables
```sql
-- Webhook configurations
CREATE TABLE webhooks (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    url TEXT NOT NULL,
    type VARCHAR(20) NOT NULL,
    config JSONB,
    severity_filter VARCHAR(20) NOT NULL,
    enabled BOOLEAN DEFAULT true,
    success_count INTEGER DEFAULT 0,
    failure_count INTEGER DEFAULT 0,
    last_error TEXT,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Notification delivery tracking
CREATE TABLE notification_queue (
    id UUID PRIMARY KEY,
    webhook_id UUID NOT NULL,
    scan_id UUID,
    user_id UUID,
    status VARCHAR(20) NOT NULL,
    payload TEXT,
    error_message TEXT,
    attempts INTEGER DEFAULT 0,
    next_attempt_at TIMESTAMP,
    sent_at TIMESTAMP,
    deduplication_key TEXT,
    created_at TIMESTAMP DEFAULT NOW()
);
```

---

## Configuration Requirements

### Environment Variables
```bash
# Backend
BASE_URL=https://your-platform.com
REDIS_URL=redis://localhost:6379
JWT_SECRET=your-secret-key

# Frontend
NEXT_PUBLIC_API_URL=http://localhost:3001
```

### Dependencies Added
```json
// Backend package.json additions
{
  "axios": "^1.6.2",
  "crypto": "built-in"
}
```

---

## Testing Strategy

### 1. Webhook Testing
- Unit tests for webhook formatting
- Integration tests for delivery system
- End-to-end tests with real webhook endpoints

### 2. Contract Testing
- Unit tests for all contract functions
- Integration tests for escrow flows
- Simulation of emergency scenarios

### 3. Security Testing
- Signature verification tests
- Authorization boundary tests
- Input validation tests

---

## Deployment Considerations

### 1. Backend
- Ensure BullMQ workers are running
- Configure Redis for queue management
- Set up database migrations

### 2. Frontend
- Configure API routes
- Set up authentication middleware
- Test webhook connectivity

### 3. Smart Contract
- Deploy to testnet first
- Verify all functions work as expected
- Test with realistic amounts

---

## Monitoring & Maintenance

### 1. Webhook Health
- Monitor delivery success rates
- Track failed notification patterns
- Alert on webhook failures

### 2. Contract Monitoring
- Monitor escrow pool balances
- Track emergency alert response times
- Audit reward distributions

### 3. Performance Metrics
- Webhook delivery latency
- Contract execution times
- Queue processing rates

---

## Future Enhancements

### 1. Webhook Improvements
- Support for more platforms (Teams, etc.)
- Custom message templates
- Advanced scheduling options

### 2. Contract Enhancements
- Multi-signature requirements
- Slashing conditions
- Governance integration

### 3. Platform Features
- Webhook event subscriptions
- Advanced filtering options
- Real-time delivery dashboard

---

## Security Considerations

### 1. Webhook Security
- HMAC signature verification
- Rate limiting per webhook
- Payload size limits

### 2. Contract Security
- Access control validation
- Reentrancy protection
- Overflow checks

### 3. Data Protection
- Sensitive data encryption
- Audit logging
- Secure key management

This implementation provides a robust foundation for security notifications and bounty management, with room for future enhancements based on user feedback and platform growth.
