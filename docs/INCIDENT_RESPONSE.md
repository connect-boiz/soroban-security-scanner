# Incident Response Procedures

> **Document Version:** 1.0  
> **Last Updated:** June 28, 2026  
> **Issue:** #338  

---

## 1. Incident Response Lifecycle

```
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│ DETECT   │───▶│  TRIAGE  │───▶│ CONTAIN  │───▶│ RESOLVE  │───▶│  LEARN   │
└──────────┘    └──────────┘    └──────────┘    └──────────┘    └──────────┘
    5 min           15 min          1-4 hrs         4-24 hrs        48 hrs
```

---

## 2. Detection & Alerting

### 2.1 Monitoring Sources

| Source | What It Detects | Alert Channel |
|--------|----------------|---------------|
| AWS CloudWatch | Service health, CPU/memory spikes | PagerDuty |
| Prometheus/Grafana | Application metrics, error rates | PagerDuty + Slack |
| Uptime checks (Route53) | External availability | PagerDuty |
| Sentry | Application errors, crashes | Slack |
| Database metrics | Connection pool, slow queries | Slack |
| Custom health endpoints | API, scanner, frontend | PagerDuty |

### 2.2 Alert Thresholds

| Metric | Warning | Critical |
|--------|---------|----------|
| API error rate | >1% | >5% |
| API latency (p95) | >500ms | >2000ms |
| Database connections | >80% pool | >95% pool |
| Disk usage | >80% | >90% |
| CPU usage | >70% (sustained) | >90% |
| Memory usage | >80% | >95% |

---

## 3. Triage & Classification

### 3.1 Severity Classification

```yaml
SEV1 - Critical:
  definition: "Complete platform outage or data loss"
  examples:
    - All API endpoints returning 5xx
    - Database unreachable
    - Security breach with data exfiltration
  response_time: "15 minutes"
  escalation: "CTO, DevOps Lead"

SEV2 - High:
  definition: "Critical service degraded, workaround available"
  examples:
    - Scan engine down but API still functional
    - Intermittent 5xx errors (>10% of requests)
    - Bounty marketplace unavailable
  response_time: "30 minutes"
  escalation: "DevOps Lead"

SEV3 - Medium:
  definition: "Non-critical service affected"
  examples:
    - Notification service down
    - Analytics dashboard unavailable
    - Slow performance (not affecting critical path)
  response_time: "2 hours"
  escalation: "On-call engineer"

SEV4 - Low:
  definition: "Minor issue, no user impact"
  examples:
    - Documentation site down
    - Non-critical cron job failure
    - Cosmetic UI issues
  response_time: "Next business day"
  escalation: "Engineering team"
```

---

## 4. Incident Commander Role

The first responder automatically becomes the **Incident Commander** until relieved. Responsibilities:

