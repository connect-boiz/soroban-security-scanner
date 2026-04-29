# Security Headers Testing Guide

This guide provides step-by-step instructions for testing the security headers implementation.

## Prerequisites

- Node.js 18+ installed
- npm or yarn package manager
- Git (for version control)
- Modern web browser (Chrome, Firefox, or Edge)

## Quick Start

### 1. Install Dependencies

```bash
cd frontend
npm install
```

This will install:
- Jest testing framework
- Testing Library for React
- Next.js testing utilities
- All required dependencies

### 2. Run All Tests

```bash
npm test
```

This runs the complete test suite including:
- Security headers unit tests (25+ test cases)
- Integration tests
- Any other existing tests

### 3. Run Only Security Header Tests

```bash
npm run test:security
```

This runs only the security header tests, which is faster for iterative development.

### 4. Run Tests in Watch Mode

```bash
npm run test:watch
```

Tests will re-run automatically when files change. Useful during development.

### 5. Generate Coverage Report

```bash
npm run test:coverage
```

Generates a code coverage report showing which lines are tested.

## Test Categories

### Unit Tests

**File**: `frontend/__tests__/security-headers.test.ts`

**What it tests**:
- CSP header is present on all responses
- CSP includes all required directives
- Nonces are cryptographically random and unique
- HSTS is configured correctly in production
- All security headers are present
- No unsafe-eval in CSP
- Permissions-Policy disables all unused features
- Environment-specific behavior (dev vs prod)

**Run**:
```bash
npm test -- security-headers.test.ts
```

**Expected output**:
```
PASS  __tests__/security-headers.test.ts
  Security Headers Middleware
    Content Security Policy (CSP)
      ✓ should set CSP header on all responses
      ✓ should include nonce in script-src directive
      ✓ should not include unsafe-eval
      ✓ should include all required CSP directives
      ...
    
Test Suites: 1 passed, 1 total
Tests:       25 passed, 25 total
```

### Integration Tests

**File**: `frontend/__tests__/security-headers.integration.test.ts`

**What it tests**:
- Full page load returns all required headers
- API endpoints include security headers
- Static assets include security headers
- CSP nonce in header matches nonce in HTML

**Note**: These tests require a running application.

**Run**:
```bash
# Terminal 1: Start the dev server
npm run dev

# Terminal 2: Run integration tests
TEST_URL=http://localhost:3000 npm test -- security-headers.integration.test.ts
```

## Manual Testing

### Browser Console Testing

This is the most important test - it verifies that CSP doesn't break your application.

#### Step 1: Start the Development Server

```bash
cd frontend
npm run dev
```

The server should start on http://localhost:3000

#### Step 2: Open Browser DevTools

1. Open your browser
2. Navigate to http://localhost:3000
3. Press F12 (or Cmd+Option+I on Mac) to open DevTools
4. Click on the "Console" tab

#### Step 3: Check for CSP Violations

Look for errors that start with:
- "Refused to load..."
- "Refused to execute..."
- "Refused to connect..."

**Expected**: Zero CSP violations

**If you see violations**:
1. Note the blocked resource URL
2. Identify the resource type (script, style, image, API, etc.)
3. Follow the troubleshooting guide in docs/security/headers.md
4. Add the resource to the appropriate CSP directive
5. Restart the dev server and test again

#### Step 4: Test All Major Pages

Navigate through all major pages and check console for violations:

- [ ] Home page (/)
- [ ] Authentication pages (/auth)
- [ ] Scanner interface
- [ ] Settings panel
- [ ] Notifications page
- [ ] Any other major features

#### Step 5: Test Functionality

Verify that everything works:

- [ ] All images load
- [ ] All styles apply correctly
- [ ] All JavaScript functionality works
- [ ] API calls succeed
- [ ] WebSocket connections work (if applicable)
- [ ] Forms submit correctly
- [ ] Authentication flows work

### Network Tab Testing

#### Step 1: Open Network Tab

1. Open DevTools (F12)
2. Click on "Network" tab
3. Reload the page

#### Step 2: Check Response Headers

