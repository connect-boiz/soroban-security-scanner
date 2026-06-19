/**
 * Security Headers Test Suite
 *
 * Tests all security headers implementation including:
 * - CSP with nonces
 * - HSTS
 * - X-Frame-Options
 * - X-Content-Type-Options
 * - Referrer-Policy
 * - Permissions-Policy
 * - Cross-Origin policies
 */

import { NextResponse } from 'next/server';
import { middleware } from '../middleware';

// Mock crypto for testing
jest.mock('crypto', () => ({
  randomBytes: jest.fn(() => ({
    toString: () => 'test-nonce-123456',
  })),
}));

describe('Security Headers Middleware', () => {
  let mockRequest: any;

  beforeEach(() => {
    mockRequest = {
      headers: new Headers(),
    };
  });

  describe('Content Security Policy (CSP)', () => {
    it('should set CSP header on all responses', () => {
      const response = middleware(mockRequest);

      const cspHeader =
        response.headers.get('Content-Security-Policy') ||
        response.headers.get('Content-Security-Policy-Report-Only');

      expect(cspHeader).toBeTruthy();
    });

    it('should include nonce in script-src directive', () => {
      const response = middleware(mockRequest);

      const cspHeader =
        response.headers.get('Content-Security-Policy') ||
        response.headers.get('Content-Security-Policy-Report-Only');

      expect(cspHeader).toContain("'nonce-");
      expect(cspHeader).toContain('script-src');
    });

    it('should not include unsafe-eval', () => {
      const response = middleware(mockRequest);

      const cspHeader =
        response.headers.get('Content-Security-Policy') ||
        response.headers.get('Content-Security-Policy-Report-Only');

      expect(cspHeader).not.toContain('unsafe-eval');
    });

    it('should include all required CSP directives', () => {
      const response = middleware(mockRequest);

      const cspHeader =
        response.headers.get('Content-Security-Policy') ||
        response.headers.get('Content-Security-Policy-Report-Only');

      const requiredDirectives = [
        'default-src',
        'script-src',
        'style-src',
        'img-src',
        'font-src',
        'connect-src',
        'frame-src',
        'frame-ancestors',
        'object-src',
        'base-uri',
        'form-action',
      ];

      requiredDirectives.forEach(directive => {
        expect(cspHeader).toContain(directive);
      });
    });

    it('should allow Stellar Horizon endpoints in connect-src', () => {
      const response = middleware(mockRequest);

      const cspHeader =
        response.headers.get('Content-Security-Policy') ||
        response.headers.get('Content-Security-Policy-Report-Only');

      expect(cspHeader).toContain('https://horizon.stellar.org');
      expect(cspHeader).toContain('https://horizon-testnet.stellar.org');
      expect(cspHeader).toContain('https://horizon-futurenet.stellar.org');
    });

    it('should set frame-ancestors to none', () => {
      const response = middleware(mockRequest);

      const cspHeader =
        response.headers.get('Content-Security-Policy') ||
        response.headers.get('Content-Security-Policy-Report-Only');

      expect(cspHeader).toContain("frame-ancestors 'none'");
    });

    it('should include upgrade-insecure-requests', () => {
      const response = middleware(mockRequest);

      const cspHeader =
        response.headers.get('Content-Security-Policy') ||
        response.headers.get('Content-Security-Policy-Report-Only');

      expect(cspHeader).toContain('upgrade-insecure-requests');
    });
  });

  describe('CSP Nonce Generation', () => {
    it('should generate unique nonce per request', () => {
      // Mock crypto to return different values
      const crypto = require('crypto');
      let callCount = 0;
      crypto.randomBytes.mockImplementation(() => ({
        toString: () => `nonce-${callCount++}`,
      }));

      const nonces = new Set();

      for (let i = 0; i < 100; i++) {
        const response = middleware(mockRequest);
        const nonce = response.headers.get('x-nonce');
        nonces.add(nonce);
      }

      // All nonces should be unique
      expect(nonces.size).toBe(100);
    });

    it('should store nonce in x-nonce header', () => {
      const response = middleware(mockRequest);
      const nonce = response.headers.get('x-nonce');

      expect(nonce).toBeTruthy();
      expect(typeof nonce).toBe('string');
    });

    it('should include same nonce in CSP header and x-nonce header', () => {
      const response = middleware(mockRequest);

      const nonce = response.headers.get('x-nonce');
      const cspHeader =
        response.headers.get('Content-Security-Policy') ||
        response.headers.get('Content-Security-Policy-Report-Only');

      expect(cspHeader).toContain(`'nonce-${nonce}'`);
    });
  });

  describe('HTTP Strict Transport Security (HSTS)', () => {
    it('should set HSTS header in production', () => {
      (process.env as any).NODE_ENV = 'production';
      const response = middleware(mockRequest);
      const hstsHeader = response.headers.get('Strict-Transport-Security');
      expect(hstsHeader).toBeTruthy();
      expect(hstsHeader).toContain('max-age=31536000');
    });

    it('should include includeSubDomains directive', () => {
      (process.env as any).NODE_ENV = 'production';
      const response = middleware(mockRequest);
      const hstsHeader = response.headers.get('Strict-Transport-Security');
      expect(hstsHeader).toContain('includeSubDomains');
    });

    it('should include preload directive', () => {
      (process.env as any).NODE_ENV = 'production';
      const response = middleware(mockRequest);
      const hstsHeader = response.headers.get('Strict-Transport-Security');
      expect(hstsHeader).toContain('preload');
    });

    it('should not set HSTS in development', () => {
      (process.env as any).NODE_ENV = 'development';
      const response = middleware(mockRequest);
      const hstsHeader = response.headers.get('Strict-Transport-Security');
      expect(hstsHeader).toBeNull();
    });
  });

  describe('X-Frame-Options', () => {
    it('should set X-Frame-Options to DENY', () => {
      const response = middleware(mockRequest);
      const xFrameOptions = response.headers.get('X-Frame-Options');

      expect(xFrameOptions).toBe('DENY');
    });
  });

  describe('X-Content-Type-Options', () => {
    it('should set X-Content-Type-Options to nosniff', () => {
      const response = middleware(mockRequest);
      const xContentTypeOptions = response.headers.get('X-Content-Type-Options');

      expect(xContentTypeOptions).toBe('nosniff');
    });
  });

  describe('Referrer-Policy', () => {
    it('should set Referrer-Policy to strict-origin-when-cross-origin', () => {
      const response = middleware(mockRequest);
      const referrerPolicy = response.headers.get('Referrer-Policy');

      expect(referrerPolicy).toBe('strict-origin-when-cross-origin');
    });
  });

  describe('Permissions-Policy', () => {
    it('should set Permissions-Policy header', () => {
      const response = middleware(mockRequest);
      const permissionsPolicy = response.headers.get('Permissions-Policy');

      expect(permissionsPolicy).toBeTruthy();
    });

    it('should disable all unused browser features', () => {
      const response = middleware(mockRequest);
      const permissionsPolicy = response.headers.get('Permissions-Policy');

      const disabledFeatures = [
        'camera=()',
        'microphone=()',
        'geolocation=()',
        'payment=()',
        'usb=()',
        'interest-cohort=()', // FLoC opt-out
      ];

      disabledFeatures.forEach(feature => {
        expect(permissionsPolicy).toContain(feature);
      });
    });
  });

  describe('Cross-Origin Policies', () => {
    it('should set Cross-Origin-Opener-Policy to same-origin', () => {
      const response = middleware(mockRequest);
      const coop = response.headers.get('Cross-Origin-Opener-Policy');

      expect(coop).toBe('same-origin');
    });

    it('should set Cross-Origin-Resource-Policy to same-origin', () => {
      const response = middleware(mockRequest);
      const corp = response.headers.get('Cross-Origin-Resource-Policy');

      expect(corp).toBe('same-origin');
    });

    it('should not set Cross-Origin-Embedder-Policy', () => {
      const response = middleware(mockRequest);
      const coep = response.headers.get('Cross-Origin-Embedder-Policy');

      // COEP is not set because it would block Stellar Horizon API calls
      expect(coep).toBeNull();
    });
  });

  describe('Environment-Specific Behavior', () => {
    it('should use CSP report-only mode in development', () => {
      (process.env as any).NODE_ENV = 'development';
      const response = middleware(mockRequest);
      expect(response.headers.get('Content-Security-Policy-Report-Only')).toBeTruthy();
      expect(response.headers.get('Content-Security-Policy')).toBeNull();
    });

    it('should use enforcing CSP mode in production', () => {
      (process.env as any).NODE_ENV = 'production';
      const response = middleware(mockRequest);
      expect(response.headers.get('Content-Security-Policy')).toBeTruthy();
      expect(response.headers.get('Content-Security-Policy-Report-Only')).toBeNull();
    });
  });

  describe('All Security Headers Present', () => {
    it('should set all required security headers', () => {
      const response = middleware(mockRequest);

      const requiredHeaders = [
        'X-Frame-Options',
        'X-Content-Type-Options',
        'Referrer-Policy',
        'Permissions-Policy',
        'Cross-Origin-Opener-Policy',
        'Cross-Origin-Resource-Policy',
        'x-nonce',
      ];

      requiredHeaders.forEach(header => {
        expect(response.headers.get(header)).toBeTruthy();
      });
    });
  });
});
