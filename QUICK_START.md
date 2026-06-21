# Security Headers - Quick Start Guide

## 🚀 Get Started in 5 Minutes

### 1. Install Dependencies (2 min)
```bash
cd frontend
npm install
```

### 2. Run Tests (1 min)
```bash
npm test
```

### 3. Start Dev Server (1 min)
```bash
npm run dev
```

### 4. Verify in Browser (1 min)
1. Open http://localhost:3000
2. Press F12 (DevTools)
3. Check Console - should be clean (no CSP violations)
4. Check Network tab → Headers → Response Headers
5. Verify security headers are present

✅ **Done!** Security headers are now active.

---

## 📚 Documentation

| Document | Purpose | When to Use |
|----------|---------|-------------|
| [INSTALLATION_CHECKLIST.md](INSTALLATION_CHECKLIST.md) | Step-by-step installation | First time setup |
| [SECURITY_HEADERS_SUMMARY.md](SECURITY_HEADERS_SUMMARY.md) | Executive overview | Understanding what was done |
| [docs/security/headers.md](docs/security/headers.md) | Complete guide | Adding resources, troubleshooting |
| [docs/security/TESTING_GUIDE.md](docs/security/TESTING_GUIDE.md) | Testing procedures | Running tests, validation |
| [docs/security/README.md](docs/security/README.md) | Quick reference | Common tasks |

---

## 🔧 Common Tasks

### Add New API Endpoint
```typescript
// Edit: frontend/middleware.ts
'connect-src': [
  "'self'",
  'https://new-api.example.com', // ← Add here
],
```

### Fix CSP Violation
1. Check browser console for error
2. Note the blocked URL
3. Add to appropriate directive in `frontend/middleware.ts`
4. Restart dev server
5. Test again

### Run Only Security Tests
```bash
npm run test:security
```

### Validate Deployed App
```bash
./scripts/validate-security-headers.sh https://your-app.com
```

---

## 🎯 What Was Implemented

✅ **Content Security Policy** - Prevents XSS attacks
✅ **HSTS** - Enforces HTTPS
✅ **X-Frame-Options** - Prevents clickjacking
✅ **X-Content-Type-Options** - Prevents MIME sniffing
✅ **Referrer-Policy** - Protects sensitive URLs
✅ **Permissions-Policy** - Reduces attack surface
✅ **Cross-Origin Policies** - Isolates browsing context

---

## 🧪 Testing

### Automated Tests
```bash
npm test                    # All tests
npm run test:security       # Security tests only
npm run test:coverage       # With coverage report
```

### Manual Testing
1. Start server: `npm run dev`
2. Open browser DevTools (F12)
3. Check Console for CSP violations
4. Check Network → Headers for security headers

### Online Scanners (After Deployment)
- [Mozilla Observatory](https://observatory.mozilla.org) - Target: Grade A
- [SecurityHeaders.com](https://securityheaders.com) - Target: Grade A
- [CSP Evaluator](https://csp-evaluator.withgoogle.com) - Target: No HIGH issues

---

## 🐛 Troubleshooting

### CSP Violation in Console
**Error**: `Refused to load...`

**Fix**: Add the blocked resource to CSP in `frontend/middleware.ts`

See: [docs/security/headers.md#troubleshooting](docs/security/headers.md#troubleshooting)

### Tests Fail
**Error**: `Cannot find module 'jest'`

**Fix**: 
```bash
npm install
```

### TypeScript Errors
**Error**: `Cannot find module 'next/server'`

**Fix**: These will resolve after `npm install`

---

## 📋 Checklist

### Before Committing
- [ ] Tests pass: `npm test`
- [ ] No console errors
- [ ] No CSP violations
- [ ] All pages work

### Before Deploying
- [ ] Staging tested
- [ ] Validation script passes
- [ ] Online scanners run
- [ ] Security team approval

---

## 🆘 Need Help?

1. **Check documentation** - See links above
2. **Search for error** - Google the CSP violation message
3. **Contact security team** - For complex issues

---

## 📊 Files Changed

### New Files (12)
- `frontend/middleware.ts` - Main implementation
- `frontend/__tests__/security-headers.test.ts` - Tests
- `docs/security/headers.md` - Documentation
- And 9 more...

### Modified Files (5)
- `frontend/next.config.js` - Static headers
- `frontend/app/layout.tsx` - Nonce support
- `frontend/package.json` - Dependencies
- And 2 more...

See [SECURITY_HEADERS_SUMMARY.md](SECURITY_HEADERS_SUMMARY.md) for complete list.

---

## ⚡ Quick Commands

```bash
# Setup
cd frontend && npm install

# Test
npm test
npm run test:security
npm run test:coverage

# Develop
npm run dev
npm run type-check
npm run lint

# Deploy
npm run build
npm start

# Validate
./scripts/validate-security-headers.sh https://your-app.com
```

---

## 🎓 Learn More

### Security Headers
- [OWASP Secure Headers](https://owasp.org/www-project-secure-headers/)
- [MDN Web Security](https://developer.mozilla.org/en-US/docs/Web/Security)
- [CSP Guide](https://content-security-policy.com/)

### Testing
- [Jest Documentation](https://jestjs.io/)
- [Testing Library](https://testing-library.com/)
- [Next.js Testing](https://nextjs.org/docs/testing)

---

**Status**: ✅ Ready to Use

**Next Step**: Run `cd frontend && npm install`

**Time Required**: 5 minutes

**Questions?** See [docs/security/README.md](docs/security/README.md)
