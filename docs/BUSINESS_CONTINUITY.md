# Business Continuity Impact Analysis

> **Document Version:** 1.0  
> **Last Updated:** June 28, 2026  
> **Issue:** #338  

---

## 1. Business Impact Analysis (BIA)

### 1.1 Critical Business Functions

| Function | Priority | Max Tolerable Downtime | Financial Impact/Hour |
|----------|----------|------------------------|----------------------|
| Scan Processing Engine | Critical | 1 hour | $5,000 |
| API / User Access | Critical | 2 hours | $8,000 |
| Vulnerability Database | High | 4 hours | $3,000 |
| Bounty Marketplace | High | 4 hours | $2,500 |
| Notification Service | Medium | 8 hours | $1,000 |
| Analytics Dashboard | Low | 24 hours | $500 |
| Documentation Portal | Low | 48 hours | $200 |

### 1.2 Dependencies

| Service | Upstream Dependencies | Downstream Dependents |
|---------|----------------------|----------------------|
| Scan Engine | Stellar RPC, PostgreSQL | API, Frontend |
| API Server | PostgreSQL, Redis, Scan Engine | Frontend, External API users |
| Database | None | All services |
| Redis Cache | None | API Server, Rate Limiter |

### 1.3 Revenue Impact

```
Scenario: 8-hour complete outage
├── Direct revenue loss: $40,000 - $64,000
├── Bounty marketplace impact: $12,500 - $20,000
├── Reputation damage (estimated): $15,000 - $30,000
└── SLA penalty risk: $5,000 - $15,000

Total estimated impact: $72,500 - $129,000
```

---

## 2. Risk Assessment

### 2.1 Threat Matrix

| Threat | Likelihood | Impact | Risk Level | Mitigation |
|--------|-----------|--------|------------|------------|
| Database corruption | Low | Critical | High | Automated backups, WAL archiving |
| AWS Region outage | Low | Critical | High | Multi-region deployment |
| Ransomware attack | Low | Critical | High | Immutable backups, air-gapped copies |
| DDoS attack | Medium | High | Medium | CloudFront + WAF + rate limiting |
| Accidental deletion | Medium | Medium | Medium | Point-in-time recovery, soft deletes |
| Configuration error | Medium | Medium | Medium | IaC, change management, canary deploys |
| Third-party dependency failure | Medium | Low | Low | Graceful degradation, circuit breakers |
| Natural disaster | Very Low | Critical | High | Multi-region, documented procedures |

### 2.2 Single Points of Failure

| Component | SPOF? | Mitigation |
|-----------|-------|------------|
| Primary RDS instance | Yes (without Multi-AZ) | Multi-AZ deployment |
| Single region | Yes | Cross-region failover |
| S3 bucket (single region) | Yes | Cross-region replication |
| DNS (Route53) | Low risk | AWS managed, multi-region |
| SSL Certificates (ACM) | Low risk | Auto-renewal, multi-region |

---

## 3. Recovery Strategies

### 3.1 Strategy Selection

| Strategy | RTO | RPO | Cost | Chosen? |
|----------|-----|-----|------|---------|
| Backup & Restore | 4-24 hours | 1-24 hours | $ | For non-critical data |
| Pilot Light | 1-4 hours | Minutes | $$ | For core database |
| Warm Standby | <1 hour | Seconds | $$$ | For API/critical services |
| Multi-Site Active/Active | <1 minute | Near-zero | $$$$ | Future consideration |

**Chosen Strategy: Warm Standby** — provides the best balance of RTO/RPO vs. cost for our requirements.

### 3.2 Recovery Priorities

```
Priority 1 (0-2 hours): Database + API Server
Priority 2 (2-4 hours): Scan Engine + Frontend
Priority 3 (4-8 hours): Bounty Marketplace + Notifications
Priority 4 (8-24 hours): Analytics + Documentation
```

---

## 4. Resource Requirements

### 4.1 Personnel

