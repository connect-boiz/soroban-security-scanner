/**
 * Shared session-cookie helpers used by the server-side route handler
 * (`app/api/auth/session/route.ts`) and its tests.
 *
 * These are kept in a separate module so the route file only exports the
 * HTTP method handlers that Next.js App Router expects — exporting extra
 * named symbols from a route file causes a build-time error.
 */

export const SESSION_COOKIE_NAME = 'soroban_auth_session';

const SESSION_MAX_AGE_SECONDS = 60 * 60 * 24 * 7; // 7 days

export interface SessionUser {
  email: string;
  name: string;
  verified?: boolean;
}

export function isSessionUser(value: unknown): value is SessionUser {
  if (typeof value !== 'object' || value === null) return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.email === 'string' &&
    candidate.email.length > 0 &&
    typeof candidate.name === 'string'
  );
}

/**
 * Build the httpOnly session cookie options. `value` is empty for deletion.
 */
export function buildSessionCookie(value: string) {
  return {
    name: SESSION_COOKIE_NAME as string,
    value,
    httpOnly: true,
    secure: process.env.NODE_ENV === 'production',
    sameSite: 'lax' as const,
    path: '/',
    maxAge: value === '' ? 0 : SESSION_MAX_AGE_SECONDS,
  };
}
