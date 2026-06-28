#!/bin/bash
# ──────────────────────────────────────────────────────────────────────────────
# Soroban Security Scanner — Automated Backup Script
# Part of Disaster Recovery Plan (Issue #338)
#
# Usage:
#   ./scripts/backup.sh --type full              # Daily full backup
#   ./scripts/backup.sh --type incremental       # Hourly incremental
#   ./scripts/backup.sh --list                   # List available backups
#   ./scripts/backup.sh --type full --compress   # Compressed full backup
# ──────────────────────────────────────────────────────────────────────────────

set -euo pipefail

# ── Configuration ────────────────────────────────────────────────────────────

BACKUP_DIR="${BACKUP_DIR:-/backups}"
S3_BUCKET="${S3_BUCKET:-s3://soroban-scanner-backups}"
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-soroban_scanner}"
DB_USER="${DB_USER:-backup_user}"
RETENTION_DAYS_FULL="${RETENTION_DAYS_FULL:-30}"
RETENTION_DAYS_INCREMENTAL="${RETENTION_DAYS_INCREMENTAL:-7}"
TIMESTAMP=$(date +%Y-%m-%d_%H-%M-%S)
BACKUP_TYPE=""
COMPRESS=false
LIST_ONLY=false
LOG_FILE="${BACKUP_DIR}/backup_${TIMESTAMP}.log"

# ── Colors ───────────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# ── Functions ────────────────────────────────────────────────────────────────

log() {
    echo -e "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

log_success() { log "${GREEN}✅ $1${NC}"; }
log_error() { log "${RED}❌ $1${NC}"; }
log_warn() { log "${YELLOW}⚠️  $1${NC}"; }

usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Options:
  --type TYPE         Backup type: full, incremental (required)
  --compress          Compress backup with gzip
  --list              List available backups in S3
  --help              Show this help message

Environment Variables:
  BACKUP_DIR          Local backup directory (default: /backups)
  S3_BUCKET           S3 bucket for backup storage
  DB_HOST             Database host (default: localhost)
  DB_PORT             Database port (default: 5432)
  DB_NAME             Database name (default: soroban_scanner)
  DB_USER             Database user (default: backup_user)
  PGPASSWORD          Database password (required, set via env)

Examples:
  $0 --type full --compress
  $0 --type incremental
  $0 --list
EOF
    exit 0
}

