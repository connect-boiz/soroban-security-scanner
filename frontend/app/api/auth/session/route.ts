import { NextRequest, NextResponse } from 'next/server';

/**
 * Server-side session management for the auth flow.
 *
 * The session is held in an `httpOnly`, `SameSite=Lax` cookie that the
 * browser sends back on every request but JavaScript cannot read or write.
 * Clients persist/load/clear their session exclusively through this endpoint,
 * so the token never touches `localStorage`/`sessionStorage` and cannot be
 * forged by editing client state.
 *
 *   GET    -> reads the current session from the httpOnly cookie (or 401)
 *   POST   -> stores the supplied session in an httpOnly cookie
 *   DELETE -> clears the httpOnly cookie (logout)
 */

export const SESSION_COOKIE_NAME = 'soroban_auth_session';

const SESSION_MAX_AGE_SECONDS = 60 * 60 * 24 * 7; // 7 days

export interface SessionUser {
  email: string;
  name: string;
  verified?: boolean;
}

function isSessionUser(value: unknown): value is SessionUser {
  if (typeof value !== 'object' || value === null) return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.email === 'string' &&
    candidate.email.length > 0 &&
    typeof candidate.name === 'string'
  );
}

function parseSessionCookie(raw: string | undefined): SessionUser | null {
  if (!raw) return null;
  try {
    const parsed: unknown = JSON.parse(raw);
    if (isSessionUser(parsed)) {
      return parsed;
    }
  } catch {
    // Corrupt cookie value — treat as no session.
  }
  return null;
}

/**
 * Build the httpOnly session cookie options. `value` is empty for deletion.
 * Exposed separately so the security attributes are unit-testable.
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

export async function GET(request: NextRequest) {
  const user = parseSessionCookie(request.cookies.get(SESSION_COOKIE_NAME)?.value);

  if (!user) {
    return NextResponse.json({ user: null }, { status: 401 });
  }

  return NextResponse.json({ user });
}

export async function POST(request: NextRequest) {
  let body: unknown;
  try {
    body = await request.json();
  } catch {
    return NextResponse.json({ error: 'Invalid request body' }, { status: 400 });
  }

  const { user, rememberMe } = (body ?? {}) as { user?: unknown; rememberMe?: unknown };
  if (!isSessionUser(user)) {
    return NextResponse.json(
      { error: 'A valid user payload is required to establish a session' },
      { status: 400 }
    );
  }

  void rememberMe; // reserved for expiry differentiation with future backends.

  const options = buildSessionCookie(JSON.stringify(user));
  const response = NextResponse.json({ ok: true, user });
  // httpOnly + SameSite=Lax keep the cookie out of reach of client scripts.
  response.cookies.set(options.name, options.value, options);

  return response;
}

export async function DELETE() {
  const options = buildSessionCookie('');
  const response = NextResponse.json({ ok: true });
  // Empty value with maxAge 0 instructs the browser to drop the cookie.
  response.cookies.set(options.name, options.value, options);
  return response;
}
