/**
 * Tests for the middleware route guard.
 *
 * The middleware both applies security headers and enforces
 * authentication: requests for protected paths are redirected to /auth
 * when no httpOnly session cookie is present, and authenticated visiters of
 * the login page are sent back to the app.
 *
 * Redirect decisions are asserted against `resolveAuthRedirect` (a pure
 * helper) for the exact target URL, while the middleware integration checks
 * confirm a redirect is emitted (307). Reading the `Location` header from a
 * NextResponse is unreliable under jsdom's headers polyfill, so the exact
 * target is covered by the helper and full-header behaviour by E2E.
 */
import { middleware, SESSION_COOKIE_NAME, resolveAuthRedirect } from '../middleware';

jest.mock('crypto', () => ({
  randomBytes: jest.fn(() => ({
    toString: () => 'test-nonce-123456',
  })),
}));

const TEST_ORIGIN = 'http://localhost:3000';

interface CookieMap {
  [key: string]: { value?: string };
}

function buildRequest(pathname: string, cookies: CookieMap = {}) {
  return {
    cookies: {
      get: (name: string) => cookies[name],
    },
    nextUrl: {
      origin: TEST_ORIGIN,
      pathname,
      clone: () => new URL(TEST_ORIGIN + pathname),
    },
    headers: new Headers(),
  } as any;
}

describe('resolveAuthRedirect (guard decision)', () => {
  it('points an unauthenticated protected path to /auth', () => {
    expect(resolveAuthRedirect('/scanner', false, TEST_ORIGIN)).toBe(`${TEST_ORIGIN}/auth`);
  });

  it('points an unauthenticated request for the app root to /auth', () => {
    expect(resolveAuthRedirect('/', false, TEST_ORIGIN)).toBe(`${TEST_ORIGIN}/auth`);
  });

  it('returns null for an authenticated protected path', () => {
    expect(resolveAuthRedirect('/scanner', true, TEST_ORIGIN)).toBeNull();
  });

  it('returns null for an unauthenticated visit to the login page', () => {
    expect(resolveAuthRedirect('/auth', false, TEST_ORIGIN)).toBeNull();
  });

  it('points an authenticated visit to the login page back to the app root', () => {
    expect(resolveAuthRedirect('/auth', true, TEST_ORIGIN)).toBe(`${TEST_ORIGIN}/`);
  });

  it('treats nested auth routes as public', () => {
    expect(resolveAuthRedirect('/auth/forgot-password', false, TEST_ORIGIN)).toBeNull();
  });
});

describe('middleware route guard (integration)', () => {
  it('redirects an unauthenticated request for a protected path', () => {
    const request = buildRequest('/scanner');
    const response = middleware(request);

    expect(response.status).toBe(307);
  });

  it('redirects an unauthenticated request for the root path', () => {
    const request = buildRequest('/');
    const response = middleware(request);

    expect(response.status).toBe(307);
  });

  it('allows a protected path to proceed when a session cookie is present', () => {
    const request = buildRequest('/scanner', { [SESSION_COOKIE_NAME]: { value: 'sess-token' } });
    const response = middleware(request);

    expect(response.status).toBe(200);
    expect(response.headers.get('location')).toBeNull();
  });

  it('allows unauthenticated access to the login page', () => {
    const request = buildRequest('/auth');
    const response = middleware(request);

    expect(response.status).toBe(200);
  });

  it('redirects an authenticated visit to the login page back to the app', () => {
    const request = buildRequest('/auth', { [SESSION_COOKIE_NAME]: { value: 'sess' } });
    const response = middleware(request);

    expect(response.status).toBe(307);
  });

  it('treats a request without a session cookie as unauthenticated', () => {
    const request = buildRequest('/scanner', {});
    const response = middleware(request);

    expect(response.status).toBe(307);
  });

  it('still applies security headers on protected responses', () => {
    const request = buildRequest('/scanner', { [SESSION_COOKIE_NAME]: { value: 'sess' } });
    const response = middleware(request);

    expect(response.headers.get('X-Frame-Options')).toBe('DENY');
    expect(response.headers.get('x-nonce')).toBeTruthy();
  });

  it('keeps header-only behavior for a request stub without routing info (back-compat)', () => {
    const request = {} as any;
    const response = middleware(request);

    // No redirect attempted, healthy headers still applied.
    expect(response.headers.get('x-nonce')).toBeTruthy();
    expect(response.headers.get('X-Frame-Options')).toBe('DENY');
  });
});
