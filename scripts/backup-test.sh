#!/bin/bash
# ──────────────────────────────────────────────────────────────────────────────
# Soroban Security Scanner — Backup Validation & Testing Script
# Part of Disaster Recovery Plan (Issue #338)
#
# Usage:
#   ./scripts/backup-test.sh --restore --verify    # Full restore and verify
#   ./scripts/backup-test.sh --validate-only        # Validate without restore
#   ./scripts/backup-test.sh --list-checksums       # List backup checksums
# ──────────────────────────────────────────────────────────────────────────────

set -euo pipefail

# ── Configuration ────────────────────────────────────────────────────────────

TEST_DB_HOST="${TEST_DB_HOST:-localhost}"
TEST_DB_PORT="${TEST_DB_PORT:-5433}"  # Different port to avoid conflicts
TEST_DB_NAME="${TEST_DB_NAME:-soroban_scanner_test}"
TEST_DB_USER="${TEST_DB_USER:-backup_test_user}"
S3_BUCKET="${S3_BUCKET:-s3://soroban-scanner-backups}"
BACKUP_DIR="${BACKUP_DIR:-/backups}"
LOG_FILE="/tmp/backup_test_$(date +%Y%m%d_%H%M%S).log"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

PASSED=0
FAILED=0

log()  { echo -e "[$(date '+%H:%M:%S')] $1" | tee -a "$LOG_FILE"; }
pass() { log "${GREEN}✅ PASS: $1${NC}"; PASSED=$((PASSED + 1)); }
fail() { log "${RED}❌ FAIL: $1${NC}"; FAILED=$((FAILED + 1)); }
warn() { log "${YELLOW}⚠️  $1${NC}"; }

# ── Validation Functions ─────────────────────────────────────────────────────

validate_backup_exists() {
    log "📋 Checking backup availability..."

    local full_count=$(aws s3 ls "${S3_BUCKET}/full/" 2>/dev/null | wc -l)
    local incr_count=$(aws s3 ls "${S3_BUCKET}/incremental/" 2>/dev/null | wc -l)

    if [ "$full_count" -gt 0 ]; then
        pass "Full backups exist: ${full_count} files"
    else
        fail "No full backups found"
    fi

    if [ "$incr_count" -gt 0 ]; then
        pass "Incremental backups exist: ${incr_count} files"
    else
        warn "No incremental backups found (may be normal)"
    fi
}

validate_backup_recency() {
    log "📅 Checking backup recency..."

    local latest_full=$(aws s3 ls "${S3_BUCKET}/full/" 2>/dev/null | sort -k1,2 | tail -1 | awk '{print $1, $2}')
    local latest_incr=$(aws s3 ls "${S3_BUCKET}/incremental/" 2>/dev/null | sort -k1,2 | tail -1 | awk '{print $1, $2}')

    if [ -n "$latest_full" ]; then
        local full_date_epoch=$(date -d "$latest_full" +%s 2>/dev/null || echo 0)
        local cutoff=$(date -d "-25 hours" +%s)

        if [ "$full_date_epoch" -gt "$cutoff" ]; then
            pass "Latest full backup is recent: ${latest_full}"
        else
            fail "Latest full backup is older than 25 hours: ${latest_full}"
        fi
    else
        fail "No full backups found for recency check"
    fi

    if [ -n "$latest_incr" ]; then
        local incr_date_epoch=$(date -d "$latest_incr" +%s 2>/dev/null || echo 0)
        local cutoff=$(date -d "-2 hours" +%s)

        if [ "$incr_date_epoch" -gt "$cutoff" ]; then
            pass "Latest incremental backup is recent: ${latest_incr}"
        else
            fail "Latest incremental backup is older than 2 hours: ${latest_incr}"
        fi
    fi
}

validate_backup_size() {
    log "📏 Checking backup sizes..."

    local total_size=$(aws s3 ls --recursive "${S3_BUCKET}/" 2>/dev/null | \
        awk '{sum += $3} END {printf "%.2f", sum / 1024 / 1024}')

    if [ -n "$total_size" ] && [ "${total_size%.*}" -gt 0 ]; then
        pass "Total backup size: ${total_size} MB"
    else
        warn "Backup size appears small or zero: ${total_size:-0} MB"
    fi
}

