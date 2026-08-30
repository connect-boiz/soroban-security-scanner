import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import AuthContainer from './AuthContainer';

const SESSION_ENDPOINT = '/api/auth/session';

let fetchMock: jest.Mock;

function mockFetchSuccess(body: unknown = { ok: true }) {
  return Promise.resolve({
    ok: true,
    status: 200,
    json: () => Promise.resolve(body),
  } as Response);
}

async function loginAndCompleteMfa(rememberMe: boolean) {
  const user = userEvent.setup();
  render(<AuthContainer />);

  await user.type(screen.getByLabelText(/email address/i), 'user@example.com');
  await user.type(screen.getByLabelText(/^password$/i), 'correct horse battery staple');

  if (rememberMe) {
    await user.click(screen.getByLabelText(/remember me/i));
  }

  await user.click(screen.getByRole('button', { name: /sign in/i }));

  // The mock login always requires MFA before a session is established.
  const codeInput = await screen.findByLabelText(/verification code/i, {}, { timeout: 3000 });

  // Demo mode surfaces the one-time code the mock "sent"; use that code
  // to complete MFA (there is no fixed/known code anymore).
  const demoHint = await screen.findByText(/demo mode/i, {}, { timeout: 3000 });
  const sentCode = demoHint.textContent?.match(/\b\d{6}\b/)?.[0];
  expect(sentCode).toBeDefined();

  await user.type(codeInput, sentCode as string);
  await user.click(screen.getByRole('button', { name: /verify code/i }));

  await waitFor(() => expect(screen.getByText(/authentication successful/i)).toBeInTheDocument(), {
    timeout: 3000,
  });
}

describe('AuthContainer - rememberMe session persistence', () => {
  beforeEach(() => {
    window.localStorage.clear();
    window.sessionStorage.clear();
    fetchMock = jest.fn().mockImplementation(() => mockFetchSuccess());
    globalThis.fetch = fetchMock as unknown as typeof fetch;
  });

  afterEach(() => {
    delete (globalThis as { fetch?: typeof fetch }).fetch;
  });

  it('does not persist a session until authentication (including MFA) fully completes', async () => {
    const user = userEvent.setup();
    render(<AuthContainer />);

    await user.type(screen.getByLabelText(/email address/i), 'user@example.com');
    await user.type(screen.getByLabelText(/^password$/i), 'correct horse battery staple');
    await user.click(screen.getByRole('button', { name: /sign in/i }));

    await screen.findByLabelText(/verification code/i, {}, { timeout: 3000 });

    expect(window.localStorage.getItem('soroban_auth_session')).toBeNull();
    expect(window.sessionStorage.getItem('soroban_auth_session')).toBeNull();
  });

  it('POSTs to the server session endpoint when rememberMe is checked (no client storage)', async () => {
    await loginAndCompleteMfa(true);

    // The server-backed session must never write to client storage.
    expect(window.localStorage.length).toBe(0);
    expect(window.sessionStorage.length).toBe(0);

    // Verify persistSession POSTed to the session endpoint with rememberMe=true.
    const sessionCalls = fetchMock.mock.calls.filter(
      ([url, init]: [string, RequestInit]) => url === SESSION_ENDPOINT && init?.method === 'POST'
    );
    expect(sessionCalls.length).toBe(1);
    const [, init] = sessionCalls[0];
    const body = JSON.parse(init.body as string);
    expect(body.rememberMe).toBe(true);
    expect(body.user).toBeDefined();
    expect(body.user.email).toBe('user@example.com');
  }, 10000);

  it('POSTs to the server session endpoint when rememberMe is unchecked (no client storage)', async () => {
    await loginAndCompleteMfa(false);

    // The server-backed session must never write to client storage.
    expect(window.localStorage.length).toBe(0);
    expect(window.sessionStorage.length).toBe(0);

    // Verify persistSession POSTed to the session endpoint with rememberMe=false.
    const sessionCalls = fetchMock.mock.calls.filter(
      ([url, init]: [string, RequestInit]) => url === SESSION_ENDPOINT && init?.method === 'POST'
    );
    expect(sessionCalls.length).toBe(1);
    const [, init] = sessionCalls[0];
    const body = JSON.parse(init.body as string);
    expect(body.rememberMe).toBe(false);
    expect(body.user).toBeDefined();
    expect(body.user.email).toBe('user@example.com');
  }, 10000);
});
