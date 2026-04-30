import '@testing-library/jest-dom';

// Mock react-joyride to avoid issues with DOM animations in tests
jest.mock('react-joyride', () => {
  return function MockJoyride() {
    return <div data-testid="mock-joyride" />;
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