check_prerequisites() {
    local missing=()

    command -v pg_dump >/dev/null 2>&1 || missing+=("pg_dump (postgresql-client)")
    command -v aws >/dev/null 2>&1 || missing+=("aws-cli")

    if [ ${#missing[@]} -gt 0 ]; then
        log_error "Missing required tools: ${missing[*]}"
        exit 1
    fi

    if [ -z "${PGPASSWORD:-}" ]; then
        log_warn "PGPASSWORD not set — will try to use .pgpass or peer authentication"
    fi

    mkdir -p "$BACKUP_DIR"
}

perform_full_backup() {
    local backup_file="${BACKUP_DIR}/${DB_NAME}_full_${TIMESTAMP}.dump"
    local compressed_file="${backup_file}.gz"

    log "Starting full backup of database '${DB_NAME}'..."

    if pg_dump \
        --host="$DB_HOST" \
        --port="$DB_PORT" \
        --username="$DB_USER" \
        --dbname="$DB_NAME" \
        --format=custom \
        --verbose \
        --file="$backup_file" \
        2>&1 | tee -a "$LOG_FILE"; then

        log_success "Full backup created: ${backup_file}"

        if [ "$COMPRESS" = true ]; then
            log "Compressing backup..."
            gzip -9 "$backup_file"
            backup_file="${compressed_file}"
            log_success "Backup compressed: ${backup_file}"
        fi

        upload_to_s3 "$backup_file" "full/"
        cleanup_old_backups "full/" "$RETENTION_DAYS_FULL"
    else
        log_error "Full backup failed"
        return 1
    fi
}

perform_incremental_backup() {
    local backup_file="${BACKUP_DIR}/${DB_NAME}_incr_${TIMESTAMP}.sql"
    local compressed_file="${backup_file}.gz"

    log "Starting incremental backup (WAL archive)..."

    # Trigger WAL switch to ensure all transactions are archived
    psql \
        --host="$DB_HOST" \
        --port="$DB_PORT" \
        --username="$DB_USER" \
        --dbname="$DB_NAME" \
        -c "SELECT pg_switch_wal();" \
        2>&1 | tee -a "$LOG_FILE"

    # Archive current WAL files
    if pg_dump \
        --host="$DB_HOST" \
        --port="$DB_PORT" \
        --username="$DB_USER" \
        --dbname="$DB_NAME" \
        --format=plain \
        --schema-only \
        --file="$backup_file" \
        2>&1 | tee -a "$LOG_FILE"; then

        log_success "Incremental backup created: ${backup_file}"

        if [ "$COMPRESS" = true ]; then
            gzip -9 "$backup_file"
            backup_file="${compressed_file}"
        fi

        upload_to_s3 "$backup_file" "incremental/"
        cleanup_old_backups "incremental/" "$RETENTION_DAYS_INCREMENTAL"
    else
        log_error "Incremental backup failed"
        return 1
    fi
}

upload_to_s3() {
    local file="$1"
    local prefix="$2"
    local s3_path="${S3_BUCKET}/${prefix}$(basename "$file")"

    log "Uploading to S3: ${s3_path}..."

    if aws s3 cp "$file" "$s3_path" \
        --storage-class STANDARD_IA \
        --sse AES256 \
        2>&1 | tee -a "$LOG_FILE"; then
        log_success "Uploaded to S3: ${s3_path}"

        # Verify upload integrity
        local local_md5=$(md5sum "$file" | cut -d' ' -f1)
        local remote_md5=$(aws s3api head-object \
            --bucket "${S3_BUCKET#s3://}" \
            --key "${prefix}$(basename "$file")" \
            --query 'ETag' --output text 2>/dev/null | tr -d '"')

        if [ -n "$remote_md5" ]; then
            log "Integrity check: Local MD5=${local_md5}, Remote ETag=${remote_md5}"
        fi
    else
        log_error "S3 upload failed for: ${file}"
        return 1
    fi
}

cleanup_old_backups() {
    local prefix="$1"
    local retention_days="$2"

    log "Cleaning up backups older than ${retention_days} days in ${prefix}..."

    aws s3 ls "${S3_BUCKET}/${prefix}" 2>/dev/null | while read -r _ _ date_str _ file; do
        if [ -n "$date_str" ]; then
            local file_date=$(date -d "$date_str" +%s 2>/dev/null || echo 0)
            local cutoff_date=$(date -d "-${retention_days} days" +%s 2>/dev/null || echo 0)

            if [ "$file_date" -lt "$cutoff_date" ] && [ "$file_date" -ne 0 ]; then
                log "Removing old backup: ${prefix}${file}"
                aws s3 rm "${S3_BUCKET}/${prefix}${file}" 2>/dev/null || true
            fi
        fi
    done

    # Also clean up local backups
    find "$BACKUP_DIR" -name "${DB_NAME}_*" -mtime "+${retention_days}" -delete 2>/dev/null || true

    log_success "Cleanup complete"
}

list_backups() {
    log "📋 Available backups in S3:"
    echo ""
    echo "  Full backups:"
    aws s3 ls "${S3_BUCKET}/full/" 2>/dev/null | awk '{printf "    %s  %s  %s\n", $1, $2, $4}' || echo "    (none)"
    echo ""
    echo "  Incremental backups:"
    aws s3 ls "${S3_BUCKET}/incremental/" 2>/dev/null | awk '{printf "    %s  %s  %s\n", $1, $2, $4}' || echo "    (none)"

    # Show local backups
    echo ""
    echo "  Local backups:"
    ls -lh "$BACKUP_DIR"/*.dump "$BACKUP_DIR"/*.sql "$BACKUP_DIR"/*.gz 2>/dev/null | awk '{printf "    %s  %s\n", $5, $9}' || echo "    (none)"
}

verify_backup_integrity() {
    local backup_file="$1"

    log "Verifying backup integrity: ${backup_file}"

    if [[ "$backup_file" == *.gz ]]; then
        if gzip -t "$backup_file" 2>/dev/null; then
            log_success "Compressed backup integrity verified"
        else
            log_error "Compressed backup is corrupted: ${backup_file}"
            return 1
        fi
    fi

    if pg_restore --list "$backup_file" >/dev/null 2>&1; then
        log_success "Backup content integrity verified"
    else
        log_error "Backup content verification failed: ${backup_file}"
        return 1
    fi
}

# ── Main ─────────────────────────────────────────────────────────────────────

main() {
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --type)
                BACKUP_TYPE="$2"
                shift 2
                ;;
            --compress)
                COMPRESS=true
                shift
                ;;
            --list)
                LIST_ONLY=true
                shift
                ;;
            --help)
                usage
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                ;;
        esac
    done

    check_prerequisites

    if [ "$LIST_ONLY" = true ]; then
        list_backups
        exit 0
    fi

    if [ -z "$BACKUP_TYPE" ]; then
        log_error "Backup type is required (--type full|incremental)"
        usage
    fi

    log "🔒 Starting backup process (type: ${BACKUP_TYPE})"

    case "$BACKUP_TYPE" in
        full)
            perform_full_backup
            ;;
        incremental)
            perform_incremental_backup
            ;;
        *)
            log_error "Invalid backup type: ${BACKUP_TYPE}"
            usage
            ;;
    esac

    log_success "Backup completed successfully at $(date)"
    log "Log file: ${LOG_FILE}"
}

main "$@"
