# Security Headers Implementation - Installation Checklist

## Pre-Installation Status

✅ **Implementation Complete**
- All code written
- All documentation created
- All tests written
- TypeScript configuration updated

⏳ **Pending Installation**
- Dependencies need to be installed
- Tests need to be run
- Manual validation needed

## Installation Steps

### Step 1: Install Dependencies

```bash
cd frontend
npm install
```

**Expected**: Installation completes without errors

**What this installs**:
- Jest testing framework
- Testing Library for React
- Next.js testing utilities
- @types/node (fixes TypeScript errors)
- All other dependencies

**Time**: ~2-3 minutes

### Step 2: Verify TypeScript Compilation

```bash
npm run type-check
```

**Expected**: No TypeScript errors

**If errors occur**:
- Check that @types/node was installed
- Verify tsconfig.json has "types": ["node"]
- Run `npm install` again if needed

### Step 3: Run All Tests

```bash
npm test
```

**Expected**: All tests pass

**Test output should show**:
```
PASS  __tests__/security-headers.test.ts
  Security Headers Middleware
    Content Security Policy (CSP)
      ✓ should set CSP header on all responses
      ✓ should include nonce in script-src directive
      ✓ should not include unsafe-eval
      ... (25+ tests)

Test Suites: 2 passed, 2 total
Tests:       25+ passed, 25+ total
```

**If tests fail**:
- Read the error messages carefully
- Check that all files were created correctly
- Verify no syntax errors in test files
- Run `npm test -- --verbose` for more details

### Step 4: Run Security Tests Only

```bash
npm run test:security
```

**Expected**: All security header tests pass

**Time**: ~5-10 seconds

### Step 5: Start Development Server

```bash
npm run dev
```

**Expected**: Server starts on http://localhost:3000

**Output should show**:
```
- ready started server on 0.0.0.0:3000, url: http://localhost:3000
- event compiled client and server successfully
```

### Step 6: Manual Browser Testing

1. **Open browser** to http://localhost:3000

2. **Open DevTools** (F12 or Cmd+Option+I)

3. **Check Console tab**
   - Expected: No CSP violations
   - Expected: No "Refused to..." errors
   - Expected: Application loads normally

4. **Check Network tab**
   - Click on first request (HTML document)
   - Click "Headers" tab
   - Scroll to "Response Headers"
   - Verify these headers are present:
     - ✅ Content-Security-Policy-Report-Only (in dev)
     - ✅ X-Frame-Options: DENY
     - ✅ X-Content-Type-Options: nosniff
     - ✅ Referrer-Policy: strict-origin-when-cross-origin
     - ✅ Permissions-Policy
     - ✅ Cross-Origin-Opener-Policy: same-origin
     - ✅ Cross-Origin-Resource-Policy: same-origin
     - ✅ x-nonce: [random string]

5. **Test all major pages**
   - Navigate to each page
   - Check console for violations
   - Verify functionality works

### Step 7: Run Coverage Report

```bash
npm run test:coverage
```

**Expected**: Coverage report generated

**Check coverage**:
- middleware.ts should have 100% coverage
- Overall coverage should be reasonable

**View report**:
```bash
# Mac
open coverage/lcov-report/index.html

# Windows
start coverage/lcov-report/index.html

# Linux
xdg-open coverage/lcov-report/index.html
```

## Post-Installation Checklist

### Code Quality
- [ ] All tests pass
- [ ] No TypeScript errors
- [ ] No ESLint errors
- [ ] Code formatted correctly

### Functionality
- [ ] Development server starts
- [ ] All pages load
- [ ] No console errors
- [ ] No CSP violations
- [ ] All features work

### Security Headers
- [ ] CSP header present
- [ ] CSP includes nonce
- [ ] All security headers present
- [ ] Headers have correct values
- [ ] No HSTS in development (expected)

### Documentation
- [ ] All docs created
- [ ] README updated
- [ ] Examples work
- [ ] Links are valid

## Troubleshooting

### Issue: npm install fails

**Error**: `EACCES: permission denied`

**Solution**:
```bash
sudo npm install
# or
npm install --unsafe-perm
```

### Issue: Tests fail with "Cannot find module"

**Error**: `Cannot find module 'jest'`

**Solution**:
```bash
rm -rf node_modules package-lock.json
npm install
```

### Issue: TypeScript errors persist

