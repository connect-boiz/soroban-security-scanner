# Security Documentation

This directory contains comprehensive security documentation for the Soroban Security Scanner application.

## Documents

### [headers.md](./headers.md)
**Comprehensive Security Headers Guide**

Complete documentation of the HTTP security headers implementation including:
- Architecture and implementation details
- Content Security Policy (CSP) with nonce explanations
- HSTS configuration and preload submission
- All security headers explained
- How to add new external resources
- Testing and validation procedures
- Troubleshooting guide

**Use this when**:
- Adding new external resources (APIs, CDNs, fonts)
- Troubleshooting CSP violations
- Understanding security header configuration
- Updating security policies

### [SECURITY_HEADERS_AUDIT.md](./SECURITY_HEADERS_AUDIT.md)
**Implementation Audit Report**

Detailed audit of the security headers implementation including:
- Pre-implementation audit findings
- External resources inventory
- Implementation details for each header
- Code changes made
- Testing implementation
- Validation results
- Known issues and limitations
- Recommendations

**Use this when**:
- Understanding what was implemented and why
- Reviewing security posture before/after
- Compliance and audit requirements
- Planning future security improvements

## Quick Start

### For Developers

**Adding a new external resource?**
1. Read [headers.md - Adding New External Resources](./headers.md#adding-new-external-resources)
2. Update `frontend/middleware.ts` with the new origin
3. Test locally for CSP violations
4. Run tests: `npm test -- security-headers`

**CSP violation in console?**
1. Read [headers.md - Troubleshooting](./headers.md#troubleshooting)
2. Identify the blocked resource
3. Add to appropriate CSP directive
4. Test and validate

### For Security Auditors

**Reviewing security implementation?**
1. Read [SECURITY_HEADERS_AUDIT.md](./SECURITY_HEADERS_AUDIT.md)
2. Review implementation details and rationale
3. Check validation results
4. Review known issues and mitigations

**Running security scans?**
1. Deploy application
2. Run validation script: `./scripts/validate-security-headers.sh https://your-app.com`
3. Run online scanners:
   - [Mozilla Observatory](https://observatory.mozilla.org)
   - [SecurityHeaders.com](https://securityheaders.com)
   - [CSP Evaluator](https://csp-evaluator.withgoogle.com)

### For DevOps/Deployment

**Deploying to production?**
1. Ensure `NODE_ENV=production` is set
2. Verify HTTPS is configured
3. Run validation script after deployment
4. Monitor for CSP violations in production logs

**Setting up new environment?**
1. Review [headers.md - Architecture](./headers.md#architecture)
2. Configure platform-specific headers if needed (Vercel, Netlify, etc.)
3. Test security headers in new environment
4. Update documentation with environment-specific notes

## Testing

### Run All Security Tests
```bash
cd frontend
npm test
```

### Run Only Security Header Tests
```bash
cd frontend
npm run test:security
```

### Validate Deployed Application
```bash
./scripts/validate-security-headers.sh https://your-app.com
```

## Key Files

### Implementation Files
- `frontend/middleware.ts` - Main security headers middleware
- `frontend/next.config.js` - Static security headers
- `frontend/app/layout.tsx` - CSP nonce support

### Test Files
- `frontend/__tests__/security-headers.test.ts` - Unit tests
- `frontend/__tests__/security-headers.integration.test.ts` - Integration tests

### Validation Tools
- `scripts/validate-security-headers.sh` - Automated validation script

## Security Headers Summary

| Header | Value | Purpose |
|--------|-------|---------|
| Content-Security-Policy | [Complex policy] | Prevent XSS attacks |
| Strict-Transport-Security | max-age=31536000; includeSubDomains; preload | Enforce HTTPS |
| X-Frame-Options | DENY | Prevent clickjacking |
| X-Content-Type-Options | nosniff | Prevent MIME sniffing |
| Referrer-Policy | strict-origin-when-cross-origin | Protect sensitive URLs |
| Permissions-Policy | [Disabled features] | Reduce attack surface |
| Cross-Origin-Opener-Policy | same-origin | Isolate browsing context |
| Cross-Origin-Resource-Policy | same-origin | Prevent cross-origin loading |

## External Resources Allowed

### APIs (connect-src)
- `https://horizon.stellar.org` - Stellar mainnet
- `https://horizon-testnet.stellar.org` - Stellar testnet
- `https://horizon-futurenet.stellar.org` - Stellar futurenet
- `wss://*.stellar.org` - Stellar WebSocket
- `ws://localhost:*` - Local development WebSocket

### Scripts (script-src)
- `'self'` - Same-origin scripts only
- `'nonce-{RANDOM}'` - Inline scripts with nonce

### Styles (style-src)
- `'self'` - Same-origin styles
- `'unsafe-inline'` - Required for CSS-in-JS

### Images (img-src)
- `'self'` - Same-origin images
- `data:` - Base64 inline images
- `blob:` - Dynamic images

### Fonts (font-src)
- `'self'` - Same-origin fonts only

## Common Tasks

### Add New API Endpoint
```typescript
// In frontend/middleware.ts
'connect-src': [
  "'self'",
  'https://horizon.stellar.org',
  'https://new-api.example.com', // ← Add here
],
```

### Add External Script
```typescript
// In frontend/middleware.ts
'script-src': [
  "'self'",
  `'nonce-${nonce}'`,
  'https://cdn.example.com', // ← Add here
],
```

### Add External Font
```typescript
// In frontend/middleware.ts
'font-src': [
  "'self'",
  'https://fonts.gstatic.com', // ← Add here
],
```

## Support

### Issues or Questions?
1. Check [headers.md - Troubleshooting](./headers.md#troubleshooting)
2. Review [SECURITY_HEADERS_AUDIT.md - Known Issues](./SECURITY_HEADERS_AUDIT.md#known-issues-and-limitations)
3. Search for similar CSP violations online
4. Consult security team

### Reporting Security Issues
If you discover a security vulnerability:
1. **Do not** open a public issue
2. Contact the security team directly
3. Provide detailed information about the vulnerability
4. Wait for acknowledgment before disclosure

## Resources

### Official Documentation
- [OWASP Secure Headers Project](https://owasp.org/www-project-secure-headers/)
- [MDN Web Security](https://developer.mozilla.org/en-US/docs/Web/Security)
- [Content Security Policy Reference](https://content-security-policy.com/)
- [HSTS Preload List](https://hstspreload.org/)

### Testing Tools
- [Mozilla Observatory](https://observatory.mozilla.org/)
- [SecurityHeaders.com](https://securityheaders.com/)
- [CSP Evaluator](https://csp-evaluator.withgoogle.com/)

### Learning Resources
- [CSP Guide by Google](https://developers.google.com/web/fundamentals/security/csp)
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [Web Security Academy](https://portswigger.net/web-security)

## Changelog

### 2024-01-XX - Initial Implementation
- Implemented comprehensive security headers
- Created CSP with nonces
- Added HSTS with preload
- Created test suite (25+ tests)
- Documented all policies and procedures
- Created validation tooling

---

**Last Updated**: [DATE]

**Maintained By**: Security Team

**Review Schedule**: Quarterly