1. Click on the first request (usually the HTML document)
2. Click on "Headers" tab
3. Scroll down to "Response Headers"

#### Step 3: Verify Security Headers

Check that these headers are present:

- [ ] Content-Security-Policy (or Content-Security-Policy-Report-Only in dev)
- [ ] X-Frame-Options: DENY
- [ ] X-Content-Type-Options: nosniff
- [ ] Referrer-Policy: strict-origin-when-cross-origin
- [ ] Permissions-Policy: (should contain camera=(), microphone=(), etc.)
- [ ] Cross-Origin-Opener-Policy: same-origin
- [ ] Cross-Origin-Resource-Policy: same-origin
- [ ] x-nonce: (should be a random string)

In production (HTTPS only):
- [ ] Strict-Transport-Security: max-age=31536000; includeSubDomains; preload

## Deployment Testing

### Validation Script

After deploying to staging or production, run the validation script:

```bash
./scripts/validate-security-headers.sh https://your-staging-url.com
```

**Expected output**:
```
==========================================
Security Headers Validation
==========================================
URL: https://your-staging-url.com

Required Security Headers:
------------------------------------------
Checking Content-Security-Policy... PASSED
Checking Content-Security-Policy contains 'default-src'... PASSED
Checking Content-Security-Policy contains 'script-src'... PASSED
Checking Content-Security-Policy contains 'nonce-'... PASSED
Checking X-Frame-Options... PASSED
Checking X-Content-Type-Options... PASSED
Checking Referrer-Policy... PASSED
Checking Permissions-Policy... PASSED
Checking Cross-Origin-Opener-Policy... PASSED
Checking Cross-Origin-Resource-Policy... PASSED

Optional Security Headers:
------------------------------------------
Checking Strict-Transport-Security... PASSED

==========================================
Summary
==========================================
Passed: 11
Failed: 0
Warnings: 0

✓ All required security headers are present!
```

### Online Security Scanners

#### Mozilla Observatory

1. Visit https://observatory.mozilla.org
2. Enter your deployment URL
3. Click "Scan Me"
4. Wait for results

**Target**: Grade A or higher

**What it checks**:
- All security headers
- TLS configuration
- Cookie security
- Subresource Integrity
- And more

**Screenshot the results** and save to docs/security/

#### SecurityHeaders.com

1. Visit https://securityheaders.com
2. Enter your deployment URL
3. Click "Scan"
4. Wait for results

**Target**: Grade A or higher

**What it checks**:
- Content-Security-Policy
- Strict-Transport-Security
- X-Frame-Options
- X-Content-Type-Options
- Referrer-Policy
- Permissions-Policy

**Screenshot the results** and save to docs/security/

#### CSP Evaluator

1. Visit https://csp-evaluator.withgoogle.com
2. Paste your CSP policy (from Network tab or validation script)
3. Click "Evaluate"
4. Review findings

**Target**: No HIGH severity issues

**What it checks**:
- CSP syntax
- Unsafe directives
- Missing directives
- Bypass opportunities
- Best practices

**Screenshot the results** and save to docs/security/

## Troubleshooting

### Tests Fail to Run

**Error**: `Cannot find module 'jest'`

**Solution**:
```bash
cd frontend
npm install
```

### CSP Violations in Console

**Error**: `Refused to load the script 'https://example.com/script.js'`

**Solution**:
1. Identify the blocked origin: `https://example.com`
2. Edit `frontend/middleware.ts`
3. Add to appropriate directive:
   ```typescript
   'script-src': [
     "'self'",
     `'nonce-${nonce}'`,
     'https://example.com', // ← Add this
   ],
   ```
4. Restart dev server
5. Test again

### HSTS Not Present in Development

**Expected behavior**: HSTS is intentionally disabled in development.

**Why**: HSTS requires HTTPS. Local development typically uses HTTP.

**Solution**: Test HSTS in staging/production with HTTPS.

### Nonce Not Working

**Symptom**: Inline scripts are blocked even with nonce

**Check**:
1. Is the nonce in the CSP header?
   - Open Network tab
   - Check Content-Security-Policy header
   - Look for `'nonce-XXXXX'`