**Error**: `Cannot find module 'next/server'`

**Solution**:
1. Verify @types/node is installed:
   ```bash
   npm list @types/node
   ```

2. If not installed:
   ```bash
   npm install --save-dev @types/node
   ```

3. Verify tsconfig.json has:
   ```json
   {
     "compilerOptions": {
       "types": ["node"]
     }
   }
   ```

### Issue: CSP violations in console

**Error**: `Refused to load...`

**Solution**:
1. This is expected in development (report-only mode)
2. Identify the blocked resource
3. Follow troubleshooting guide in docs/security/headers.md
4. Add resource to appropriate CSP directive if legitimate

### Issue: Server won't start

**Error**: `Port 3000 is already in use`

**Solution**:
```bash
# Kill process on port 3000
# Mac/Linux
lsof -ti:3000 | xargs kill -9

# Windows
netstat -ano | findstr :3000
taskkill /PID [PID] /F
```

## Expected File Structure

After installation, you should have:

```
frontend/
├── __tests__/
│   ├── security-headers.test.ts ✅
│   └── security-headers.integration.test.ts ✅
├── app/
│   ├── layout.tsx ✅ (modified)
│   └── globals.css ✅ (modified)
├── components/
│   └── notifications/
│       └── Toast.tsx ✅ (modified)
├── middleware.ts ✅ (new)
├── jest.config.js ✅ (new)
├── jest.setup.js ✅ (new)
├── next.config.js ✅ (modified)
├── package.json ✅ (modified)
├── tsconfig.json ✅ (modified)
└── node_modules/ ⏳ (after npm install)

docs/security/
├── headers.md ✅
├── SECURITY_HEADERS_AUDIT.md ✅
├── README.md ✅
└── TESTING_GUIDE.md ✅

scripts/
└── validate-security-headers.sh ✅

Root:
├── SECURITY_HEADERS_COMMIT.md ✅
├── SECURITY_HEADERS_SUMMARY.md ✅
└── INSTALLATION_CHECKLIST.md ✅ (this file)
```

## Success Criteria

Installation is successful when:

✅ **All dependencies installed** - `npm install` completes
✅ **No TypeScript errors** - `npm run type-check` passes
✅ **All tests pass** - `npm test` shows all green
✅ **Server starts** - `npm run dev` works
✅ **No CSP violations** - Browser console is clean
✅ **All headers present** - Network tab shows security headers
✅ **Functionality works** - All features operational

## Next Steps After Installation

### 1. Code Review
- Review all changes
- Verify implementation matches requirements
- Check for any issues

### 2. Commit Changes
```bash
git add .
git commit -F SECURITY_HEADERS_COMMIT.md
```

### 3. Create Pull Request
- Use SECURITY_HEADERS_SUMMARY.md as PR description
- Link to documentation
- Request security team review

### 4. Deploy to Staging
- Deploy to staging environment
- Run validation script
- Run online scanners

### 5. Production Deployment
- Get security team approval
- Deploy to production
- Monitor for issues
- Document scan results

## Timeline Estimate

| Task | Time | Status |
|------|------|--------|
| Install dependencies | 2-3 min | ⏳ |
| Run tests | 1-2 min | ⏳ |
| Manual browser testing | 5-10 min | ⏳ |
| Code review | 30-60 min | ⏳ |
| Staging deployment | 10-20 min | ⏳ |
| Online scanner validation | 5-10 min | ⏳ |
| Production deployment | 10-20 min | ⏳ |
| **Total** | **~1-2 hours** | ⏳ |

## Support

If you encounter any issues during installation:

1. Check this checklist for troubleshooting steps
2. Review [TESTING_GUIDE.md](docs/security/TESTING_GUIDE.md)
3. Check [headers.md - Troubleshooting](docs/security/headers.md#troubleshooting)
4. Contact security team

## Quick Commands Reference

```bash
# Install dependencies
cd frontend && npm install

# Run all tests
npm test

# Run security tests only
npm run test:security

# Type check
npm run type-check

# Start dev server
npm run dev

# Generate coverage
npm run test:coverage

# Lint code
npm run lint

# Build for production
npm run build
```

---

**Status**: ⏳ Ready for Installation

**Next Action**: Run `cd frontend && npm install`

**Estimated Time**: 2-3 minutes

**Contact**: Security Team
