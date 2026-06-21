#!/usr/bin/env bash
# Backup and Recovery Test Suite — CI entry point (issue #347)
#
# Runs the BackupRecoveryTestSuite from src/backup_testing/suite.rs,
# captures output, and writes a Markdown report to target/backup-recovery-report.md.
#
# Exit codes:
#   0 — all backup/recovery checks passed
#   1 — one or more checks failed
#   2 — script error

set -euo pipefail

REPORT_PATH="${REPORT_PATH:-target/backup-recovery-report.md}"
TEST_FILTER="${TEST_FILTER:-backup_testing}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "▶ Running Backup and Recovery Test Suite…"
echo "  - root:   $ROOT_DIR"
echo "  - filter: $TEST_FILTER"
echo "  - report: $REPORT_PATH"

if ! command -v cargo >/dev/null 2>&1; then
  echo "❌ cargo not found in PATH" >&2
  exit 2
fi

mkdir -p "$(dirname "$REPORT_PATH")"
cd "$ROOT_DIR"

set +e
OUTPUT=$(cargo test --lib "$TEST_FILTER" --locked -- --nocapture 2>&1)
STATUS=$?
set -e

PASS_LINE=$(printf '%s' "$OUTPUT" | grep -E '^test result:' | head -1 || true)

if [ "$STATUS" -eq 0 ]; then
  RESULT_LINE="✅ PASS"
else
  RESULT_LINE="❌ FAIL"
fi

cat >"$REPORT_PATH" <<EOF
# Backup and Recovery Test Report

**Generated:** $(date -u "+%Y-%m-%dT%H:%M:%SZ")
**Commit:**  ${GITHUB_SHA:-local}
**Result:**  $RESULT_LINE

## Summary

| Metric | Value |
|--------|-------|
| Exit status | \`$STATUS\` |
| Cargo summary | \`${PASS_LINE:-<not captured>}\` |

## What the suite checks

- Backup integrity verification (SHA-256 checksum)
- Recovery roundtrip testing
- Tampered backup detection
- Encryption on sensitive backups
- RTO/RPO target compliance
- Cross-region replication health
- Retention policy with auto-cleanup
- Notification system for success/failure
- All backup format coverage
- Performance within thresholds
- Monthly recovery drill schedule

## How to reproduce

\`\`\`bash
./scripts/backup_recovery_test.sh
\`\`\`

## References

- Issue: <https://github.com/connect-boiz/soroban-security-scanner/issues/347>
- Module: \`src/backup_testing/\`
- Documentation: \`docs/BACKUP_RECOVERY_TESTING.md\`
EOF

echo "$OUTPUT" | tail -25

if [ "$STATUS" -ne 0 ]; then
  echo "❌ Backup and Recovery Test Suite FAILED"
  exit 1
fi

echo "✅ Backup and Recovery Test Suite PASSED"
echo "  - report written to: $REPORT_PATH"