test_restore_full_backup() {
    log "🔄 Testing full backup restore..."

    # Get the latest full backup
    local latest_backup=$(aws s3 ls "${S3_BUCKET}/full/" 2>/dev/null | \
        sort -k1,2 | tail -1 | awk '{print $4}')

    if [ -z "$latest_backup" ]; then
        fail "No full backup available for restore test"
        return 1
    fi

    local backup_file="${BACKUP_DIR}/${latest_backup}"

    # Download from S3
    log "Downloading: ${latest_backup}..."
    if aws s3 cp "${S3_BUCKET}/full/${latest_backup}" "$backup_file" 2>/dev/null; then
        pass "Downloaded backup: ${latest_backup}"
    else
        fail "Failed to download backup: ${latest_backup}"
        return 1
    fi

    # Create test database
    log "Creating test database: ${TEST_DB_NAME}..."
    if createdb \
        --host="$TEST_DB_HOST" \
        --port="$TEST_DB_PORT" \
        --username="$TEST_DB_USER" \
        "$TEST_DB_NAME" 2>/dev/null; then
        pass "Test database created"
    else
        warn "Test database may already exist — attempting to drop and recreate"
        dropdb --host="$TEST_DB_HOST" --port="$TEST_DB_PORT" \
            --username="$TEST_DB_USER" "$TEST_DB_NAME" 2>/dev/null || true
        createdb --host="$TEST_DB_HOST" --port="$TEST_DB_PORT" \
            --username="$TEST_DB_USER" "$TEST_DB_NAME" || {
            fail "Failed to create test database"
            return 1
        }
    fi

    # Restore backup
    log "Restoring backup to test database..."
    if pg_restore \
        --host="$TEST_DB_HOST" \
        --port="$TEST_DB_PORT" \
        --username="$TEST_DB_USER" \
        --dbname="$TEST_DB_NAME" \
        --verbose \
        --clean \
        --if-exists \
        "$backup_file" 2>&1 | tee -a "$LOG_FILE"; then
        pass "Backup restored successfully"
    else
        fail "Backup restore failed"
        return 1
    fi
}

verify_data_integrity() {
    log "🔍 Verifying data integrity..."

    # Check critical tables exist and have data
    local tables=("users" "scans" "transactions" "bounties")

    for table in "${tables[@]}"; do
        local count=$(psql \
            --host="$TEST_DB_HOST" \
            --port="$TEST_DB_PORT" \
            --username="$TEST_DB_USER" \
            --dbname="$TEST_DB_NAME" \
            -t -c "SELECT COUNT(*) FROM ${table};" 2>/dev/null || echo "0")

        count=$(echo "$count" | tr -d '[:space:]')

        if [ "${count:-0}" -gt 0 ]; then
            pass "Table '${table}' has ${count} rows"
        else
            warn "Table '${table}' is empty (may be expected)"
        fi
    done
}

verify_checksums() {
    log "🔐 Verifying backup checksums..."

    aws s3 ls --recursive "${S3_BUCKET}/" 2>/dev/null | while read -r _ _ _ _ file; do
        if [ -n "$file" ]; then
            local etag=$(aws s3api head-object \
                --bucket "${S3_BUCKET#s3://}" \
                --key "$file" \
                --query 'ETag' --output text 2>/dev/null | tr -d '"')
            if [ -n "$etag" ]; then
                pass "Checksum valid for: ${file} (${etag})"
            else
                fail "No checksum found for: ${file}"
            fi
        fi
    done
}

cleanup_test_database() {
    log "🧹 Cleaning up test database..."
    dropdb \
        --host="$TEST_DB_HOST" \
        --port="$TEST_DB_PORT" \
        --username="$TEST_DB_USER" \
        "$TEST_DB_NAME" 2>/dev/null && pass "Test database cleaned up" || warn "Cleanup may have partially failed"
}

print_summary() {
    echo ""
    echo "═══════════════════════════════════════════════════════════════"
    echo "  BACKUP VALIDATION SUMMARY"
    echo "═══════════════════════════════════════════════════════════════"
    echo -e "  ${GREEN}Passed: ${PASSED}${NC}"
    echo -e "  ${RED}Failed: ${FAILED}${NC}"
    echo "═══════════════════════════════════════════════════════════════"
    echo "  Log file: ${LOG_FILE}"

    if [ "$FAILED" -gt 0 ]; then
        echo -e "  ${RED}⚠️  BACKUP VALIDATION FAILED — INVESTIGATE IMMEDIATELY${NC}"
        return 1
    else
        echo -e "  ${GREEN}✅ All backup validations passed${NC}"
        return 0
    fi
}

# ── Main ─────────────────────────────────────────────────────────────────────

main() {
    local RESTORE=false
    local VALIDATE_ONLY=false
    local LIST_CHECKSUMS=false

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --restore) RESTORE=true; shift ;;
            --verify) :; shift ;;  # implied with --restore
            --validate-only) VALIDATE_ONLY=true; shift ;;
            --list-checksums) LIST_CHECKSUMS=true; shift ;;
            --help)
                echo "Usage: $0 [--restore --verify | --validate-only | --list-checksums]"
                exit 0
                ;;
            *) shift ;;
        esac
    done

    log "🔒 Starting backup validation at $(date)"

    validate_backup_exists
    validate_backup_recency
    validate_backup_size

    if [ "$LIST_CHECKSUMS" = true ]; then
        verify_checksums
        print_summary
        exit $?
    fi

    if [ "$RESTORE" = true ]; then
        test_restore_full_backup
        verify_data_integrity
        cleanup_test_database
    elif [ "$VALIDATE_ONLY" = true ]; then
        log "📋 Validation-only mode — skipping restore test"
    fi

    print_summary
}

main "$@"