1. **Declare** the incident severity level
2. **Open** a dedicated Slack channel (#incident-{date}-{brief})
3. **Assign** roles: Communications Lead, Operations Lead, Engineering Lead
4. **Update** the status page within 15 minutes (SEV1/SEV2)
5. **Document** timeline in the incident channel
6. **Escalate** if not resolved within RTO
7. **Declare** incident resolved and initiate post-mortem

---

## 5. Communication Templates

### 5.1 Initial Status Page Update (SEV1/SEV2)

```
Title: Investigating service disruption

We are currently investigating reports of [brief description of issue].
Users may experience [symptoms].

Our engineering team has been engaged and is working to identify the cause.

Next update: [time, usually 30 minutes from now]
```

### 5.2 Status Update (During Incident)

```
Title: [Issue identified / Fix in progress / Monitoring]

[Brief description of what's happening]

Current status: [what's been done, what's in progress]
Estimated resolution: [time or "unknown"]

Next update: [time]
```

### 5.3 Resolution Announcement

```
Title: Service restored

The [issue] has been resolved and all services are operating normally.

Root cause: [brief description]
Duration: [start time] to [end time] ([duration])

A detailed post-mortem will be published within 48 hours.

We apologize for the disruption.
```

### 5.4 Internal Slack Notification

```
🚨 INCIDENT DECLARED: SEV[1-4]

Description: [brief]
Start time: [time]
Impact: [what's affected]
Incident Commander: @username
Channel: #incident-[date]-[brief]

Current action: [what's being done]
Next update: [time]
```

### 5.5 Enterprise Customer Email

```
Subject: [URGENT/INFORMATIONAL] Soroban Security Scanner - Service Update

Dear [Customer Name],

We are writing to inform you of [a service disruption / scheduled maintenance]
affecting the Soroban Security Scanner platform.

WHAT HAPPENED:
[Brief, clear description]

IMPACT TO YOU:
[Specific impact, if any]

WHAT WE'RE DOING:
[Actions taken]

ESTIMATED RESOLUTION:
[Time or "we will update you within X minutes"]

We apologize for any inconvenience. Our team is fully engaged on this issue.

For urgent concerns, reply to this email or contact [phone].

Sincerely,
Soroban Security Scanner Team
```

---

## 6. Post-Incident Process

### 6.1 Post-Mortem Template

Every SEV1 and SEV2 incident must have a written post-mortem within 48 hours.

```markdown
# Incident Post-Mortem: [Title]

**Date:** [date]
**Duration:** [start] - [end] ([duration])
**Severity:** SEV[1-4]
**Incident Commander:** [name]

## Summary
[2-3 sentence summary]

## Timeline (UTC)
| Time | Event |
|------|-------|
| HH:MM | Incident detected by [monitoring/alert/report] |
| HH:MM | Incident Commander engaged |
| HH:MM | Root cause identified |
| HH:MM | Fix deployed |
| HH:MM | Services restored |
| HH:MM | Incident resolved |

## Root Cause
[Detailed technical explanation]

## Impact
- Users affected: [number/percentage]
- Data loss: [yes/no, details]
- Revenue impact: [estimate]
- Services affected: [list]

## Resolution
[What was done to fix the issue]

## Detection
- How was it detected? [monitoring/customer report]
- Time to detect: [minutes]
- Time to resolve: [minutes]
- Could detection have been faster? How?

## Prevention
- [ ] Action item 1 (owner: @username, due: [date])
- [ ] Action item 2 (owner: @username, due: [date])
- [ ] Action item 3 (owner: @username, due: [date])

## Lessons Learned
[What went well, what could be improved]
```

### 6.2 Action Item Tracking

- All action items from post-mortems are tracked as GitHub issues
- Label: `post-mortem-action`
- Reviewed at weekly engineering sync
- Escalated if past due date

---

## 7. Emergency Contacts

### 7.1 Internal Team

| Name | Role | Phone | Email |
|------|------|-------|-------|
| DevOps Lead | Primary on-call | [REDACTED] | devops@soroban-scanner.com |
| Security Lead | Security incidents | [REDACTED] | security@soroban-scanner.com |
| Engineering Manager | Escalation | [REDACTED] | eng-mgr@soroban-scanner.com |
| CTO | SEV1 escalation | [REDACTED] | cto@soroban-scanner.com |

### 7.2 External Contacts

| Service | Support Contact | SLA |
|---------|----------------|-----|
| AWS Support | aws.amazon.com/support | Business (1 hour) |
| Stellar.org | stellar.org/community | Community |
| Twilio (SMS) | twilio.com/console/support | Standard |
| SendGrid (Email) | sendgrid.com/support | Standard |

### 7.3 On-Call Rotation

The on-call rotation is managed in PagerDuty:
- **Primary:** DevOps engineer (weekly rotation)
- **Secondary:** Backend engineer (weekly rotation)
- **Escalation:** DevOps Lead (always)

---

## 8. Incident Runbooks

### 8.1 Database Failover

See: `scripts/failover.sh --promote-database`

### 8.2 Full Region Failover

See: `scripts/failover.sh --full-failover`

### 8.3 Backup Restoration

See: `scripts/backup-test.sh --restore --verify`

### 8.4 Service Restart

```bash
# Restart all services in order
kubectl rollout restart deployment/soroban-api
kubectl rollout restart deployment/soroban-scanner
kubectl rollout restart deployment/soroban-frontend
kubectl rollout restart deployment/soroban-notification

# Verify
kubectl get pods -w
```

---

## 9. Document Control

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-06-28 | Emmanuel-Ugochukwu1 | Initial incident response procedures (Issue #338) |

---

*For emergencies, contact the on-call engineer via PagerDuty or call [REDACTED]*
