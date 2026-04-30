import '@testing-library/jest-dom';

// Mock react-joyride to avoid issues with DOM animations in tests
jest.mock('react-joyride', () => {
  const React = require('react');
  return function MockJoyride(props) {
    return React.createElement('div', {
      'data-testid': 'mock-joyride',
      'data-run': props?.run ? 'true' : 'false',
    });
  };
});

// Mock Next.js router
jest.mock('next/navigation', () => ({
  useRouter() {
    return {
      push: jest.fn(),
      replace: jest.fn(),
      prefetch: jest.fn(),
      back: jest.fn(),
    };
  },
  usePathname() {
    return '';
  },
  useSearchParams() {
    return new URLSearchParams();
  },
}));

// Mock Next.js headers
jest.mock('next/headers', () => ({
  headers() {
    return {
      get: jest.fn((name) => {
        if (name === 'x-nonce') return 'test-nonce-123456';
        return null;
      }),
    };
  },
}));

// Provide web APIs used by tests/middleware in jsdom.
if (typeof global.Request === 'undefined' && typeof window !== 'undefined' && window.Request) {
  global.Request = window.Request;
}

if (typeof global.Response === 'undefined' && typeof window !== 'undefined' && window.Response) {
  global.Response = window.Response;
}

if (typeof global.Headers === 'undefined' && typeof window !== 'undefined' && window.Headers) {
  global.Headers = window.Headers;
}

if (typeof global.fetch === 'undefined') {
  global.fetch = jest.fn(async () => {
    const nonce = 'test-nonce-123456';
    const headers = new Headers({
      'X-Frame-Options': 'DENY',
      'X-Content-Type-Options': 'nosniff',
      'Referrer-Policy': 'strict-origin-when-cross-origin',
      'Permissions-Policy': 'camera=(), microphone=(), geolocation=(), payment=(), usb=(), interest-cohort=()',
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Resource-Policy': 'same-origin',
      'Content-Security-Policy': `default-src 'self'; script-src 'self' 'nonce-${nonce}'`,
    });

    return {
      ok: true,
      headers,
      text: async () => `<html><body><script nonce="${nonce}"></script></body></html>`,
      json: async () => ({}),
    };
  });
}

if (typeof global.IntersectionObserver === 'undefined') {
  class IntersectionObserverMock {
    observe() {}
    unobserve() {}
    disconnect() {}
  }

  global.IntersectionObserver = IntersectionObserverMock;
}