2. Does the script tag have the nonce attribute?
   - View page source
   - Find the script tag
   - Check for `nonce="XXXXX"`

3. Do the nonces match?
   - Compare nonce in header vs nonce in script tag
   - They must be identical

**Solution**: Ensure you're using the nonce from `headers().get('x-nonce')`

### Tests Pass But Browser Shows Violations

**Possible causes**:
1. Tests are mocked and don't reflect real behavior
2. External resources not accounted for in tests
3. Dynamic content loaded after page load

**Solution**:
1. Trust browser console over tests
2. Fix CSP violations in browser
3. Update tests to match real behavior

## Test Coverage Goals

### Current Coverage

Run coverage report:
```bash
npm run test:coverage
```

### Coverage Targets

- **middleware.ts**: 100% (all security header logic)
- **Overall**: 80%+ (reasonable for a web application)

### Viewing Coverage Report

After running `npm run test:coverage`:

```bash
# Open coverage report in browser
open coverage/lcov-report/index.html  # Mac
start coverage/lcov-report/index.html # Windows
xdg-open coverage/lcov-report/index.html # Linux
```

## Continuous Integration

### GitHub Actions Example

```yaml
name: Security Tests

on: [push, pull_request]

jobs:
  security-tests:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '18'
      
      - name: Install dependencies
        run: |
          cd frontend
          npm ci
      
      - name: Run security header tests
        run: |
          cd frontend
          npm run test:security
      
      - name: Run all tests
        run: |
          cd frontend
          npm test
```

## Checklist

### Before Committing

- [ ] All tests pass locally
- [ ] Zero CSP violations in browser console
- [ ] All pages load correctly
- [ ] All functionality works
- [ ] Code reviewed

### Before Deploying to Staging

- [ ] All tests pass in CI
- [ ] Manual browser testing complete
- [ ] No CSP violations
- [ ] Documentation updated

### Before Deploying to Production

- [ ] Staging validation complete
- [ ] Validation script passes on staging
- [ ] Mozilla Observatory: Grade A
- [ ] SecurityHeaders.com: Grade A
- [ ] CSP Evaluator: No HIGH issues
- [ ] Load testing complete
- [ ] Security team approval

### After Deploying to Production

- [ ] Validation script passes on production
- [ ] Online scanners run on production
- [ ] Results documented
- [ ] Monitoring configured for CSP violations
- [ ] Incident response plan ready

## Monitoring in Production

### CSP Violation Reporting

Consider adding CSP violation reporting:

```typescript
// In frontend/middleware.ts
const directives: CSPDirectives = {
  // ... existing directives ...
  'report-uri': ['/api/csp-report'],
  'report-to': ['csp-endpoint'],
};
```

Then create an API endpoint to receive reports:

```typescript
// frontend/app/api/csp-report/route.ts
export async function POST(request: Request) {
  const report = await request.json();
  
  // Log to monitoring service
  console.error('CSP Violation:', report);
  
  // Send to error tracking (Sentry, etc.)
  // trackCSPViolation(report);
  
  return new Response('OK', { status: 200 });
}
```

### Monitoring Tools

- **Sentry**: Automatic CSP violation tracking
- **LogRocket**: Session replay with CSP violations
- **Datadog**: Custom CSP violation metrics
- **CloudWatch**: AWS-based monitoring

## Resources

### Documentation
- [Main Headers Guide](./headers.md)
- [Implementation Audit](./SECURITY_HEADERS_AUDIT.md)
- [Quick Reference](./README.md)

### External Resources
- [Jest Documentation](https://jestjs.io/)
- [Testing Library](https://testing-library.com/)
- [Next.js Testing](https://nextjs.org/docs/testing)
- [CSP Testing Guide](https://content-security-policy.com/examples/testing/)

## Support

If you encounter issues:

1. Check [headers.md - Troubleshooting](./headers.md#troubleshooting)
2. Review test output carefully
3. Check browser console for CSP violations
4. Search for similar issues online
5. Contact security team

---

**Last Updated**: [DATE]

**Maintained By**: Security Team
