# Kubernetes Isolated Scanner Configuration

This directory contains Kubernetes manifests for deploying the Stellar Security Scanner with complete tenant isolation.

## Architecture Overview

The scanner uses isolated Kubernetes namespaces for each scan to prevent cross-tenant data leakage:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Kubernetes Cluster                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │  scan-abc123    │  │  scan-def456    │  │  scan-ghi789    │ │
│  │  Namespace      │  │  Namespace      │  │  Namespace      │ │
│  │                 │  │                 │  │                 │ │
│  │ ┌─────────────┐ │  │ ┌─────────────┐ │  │ ┌─────────────┐ │ │
│  │ │Scanner Pod  │ │  │ │Scanner Pod  │ │  │ │Scanner Pod  │ │ │ │
│  │ │+ Log Sidecar│ │  │ │+ Log Sidecar│ │  │ │+ Log Sidecar│ │ │ │
│  │ └─────────────┘ │  │ └─────────────┘ │  │ └─────────────┘ │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │           stellar-security-scanner Namespace              │ │
│  │                                                             │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │ │
│  │  │   API Pods  │  │   Cleanup   │  │  Auto-scaler &      │  │ │
│  │  │             │  │   CronJob   │  │  Resource Quotas    │  │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘  │ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Security Features

### 1. **Complete Isolation**
- Each scan runs in its own namespace
- ResourceQuotas prevent resource exhaustion
- NetworkPolicies block all egress traffic
- Encrypted ephemeral volumes for data-at-rest

### 2. **Resource Management**
- CPU/RAM limits per scan (configurable)
- Maximum concurrent scans enforcement
- Automatic cleanup of stuck/old scans
- Horizontal pod autoscaling for API

### 3. **Log Streaming**
- Sidecar containers for real-time log collection
- Fluent-bit integration for log aggregation
- Secure log transmission to main API

### 4. **Auto-scaling**
- HPA for API pods based on CPU/memory
- Configurable scan concurrency limits
- Load-based scaling decisions

## Deployment

### Prerequisites
- Kubernetes 1.28+
- kubectl configured
- Sufficient cluster resources

### Installation

1. **Deploy Infrastructure:**
```bash
kubectl apply -f 00-namespace-rbac.yaml
kubectl apply -f 01-security-policies.yaml
```

2. **Deploy Application:**
```bash
kubectl apply -f 02-api-deployment.yaml
kubectl apply -f 03-cleanup-autoscaling.yaml
```

3. **Configure Secrets:**
```bash
# Update secrets in 04-secrets-config.yaml with actual values
kubectl apply -f 04-secrets-config.yaml
```

### Configuration

Edit `scanner-config` ConfigMap to customize:
- Default resource limits
- Cleanup intervals
- Network policy settings
- Auto-scaling thresholds

## Usage

### CLI Commands

```bash
# Run isolated scan
stellar-scanner k8s-scan /path/to/contract.wasm \
  --cpu-limit 500m \
  --memory-limit 1Gi \
  --timeout 300

# List active scans
stellar-scanner k8s-manage list

# Cleanup old scans
stellar-scanner k8s-manage cleanup --age-minutes 15

# Check system status
stellar-scanner k8s-manage status
```

### API Integration

```rust
use stellar_security_scanner::kubernetes::{K8sScanManager, ScanPodConfig};

let config = ScanPodConfig {
    cpu_limit: "500m".to_string(),
    memory_limit: "1Gi".to_string(),
    timeout: Duration::from_secs(300),
    ..Default::default()
};

let manager = K8sScanManager::new(config).await?;
let result = manager.execute_scan(&scan_id, &scanner_config, &contract_code).await?;
```

## Monitoring

### Key Metrics
- Active scan count
- Resource utilization per scan
- Scan completion rates
- Cleanup job success rates

### Alerts
- High resource utilization
- Stuck scans (>30 minutes)
- Failed cleanup operations
- Network policy violations

## Security Considerations

1. **Network Isolation**: All egress traffic blocked by default
2. **Resource Limits**: Strict quotas prevent DoS attacks
3. **Encryption**: Ephemeral volumes use in-memory storage
4. **Cleanup**: Automatic removal of all scan artifacts
5. **RBAC**: Minimal permissions for scanner service account

## Troubleshooting

### Common Issues

1. **Scan pods stuck in Pending**:
   - Check resource quotas
   - Verify node availability
   - Review ResourceQuota limits

2. **NetworkPolicy blocking legitimate traffic**:
   - Review egress rules
   - Check namespace selectors
   - Verify required ports

3. **Cleanup not working**:
   - Check CronJob status
   - Verify RBAC permissions
   - Review service account tokens

### Debug Commands

```bash
# Check scan namespaces
kubectl get namespaces -l app=stellar-security-scanner,managed-by=scanner

# Inspect specific scan pod
kubectl describe pod scanner-<scan-id> -n scan-<scan-id>

# Check resource quotas
kubectl describe resourcequota scan-quota -n scan-<scan-id>

# View network policies
kubectl get networkpolicy -n scan-<scan-id>

# Check cleanup job
kubectl get cronjob scanner-cleanup -n stellar-security-scanner
kubectl logs job/<cleanup-job-name> -n stellar-security-scanner
```

## Performance Tuning

### Resource Allocation
- Adjust CPU/memory limits based on contract complexity
- Monitor resource utilization patterns
- Consider node sizing for concurrent scans

### Scaling Strategies
- Increase HPA max replicas during peak load
- Adjust cleanup intervals based on scan volume
- Optimize scanner container image size

### Cost Optimization
- Use spot instances for non-critical scans
- Implement scan prioritization
- Consider regional cluster distribution
