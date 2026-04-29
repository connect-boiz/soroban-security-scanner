#!/bin/bash

# Security Headers Validation Script
# 
# This script validates that all required security headers are present
# and correctly configured on a deployed application.
#
# Usage:
#   ./scripts/validate-security-headers.sh https://your-app.com

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if URL is provided
if [ -z "$1" ]; then
    echo -e "${RED}Error: No URL provided${NC}"
    echo "Usage: $0 <url>"
    echo "Example: $0 https://your-app.com"
    exit 1
fi

URL="$1"
PASSED=0
FAILED=0
WARNINGS=0

echo "=========================================="
echo "Security Headers Validation"
echo "=========================================="
echo "URL: $URL"
echo ""

# Function to check header
check_header() {
    local header_name="$1"
    local expected_value="$2"
    local is_required="$3"
    
    echo -n "Checking $header_name... "
    
    # Get the header value
    header_value=$(curl -s -I "$URL" | grep -i "^$header_name:" | cut -d' ' -f2- | tr -d '\r\n')
    
    if [ -z "$header_value" ]; then
        if [ "$is_required" = "required" ]; then
            echo -e "${RED}FAILED${NC} - Header not found"
            FAILED=$((FAILED + 1))
        else
            echo -e "${YELLOW}WARNING${NC} - Header not found (optional)"
            WARNINGS=$((WARNINGS + 1))
        fi
        return 1
    fi
    
    if [ -n "$expected_value" ]; then
        if [[ "$header_value" == *"$expected_value"* ]]; then
            echo -e "${GREEN}PASSED${NC}"
            PASSED=$((PASSED + 1))
        else
            echo -e "${RED}FAILED${NC} - Expected: $expected_value, Got: $header_value"
            FAILED=$((FAILED + 1))
        fi
    else
        echo -e "${GREEN}PASSED${NC} - Value: $header_value"
        PASSED=$((PASSED + 1))
    fi
}

# Function to check header contains
check_header_contains() {
    local header_name="$1"
    local expected_substring="$2"
    
    echo -n "Checking $header_name contains '$expected_substring'... "
    
    header_value=$(curl -s -I "$URL" | grep -i "^$header_name:" | cut -d' ' -f2- | tr -d '\r\n')
    
    if [ -z "$header_value" ]; then
        echo -e "${RED}FAILED${NC} - Header not found"
        FAILED=$((FAILED + 1))
        return 1
    fi
    
    if [[ "$header_value" == *"$expected_substring"* ]]; then
        echo -e "${GREEN}PASSED${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}FAILED${NC} - '$expected_substring' not found in: $header_value"
        FAILED=$((FAILED + 1))
    fi
}

echo "Required Security Headers:"
echo "------------------------------------------"

# Check Content-Security-Policy
check_header "Content-Security-Policy" "" "required"
check_header_contains "Content-Security-Policy" "default-src"
check_header_contains "Content-Security-Policy" "script-src"
check_header_contains "Content-Security-Policy" "nonce-"

# Check X-Frame-Options
check_header "X-Frame-Options" "DENY" "required"

# Check X-Content-Type-Options
check_header "X-Content-Type-Options" "nosniff" "required"

# Check Referrer-Policy
check_header "Referrer-Policy" "strict-origin-when-cross-origin" "required"

# Check Permissions-Policy
check_header "Permissions-Policy" "" "required"

# Check Cross-Origin policies
check_header "Cross-Origin-Opener-Policy" "same-origin" "required"
check_header "Cross-Origin-Resource-Policy" "same-origin" "required"

echo ""
echo "Optional Security Headers:"
echo "------------------------------------------"

# Check HSTS (only on HTTPS)
if [[ "$URL" == https://* ]]; then
    check_header "Strict-Transport-Security" "max-age=" "optional"
else
    echo -e "${YELLOW}Skipping HSTS check (not HTTPS)${NC}"
fi

echo ""
echo "=========================================="
echo "Summary"
echo "=========================================="
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"
echo -e "${YELLOW}Warnings: $WARNINGS${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All required security headers are present!${NC}"
    echo ""
    echo "Next steps:"
    echo "1. Run Mozilla Observatory scan: https://observatory.mozilla.org"
    echo "2. Run SecurityHeaders.com scan: https://securityheaders.com"
    echo "3. Validate CSP: https://csp-evaluator.withgoogle.com"
    exit 0
else
    echo -e "${RED}✗ Some security headers are missing or incorrect${NC}"
    echo ""
    echo "Please review the failed checks above and update your configuration."
    echo "See docs/security/headers.md for detailed information."
    exit 1
fi
