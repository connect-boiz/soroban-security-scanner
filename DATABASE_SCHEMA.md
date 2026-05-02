# Database Schema Documentation

This document provides comprehensive documentation for the Soroban Security Scanner database schema, including tables, relationships, indexes, and migration procedures.

## Overview

The database schema is designed to support a comprehensive security scanning platform for Soroban smart contracts on the Stellar network. It includes modules for user management, wallet operations, transactions, multi-signature operations, bounty systems, security monitoring, and analytics.

## Database Technology

- **Database**: PostgreSQL 14+
- **ORM**: SQLx with Rust
- **Migration System**: SQLx migrations
- **Connection Pool**: PgPool with configurable limits

## Core Tables

### Users Table

Stores user account information and authentication data.

**Columns:**
- `id` (UUID, Primary Key) - Unique user identifier
- `email` (VARCHAR(255), Unique) - User email address
- `username` (VARCHAR(100), Unique) - Unique username
- `password_hash` (VARCHAR(255)) - Bcrypt password hash
- `stellar_address` (VARCHAR(56), Unique) - Stellar public key
- `role` (ENUM) - User role (admin, security_researcher, developer, auditor, user)
- `status` (ENUM) - Account status (active, inactive, suspended, pending_verification)
- `email_verified` (BOOLEAN) - Email verification status
- `two_factor_enabled` (BOOLEAN) - 2FA status
- `two_factor_secret` (VARCHAR(32)) - 2FA secret key
- `profile` (JSONB) - Additional profile data
- `created_at` (TIMESTAMP) - Account creation time
- `updated_at` (TIMESTAMP) - Last update time
- `last_login_at` (TIMESTAMP) - Last login time
- `login_count` (INTEGER) - Total login attempts
- `reputation_score` (INTEGER) - User reputation (0-100)
- `is_verified` (BOOLEAN) - Identity verification status
- `verification_token` (VARCHAR(255)) - Email verification token
- `password_reset_token` (VARCHAR(255)) - Password reset token
- `password_reset_expires` (TIMESTAMP) - Password reset expiry

**Security Fields:**
- `failed_login_attempts` (INTEGER) - Failed login count
- `last_failed_login_at` (TIMESTAMP) - Last failed login time
- `account_locked_until` (TIMESTAMP) - Account lock expiry
- `security_questions` (JSONB) - Security question answers
- `backup_codes` (JSONB) - 2FA backup codes
- `ip_whitelist` (JSONB) - Allowed IP addresses
- `device_fingerprints` (JSONB) - Trusted device fingerprints
- `risk_score` (INTEGER) - Security risk assessment (0-100)

**Indexes:**
- Primary key on `id`
- Unique indexes on `email`, `username`, `stellar_address`
- Performance indexes on `role`, `status`, `created_at`

### Wallets Table

Stores user wallet information and balances.

**Columns:**
- `id` (UUID, Primary Key) - Unique wallet identifier
- `user_id` (UUID, Foreign Key) - Owner user ID
- `stellar_address` (VARCHAR(56), Unique) - Stellar public key
- `wallet_name` (VARCHAR(100)) - User-defined wallet name
- `description` (TEXT) - Wallet description
- `wallet_type` (VARCHAR(50)) - Wallet type (standard, hardware, multisig)
- `status` (ENUM) - Wallet status (active, inactive, frozen, compromised)
- `balance_lumens` (DECIMAL(19,7)) - XLM balance
- `native_balance` (DECIMAL(19,7)) - Native token balance
- `is_primary` (BOOLEAN) - Primary wallet flag
- `is_verified` (BOOLEAN) - Verification status
- `verification_level` (INTEGER) - Verification level (0-3)
- `metadata` (JSONB) - Additional wallet metadata
- `created_at` (TIMESTAMP) - Creation time
- `updated_at` (TIMESTAMP) - Last update time
- `last_transaction_at` (TIMESTAMP) - Last transaction time
- `transaction_count` (INTEGER) - Total transaction count
- `frozen_reason` (TEXT) - Reason for wallet freeze
- `security_score` (INTEGER) - Security rating (0-100)

**Security Fields:**
- `last_security_scan_at` (TIMESTAMP) - Last security scan
- `security_scan_result` (JSONB) - Scan results
- `suspicious_activity_count` (INTEGER) - Suspicious activity count
- `last_suspicious_activity_at` (TIMESTAMP) - Last suspicious activity
- `transaction_limits` (JSONB) - Transaction limits
- `approved_origins` (JSONB) - Approved transaction origins

