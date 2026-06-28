#!/bin/bash
# ──────────────────────────────────────────────────────────────────────────────
# Soroban Security Scanner — Cross-Region Failover Script
# Part of Disaster Recovery Plan (Issue #338)
#
# Usage:
#   ./scripts/failover.sh --check-primary      # Verify primary region status
#   ./scripts/failover.sh --promote-database   # Promote secondary DB to primary
#   ./scripts/failover.sh --scale-up-k8s       # Scale up secondary K8s cluster
#   ./scripts/failover.sh --update-dns          # Switch DNS to secondary
#   ./scripts/failover.sh --verify-health       # Verify services are healthy
#   ./scripts/failover.sh --sync-back           # Sync data back to primary
#   ./scripts/failover.sh --restore-dns         # Restore DNS to primary
#   ./scripts/failover.sh --full-failover       # Execute complete failover
# ──────────────────────────────────────────────────────────────────────────────

set -euo pipefail

# ── Configuration ────────────────────────────────────────────────────────────

PRIMARY_REGION="${PRIMARY_REGION:-us-east-1}"
SECONDARY_REGION="${SECONDARY_REGION:-eu-west-1}"
PRIMARY_DB_ID="${PRIMARY_DB_ID:-soroban-scanner-primary}"
SECONDARY_DB_ID="${SECONDARY_DB_ID:-soroban-scanner-secondary}"
K8S_CLUSTER_PRIMARY="${K8S_CLUSTER_PRIMARY:-soroban-scanner-primary}"
K8S_CLUSTER_SECONDARY="${K8S_CLUSTER_SECONDARY:-soroban-scanner-secondary}"
DNS_ZONE_ID="${DNS_ZONE_ID:-Z1234567890ABC}"
DOMAIN_NAME="${DOMAIN_NAME:-api.soroban-scanner.com}"
HEALTH_CHECK_URL="${HEALTH_CHECK_URL:-https://api.soroban-scanner.com/health}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log()   { echo -e "[$(date '+%H:%M:%S')] $1"; }
ok()    { echo -e "${GREEN}✅ $1${NC}"; }
err()   { echo -e "${RED}❌ $1${NC}"; }
warn()  { echo -e "${YELLOW}⚠️  $1${NC}"; }
info()  { echo -e "${BLUE}ℹ️  $1${NC}"; }

# ── Health Check ─────────────────────────────────────────────────────────────

check_primary_health() {
    info "Checking primary region health (${PRIMARY_REGION})..."

    # Check if primary API is responding
    if curl -s -o /dev/null -w "%{http_code}" --max-time 10 "$HEALTH_CHECK_URL" | grep -q "200"; then
        ok "Primary API is healthy"
        return 0
    else
        err "Primary API is not responding"
    fi

    # Check RDS status
    local db_status=$(aws rds describe-db-instances \
        --db-instance-identifier "$PRIMARY_DB_ID" \
        --region "$PRIMARY_REGION" \
        --query 'DBInstances[0].DBInstanceStatus' \
        --output text 2>/dev/null || echo "unknown")

    if [ "$db_status" = "available" ]; then
        ok "Primary database status: ${db_status}"
    else
        err "Primary database status: ${db_status}"
    fi

    # Check if failover is needed
    if [ "$db_status" != "available" ]; then
        warn "Primary region appears degraded — failover may be needed"
        return 1
    fi

    return 0
}

# ── Database Failover ────────────────────────────────────────────────────────

promote_secondary_database() {
    info "Promoting secondary database in ${SECONDARY_REGION}..."

    # Check if secondary is a read replica
    local is_replica=$(aws rds describe-db-instances \
        --db-instance-identifier "$SECONDARY_DB_ID" \
        --region "$SECONDARY_REGION" \
        --query 'DBInstances[0].ReadReplicaSourceDBInstanceIdentifier' \
        --output text 2>/dev/null || echo "")

    if [ -n "$is_replica" ] && [ "$is_replica" != "None" ]; then
        info "Promoting read replica to standalone..."
        aws rds promote-read-replica \
            --db-instance-identifier "$SECONDARY_DB_ID" \
            --region "$SECONDARY_REGION" 2>&1

        # Wait for promotion to complete
        info "Waiting for database promotion (this may take 5-10 minutes)..."
        aws rds wait db-instance-available \
            --db-instance-identifier "$SECONDARY_DB_ID" \
            --region "$SECONDARY_REGION"

        ok "Secondary database promoted to primary"
    else
        ok "Secondary database is already standalone"
    fi

    # Get the new primary endpoint
    local endpoint=$(aws rds describe-db-instances \
        --db-instance-identifier "$SECONDARY_DB_ID" \
        --region "$SECONDARY_REGION" \
        --query 'DBInstances[0].Endpoint.Address' \
        --output text)

    info "New primary database endpoint: ${endpoint}"
    echo "$endpoint"
}

