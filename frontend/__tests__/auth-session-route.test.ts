/**
 * Tests for the server-side session route handler (app/api/auth/session).
 *
 * The session is stored in an `httpOnly`, `SameSite=Lax` cookie that client
 * scripts cannot read or write — the core fix for the client-forgery issue.
 *
 * Cookie security attributes are pure data produced by `buildSessionCookie`,
 * and the session-parsing logic is pure, so both are asserted directly. The
 * handlers' status codes and JSON bodies are asserted via GET/POST/DELETE
 * where they do not depend on jsdom's `Headers` implementation.
 */
import { ResponseCookies } from 'next/dist/compiled/@edge-runtime/cookies';
import { GET, POST, DELETE } from '../app/api/auth/session/route';
import { SESSION_COOKIE_NAME, buildSessionCookie } from '../lib/auth/session-cookie';

const VALID_USER = { email: 'demo@example.com', name: 'Demo' };

import { NextResponse } from 'next/server';

// jsdom does not implement the parts of the fetch `Response` API that Next's
// NextResponse relies on (no static `Response.json`, no readable body, no
// working instance `.json()`). To exercise the handlers under jsdom we stub
// `NextResponse.json` itself: the handler's real validation and cookie logic
// still run, and the response carries a readable body so assertions work.
type BodyResponse = Response & { _readableBody?: string };

let jsonSpy: jest.SpyInstance;

beforeAll(() => {
  jsonSpy = jest
    .spyOn(NextResponse, 'json')
    .mockImplementation(<T>(data: T, init?: ResponseInit) => {
      // Build a NextResponse from the init (carries status/headers), and stash
      // the serialized body so the instance `.json()` polyfill below can return
      // it. Cookies set afterwards land on a real NextResponse `cookies` store.
      const nextResponse = new NextResponse(null, init) as BodyResponse;
      nextResponse._readableBody = JSON.stringify(data);
      return nextResponse;
    });

  if (typeof Response.prototype.json !== 'function') {
    Object.defineProperty(Response.prototype, 'json', {
      configurable: true,
      async value(this: Response) {
        const body = (this as BodyResponse)._readableBody;
        return body ? JSON.parse(body) : {};
      },
    });
  }
});

afterAll(() => {
  jsonSpy.mockRestore();
});

// Next's ResponseCookies.set() drives the underlying Headers, which jsdom's
// polyfill does not fully implement. Mock the setter on the shared prototype
// so the handlers' cookie-write intent can still be asserted without crashing.
let writeCookie: jest.Mock;

beforeAll(() => {
  writeCookie = jest.spyOn(ResponseCookies.prototype, 'set').mockImplementation(function (
    this: unknown,
    name: string,
    value: string,
    options?: Record<string, unknown>
  ) {
    return this as unknown as ResponseCookies;
  }) as unknown as jest.Mock;
});

afterAll(() => {
  (writeCookie as unknown as jest.SpyInstance).mockRestore();
});

describe('buildSessionCookie', () => {
  it('produces an httpOnly, SameSite=Lax cookie for the session', () => {
    const cookie = buildSessionCookie(JSON.stringify(VALID_USER));

    expect(cookie.name).toBe(SESSION_COOKIE_NAME);
    expect(cookie.value).toBe(JSON.stringify(VALID_USER));
    expect(cookie.httpOnly).toBe(true);
    expect(cookie.sameSite).toBe('lax');
    expect(cookie.path).toBe('/');
    expect(cookie.maxAge).toBeGreaterThan(0);
  });

  it('produces a positive max-age for an active session', () => {
    expect(buildSessionCookie('x').maxAge).toBeGreaterThan(0);
  });

  it('clears the cookie (maxAge 0, empty value) on deletion', () => {
    const cookie = buildSessionCookie('');
    expect(cookie.value).toBe('');
    expect(cookie.maxAge).toBe(0);
    expect(cookie.httpOnly).toBe(true);
  });
});

describe('session route handler', () => {
  describe('GET', () => {
    it('returns 401 when no session cookie is present', async () => {
      const request = { cookies: { get: () => undefined } } as any;
      const response = await GET(request);
      expect(response.status).toBe(401);
      expect(await response.json()).toEqual({ user: null });
    });

    it('returns the session user when a valid cookie is present', async () => {
      const request = {
        cookies: {
          get: () => ({ value: JSON.stringify(VALID_USER) }),
        },
      } as any;
      const response = await GET(request);
      expect(response.status).toBe(200);
      expect(await response.json()).toEqual({ user: VALID_USER });
    });

    it('treats a corrupt cookie value as no session', async () => {
      const request = { cookies: { get: () => ({ value: '{not-json' }) } } as any;
      const response = await GET(request);
      expect(response.status).toBe(401);
    });

    it('rejects values that are not a valid user payload', async () => {
      const request = {
        cookies: { get: () => ({ value: encodeURIComponent(JSON.stringify({ foo: 1 })) }) },
      } as any;
      const response = await GET(request);
      expect(response.status).toBe(401);
    });
  });

  describe('POST', () => {
    beforeEach(() => (writeCookie as unknown as jest.SpyInstance).mockClear());

    it('writes an httpOnly SameSite=Lax session cookie for a valid user', async () => {
      const request = {
        json: () => Promise.resolve({ user: VALID_USER, rememberMe: true }),
      } as any;
      const response = await POST(request);
      expect(response.status).toBe(200);
      expect(await response.json()).toEqual({ ok: true, user: VALID_USER });

      const [name, value, options] = writeCookie.mock.calls[0];
      expect(name).toBe(SESSION_COOKIE_NAME);
      expect(JSON.parse(value)).toEqual(VALID_USER);
      expect(options.httpOnly).toBe(true);
      expect(options.sameSite).toBe('lax');
      expect(options.path).toBe('/');
      expect(options.maxAge).toBeGreaterThan(0);
    });

    it('rejects a request without a valid user payload', async () => {
      const request = { json: () => Promise.resolve({ user: { nope: true } }) } as any;
      const response = await POST(request);
      expect(response.status).toBe(400);
      expect(writeCookie).not.toHaveBeenCalled();
    });

    it('rejects malformed JSON', async () => {
      const request = {
        json: () => Promise.reject(new Error('Invalid body')),
      } as any;
      const response = await POST(request);
      expect(response.status).toBe(400);
    });
  });

  describe('DELETE', () => {
    beforeEach(() => (writeCookie as unknown as jest.SpyInstance).mockClear());

    it('clears the session cookie with maxAge 0', async () => {
      const response = await DELETE();
      expect(response.status).toBe(200);
      expect(await response.json()).toEqual({ ok: true });

      const [name, value, options] = writeCookie.mock.calls[0];
      expect(name).toBe(SESSION_COOKIE_NAME);
      expect(value).toBe('');
      expect(options.maxAge).toBe(0);
      expect(options.httpOnly).toBe(true);
    });
  });
});
