/**
 * Tests for the auth service boundary (see lib/auth/authService.ts).
 *
 * Focus areas:
 *   - No hardcoded credentials: login accepts any well-formed credentials.
 *   - No fixed MFA code: verification only accepts the one-time code the
 *     mock actually "sent".
 *   - Production guard: mock auth is disabled in production builds unless
 *     explicitly enabled via NEXT_PUBLIC_ENABLE_MOCK_AUTH=true.
 */
import {
  login,
  signUp,
  sendMfaCode,
  resendMfaCode,
  verifyMfa,
  getPendingMfaCode,
  isMockAuthEnabled,
  resetMfaState,
  setMfaCooldownSeconds,
  getMfaStatus,
} from '../lib/auth/authService';

const credentials = {
  email: 'user@example.com',
  password: 'correct horse battery staple',
  rememberMe: false,
};

// process.env.NODE_ENV is typed as read-only; cast to a mutable record
// so the production-build guard can be exercised at runtime.
const mutableEnv = process.env as Record<string, string | undefined>;

beforeEach(() => {
  delete process.env.NEXT_PUBLIC_ENABLE_MOCK_AUTH;
  mutableEnv.NODE_ENV = 'test';
  resetMfaState();
});

describe('authService - no hardcoded credentials', () => {
  it('logs in with arbitrary well-formed credentials', async () => {
    const result = await login(credentials);

    expect(result.requiresMfa).toBe(true);
    expect(result.user.email).toBe('user@example.com');
  }, 10000);

  it('signs up without any demo account checks', async () => {
    const user = await signUp({
      firstName: 'Ada',
      lastName: 'Lovelace',
      email: 'ada@example.com',
      password: 'not-password123',
    });

    expect(user.email).toBe('ada@example.com');
    expect(user.name).toBe('Ada Lovelace');
  }, 10000);
});

describe('authService - MFA uses a generated one-time code', () => {
  it('rejects the previously hardcoded code 123456', async () => {
    await sendMfaCode('totp');

    await expect(verifyMfa('123456', 'totp')).rejects.toThrow('Invalid verification code');
  }, 10000);

  it('rejects verification when no code has been sent', async () => {
    await expect(verifyMfa('000000', 'totp')).rejects.toThrow('Invalid verification code');
  }, 10000);

  it('only accepts the code that was actually sent', async () => {
    await sendMfaCode('totp');
    const sentCode = getPendingMfaCode();

    expect(sentCode).toMatch(/^\d{6}$/);

    await expect(verifyMfa('000000', 'totp')).rejects.toThrow('Invalid verification code');
    await expect(verifyMfa(sentCode as string, 'totp')).resolves.toBeUndefined();
  }, 10000);

  it('invalidates the code after a successful verification', async () => {
    await sendMfaCode('totp');
    const sentCode = getPendingMfaCode();

    await verifyMfa(sentCode as string, 'totp');
    await expect(verifyMfa(sentCode as string, 'totp')).rejects.toThrow(
      'Invalid verification code'
    );
  }, 10000);

  it('generates a new code on resend', async () => {
    await sendMfaCode('totp');
    const firstCode = getPendingMfaCode();

    await resendMfaCode('sms', { force: true });
    const secondCode = getPendingMfaCode();

    expect(secondCode).toMatch(/^\d{6}$/);
    expect(secondCode).not.toBe(firstCode);

    // The first code is no longer valid.
    await expect(verifyMfa(firstCode as string, 'sms')).rejects.toThrow(
      'Invalid verification code'
    );
  }, 10000);
});

describe('authService - MFA rate limiting, throttling, and lockout', () => {
  beforeEach(() => {
    resetMfaState();
  });

  it('enforces resend cooldown throttling', async () => {
    await sendMfaCode('totp');
    // Immediate resend without force should be throttled
    await expect(resendMfaCode('totp')).rejects.toThrow(
      /Please wait \d+s before requesting a new code/
    );
  }, 10000);

  it('allows resend once cooldown is bypassed or reset', async () => {
    await sendMfaCode('totp');
    setMfaCooldownSeconds(0);
    await expect(resendMfaCode('totp')).resolves.toBeUndefined();
  }, 10000);

  it('locks account after 5 failed verification attempts', async () => {
    await sendMfaCode('totp');

    // 4 failed attempts should report remaining attempts
    for (let i = 1; i <= 4; i++) {
      await expect(verifyMfa('999999', 'totp')).rejects.toThrow(
        `Invalid verification code. ${5 - i} attempt(s) remaining.`
      );
    }

    // 5th failed attempt triggers lockout
    await expect(verifyMfa('999999', 'totp')).rejects.toThrow(
      'Too many failed attempts. Account is temporarily locked. Please try again in 15 minutes.'
    );

    // Subsequent verify, send, or resend attempts are blocked
    await expect(verifyMfa('999999', 'totp')).rejects.toThrow(/temporarily locked/);
    await expect(sendMfaCode('totp')).rejects.toThrow(/temporarily locked/);
    await expect(resendMfaCode('totp', { force: true })).rejects.toThrow(/temporarily locked/);
  }, 15000);
});

describe('authService - production guard', () => {
  it('blocks mock auth in production builds by default', async () => {
    mutableEnv.NODE_ENV = 'production';
    delete process.env.NEXT_PUBLIC_ENABLE_MOCK_AUTH;

    expect(isMockAuthEnabled()).toBe(false);
    await expect(login(credentials)).rejects.toThrow(/mock auth is disabled/i);
    await expect(sendMfaCode('totp')).rejects.toThrow(/mock auth is disabled/i);
  }, 10000);

  it('enables mock auth in production only when explicitly opted in', async () => {
    mutableEnv.NODE_ENV = 'production';
    process.env.NEXT_PUBLIC_ENABLE_MOCK_AUTH = 'true';

    expect(isMockAuthEnabled()).toBe(true);
    await expect(login(credentials)).resolves.toMatchObject({ requiresMfa: true });
  }, 10000);

  it('allows mock auth outside production by default', async () => {
    expect(isMockAuthEnabled()).toBe(true);
    await expect(login(credentials)).resolves.toMatchObject({ requiresMfa: true });
  }, 10000);
});