# ── Kubernetes Scaling ───────────────────────────────────────────────────────

scale_up_secondary_k8s() {
    info "Scaling up secondary Kubernetes cluster in ${SECONDARY_REGION}..."

    # Update kubeconfig to secondary cluster
    aws eks update-kubeconfig \
        --region "$SECONDARY_REGION" \
        --name "$K8S_CLUSTER_SECONDARY" 2>/dev/null

    # Scale up critical deployments
    local deployments=(
        "soroban-api"
        "soroban-scanner"
        "soroban-frontend"
        "soroban-notification"
    )

    for deployment in "${deployments[@]}"; do
        info "Scaling ${deployment} to production replicas..."
        kubectl scale deployment "$deployment" --replicas=3 2>/dev/null || \
            warn "Could not scale ${deployment} (may not exist yet)"
    done

    # Wait for pods to be ready
    info "Waiting for pods to be ready..."
    kubectl wait --for=condition=ready pod \
        -l app.kubernetes.io/part-of=soroban-scanner \
        --timeout=300s 2>/dev/null || warn "Some pods may not be ready"

    ok "Secondary K8s cluster scaled up"
}

# ── DNS Management ───────────────────────────────────────────────────────────

update_dns_to_secondary() {
    info "Updating DNS to point to secondary region..."

    # Get secondary load balancer endpoint
    local secondary_lb=$(aws elbv2 describe-load-balancers \
        --region "$SECONDARY_REGION" \
        --names "soroban-scanner-secondary" \
        --query 'LoadBalancers[0].DNSName' \
        --output text 2>/dev/null)

    if [ -z "$secondary_lb" ] || [ "$secondary_lb" = "None" ]; then
        err "Could not find secondary load balancer"
        return 1
    fi

    # Update Route53 record
    aws route53 change-resource-record-sets \
        --hosted-zone-id "$DNS_ZONE_ID" \
        --change-batch '{
            "Changes": [{
                "Action": "UPSERT",
                "ResourceRecordSet": {
                    "Name": "'"$DOMAIN_NAME"'",
                    "Type": "CNAME",
                    "TTL": 60,
                    "ResourceRecords": [{"Value": "'"$secondary_lb"'"}]
                }
            }]
        }' 2>/dev/null

    ok "DNS updated — propagating (TTL: 60s)"
    info "New endpoint: ${secondary_lb}"
}

restore_dns_to_primary() {
    info "Restoring DNS to primary region..."

    local primary_lb=$(aws elbv2 describe-load-balancers \
        --region "$PRIMARY_REGION" \
        --names "soroban-scanner-primary" \
        --query 'LoadBalancers[0].DNSName' \
        --output text 2>/dev/null)

    if [ -z "$primary_lb" ] || [ "$primary_lb" = "None" ]; then
        err "Could not find primary load balancer"
        return 1
    fi

    aws route53 change-resource-record-sets \
        --hosted-zone-id "$DNS_ZONE_ID" \
        --change-batch '{
            "Changes": [{
                "Action": "UPSERT",
                "ResourceRecordSet": {
                    "Name": "'"$DOMAIN_NAME"'",
                    "Type": "CNAME",
                    "TTL": 300,
                    "ResourceRecords": [{"Value": "'"$primary_lb"'"}]
                }
            }]
        }' 2>/dev/null

    ok "DNS restored to primary region"
}

# ── Health Verification ──────────────────────────────────────────────────────

verify_service_health() {
    info "Verifying service health..."

    local max_retries=12
    local retry_delay=10

    for i in $(seq 1 $max_retries); do
        local status=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 "$HEALTH_CHECK_URL" 2>/dev/null || echo "000")

        if [ "$status" = "200" ]; then
            ok "Services are healthy (HTTP ${status})"
            return 0
        fi

        warn "Attempt ${i}/${max_retries}: HTTP ${status} — retrying in ${retry_delay}s..."
        sleep "$retry_delay"
    done

    err "Services failed health check after ${max_retries} attempts"
    return 1
}

# ── Data Synchronization ─────────────────────────────────────────────────────

