/**
 * Client-side session persistence for the auth flows in `components/auth`.
 *
 * Previously the "Remember me" flag chose between `localStorage` and
 * `sessionStorage` for the session token, and both storages are fully
 * readable and writable by client-side JavaScript. That meant any user
 * could trivially forge a session by editing client state, and protected
 * pages were not actually protected (see issue #496).
 *
 * The session is now owned by the server. The token lives in an
 * `httpOnly` cookie that client scripts cannot read or write, and it is
 * established/refreshed/cleared only through the server-side endpoint
 * `app/api/auth/session` (wired into the middleware route guard). All of
 * the functions in this module delegate to that endpoint, so there is no
 * client-accessible token to forge.
 *
 *   - persisSession(user, rememberMe) -> POST /api/auth/session to set the httpOnly cookie
 *   - loadSession()                  -> GET /api/auth/session to read the session from the server
 *   - clearSession()                 -> DELETE /api/auth/session to clear the httpOnly cookie
 */

export interface SessionUser {
  email: string;
  name: string;
  verified?: boolean;
}

/** Shape of the authoritative session returned by the server endpoint. */
export interface AuthSession {
  user: SessionUser | null;
}

const SESSION_ENDPOINT = '/api/auth/session';

/**
 * Persist a session for the given user by asking the server to set an
 * httpOnly cookie. `rememberMe` is forwarded so a future backend can apply
 * a different expiry; the important guarantee is that the value itself is
 * never readable or writable from the client.
 */
export async function persistSession<T = SessionUser>(user: T, rememberMe: boolean): Promise<void> {
  const response = await fetch(SESSION_ENDPOINT, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ user, rememberMe }),
  });

  if (!response.ok) {
    throw new Error('Failed to establish a server-side session');
  }
}

/**
 * Load the current session from the server. Returns the session user, or
 * `null` when no valid httpOnly session cookie is present.
 */
export async function loadSession<T = SessionUser>(): Promise<T | null> {
  const response = await fetch(SESSION_ENDPOINT, { method: 'GET' });

  if (!response.ok) {
    return null;
  }

  const data = (await response.json()) as { user?: T | null };
  return data.user ?? null;
}

/** Clear the server-side session by asking the server to delete the cookie. */
export async function clearSession(): Promise<void> {
  await fetch(SESSION_ENDPOINT, { method: 'DELETE' });
}
