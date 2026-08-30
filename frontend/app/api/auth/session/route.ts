import { NextRequest, NextResponse } from 'next/server';
import {
  SESSION_COOKIE_NAME,
  isSessionUser,
  buildSessionCookie,
} from '../../../../lib/auth/session-cookie';

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

function parseSessionCookie(
  raw: string | undefined
): { email: string; name: string; verified?: boolean } | null {
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
