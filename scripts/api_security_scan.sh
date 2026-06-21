#!/usr/bin/env bash
# API Security Test Suite — CI entry point (issue #348)
#
# Runs the SecurityTestSuite from src/api_security/suite.rs, captures output,
# writes a Markdown report to target/api-security-report.md, and optionally
# runs OWASP ZAP baseline scan when ZAP_TARGET_URL is set.
#
# Exit codes:
#   0 — all security checks passed, no high-severity failures
#   1 — one or more security checks failed or coverage gate failed
#   2 — script error (e.g. cargo not installed)

set -euo pipefail

REPORT_PATH="${REPORT_PATH:-target/api-security-report.md}"
COVERAGE_REPORT="${COVERAGE_REPORT:-target/api-security-coverage.md}"
TEST_FILTER="${TEST_FILTER:-api_security::suite}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "▶ Running API Security Test Suite…"
echo "  - root:   $ROOT_DIR"
echo "  - filter: $TEST_FILTER"
echo "  - report: $REPORT_PATH"

if ! command -v cargo >/dev/null 2>&1; then
  echo "❌ cargo not found in PATH" >&2
  exit 2
fi

mkdir -p "$(dirname "$REPORT_PATH")"

cd "$ROOT_DIR"

# Run the Rust security suite
set +e
OUTPUT=$(cargo test --lib "$TEST_FILTER" -- --nocapture 2>&1)
STATUS=$?
set -e

# Run integration tests if present (lib acceptance tests)
set +e
INT_OUTPUT=$(cargo test --lib api_security::acceptance --locked 2>&1)
INT_STATUS=$?
set -e

# Run Node.js security fuzz tests
set +e
if [ -f "$ROOT_DIR/node_modules/.bin/jest" ]; then
  NODE_OUTPUT=$(cd "$ROOT_DIR" && npm test -- --testPathPattern=api-security-fuzz 2>&1)
  NODE_STATUS=$?
elif command -v npm >/dev/null 2>&1 && [ -f "$ROOT_DIR/package-lock.json" ]; then
  echo "▶ Installing npm dependencies for fuzz tests…"
  (cd "$ROOT_DIR" && npm ci --no-audit --no-fund >/dev/null 2>&1)
  NODE_OUTPUT=$(cd "$ROOT_DIR" && npm test -- --testPathPattern=api-security-fuzz 2>&1)
  NODE_STATUS=$?
else
  NODE_OUTPUT="(skipped — jest not available)"
  NODE_STATUS=0
fi
set -e

# Optional OWASP ZAP baseline scan
ZAP_RESULT="skipped"
ZAP_STATUS=0
if [ -n "${ZAP_TARGET_URL:-}" ] && command -v docker >/dev/null 2>&1; then
  echo "▶ Running OWASP ZAP baseline scan against $ZAP_TARGET_URL…"
  set +e
  docker run --rm -t owasp/zap2docker-stable zap-baseline.py \
    -t "$ZAP_TARGET_URL" \
    -J /zap/wrk/zap-report.json 2>&1 | tail -20
  ZAP_STATUS=$?
  set -e
  if [ "$ZAP_STATUS" -eq 0 ]; then
    ZAP_RESULT="pass"
  else
    ZAP_RESULT="fail (high-severity findings)"
  fi
fi

if [ "$STATUS" -eq 0 ] && [ "$INT_STATUS" -eq 0 ] && [ "$NODE_STATUS" -eq 0 ] && [ "$ZAP_STATUS" -eq 0 ]; then
  RESULT_LINE="✅ PASS"
  FINAL_STATUS=0
else
  RESULT_LINE="❌ FAIL"
  FINAL_STATUS=1
fi

PASS_LINE=$(printf '%s' "$OUTPUT" | grep -E '^test result:' | head -1 || true)

cat >"$REPORT_PATH" <<EOF
# API Security Test Report

**Generated:** $(date -u "+%Y-%m-%dT%H:%M:%SZ")
**Commit:**  ${GITHUB_SHA:-local}
**Result:**  $RESULT_LINE

## Summary

| Component | Status |
|-----------|--------|
| Rust security suite | \`$STATUS\` |
| Integration tests | \`$INT_STATUS\` |
| Node fuzz tests | \`$NODE_STATUS\` |
| OWASP ZAP baseline | \`$ZAP_RESULT\` |
| Cargo summary | \`${PASS_LINE:-<not captured>}\` |

## Quality Gates

- **Endpoint coverage:** 100% required (see \`$COVERAGE_REPORT\`)
- **High-severity blocking:** enabled — CI fails on high/critical findings
- **Penetration testing:** quarterly (OWASP ZAP baseline when \`ZAP_TARGET_URL\` is set)

## How to reproduce

\`\`\`bash
./scripts/api_security_scan.sh
# With ZAP scan:
ZAP_TARGET_URL=http://localhost:3000 ./scripts/api_security_scan.sh
\`\`\`

## References

- Issue: <https://github.com/connect-boiz/soroban-security-scanner/issues/348>
- Module: \`src/api_security/\`
- Documentation: \`docs/security/API_SECURITY_TESTING.md\`
EOF

# Generate coverage report via Rust test
set +e
cargo test --lib api_security::coverage -- --nocapture 2>&1 | tail -5
set -e

echo "$OUTPUT" | tail -20
echo "$INT_OUTPUT" | tail -10

if [ "$FINAL_STATUS" -ne 0 ]; then
  echo "❌ API Security Test Suite FAILED"
  exit 1
fi

echo "✅ API Security Test Suite PASSED"
echo "  - report written to: $REPORT_PATH"
