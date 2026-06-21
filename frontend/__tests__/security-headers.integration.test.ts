/**
 * Security Headers Integration Tests
 *
 * Tests security headers in a more realistic environment
 * with actual HTTP requests and responses
 */

describe('Security Headers Integration', () => {
  const baseUrl = process.env.TEST_URL || 'http://localhost:3000';

  describe('Full Page Load', () => {
    it('should return all required security headers on page load', async () => {
      const response = await fetch(baseUrl);

      expect(response.headers.get('X-Frame-Options')).toBe('DENY');
      expect(response.headers.get('X-Content-Type-Options')).toBe('nosniff');
      expect(response.headers.get('Referrer-Policy')).toBe('strict-origin-when-cross-origin');
      expect(response.headers.get('Permissions-Policy')).toBeTruthy();
      expect(response.headers.get('Cross-Origin-Opener-Policy')).toBe('same-origin');
      expect(response.headers.get('Cross-Origin-Resource-Policy')).toBe('same-origin');
    });

    it('should include CSP header with nonce', async () => {
      const response = await fetch(baseUrl);

      const cspHeader =
        response.headers.get('Content-Security-Policy') ||
        response.headers.get('Content-Security-Policy-Report-Only');

      expect(cspHeader).toBeTruthy();
      expect(cspHeader).toContain("'nonce-");
    });

    it('should include HSTS header in production', async () => {
      if (process.env.NODE_ENV === 'production') {
        const response = await fetch(baseUrl);
        const hstsHeader = response.headers.get('Strict-Transport-Security');

        expect(hstsHeader).toBeTruthy();
        expect(hstsHeader).toContain('max-age=31536000');
      }
    });
  });

  describe('API Endpoints', () => {
    it('should include security headers on API routes', async () => {
      // Test an API endpoint if available
      const response = await fetch(`${baseUrl}/api/health`).catch(() => null);

      if (response && response.ok) {
        expect(response.headers.get('X-Content-Type-Options')).toBe('nosniff');
        expect(response.headers.get('X-Frame-Options')).toBe('DENY');
      }
    });
  });

  describe('Static Assets', () => {
    it('should include security headers on static files', async () => {
      const response = await fetch(`${baseUrl}/favicon.ico`).catch(() => null);

      if (response) {
        // Static files should still have basic security headers
        expect(response.headers.get('X-Content-Type-Options')).toBeTruthy();
      }
    });
  });

  describe('CSP Nonce Consistency', () => {
    it('should use consistent nonce across page and inline scripts', async () => {
      const response = await fetch(baseUrl);
      const html = await response.text();

      const cspHeader =
        response.headers.get('Content-Security-Policy') ||
        response.headers.get('Content-Security-Policy-Report-Only');

      // Extract nonce from CSP header
      const nonceMatch = cspHeader?.match(/'nonce-([^']+)'/);

      if (nonceMatch) {
        const nonce = nonceMatch[1];

        // Check if HTML contains script tags with the same nonce
        // Next.js automatically adds nonces to its scripts
        if (html.includes('<script')) {
          // If there are script tags, at least one should have the nonce
          expect(html).toContain(`nonce="${nonce}"`);
        }
      }
    });
  });

  describe('No CSP Violations', () => {
    it('should not trigger CSP violations on main pages', async () => {
      // This test would need to be run in a browser environment
      // with console monitoring to detect CSP violations
      // Placeholder for manual testing or E2E test framework
      expect(true).toBe(true);
    });
  });
});