| Role | Count | Required Certifications |
|------|-------|------------------------|
| DevOps Engineer | 2 | AWS Certified, Kubernetes (CKA) |
| Database Administrator | 1 | PostgreSQL Certified |
| Security Engineer | 1 | CISSP or equivalent |
| Engineering Manager | 1 | N/A |

### 4.2 Infrastructure (Secondary Region - Warm Standby)

| Resource | Specification | Cost/Month |
|----------|--------------|------------|
| RDS Instance | db.t3.medium (single-AZ) | $300 |
| EKS Cluster | Minimal nodes (2x t3.medium) | $100 |
| ElastiCache | cache.t3.micro | $30 |
| S3 Storage | 1TB (cross-region replication) | $25 |
| Data Transfer | ~100GB/month cross-region | $20 |
| **Total** | | **~$475/month** |

### 4.3 Tools & Software

- Terraform (IaC for multi-region deployment)
- AWS CLI + SDK
- PostgreSQL client tools
- kubectl + Helm
- Prometheus + Grafana (monitoring)
- PagerDuty (alerting)

---

## 5. Testing & Maintenance

### 5.1 Testing Cadence

| Activity | Frequency | Owner | Success Criteria |
|----------|-----------|-------|-----------------|
| Backup restore test | Monthly | DevOps | Full restore < 2 hours |
| Failover drill | Quarterly | DevOps Lead | RTO < 4 hours, RPO < 1 hour |
| Tabletop exercise | Semi-annual | Engineering Manager | All team members know their roles |
| Plan review | Quarterly | DevOps Lead | Plan updated with lessons learned |
| Security audit | Annual | Security Engineer | No critical findings |

### 5.2 Continuous Improvement

- After each incident: Post-mortem within 48 hours
- After each drill: Lessons learned document within 1 week
- Quarterly: Update BIA with new services/dependencies
- Annually: Full plan revision

---

## 6. Communication Plan

### 6.1 Stakeholder Notification Matrix

| Audience | SEV1 | SEV2 | SEV3 | SEV4 |
|----------|------|------|------|------|
| CTO | Immediate | 15 min | 1 hour | Daily summary |
| Engineering Team | Immediate | 15 min | 1 hour | Daily summary |
| Users (status page) | 15 min | 30 min | 1 hour | N/A |
| Enterprise Customers | 30 min | 1 hour | 4 hours | N/A |
| Public | 1 hour | 2 hours | N/A | N/A |

### 6.2 Communication Channels

- **Internal:** Slack #incidents channel, PagerDuty
- **External Status:** status.soroban-scanner.com
- **Enterprise:** Direct email + Slack Connect
- **Public:** Twitter/X @SorobanScanner

---

## 7. Compliance & Audit

### 7.1 Regulatory Requirements

| Requirement | How We Meet It |
|-------------|---------------|
| Data backup (GDPR Art. 32) | Encrypted daily backups, cross-region replication |
| Business continuity (SOC 2 CC7) | Documented DR plan, quarterly testing |
| Incident response (SOC 2 CC7) | Defined procedures, communication templates |
| Data integrity (SOC 2 CC6) | Backup verification, checksums, audit trail |

### 7.2 Audit Trail

All recovery actions are logged in the immutable event log (`src/event_logging.rs`) with:
- Timestamp
- Actor (who performed the action)
- Action type (failover, restore, etc.)
- Result (success/failure)
- Recovery time achieved

---

## 8. Plan Maintenance

This document and all associated procedures are reviewed and updated:

- **Quarterly:** Full review by DevOps Lead
- **After each incident:** Update based on lessons learned
- **After each infrastructure change:** Update dependencies and architecture
- **Annually:** External review by security auditor

### Document Control

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-06-28 | Emmanuel-Ugochukwu1 | Initial BIA and BC plan (Issue #338) |

---

*For questions about business continuity, contact: devops@soroban-security-scanner.com*