**Indexes:**
- Primary key on `id`
- Foreign key index on `user_id`
- Unique index on `stellar_address`
- Performance indexes on `status`, `wallet_type`, `created_at`

### Transactions Table

Stores all transaction records and metadata.

**Columns:**
- `id` (UUID, Primary Key) - Unique transaction identifier
- `transaction_hash` (VARCHAR(64), Unique) - Stellar transaction hash
- `from_wallet_id` (UUID, Foreign Key) - Source wallet
- `to_wallet_id` (UUID, Foreign Key) - Destination wallet
- `user_id` (UUID, Foreign Key) - Initiating user
- `transaction_type` (ENUM) - Transaction type
- `status` (ENUM) - Transaction status
- `amount_lumens` (DECIMAL(19,7)) - XLM amount
- `amount_native` (DECIMAL(19,7)) - Native token amount
- `fee_paid` (DECIMAL(19,7)) - Transaction fee
- `memo` (TEXT) - Transaction memo
- `memo_type` (VARCHAR(20)) - Memo type
- `stellar_ledger_sequence` (BIGINT) - Stellar ledger number
- `stellar_operation_count` (INTEGER) - Operation count
- `envelope` (JSONB) - Full transaction envelope
- `result` (JSONB) - Transaction result
- `error_message` (TEXT) - Error details
- `created_at` (TIMESTAMP) - Creation time
- `updated_at` (TIMESTAMP) - Last update time
- `confirmed_at` (TIMESTAMP) - Confirmation time
- `expires_at` (TIMESTAMP) - Expiry time
- `related_scan_id` (UUID) - Related security scan
- `related_bounty_id` (UUID) - Related bounty
- `batch_transaction_id` (UUID) - Batch operation ID
- `metadata` (JSONB) - Additional metadata

**Security Fields:**
- `risk_level` (VARCHAR(20)) - Risk assessment (low, medium, high, critical)
- `fraud_score` (INTEGER) - Fraud likelihood (0-100)
- `ip_address` (INET) - Source IP address
- `device_fingerprint` (VARCHAR(255)) - Device identifier
- `geolocation` (JSONB) - Geographic data
- `is_suspicious` (BOOLEAN) - Suspicious flag
- `requires_review` (BOOLEAN) - Manual review required
- `reviewed_by` (UUID) - Reviewer user ID
- `reviewed_at` (TIMESTAMP) - Review time
- `review_notes` (TEXT) - Review comments

**Indexes:**
- Primary key on `id`
- Unique index on `transaction_hash`
- Foreign key indexes on `from_wallet_id`, `to_wallet_id`, `user_id`
- Performance indexes on `transaction_type`, `status`, `created_at`

### Multi-Signature Operations

#### multi_signature_operations Table

Stores multi-signature transaction operations.

**Columns:**
- `id` (UUID, Primary Key) - Unique operation ID
- `user_id` (UUID, Foreign Key) - Initiating user
- `operation_name` (VARCHAR(255)) - Operation description
- `description` (TEXT) - Detailed description
- `stellar_address` (VARCHAR(56)) - Multi-sig account address
- `threshold_signers` (INTEGER) - Required signatures
- `total_signers` (INTEGER) - Total signers
- `status` (ENUM) - Operation status
- `transaction_envelope` (JSONB) - Transaction envelope
- `created_at` (TIMESTAMP) - Creation time
- `updated_at` (TIMESTAMP) - Last update time
- `expires_at` (TIMESTAMP) - Expiry time
- `executed_at` (TIMESTAMP) - Execution time
- `executed_transaction_hash` (VARCHAR(64)) - Executed transaction hash
- `metadata` (JSONB) - Additional metadata

#### multi_signature_signers Table

Stores individual signer information for multi-sig operations.

**Columns:**
- `id` (UUID, Primary Key) - Unique signer ID
- `multi_sig_operation_id` (UUID, Foreign Key) - Parent operation
- `signer_address` (VARCHAR(56)) - Signer public key
- `signer_wallet_id` (UUID, Foreign Key) - Signer wallet
- `signer_user_id` (UUID, Foreign Key) - Signer user
- `weight` (INTEGER) - Signature weight
- `status` (ENUM) - Signature status
- `signature_data` (TEXT) - Base64 encoded signature
- `signed_at` (TIMESTAMP) - Signature time
- `created_at` (TIMESTAMP) - Creation time
- `updated_at` (TIMESTAMP) - Last update time
- `comments` (TEXT) - Signer comments

