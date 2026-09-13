import { middleware } from '../middleware';
import {
  CSRF_COOKIE_NAME,
  CSRF_HEADER_NAME,
  generateCsrfToken,
  isSameOriginRequest,
  isStateChangingMethod,
  tokensMatch,
} from '../lib/security/csrf';

const SITE = 'https://app.example.com';

// Minimal NextRequest stub, matching security-headers.test.ts (the jest
// polyfills in this repo do not support constructing a real NextRequest).
function request(path: string, init: { method?: string; headers?: Record<string, string> } = {}) {
  const headers = new Headers({ host: 'app.example.com', ...init.headers });
  const cookies = new Map(
    (headers.get('cookie') ?? '')
      .split('; ')
      .filter(Boolean)
      .map(pair => pair.split('=') as [string, string])
  );
  return {
    method: init.method ?? 'GET',
    headers,
    nextUrl: { pathname: path },
    cookies: { get: (name: string) => (cookies.has(name) ? { name, value: cookies.get(name) } : undefined) },
  } as any;
}

describe('csrf helpers', () => {
  it('treats only unsafe methods as state-changing', () => {
    expect(isStateChangingMethod('GET')).toBe(false);
    expect(isStateChangingMethod('head')).toBe(false);
    expect(isStateChangingMethod('POST')).toBe(true);
    expect(isStateChangingMethod('delete')).toBe(true);
    expect(isStateChangingMethod(undefined)).toBe(false);
  });

  it('generates distinct 64-char hex tokens', () => {
    const a = generateCsrfToken();
    expect(a).toMatch(/^[0-9a-f]{64}$/);
    expect(generateCsrfToken()).not.toBe(a);
  });

  it('compares tokens exactly', () => {
    expect(tokensMatch('abc', 'abc')).toBe(true);
    expect(tokensMatch('abc', 'abd')).toBe(false);
    expect(tokensMatch('abc', 'abcd')).toBe(false);
    expect(tokensMatch(null, 'abc')).toBe(false);
    expect(tokensMatch('', '')).toBe(false);
  });

  it('prefers Sec-Fetch-Site when present', () => {
    expect(isSameOriginRequest(new Headers({ 'sec-fetch-site': 'same-origin' }))).toBe(true);
    expect(isSameOriginRequest(new Headers({ 'sec-fetch-site': 'cross-site', origin: SITE, host: 'app.example.com' }))).toBe(false);
    expect(isSameOriginRequest(new Headers({ 'sec-fetch-site': 'same-site' }))).toBe(false);
  });

  it('falls back to Origin or Referer host matching', () => {
    expect(isSameOriginRequest(new Headers({ host: 'app.example.com', origin: SITE }))).toBe(true);
    expect(isSameOriginRequest(new Headers({ host: 'app.example.com', origin: 'https://evil.test' }))).toBe(false);
    expect(isSameOriginRequest(new Headers({ host: 'app.example.com', referer: `${SITE}/login` }))).toBe(true);
    expect(isSameOriginRequest(new Headers({ host: 'app.example.com' }))).toBe(false);
    expect(isSameOriginRequest(new Headers({ host: 'app.example.com', origin: 'null' }))).toBe(false);
  });
});

describe('middleware CSRF enforcement', () => {
  it('lets safe requests through and issues a SameSite=Strict token cookie', () => {
    const res = middleware(request('/login'));
    expect(res.status).toBe(200);
    const cookie = res.cookies.get(CSRF_COOKIE_NAME);
    expect(cookie?.value).toMatch(/^[0-9a-f]{64}$/);
    expect(cookie?.sameSite).toBe('strict');
    expect(cookie?.httpOnly).toBe(false);
  });

  it('does not reissue the cookie when one already exists', () => {
    const res = middleware(request('/login', { headers: { cookie: `${CSRF_COOKIE_NAME}=existing` } }));
    expect(res.cookies.get(CSRF_COOKIE_NAME)).toBeUndefined();
  });

  it('blocks cross-site form posts', () => {
    const res = middleware(request('/login', { method: 'POST', headers: { origin: 'https://evil.test' } }));
    expect(res.status).toBe(403);
  });

  it('allows same-origin posts to pages', () => {
    const res = middleware(request('/login', { method: 'POST', headers: { origin: SITE } }));
    expect(res.status).toBe(200);
  });

  it('requires a matching token header on mutating API requests', () => {
    const base = { origin: SITE, cookie: `${CSRF_COOKIE_NAME}=tok123` };
    expect(middleware(request('/api/scan', { method: 'POST', headers: base })).status).toBe(403);
    expect(
      middleware(request('/api/scan', { method: 'POST', headers: { ...base, [CSRF_HEADER_NAME]: 'wrong1' } })).status
    ).toBe(403);
    expect(
      middleware(request('/api/scan', { method: 'POST', headers: { ...base, [CSRF_HEADER_NAME]: 'tok123' } })).status
    ).toBe(200);
  });
});