sync_data_back_to_primary() {
    info "Syncing data from secondary back to primary..."

    # Create a fresh backup from secondary
    local backup_file="failback_$(date +%Y%m%d_%H%M%S).dump"

    info "Creating backup from secondary database..."
    pg_dump \
        --host="$(aws rds describe-db-instances \
            --db-instance-identifier "$SECONDARY_DB_ID" \
            --region "$SECONDARY_REGION" \
            --query 'DBInstances[0].Endpoint.Address' --output text)" \
        --username="backup_user" \
        --dbname="soroban_scanner" \
        --format=custom \
        --file="/tmp/${backup_file}" 2>/dev/null || {
        err "Failed to create backup from secondary"
        return 1
    }

    # Restore to primary
    info "Restoring to primary database..."
    pg_restore \
        --host="$(aws rds describe-db-instances \
            --db-instance-identifier "$PRIMARY_DB_ID" \
            --region "$PRIMARY_REGION" \
            --query 'DBInstances[0].Endpoint.Address' --output text)" \
        --username="backup_user" \
        --dbname="soroban_scanner" \
        --clean \
        --if-exists \
        "/tmp/${backup_file}" 2>/dev/null || {
        err "Failed to restore to primary"
        return 1
    }

    ok "Data synced back to primary"
}

# ── Scale Down ───────────────────────────────────────────────────────────────

scale_down_secondary() {
    info "Scaling down secondary region..."

    aws eks update-kubeconfig \
        --region "$SECONDARY_REGION" \
        --name "$K8S_CLUSTER_SECONDARY" 2>/dev/null

    kubectl scale deployment --all --replicas=0 2>/dev/null || true
    ok "Secondary K8s cluster scaled down"
}

# ── Full Failover ────────────────────────────────────────────────────────────

execute_full_failover() {
    echo ""
    echo "═══════════════════════════════════════════════════════════"
    echo "  🚨 EXECUTING CROSS-REGION FAILOVER"
    echo "  From: ${PRIMARY_REGION} → To: ${SECONDARY_REGION}"
    echo "  Time: $(date)"
    echo "═══════════════════════════════════════════════════════════"
    echo ""

    warn "This will redirect all traffic to the secondary region."
    read -p "Are you sure you want to continue? (yes/no): " confirm
    if [ "$confirm" != "yes" ]; then
        info "Failover cancelled"
        exit 0
    fi

    # Step 1: Verify primary is down
    info "Step 1/5: Checking primary health..."
    if check_primary_health; then
        warn "Primary appears healthy — are you sure you need to failover?"
        read -p "Continue anyway? (yes/no): " force
        [ "$force" != "yes" ] && { info "Aborted."; exit 0; }
    fi

    # Step 2: Promote secondary database
    info "Step 2/5: Promoting secondary database..."
    promote_secondary_database

    # Step 3: Scale up secondary K8s
    info "Step 3/5: Scaling up secondary Kubernetes..."
    scale_up_secondary_k8s

    # Step 4: Update DNS
    info "Step 4/5: Updating DNS..."
    update_dns_to_secondary

    # Step 5: Verify health
    info "Step 5/5: Verifying service health..."
    sleep 30  # Allow DNS propagation
    verify_service_health

    echo ""
    ok "✅ FAILOVER COMPLETE — Services running in ${SECONDARY_REGION}"
    echo ""
}

# ── Main ─────────────────────────────────────────────────────────────────────

main() {
    case "${1:-}" in
        --check-primary)
            check_primary_health
            ;;
        --promote-database)
            promote_secondary_database
            ;;
        --scale-up-k8s)
            scale_up_secondary_k8s
            ;;
        --update-dns)
            update_dns_to_secondary
            ;;
        --verify-health)
            verify_service_health
            ;;
        --sync-back)
            sync_data_back_to_primary
            ;;
        --restore-dns)
            restore_dns_to_primary
            ;;
        --scale-down-secondary)
            scale_down_secondary
            ;;
        --full-failover)
            execute_full_failover
            ;;
        *)
            cat << EOF
Usage: $0 COMMAND

Commands:
  --check-primary        Verify primary region health
  --promote-database     Promote secondary database to primary
  --scale-up-k8s         Scale up secondary Kubernetes cluster
  --update-dns           Switch DNS to secondary region
  --verify-health        Verify services are healthy
  --sync-back            Sync data back to primary region
  --restore-dns          Restore DNS to primary region
  --scale-down-secondary Scale down secondary region
  --full-failover        Execute complete failover sequence

Environment:
  PRIMARY_REGION         Primary AWS region (default: us-east-1)
  SECONDARY_REGION       Secondary AWS region (default: eu-west-1)
EOF
            exit 1
            ;;
    esac
}

main "$@"
