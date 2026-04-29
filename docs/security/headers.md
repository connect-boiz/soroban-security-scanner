# Security Headers Documentation

## Overview

This document describes the comprehensive HTTP security headers implementation for the Soroban Security Scanner application. The implementation protects against XSS, clickjacking, MIME sniffing, and other common web vulnerabilities.

## Table of Contents

1. [Architecture](#architecture)
2. [Content Security Policy (CSP)](#content-security-policy-csp)
3. [HTTP Strict Transport Security (HSTS)](#http-strict-transport-security-hsts)
4. [Other Security Headers](#other-security-headers)
5. [Adding New External Resources](#adding-new-external-resources)
6. [Testing and Validation](#testing-and-validation)
7. [Troubleshooting](#troubleshooting)

## Architecture

Security headers are implemented at two levels:

### 1. Next.js Middleware (`frontend/middleware.ts`)

The primary implementation uses Next.js Edge Middleware to set headers on all dynamic routes. This approach:

- Generates unique cryptographic nonces per request for CSP
- Sets environment-specific headers (e.g., HSTS only in production)
- Applies to all routes except static assets

### 2. Next.js Config (`frontend/next.config.js`)

Static headers that don't require per-request logic are set in the Next.js configuration:

- `X-DNS-Prefetch-Control`
- `X-XSS-Protection`

## Content Security Policy (CSP)

### Current Policy

```
Content-Security-Policy:
  default-src 'self';
  script-src 'self' 'nonce-{RANDOM}';
  style-src 'self' 'unsafe-inline';
  img-src 'self' data: blob:;
  font-src 'self';
  connect-src 'self' https://horizon.stellar.org https://horizon-testnet.stellar.org https://horizon-futurenet.stellar.org ws://localhost:* wss://*.stellar.org;
  frame-src 'none';
  frame-ancestors 'none';
  object-src 'none';
  base-uri 'self';
  form-action 'self';
  upgrade-insecure-requests;
  block-all-mixed-content;
```

### Directive Explanations

#### `default-src 'self'`
Fallback for all resource types. Only allows resources from the same origin.

#### `script-src 'self' 'nonce-{RANDOM}'`
- **'self'**: Allows scripts from the same origin
- **'nonce-{RANDOM}'**: Allows inline scripts with matching nonce attribute
- **No unsafe-eval**: Prevents `eval()` and similar dangerous functions
- **No unsafe-inline**: Prevents inline scripts without nonces

**Why nonces?** Nonces provide a secure way to allow specific inline scripts while blocking XSS attacks. Each request gets a unique cryptographic nonce that must match between the CSP header and script tags.

#### `style-src 'self' 'unsafe-inline'`
- **'self'**: Allows stylesheets from the same origin
- **'unsafe-inline'**: Required for:
  - Next.js styled-jsx
  - Tailwind CSS utility classes
  - React inline styles

**Note:** `unsafe-inline` for styles is acceptable because CSS injection is significantly less dangerous than JavaScript injection. CSS cannot execute arbitrary code or steal sensitive data.

#### `img-src 'self' data: blob:`
- **'self'**: Same-origin images
- **data:**: Base64-encoded inline images
- **blob:**: Dynamically generated images (canvas, file uploads)

#### `font-src 'self'`
Only allows fonts from the same origin. No external font CDNs are used.

#### `connect-src 'self' https://horizon.stellar.org ...`
Controls where the application can make network requests:

- **'self'**: API calls to the same origin
- **https://horizon.stellar.org**: Stellar mainnet Horizon API
- **https://horizon-testnet.stellar.org**: Stellar testnet Horizon API
- **https://horizon-futurenet.stellar.org**: Stellar futurenet Horizon API
- **ws://localhost:***: Local WebSocket development
- **wss://*.stellar.org**: Stellar WebSocket endpoints

#### `frame-src 'none'`
Completely disables embedding of frames/iframes. The application does not use iframes.

#### `frame-ancestors 'none'`
Prevents this site from being embedded in iframes on other sites. This is the CSP equivalent of `X-Frame-Options: DENY`.

#### `object-src 'none'`
Disables plugins like Flash, Java applets, and other legacy embedded objects.

#### `base-uri 'self'`
Restricts the `<base>` tag to prevent base tag hijacking attacks.

#### `form-action 'self'`
Restricts form submissions to the same origin.

#### `upgrade-insecure-requests`
Automatically upgrades HTTP requests to HTTPS in supporting browsers.

#### `block-all-mixed-content`
Prevents loading any HTTP resources when the page is served over HTTPS.

### CSP Nonce Implementation

#### How It Works

1. **Middleware generates nonce** (`frontend/middleware.ts`):
   ```typescript
   const nonce = randomBytes(16).toString('base64');
   ```

2. **Nonce added to CSP header**:
   ```typescript
   script-src 'self' 'nonce-{nonce}';
   ```

3. **Nonce stored in response header**:
   ```typescript
   response.headers.set('x-nonce', nonce);
   ```

4. **Layout retrieves nonce** (`frontend/app/layout.tsx`):
   ```typescript
   const nonce = headers().get('x-nonce') || '';
   ```

5. **Next.js automatically applies nonce** to its own scripts

6. **Custom inline scripts** must include the nonce attribute:
   ```html
   <script nonce={nonce}>
     // Your code here
   </script>
   ```

#### Security Properties

- **Unique per request**: Each page load gets a new nonce
- **Cryptographically random**: Uses `crypto.randomBytes(16)` (128 bits of entropy)
- **Unpredictable**: Attackers cannot guess or reuse nonces
- **Automatic**: Next.js handles nonce injection for framework scripts

### Development vs Production

- **Development**: Uses `Content-Security-Policy-Report-Only` header
  - Violations are logged to console but not blocked
  - Allows testing without breaking functionality
  
- **Production**: Uses enforcing `Content-Security-Policy` header
  - Violations are blocked
  - Provides actual security protection

## HTTP Strict Transport Security (HSTS)

### Configuration

```
Strict-Transport-Security: max-age=31536000; includeSubDomains; preload
```

### Directive Explanations

- **max-age=31536000**: Enforce HTTPS for 1 year (365 days)
- **includeSubDomains**: Apply to all subdomains
- **preload**: Eligible for browser HSTS preload list

### Important Notes

1. **HTTPS Only**: HSTS is only set on HTTPS responses. Setting it on HTTP would be ineffective and could cause issues.

2. **Production Only**: HSTS is only enabled in production (`NODE_ENV=production`) to avoid issues in local development.

3. **Subdomain Consideration**: The `includeSubDomains` directive means ALL subdomains must support HTTPS. If you have any HTTP-only subdomains, remove this directive.

4. **Preload List**: The `preload` directive indicates intent to submit to the [HSTS preload list](https://hstspreload.org/). This is optional but recommended for maximum security.

### HSTS Preload Submission

To submit your domain to the HSTS preload list:

1. Ensure HSTS header is set with:
   - `max-age` of at least 31536000 (1 year)
   - `includeSubDomains` directive
   - `preload` directive

2. Verify all subdomains support HTTPS

3. Submit at https://hstspreload.org/

4. Wait for inclusion in browser preload lists (can take months)

## Other Security Headers

### X-Frame-Options

```
X-Frame-Options: DENY
```

Prevents the site from being embedded in iframes, protecting against clickjacking attacks.

- **DENY**: Never allow framing
- **Redundant with CSP**: `frame-ancestors 'none'` provides the same protection, but `X-Frame-Options` is kept for older browser compatibility

### X-Content-Type-Options

```
X-Content-Type-Options: nosniff
```

Prevents browsers from MIME-sniffing responses away from the declared `Content-Type`.

- **Always set**: No configuration needed
- **Prevents**: Attacks where an attacker uploads a file with malicious content disguised as a safe MIME type

### Referrer-Policy

```
Referrer-Policy: strict-origin-when-cross-origin
```

Controls how much referrer information is sent with requests:

- **Same-origin requests**: Full URL (including path and query)
- **Cross-origin HTTPS requests**: Origin only (no path/query)
- **Cross-origin HTTP requests**: No referrer

**Why this policy?**
- Protects sensitive URL parameters from leaking to third parties
- Still provides useful analytics data for same-origin requests
- Balances privacy and functionality

### Permissions-Policy

```
Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=(), usb=(), interest-cohort=()
```

Disables browser features the application doesn't use:

- **camera=()**: No camera access
- **microphone=()**: No microphone access
- **geolocation=()**: No location access
- **payment=()**: No Payment Request API
- **usb=()**: No WebUSB API
- **interest-cohort=()**: Opt out of Google FLoC/Topics API

**Note**: If you need to enable a feature (e.g., camera for KYC), update the policy:
```
camera=(self "https://kyc.example.com")
```

### Cross-Origin Policies

#### Cross-Origin-Opener-Policy (COOP)

```
Cross-Origin-Opener-Policy: same-origin
```

Isolates the browsing context, preventing other origins from accessing the window object.

#### Cross-Origin-Resource-Policy (CORP)

```
Cross-Origin-Resource-Policy: same-origin
```

Prevents cross-origin resource loading.

#### Cross-Origin-Embedder-Policy (COEP)

**Not currently set** because it would block Stellar Horizon API calls that don't send CORP headers.

To enable COEP:
1. Verify all external resources send `Cross-Origin-Resource-Policy: cross-origin`
2. Add to middleware:
   ```typescript
   'Cross-Origin-Embedder-Policy': 'require-corp'
   ```

## Adding New External Resources

When you need to add a new external resource (API, CDN, font, etc.), follow these steps:

### 1. Identify the Resource Type

Determine which CSP directive applies:

- **JavaScript**: `script-src`
- **CSS**: `style-src`
- **Images**: `img-src`
- **Fonts**: `font-src`
- **API/WebSocket**: `connect-src`
- **Iframes**: `frame-src`

### 2. Update the CSP Policy

Edit `frontend/middleware.ts` and add the origin to the appropriate directive:

```typescript
const directives: CSPDirectives = {
  // ... existing directives ...
  
  'connect-src': [
    "'self'",
    'https://horizon.stellar.org',
    'https://new-api.example.com', // ← Add new origin here
  ],
};
```

### 3. Test Locally

1. Start the development server:
   ```bash
   cd frontend
   npm run dev
   ```

2. Open browser DevTools Console

3. Load the page that uses the new resource

4. Check for CSP violations:
   - Look for red errors starting with "Refused to load..."
   - Verify the resource loads successfully

### 4. Validate the Change

Run the security header tests:

```bash
cd frontend
npm test -- security-headers
```

### 5. Document the Change

Update this document with:
- Why the origin was added
- What functionality requires it
- Any security considerations

### Example: Adding Google Fonts

If you need to add Google Fonts:

1. **Update CSP** in `frontend/middleware.ts`:
   ```typescript
   'style-src': [
     "'self'",
     "'unsafe-inline'",
     'https://fonts.googleapis.com', // ← Google Fonts CSS
   ],
   'font-src': [
     "'self'",
     'https://fonts.gstatic.com', // ← Google Fonts files
   ],
   ```

2. **Add to HTML**:
   ```html
   <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;600;700&display=swap" rel="stylesheet">
   ```

3. **Test**: Verify fonts load without CSP violations

## Testing and Validation

### Automated Tests

Run the test suite:

```bash
cd frontend
npm test
```

Tests cover:
- ✅ All security headers are present
- ✅ CSP includes all required directives
- ✅ Nonces are unique per request
- ✅ HSTS is set correctly in production
- ✅ No unsafe-eval in CSP
- ✅ All browser features are disabled in Permissions-Policy

### Manual Browser Testing

1. **Start the application**:
   ```bash
   cd frontend
   npm run dev
   ```

2. **Open DevTools Console** (F12)

3. **Load each major page**:
   - Home page
   - Authentication pages
   - Scanner interface
   - Settings panel

4. **Check for CSP violations**:
   - Look for red errors starting with "Refused to..."
   - There should be ZERO CSP violations

5. **Verify functionality**:
   - All images load
   - All styles apply
   - All JavaScript works
   - API calls succeed
   - WebSocket connections work

### Online Security Scanners

#### Mozilla Observatory

1. Deploy to staging/production
2. Visit https://observatory.mozilla.org
3. Enter your URL
4. Run scan
5. **Target**: Grade A or higher

**Current Score**: [To be filled after deployment]

#### SecurityHeaders.com

1. Visit https://securityheaders.com
2. Enter your URL
3. Run scan
4. **Target**: Grade A or higher

**Current Score**: [To be filled after deployment]

#### CSP Evaluator

1. Visit https://csp-evaluator.withgoogle.com
2. Paste your CSP policy
3. Review findings
4. **Target**: No HIGH severity issues

**Current Score**: [To be filled after deployment]

### Testing Checklist

Before deploying to production:

- [ ] All automated tests pass
- [ ] Zero CSP violations in browser console on all pages
- [ ] All external resources load correctly
- [ ] Authentication flows work end-to-end
- [ ] API calls to Stellar Horizon succeed
- [ ] WebSocket connections establish successfully
- [ ] Mozilla Observatory grade A or higher
- [ ] SecurityHeaders.com grade A or higher
- [ ] CSP Evaluator shows no HIGH severity issues
- [ ] HSTS header present in production
- [ ] All security headers present on all routes

## Troubleshooting

### CSP Violation: Refused to load script

**Error**:
```
Refused to load the script 'https://example.com/script.js' because it violates the following Content Security Policy directive: "script-src 'self' 'nonce-...'"
```

**Solution**:
1. Identify the blocked origin: `https://example.com`
2. Add to `script-src` in `frontend/middleware.ts`:
   ```typescript
   'script-src': [
     "'self'",
     `'nonce-${nonce}'`,
     'https://example.com', // ← Add this
   ],
   ```

### CSP Violation: Refused to connect

**Error**:
```
Refused to connect to 'https://api.example.com' because it violates the following Content Security Policy directive: "connect-src 'self' ..."
```

**Solution**:
1. Add the API origin to `connect-src`:
   ```typescript
   'connect-src': [
     "'self'",
     'https://api.example.com', // ← Add this
     // ... other origins
   ],
   ```

### Inline script blocked

**Error**:
```
Refused to execute inline script because it violates the following Content Security Policy directive: "script-src 'self' 'nonce-...'"
```

**Solution**:
1. Get the nonce in your component:
   ```typescript
   import { headers } from 'next/headers';
   
   const nonce = headers().get('x-nonce') || '';
   ```

2. Add nonce to script tag:
   ```html
   <script nonce={nonce}>
     // Your code
   </script>
   ```

### HSTS not working in development

**Expected behavior**: HSTS is intentionally disabled in development to avoid issues with localhost.

**Solution**: Test HSTS in a staging or production environment with HTTPS.

### Fonts not loading

**Check**:
1. Are fonts from an external CDN? Add the CDN to `font-src`
2. Are fonts self-hosted? Ensure they're in the `public` directory
3. Check browser console for CSP violations

### Images not loading

**Check**:
1. External images? Add the domain to `img-src`
2. Base64 images? Ensure `data:` is in `img-src`
3. Blob URLs? Ensure `blob:` is in `img-src`

### API calls failing

**Check**:
1. Is the API origin in `connect-src`?
2. Is CORS configured on the API server?
3. Check browser console for CSP violations

### WebSocket connection fails

**Check**:
1. Is the WebSocket origin in `connect-src`?
2. Use `ws://` for HTTP and `wss://` for HTTPS
3. Ensure the WebSocket server is running

## Security Considerations

### Defense in Depth

This implementation follows defense-in-depth principles:

1. **CSP**: Primary defense against XSS
2. **X-Frame-Options**: Backup for older browsers
3. **HSTS**: Enforces HTTPS
4. **X-Content-Type-Options**: Prevents MIME confusion
5. **Referrer-Policy**: Protects sensitive URLs
6. **Permissions-Policy**: Reduces attack surface

### Regular Audits

Schedule regular security audits:

- **Monthly**: Run online scanners (Observatory, SecurityHeaders.com)
- **Quarterly**: Review CSP policy for unnecessary origins
- **Annually**: Full security audit by security professionals

### Incident Response

If a CSP violation is reported in production:

1. **Investigate**: Is it a legitimate resource or an attack?
2. **Legitimate**: Add the origin to CSP
3. **Attack**: Investigate how the malicious code was injected
4. **Document**: Record the incident and response

### Keeping Up to Date

Security headers evolve. Stay informed:

- Subscribe to [OWASP Secure Headers Project](https://owasp.org/www-project-secure-headers/)
- Follow [Mozilla Web Security Guidelines](https://infosec.mozilla.org/guidelines/web_security)
- Monitor [CSP specification updates](https://www.w3.org/TR/CSP/)

## References

- [OWASP Secure Headers Project](https://owasp.org/www-project-secure-headers/)
- [MDN Web Security](https://developer.mozilla.org/en-US/docs/Web/Security)
- [Content Security Policy Reference](https://content-security-policy.com/)
- [HSTS Preload List](https://hstspreload.org/)
- [Mozilla Observatory](https://observatory.mozilla.org/)
- [SecurityHeaders.com](https://securityheaders.com/)
- [CSP Evaluator](https://csp-evaluator.withgoogle.com/)

## Changelog

### 2024-01-XX - Initial Implementation

- Implemented CSP with nonces
- Added HSTS with preload
- Set all recommended security headers
- Created comprehensive test suite
- Documented all policies and procedures

**Scan Results**:
- Mozilla Observatory: [To be filled]
- SecurityHeaders.com: [To be filled]
- CSP Evaluator: [To be filled]
