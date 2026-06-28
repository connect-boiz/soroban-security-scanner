# Disaster Recovery & Business Continuity Plan

> **Document Version:** 1.0  
> **Last Updated:** June 28, 2026  
> **Owner:** DevOps Team  
> **Issue:** #338  

---

## 1. Executive Summary

This document defines the disaster recovery (DR) and business continuity (BC) plan for the Soroban Security Scanner platform. It establishes recovery time objectives (RTO), recovery point objectives (RPO), backup procedures, failover mechanisms, and incident response protocols to ensure platform resilience and data integrity.

### 1.1 Recovery Objectives

| Metric | Target | Definition |
|--------|--------|-----------|
| **RTO (Recovery Time Objective)** | 4 hours | Maximum time to restore service after a disaster |
| **RPO (Recovery Point Objective)** | 1 hour | Maximum acceptable data loss measured in time |
| **Availability Target** | 99.9% | Uptime guarantee (~8.76 hours downtime/year) |
| **Backup Frequency** | Hourly (incremental), Daily (full) | How often backups are created |
| **DR Test Frequency** | Quarterly | How often DR procedures are tested |

---

## 2. System Architecture & Dependencies

### 2.1 Critical Services

| Service | Priority | Dependency | Impact of Failure |
|---------|----------|------------|-------------------|
| **PostgreSQL Database** | Critical | None | All data inaccessible |
| **Redis Cache** | High | None | Rate limiting, session store degraded |
| **Scanner Engine (Rust)** | Critical | PostgreSQL, Stellar RPC | No scan processing |
| **API Server (Node.js)** | Critical | PostgreSQL, Redis | No user access |
| **Frontend (Next.js)** | High | API Server | Users cannot view results |
| **Stellar RPC Node** | High | None | Cannot submit/verify transactions |
| **Notification Service** | Medium | SMTP, Twilio | No alerts delivered |
| **Kubernetes Cluster** | Critical | All services | Entire platform down |

### 2.2 Infrastructure Diagram

```
┌─────────────────────────────────────────────────────┐
│                   PRIMARY REGION (us-east-1)         │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │  K8s     │  │  RDS     │  │  ElastiCache     │  │
│  │  Cluster │  │  Primary │  │  (Redis)         │  │
│  └────┬─────┘  └────┬─────┘  └────────┬─────────┘  │
│       │             │                 │             │
│       └─────────────┼─────────────────┘             │
│                     │                               │
│              ┌──────┴──────┐                        │
│              │  S3 Backups │                        │
│              └──────┬──────┘                        │
└─────────────────────┼───────────────────────────────┘
                      │ Cross-Region Replication
┌─────────────────────┼───────────────────────────────┐
│          SECONDARY REGION (eu-west-1)                │
│              ┌──────┴──────┐                        │
│              │  S3 Backups │                        │
│              └──────┬──────┘                        │
│  ┌──────────┐  ┌────┴─────┐  ┌──────────────────┐  │
│  │  K8s     │  │  RDS     │  │  ElastiCache     │  │
│  │  Cluster │  │  Replica │  │  (Standby)       │  │
│  │ (Standby)│  │          │  │                  │  │
│  └──────────┘  └──────────┘  └──────────────────┘  │
└─────────────────────────────────────────────────────┘
```

---

## 3. Backup Strategy

### 3.1 Backup Schedule

| Backup Type | Frequency | Retention | Storage | Encryption |
|-------------|-----------|-----------|---------|------------|
| **Full Database** | Daily (02:00 UTC) | 30 days | S3 (cross-region) | AES-256 |
| **Incremental DB** | Hourly | 7 days | S3 | AES-256 |
| **WAL Archiving** | Continuous | 7 days | S3 | AES-256 |
| **Configuration** | Daily | 90 days | S3 + Git | AES-256 |
| **Secrets** | On change | Permanent | Vault | AES-256-GCM |
| **Contract WASM** | Daily | 90 days | S3 | AES-256 |

### 3.2 Backup Automation

The backup process is automated via `scripts/backup.sh` and runs as a Kubernetes CronJob:

```bash
# Run daily full backup
./scripts/backup.sh --type full --compress

# Run hourly incremental backup
./scripts/backup.sh --type incremental

# List available backups
./scripts/backup.sh --list
```

### 3.3 Backup Verification

Monthly backup validation via `scripts/backup-test.sh`:

```bash
# Test restore from latest backup
./scripts/backup-test.sh --restore --verify

# Dry-run validation
./scripts/backup-test.sh --validate-only
```

---

## 4. Failover Procedures

### 4.1 Automatic Failover

The platform uses health checks and automatic failover for:

- **Database:** RDS Multi-AZ with automatic failover to standby (< 60 seconds)
- **Redis:** ElastiCache with automatic failover to replica (< 60 seconds)
- **Kubernetes:** Pod auto-restart, node auto-replacement

### 4.2 Manual Failover (Cross-Region)

Triggered when the primary region is completely unavailable.

**Procedure (Run from secondary region):**

```bash
# Step 1: Verify primary region is down
./scripts/failover.sh --check-primary

# Step 2: Promote secondary database
./scripts/failover.sh --promote-database

# Step 3: Scale up secondary K8s cluster
./scripts/failover.sh --scale-up-k8s

# Step 4: Update DNS to point to secondary region
./scripts/failover.sh --update-dns

# Step 5: Verify services are healthy
./scripts/failover.sh --verify-health
```

**Estimated Time:** 15-30 minutes

### 4.3 Failback Procedure

