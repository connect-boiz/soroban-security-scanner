# Backup and Recovery Testing Guide

This document describes the comprehensive backup and recovery testing framework,
recovery procedures, and schedules for the Soroban Security Scanner platform.

**Issue:** [#347 — Missing Comprehensive Backup and Recovery Testing](https://github.com/connect-boiz/soroban-security-scanner/issues/347)

## Overview

The `src/backup_testing/` module provides:

- **BackupIntegrityVerifier** — SHA-256 checksum validation for all backup artifacts
- **BackupRecoveryTestSuite** — automated backup/recovery verification
- **RecoveryMetrics** — RTO/RPO tracking and performance monitoring
- **ReplicationManager** — cross-region replication health checks
- **RetentionPolicy** — automatic cleanup with tiered retention
- **BackupNotificationService** — success/failure alerting

## Quick Start

```bash
./scripts/backup_recovery_test.sh
```

### Run Rust tests only

```bash
cargo test --lib backup_testing
```

## Recovery Procedures

### 1. Transaction Engine State Recovery

**RTO:** 4 hours | **RPO:** 1 hour

1. Stop the transaction processing engine
2. Locate the latest state backup: `POST /state/export?path=/backups/state-latest.json`
3. Verify checksum:
   ```bash
   sha256sum /backups/state-latest.json
   ```
4. Import state: `POST /state/import?path=/backups/state-latest.json`
5. Verify metrics: `GET /metrics/transactions`
6. Restart the engine and confirm queue stats match pre-failure state

### 2. Wallet Export Recovery

**RTO:** 2 hours | **RPO:** 24 hours

1. Obtain the encrypted wallet export file (`.json`)
2. Verify HMAC integrity: `WalletExport::verify_export_hmac()`
3. Restore with user password: `WalletService::restore_wallet()`
4. Confirm Stellar address matches the original wallet
5. Log recovery in `wallet_export_audit_log`

### 3. Database Recovery

**RTO:** 4 hours | **RPO:** 1 hour

1. Stop application services connected to PostgreSQL
2. Restore from latest `pg_dump` backup:
   ```bash
   pg_restore -d soroban_scanner /backups/db-latest.dump
   ```
3. Run migrations if needed: `sqlx migrate run`
4. Verify row counts against backup metadata
5. Restart application services

### 4. Cross-Region Failover

**RTO:** 1 hour | **RPO:** 15 minutes

1. Confirm primary region is unavailable
2. Promote replica in `us-west-2` or `eu-west-1`
3. Verify replicated backup checksum matches primary
4. Update DNS/load balancer to point to replica region
5. Run recovery test suite against promoted replica

## Test Schedule

| Activity | Frequency | Tool |
|----------|-----------|------|
| Automated backup/recovery suite | Every CI run | `BackupRecoveryTestSuite` |
| Integrity checksum verification | Every CI run | `BackupIntegrityVerifier` |
| Full recovery drill | **Monthly** | Manual + automated |
| Cross-region replication check | Weekly | `ReplicationManager` |
| Retention cleanup audit | Daily | `RetentionPolicy` |

## RTO/RPO Targets

| Component | RTO | RPO |
|-----------|-----|-----|
| Transaction engine state | 4 hours | 1 hour |
| Wallet exports | 2 hours | 24 hours |
| PostgreSQL database | 4 hours | 1 hour |
| Cross-region failover | 1 hour | 15 minutes |

## Backup Formats

| Format | Source | Encryption | Integrity |
|--------|--------|------------|-----------|
| `json_state` | Transaction engine | Optional | SHA-256 |
| `wallet_export` | Wallet service | AES-256-GCM + PBKDF2 | HMAC-SHA256 |
| `database_dump` | PostgreSQL | At-rest encryption | SHA-256 |
| `compressed_archive` | Full system | AES-256 | SHA-256 |

## Retention Policy

| Tier | Retention | Max Count |
|------|-----------|-----------|
| Hourly | 1 day | 24 |
| Daily | 30 days | 30 |
| Weekly | 90 days | 12 |
| Monthly | 1 year | 12 |
| Yearly | 7 years | 7 |

Automatic cleanup is enabled by default.

## Notification Channels

| Event | Channel | Severity |
|-------|---------|----------|
| Backup success | Slack | Info |
| Backup failure | Slack + PagerDuty | Critical |
| Recovery test success | Email | Info |
| Recovery test failure | PagerDuty | Critical |
| Replication lag | Slack | Warning |

## CI/CD Integration

The `backup-recovery` job in `.github/workflows/ci.yml`:

1. Runs `cargo test --lib backup_testing`
2. Runs `./scripts/backup_recovery_test.sh`
3. Uploads report as artifact
4. Blocks merge on failure

## File Reference

| File | Purpose |
|------|---------|
| `src/backup_testing/suite.rs` | Main test suite |
| `src/backup_testing/integrity.rs` | Checksum verification |
| `src/backup_testing/metrics.rs` | RTO/RPO tracking |
| `src/backup_testing/replication.rs` | Cross-region replication |
| `src/backup_testing/retention.rs` | Retention policies |
| `src/backup_testing/notifications.rs` | Alert system |
| `scripts/backup_recovery_test.sh` | CI entry point |
| `docs/BACKUP_RECOVERY_TESTING.md` | This document |

## Training

All operations team members should complete recovery drill training quarterly.
See the recovery procedures above and practice on staging before production drills.