**Indexes:**
- Primary key on `id`
- Foreign key index on `multi_sig_operation_id`
- Unique constraint on (`multi_sig_operation_id`, `signer_address`)

## Security Tables

### Security Alerts Table

Stores security-related alerts and notifications.

**Columns:**
- `id` (UUID, Primary Key) - Unique alert ID
- `user_id` (UUID, Foreign Key) - Affected user
- `wallet_id` (UUID, Foreign Key) - Affected wallet
- `transaction_id` (UUID, Foreign Key) - Related transaction
- `alert_type` (VARCHAR(50)) - Alert type
- `severity` (VARCHAR(20)) - Alert severity
- `title` (VARCHAR(255)) - Alert title
- `description` (TEXT) - Alert description
- `alert_data` (JSONB) - Alert-specific data
- `status` (VARCHAR(20)) - Alert status
- `resolved_by` (UUID, Foreign Key) - Resolver user
- `resolved_at` (TIMESTAMP) - Resolution time
- `resolution_notes` (TEXT) - Resolution details
- `created_at` (TIMESTAMP) - Creation time
- `updated_at` (TIMESTAMP) - Last update time

### Rate Limits Table

Stores rate limiting information for API endpoints.

**Columns:**
- `id` (UUID, Primary Key) - Unique limit ID
- `identifier` (VARCHAR(255)) - Rate limit identifier (IP, user ID)
- `resource_type` (VARCHAR(50)) - Resource type
- `window_start` (TIMESTAMP) - Window start time
- `window_end` (TIMESTAMP) - Window end time
- `request_count` (INTEGER) - Current request count
- `max_allowed` (INTEGER) - Maximum allowed requests
- `is_blocked` (BOOLEAN) - Block status
- `block_expires_at` (TIMESTAMP) - Block expiry
- `metadata` (JSONB) - Additional metadata
- `created_at` (TIMESTAMP) - Creation time

### User Devices Table

Stores device tracking information for security.

**Columns:**
- `id` (UUID, Primary Key) - Unique device ID
- `user_id` (UUID, Foreign Key) - Device owner
- `device_fingerprint` (VARCHAR(255)) - Device fingerprint
- `device_name` (VARCHAR(100)) - Device name
- `device_type` (VARCHAR(50)) - Device type
- `operating_system` (VARCHAR(100)) - OS information
- `browser` (VARCHAR(100)) - Browser information
- `ip_address` (INET) - Last known IP
- `user_agent` (TEXT) - User agent string
- `is_trusted` (BOOLEAN) - Trust status
- `last_seen_at` (TIMESTAMP) - Last activity
- `first_seen_at` (TIMESTAMP) - First activity
- `usage_count` (INTEGER) - Usage count
- `metadata` (JSONB) - Additional metadata

## Bounty System Tables

### Projects Table

Organizes bounties by project.

**Columns:**
- `id` (UUID, Primary Key) - Unique project ID
- `name` (VARCHAR(255)) - Project name
- `description` (TEXT) - Project description
- `repository_url` (VARCHAR(500)) - Repository URL
- `contract_address` (VARCHAR(56)) - Contract address
- `owner_id` (UUID, Foreign Key) - Project owner
- `is_active` (BOOLEAN) - Active status
- `is_public` (BOOLEAN) - Public visibility
- `total_budget` (DECIMAL(19,7)) - Total budget
- `budget_currency` (VARCHAR(10)) - Budget currency
- `metadata` (JSONB) - Additional metadata
- `created_at` (TIMESTAMP) - Creation time
- `updated_at` (TIMESTAMP) - Last update time

### Bounties Table

Stores bounty information and requirements.

