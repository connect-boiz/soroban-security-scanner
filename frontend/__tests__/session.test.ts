import { persistSession, loadSession, clearSession } from '../lib/auth/session';

/**
 * The client session module now delegates to the server-side session
 * endpoint (httpOnly cookie) instead of client storages. The token must
 * never be written to localStorage/sessionStorage, so these tests assert
 * the fetch contract against `/api/auth/session` and verify that client
 * storages stay untouched.
 */

const SESSION_ENDPOINT = '/api/auth/session';

function mockFetchResponse(body: unknown, ok = true, status = ok ? 200 : 401) {
  return Promise.resolve({
    ok,
    status,
    json: () => Promise.resolve(body),
  } as Response);
}

let fetchMock: jest.Mock;

beforeEach(() => {
  window.localStorage.clear();
  window.sessionStorage.clear();
  fetchMock = jest.fn();
  globalThis.fetch = fetchMock as unknown as typeof fetch;
});

afterEach(() => {
  delete (globalThis as { fetch?: typeof fetch }).fetch;
});

describe('session persistence (server-backed httpOnly cookie)', () => {
  it('persistSession POSTs the user and rememberMe flag to the session endpoint', async () => {
    fetchMock.mockResolvedValueOnce(mockFetchResponse({ ok: true, user: { email: 'a@b.c' } }));

    await persistSession({ email: 'demo@example.com' }, true);

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe(SESSION_ENDPOINT);
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body)).toEqual({
      user: { email: 'demo@example.com' },
      rememberMe: true,
    });
  });

  it('never writes the session token to localStorage or sessionStorage', async () => {
    fetchMock.mockResolvedValueOnce(mockFetchResponse({ ok: true, user: { email: 'a@b.c' } }));

    await persistSession({ email: 'demo@example.com' }, true);

    expect(window.localStorage.length).toBe(0);
    expect(window.sessionStorage.length).toBe(0);
  });

  it('throws when the server rejects a session write', async () => {
    fetchMock.mockResolvedValueOnce(mockFetchResponse({ error: 'bad' }, false, 400));

    await expect(persistSession({ email: 'demo@example.com' }, false)).rejects.toThrow(
      'Failed to establish a server-side session'
    );
  });

  it('loadSession returns the user from the server when a session exists', async () => {
    fetchMock.mockResolvedValueOnce(
      mockFetchResponse({ user: { email: 'demo@example.com', name: 'Demo' } })
    );

    const session = await loadSession<{ email: string }>();
    expect(session).toEqual({ email: 'demo@example.com', name: 'Demo' });
    expect(fetchMock).toHaveBeenCalledWith(SESSION_ENDPOINT, { method: 'GET' });
  });

  it('loadSession returns null when the server reports no session', async () => {
    fetchMock.mockResolvedValueOnce(mockFetchResponse({ user: null }, false, 401));

    expect(await loadSession()).toBeNull();
  });

  it('clearSession DELETEs the server-side session', async () => {
    fetchMock.mockResolvedValueOnce(mockFetchResponse({ ok: true }));

    await clearSession();
    expect(fetchMock).toHaveBeenCalledWith(SESSION_ENDPOINT, { method: 'DELETE' });
  });

  it('does not read a forged entry from client storages', async () => {
    // Even if an attacker planted a token in storage, loadSession must only
    // trust the server.
    window.localStorage.setItem('soroban_auth_session', JSON.stringify({ user: { email: 'x' } }));
    fetchMock.mockResolvedValueOnce(mockFetchResponse({ user: null }, false, 401));

    expect(await loadSession()).toBeNull();
  });
});
