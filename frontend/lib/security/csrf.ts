/**
 * CSRF protection primitives.
 *
 * Two independent layers, both enforced in middleware for state-changing
 * requests (POST, PUT, PATCH, DELETE):
 *
 * 1. Origin verification: the request must come from this site, judged by
 *    `Sec-Fetch-Site` when the browser sends it, otherwise by comparing the
 *    `Origin` (or `Referer`) host with the request host.
 * 2. Double-submit cookie: middleware issues a random token in a
 *    `SameSite=Strict` cookie, and mutating `/api/` requests must echo that
 *    token in the `x-csrf-token` header. A cross-site page cannot read the
 *    cookie, so it cannot forge the header.
 *
 * Client code attaches the header with `withCsrfHeader()`.
 */

export const CSRF_COOKIE_NAME = 'csrf_token';
export const CSRF_HEADER_NAME = 'x-csrf-token';

const TOKEN_BYTES = 32;
const SAFE_METHODS = new Set(['GET', 'HEAD', 'OPTIONS', 'TRACE']);

export function isStateChangingMethod(method: string | undefined): boolean {
  return !!method && !SAFE_METHODS.has(method.toUpperCase());
}

/** Random URL-safe token generated with the Web Crypto API (Edge compatible). */
export function generateCsrfToken(): string {
  const bytes = new Uint8Array(TOKEN_BYTES);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, b => b.toString(16).padStart(2, '0')).join('');
}

/** Constant-time comparison so token checks do not leak timing information. */
export function tokensMatch(a: string | null | undefined, b: string | null | undefined): boolean {
  if (!a || !b || a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) {
    diff |= a.charCodeAt(i) ^ b.charCodeAt(i);
  }
  return diff === 0;
}

function hostOf(url: string | null): string | null {
  if (!url) return null;
  try {
    return new URL(url).host;
  } catch {
    return null;
  }
}

/**
 * Whether a request originates from this site.
 *
 * `Sec-Fetch-Site` is authoritative when present. Without it, an `Origin`
 * or `Referer` host must match `Host`. A request carrying none of these
 * headers is rejected: modern browsers always send `Origin` on cross-origin
 * and same-origin mutating requests.
 */
export function isSameOriginRequest(headers: Headers): boolean {
  const fetchSite = headers.get('sec-fetch-site');
  if (fetchSite) {
    return fetchSite === 'same-origin' || fetchSite === 'none';
  }

  const host = headers.get('x-forwarded-host') ?? headers.get('host');
  if (!host) return false;

  const sourceHost = hostOf(headers.get('origin')) ?? hostOf(headers.get('referer'));
  return sourceHost === host;
}

/** Read the CSRF token from `document.cookie` (browser only). */
export function readCsrfToken(): string | null {
  if (typeof document === 'undefined') return null;
  const prefix = `${CSRF_COOKIE_NAME}=`;
  const entry = document.cookie.split('; ').find(part => part.startsWith(prefix));
  return entry ? decodeURIComponent(entry.slice(prefix.length)) : null;
}

/** Return fetch init with the CSRF header added. Does not mutate `init`. */
export function withCsrfHeader(init: RequestInit = {}): RequestInit {
  const headers = new Headers(init.headers);
  const token = readCsrfToken();
  if (token) headers.set(CSRF_HEADER_NAME, token);
  return { ...init, headers };
}