**Columns:**
- `id` (UUID, Primary Key) - Unique bounty ID
- `project_id` (UUID, Foreign Key) - Parent project
- `title` (VARCHAR(255)) - Bounty title
- `description` (TEXT) - Bounty description
- `category` (ENUM) - Bounty category
- `severity` (ENUM) - Bounty severity
- `status` (ENUM) - Bounty status
- `reward_amount` (DECIMAL(19,7)) - Reward amount
- `reward_currency` (VARCHAR(10)) - Reward currency
- `max_reward_amount` (DECIMAL(19,7)) - Maximum reward
- `assignee_id` (UUID, Foreign Key) - Assigned user
- `submitter_id` (UUID, Foreign Key) - Submitting user
- `reviewer_id` (UUID, Foreign Key) - Reviewing user
- `deadline` (TIMESTAMP) - Submission deadline
- `requirements` (JSONB) - Bounty requirements
- `submission_guidelines` (TEXT) - Guidelines
- `evaluation_criteria` (JSONB) - Evaluation criteria
- `tags` (JSONB) - Bounty tags
- `view_count` (INTEGER) - View count
- `applicant_count` (INTEGER) - Applicant count
- `submission_count` (INTEGER) - Submission count
- `created_at` (TIMESTAMP) - Creation time
- `updated_at` (TIMESTAMP) - Last update time
- `submitted_at` (TIMESTAMP) - Submission time
- `reviewed_at` (TIMESTAMP) - Review time
- `accepted_at` (TIMESTAMP) - Acceptance time
- `paid_at` (TIMESTAMP) - Payment time
- `metadata` (JSONB) - Additional metadata

### Escrow Accounts Table

Manages escrow for bounty payments.

**Columns:**
- `id` (UUID, Primary Key) - Unique escrow ID
- `bounty_id` (UUID, Foreign Key) - Related bounty
- `funder_id` (UUID, Foreign Key) - Funding user
- `beneficiary_id` (UUID, Foreign Key) - Beneficiary user
- `amount` (DECIMAL(19,7)) - Escrow amount
- `currency` (VARCHAR(10)) - Currency type
- `status` (ENUM) - Escrow status
- `release_conditions` (JSONB) - Release conditions
- `dispute_reason` (TEXT) - Dispute reason
- `dispute_evidence` (JSONB) - Dispute evidence
- `stellar_transaction_hash` (VARCHAR(64)) - Funding transaction
- `release_transaction_hash` (VARCHAR(64)) - Release transaction
- `refund_transaction_hash` (VARCHAR(64)) - Refund transaction
- `created_at` (TIMESTAMP) - Creation time
- `updated_at` (TIMESTAMP) - Last update time
- `funded_at` (TIMESTAMP) - Funding time
- `released_at` (TIMESTAMP) - Release time
- `refunded_at` (TIMESTAMP) - Refund time
- `disputed_at` (TIMESTAMP) - Dispute time
- `resolved_at` (TIMESTAMP) - Resolution time
- `expires_at` (TIMESTAMP) - Expiry time
- `metadata` (JSONB) - Additional metadata

## Analytics Tables

### Analytics Events Table

Stores raw analytics event data.

**Columns:**
- `id` (UUID, Primary Key) - Unique event ID
- `event_type` (ENUM) - Event type
- `user_id` (UUID, Foreign Key) - Event user
- `session_id` (UUID, Foreign Key) - User session
- `wallet_id` (UUID, Foreign Key) - Related wallet
- `transaction_id` (UUID, Foreign Key) - Related transaction
- `bounty_id` (UUID, Foreign Key) - Related bounty
- `project_id` (UUID, Foreign Key) - Related project
- `ip_address` (INET) - Source IP
- `user_agent` (TEXT) - User agent
- `event_data` (JSONB) - Event-specific data
- `timestamp` (TIMESTAMP) - Event timestamp
- `processed` (BOOLEAN) - Processing status
- `created_at` (TIMESTAMP) - Creation time

### Aggregated Metrics Table

Stores pre-aggregated metrics for reporting.

**Columns:**
- `id` (UUID, Primary Key) - Unique metric ID
- `metric_name` (VARCHAR(100)) - Metric name
- `metric_type` (VARCHAR(50)) - Metric type
- `aggregation_period` (ENUM) - Aggregation period
- `period_start` (TIMESTAMP) - Period start
- `period_end` (TIMESTAMP) - Period end
- `value` (DECIMAL(19,4)) - Metric value
- `dimensions` (JSONB) - Filter dimensions
- `metadata` (JSONB) - Additional metadata
- `created_at` (TIMESTAMP) - Creation time
- `updated_at` (TIMESTAMP) - Last update time

## Database Views

### Predefined Views

