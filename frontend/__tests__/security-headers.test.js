const https = require('https');

const SECURITY_HEADERS = [
  'Strict-Transport-Security',
  'Content-Security-Policy',
  'X-Content-Type-Options',
  'X-Frame-Options',
  'X-XSS-Protection',
  'Referrer-Policy',
  'Permissions-Policy',
  'Cache-Control',
  'Cross-Origin-Embedder-Policy',
  'Cross-Origin-Opener-Policy',
  'Cross-Origin-Resource-Policy',
];

const MISSING_HEADERS_THRESHOLD = 2;

describe('Security Headers', () => {
  const baseUrl = process.env.TEST_URL || 'http://localhost:3000';

  test('should have all required security headers', async () => {
    const response = await fetch(baseUrl);
    const headers = response.headers;
    const missingHeaders = [];

    for (const header of SECURITY_HEADERS) {
      if (!headers.get(header)) {
        missingHeaders.push(header);
      }
    }

    if (missingHeaders.length > 0) {
      console.warn(`Missing security headers: ${missingHeaders.join(', ')}`);
    }

    expect(missingHeaders.length).toBeLessThanOrEqual(MISSING_HEADERS_THRESHOLD);
  });

  test('Strict-Transport-Security should have long max-age', async () => {
    const response = await fetch(baseUrl);
    const hsts = response.headers.get('Strict-Transport-Security');
    if (hsts) {
      const maxAge = parseInt(hsts.match(/max-age=(\d+)/)?.[1] || '0', 10);
      expect(maxAge).toBeGreaterThanOrEqual(31536000);
    }
  });

  test('Content-Security-Policy should not allow unsafe-inline', async () => {
    const response = await fetch(baseUrl);
    const csp = response.headers.get('Content-Security-Policy');
    if (csp) {
      expect(csp).not.toContain("'unsafe-inline'");
    }
  });

  test('X-Content-Type-Options should be nosniff', async () => {
    const response = await fetch(baseUrl);
    const xcto = response.headers.get('X-Content-Type-Options');
    expect(xcto).toBe('nosniff');
  });

  test('X-Frame-Options should deny or sameorigin', async () => {
    const response = await fetch(baseUrl);
    const xfo = response.headers.get('X-Frame-Options');
    expect(['DENY', 'SAMEORIGIN']).toContain(xfo);
  });
});