After the primary region is restored:

```bash
# Step 1: Sync data from secondary to primary
./scripts/failover.sh --sync-back

# Step 2: Restore DNS to primary
./scripts/failover.sh --restore-dns

# Step 3: Scale down secondary
./scripts/failover.sh --scale-down-secondary
```

---

## 5. Data Replication

### 5.1 Cross-Region Replication

| Data | Method | Latency | RPO Impact |
|------|--------|---------|------------|
| PostgreSQL | Streaming replication + WAL shipping | <100ms | <1 second |
| S3 Backups | Cross-region replication | <1 hour | <1 hour |
| Redis | AOF persistence + S3 export | <5 minutes | <5 minutes |
| Secrets | HashiCorp Vault replication | <1 minute | <1 minute |

### 5.2 Consistency Guarantees

- Database: Strong consistency within region, eventual consistency across regions
- Backups: Eventually consistent (within 1 hour)
- Cache: Best-effort (cache can be rebuilt from database)

---

## 6. Disaster Recovery Testing

### 6.1 Testing Schedule

| Test Type | Frequency | Duration | Scope |
|-----------|-----------|----------|-------|
| Backup Restore | Monthly | 2 hours | Restore to test environment |
| Failover Drill | Quarterly | 4 hours | Full cross-region failover |
| Tabletop Exercise | Semi-annual | 2 hours | Team walkthrough of scenarios |
| Chaos Engineering | Monthly | 1 hour | Random service termination |

### 6.2 Test Scenarios

1. **Database Corruption:** Restore from backup, verify data integrity
2. **Region Outage:** Execute full cross-region failover
3. **Ransomware Attack:** Restore from clean backups, verify no data loss
4. **Accidental Deletion:** Point-in-time recovery from WAL archives
5. **Dependency Failure:** Test degradation modes when Stellar RPC is unavailable

---

## 7. Incident Response

### 7.1 Severity Levels

| Level | Description | Response Time | Escalation |
|-------|-------------|---------------|------------|
| **SEV1** | Platform completely down | 15 minutes | CTO, DevOps Lead |
| **SEV2** | Critical service degraded | 30 minutes | DevOps Lead |
| **SEV3** | Non-critical service affected | 2 hours | On-call engineer |
| **SEV4** | Minor issue, no user impact | Next business day | Engineering team |

### 7.2 Communication Templates

See `docs/INCIDENT_RESPONSE.md` for detailed communication procedures and templates.

---

## 8. Recovery Runbook

### 8.1 Database Recovery

```bash
# Point-in-time recovery to specific timestamp
pg_restore --host=restore-instance.xxx.rds.amazonaws.com \
  --dbname=soroban_scanner \
  --verbose \
  --clean \
  /backups/soroban_scanner_2026-06-28_02-00-00.dump

# Verify data integrity
psql -h restore-instance.xxx.rds.amazonaws.com -d soroban_scanner \
  -c "SELECT count(*) FROM scans; SELECT count(*) FROM users;"
```

### 8.2 Full Stack Recovery (Estimated: 2-4 hours)

1. **T+0min:** Declare incident, notify stakeholders
2. **T+15min:** Provision infrastructure in secondary region (automated via Terraform)
3. **T+30min:** Restore database from latest backup
4. **T+45min:** Apply WAL logs to reach RPO target
5. **T+60min:** Deploy application containers
6. **T+90min:** Verify service health, run smoke tests
7. **T+120min:** Update DNS, monitor traffic
8. **T+180min:** Full verification, declare recovery complete

---

## 9. Roles & Responsibilities

| Role | Responsibility |
|------|---------------|
| **DevOps Lead** | Owns DR plan, coordinates failover, maintains infrastructure |
| **Security Lead** | Ensures encryption, reviews access controls during recovery |
| **Engineering Manager** | Coordinates team response, communicates with stakeholders |
| **On-Call Engineer** | First responder, executes initial diagnostic and recovery steps |
| **CTO** | Final authority on SEV1 decisions, external communications |

---

## 10. Compliance & Audit

- DR procedures reviewed quarterly
- Backup integrity validated monthly
- Access to backups restricted to authorized personnel (audit logged)
- All recovery actions logged in immutable audit trail
- Annual third-party DR assessment

---

## 11. Appendix

### 11.1 RTO/RPO Justification

- **RTO 4 hours:** Maximum acceptable downtime before business impact exceeds $10,000/hour in lost revenue and reputation damage
- **RPO 1 hour:** Maximum acceptable data loss is 1 hour of scan results and vulnerability reports, which can typically be regenerated

### 11.2 Cost Estimate

| Component | Monthly Cost (Est.) |
|-----------|---------------------|
| Secondary RDS instance | $300 |
| Cross-region data transfer | $150 |
| S3 backup storage (1TB) | $23 |
| Secondary K8s cluster (idle) | $100 |
| **Total** | **~$573/month** |

### 11.3 References

- [AWS Disaster Recovery Whitepaper](https://docs.aws.amazon.com/whitepapers/latest/disaster-recovery-workloads-on-aws/welcome.html)
- [PostgreSQL Backup Documentation](https://www.postgresql.org/docs/current/backup.html)
- [Kubernetes Disaster Recovery](https://kubernetes.io/docs/tasks/administer-cluster/)
- [NIST SP 800-34 (Contingency Planning)](https://csrc.nist.gov/publications/detail/sp/800-34/rev-1/final)

---

*For questions or to report an incident, contact: devops@soroban-security-scanner.com*