1. **active_users** - All active, verified users
2. **user_wallet_summary** - User wallet statistics
3. **transaction_summary** - Daily transaction summaries
4. **high_risk_users** - Users with high risk scores
5. **suspicious_transactions** - Flagged transactions
6. **active_security_alerts** - Open security alerts
7. **active_bounties** - Open and assigned bounties
8. **user_bounty_stats** - User bounty statistics
9. **bounty_analytics** - Monthly bounty analytics
10. **platform_overview** - Platform metrics overview

## Stored Procedures

### Security Procedures

1. **handle_failed_login** - Manages failed login attempts and account locking
2. **assess_transaction_risk** - Evaluates transaction risk and creates alerts

### Multi-Signature Procedures

1. **create_bounty_escrow** - Creates escrow for bounty payments
2. **release_escrow_payment** - Releases escrow to beneficiaries

### Analytics Procedures

1. **aggregate_daily_metrics** - Aggregates daily platform metrics
2. **cleanup_analytics_events** - Cleans up old analytics data
3. **generate_daily_report** - Generates daily platform report

## Migration Structure

### Migration Files

1. **001_initial_schema.sql** - Core tables and relationships
2. **002_add_security_features.sql** - Security enhancements and monitoring
3. **003_add_bounty_system.sql** - Bounty marketplace and escrow system
4. **004_add_analytics_and_reporting.sql** - Analytics and reporting features

### Running Migrations

```bash
# Using SQLx CLI
sqlx migrate run --database-url "postgresql://user:password@localhost:5432/soroban_security_scanner"

# Using Rust code
let db = Database::new(config).await?;
db.run_migrations().await?;
```

## Performance Optimization

### Indexing Strategy

- **Primary Keys**: All tables have UUID primary keys
- **Foreign Keys**: All foreign key columns are indexed
- **Query Patterns**: Indexes for common query patterns
- **Time-based**: Timestamp columns for time-range queries
- **Unique Constraints**: Email, username, wallet addresses

### Connection Pooling

- **Default**: 20 max connections, 5 min connections
- **Timeouts**: 30s connect, 10min idle, 30min max lifetime
- **Health Checks**: Periodic connection validation

### Partitioning (Future Enhancement)

Consider partitioning large tables by date:
- `transactions` by month
- `analytics_events` by week
- `security_alerts` by month

## Security Considerations

### Data Encryption

- **Passwords**: Bcrypt hashing with salt
- **2FA**: TOTP secret encryption
- **PII**: Consider field-level encryption for sensitive data

### Access Control

- **Row Level Security**: Implement RLS for user data isolation
- **Database Roles**: Separate roles for application access
- **Audit Logging**: Comprehensive audit trail for all operations

### Backup Strategy

- **Daily Backups**: Automated daily database backups
- **Point-in-Time Recovery**: WAL archiving for PITR
- **Testing**: Regular backup restoration testing

## Monitoring and Maintenance

### Health Checks

- **Connectivity**: Database connection testing
- **Performance**: Query performance monitoring
- **Resource Usage**: Connection pool and resource monitoring

### Maintenance Tasks

- **Vacuum**: Regular table vacuuming
- **Analyze**: Update table statistics
- **Reindex**: Periodic index rebuilding
- **Cleanup**: Old data cleanup procedures

## Troubleshooting

### Common Issues

1. **Connection Timeouts**: Check connection pool settings
2. **Slow Queries**: Review execution plans and indexes
3. **Lock Contention**: Monitor long-running transactions
4. **Disk Space**: Monitor table sizes and cleanup

### Performance Tuning

1. **Query Optimization**: Use EXPLAIN ANALYZE for slow queries
2. **Index Usage**: Monitor index effectiveness
3. **Configuration**: Tune PostgreSQL configuration parameters
4. **Connection Pooling**: Optimize pool size based on load

## Development Guidelines

### Schema Changes

1. **Always Use Migrations**: Never modify schema directly
2. **Backward Compatibility**: Consider existing data
3. **Testing**: Test migrations on staging first
4. **Rollback Plans**: Always have rollback strategies

### Code Integration

1. **Type Safety**: Use Rust structs for all database models
2. **Error Handling**: Comprehensive error handling for database operations
3. **Transactions**: Use database transactions for multi-table operations
4. **Connection Management**: Proper connection lifecycle management

This schema provides a robust foundation for the Soroban Security Scanner platform with comprehensive security, scalability, and analytics capabilities.
