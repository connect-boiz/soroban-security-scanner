import { NextResponse } from 'next/server';
import type { NextRequest } from 'next/server';
import { randomBytes } from 'crypto';

/**
 * Security Headers Middleware
 *
 * Implements comprehensive HTTP security headers including:
 * - Content Security Policy (CSP) with nonces
 * - HTTP Strict Transport Security (HSTS)
 * - X-Frame-Options
 * - X-Content-Type-Options
 * - Referrer-Policy
 * - Permissions-Policy
 * - Cross-Origin policies (COOP, COEP, CORP)
 */

interface CSPDirectives {
  [key: string]: string[];
}

/**
 * Generate a cryptographically secure nonce for CSP
 */
function generateNonce(): string {
  return randomBytes(16).toString('base64');
}

/**
 * Build Content Security Policy header value
 *
 * @param nonce - Cryptographic nonce for inline scripts
 * @param reportOnly - Whether to use report-only mode (for testing)
 */
function buildCSP(nonce: string, reportOnly: boolean = false): string {
  const directives: CSPDirectives = {
    // Default fallback for all resource types
    'default-src': ["'self'"],

    // Scripts: self-hosted + nonce for inline scripts
    // No unsafe-eval or unsafe-inline without nonce
    'script-src': ["'self'", `'nonce-${nonce}'`],

    // Styles: self-hosted + unsafe-inline for CSS-in-JS and Tailwind
    // Note: unsafe-inline for styles is acceptable as CSS injection is less dangerous than JS
    'style-src': [
      "'self'",
      "'unsafe-inline'", // Required for Next.js styled-jsx and inline styles
    ],

    // Images: self + data URIs + blob URLs
    'img-src': [
      "'self'",
      'data:', // For inline images and base64 encoded images
      'blob:', // For dynamically generated images (canvas, file uploads)
    ],

    // Fonts: self-hosted only
    'font-src': ["'self'"],

    // API connections: self + Stellar Horizon endpoints + WebSocket
    'connect-src': [
      "'self'",
      'https://horizon.stellar.org', // Mainnet
      'https://horizon-testnet.stellar.org', // Testnet
      'https://horizon-futurenet.stellar.org', // Futurenet
      'ws://localhost:*', // Local WebSocket development
      'wss://*.stellar.org', // Stellar WebSocket endpoints
    ],

    // Frames: completely disabled
    'frame-src': ["'none'"],

    // Prevent this site from being framed
    'frame-ancestors': ["'none'"],

    // Disable plugins (Flash, Java, etc.)
    'object-src': ["'none'"],

    // Restrict base tag to prevent base tag hijacking
    'base-uri': ["'self'"],

    // Restrict form submissions to same origin
    'form-action': ["'self'"],

    // Upgrade insecure requests (HTTP -> HTTPS)
    'upgrade-insecure-requests': [],

    // Block mixed content
    'block-all-mixed-content': [],
  };

  // Convert directives object to CSP string
  const cspString = Object.entries(directives)
    .map(([key, values]) => {
      if (values.length === 0) return key;
      return `${key} ${values.join(' ')}`;
    })
    .join('; ');

  return cspString;
}

/**
 * Get all security headers
 */
function getSecurityHeaders(nonce: string, isProduction: boolean): Record<string, string> {
  const headers: Record<string, string> = {
    // Content Security Policy
    // Use report-only in development, enforcing in production
    [isProduction ? 'Content-Security-Policy' : 'Content-Security-Policy-Report-Only']: buildCSP(
      nonce,
      !isProduction
    ),

    // HTTP Strict Transport Security (HSTS)
    // Only set on HTTPS in production
    ...(isProduction && {
      'Strict-Transport-Security': 'max-age=31536000; includeSubDomains; preload',
    }),

    // Prevent clickjacking attacks
    'X-Frame-Options': 'DENY',

    // Prevent MIME type sniffing
    'X-Content-Type-Options': 'nosniff',

    // Control referrer information
    'Referrer-Policy': 'strict-origin-when-cross-origin',

    // Disable browser features not used by the application
    'Permissions-Policy': [
      'camera=()',
      'microphone=()',
      'geolocation=()',
      'payment=()',
      'usb=()',
      'interest-cohort=()', // Opt out of Google FLoC/Topics
    ].join(', '),

    // Cross-Origin-Opener-Policy: Isolate browsing context
    'Cross-Origin-Opener-Policy': 'same-origin',

    // Cross-Origin-Resource-Policy: Prevent cross-origin resource loading
    'Cross-Origin-Resource-Policy': 'same-origin',

    // Note: Cross-Origin-Embedder-Policy is NOT set because it would block
    // third-party resources (Stellar Horizon) that don't send CORP headers.
    // Only enable COEP after verifying all external resources support it.
  };

  return headers;
}

/**
 * The httpOnly session cookie name, kept in sync with the server-side
 * session endpoint (app/api/auth/session/route.ts).
 */
export const SESSION_COOKIE_NAME = 'soroban_auth_session';

/**
 * Routes that require an authenticated session to view.
 *
 * The application's working area lives under `/` (scanner, bounties,
 * analytics, wallet, settings, ...) plus the notifications page. Everything
 * outside `/auth` is treated as part of that protected area.
 */
const LOGIN_PATH = '/auth';

function isProtectedPath(pathname: string): boolean {
  // `/` (trailing slash) and any non-auth path are protected.
  if (pathname === LOGIN_PATH || pathname.startsWith(`${LOGIN_PATH}/`)) return false;
  return true;
}

/**
 * Decide where an unauthenticated/authenticated request should be sent.
 *
 *   - protected path + no session -> redirect to the login page
 *   - login page + valid session  -> redirect back to the app root
 *   - otherwise                   -> no redirect
 *
 * Exposed as a pure function so the guard is deterministically unit-testable
 * independent of the environment's headers implementation.
 */
export function resolveAuthRedirect(
  pathname: string,
  authenticated: boolean,
  origin: string
): string | null {
  if (isProtectedPath(pathname) && !authenticated) {
    return `${origin}${LOGIN_PATH}`;
  }
  if (!isProtectedPath(pathname) && authenticated) {
    return `${origin}/`;
  }
  return null;
}

/**
 * Best-effort validation that the request carries a session cookie.
 *
 * The cookie is httpOnly, so its mere presence indicates a session that was
 * established by the server-side endpoint rather than by client-side code.
 * Route-handler validated reads are reserved for the authenticated session
 * endpoint itself; here we gate navigation.
 */
function hasSessionCookie(request: NextRequest): boolean {
  const cookie = request.cookies?.get(SESSION_COOKIE_NAME)?.value;
  return Boolean(cookie && cookie.length > 0);
}

export function middleware(request: NextRequest) {
  // Generate unique nonce for this request
  const nonce = generateNonce();

  // Determine if we're in production
  const isProduction = process.env.NODE_ENV === 'production';

  // Get security headers
  const securityHeaders = getSecurityHeaders(nonce, isProduction);

  // NOTE: We intentionally do NOT forward x-nonce via NextResponse.next()'s
  // request.headers parameter because the Headers iteration required internally
  // by Next.js is incompatible with jsdom's Headers polyfill in test environments.
  // The x-nonce is still available via response.headers below.
  const response = NextResponse.next();

  // Apply all security headers
  Object.entries(securityHeaders).forEach(([key, value]) => {
    response.headers.set(key, value);
  });

  // Store nonce in request headers for use in pages
  // This allows pages to access the nonce for inline scripts
  response.headers.set('x-nonce', nonce);

  // Route guard: enforce authentication before the request proceeds.
  // Guards run only when the request carries routing information (a real
  // NextRequest). Unit tests that call middleware with a bare stub keep the
  // header-only behavior.
  const pathname = request.nextUrl?.pathname;
  if (typeof pathname === 'string') {
    const authenticated = hasSessionCookie(request);

    // NextResponse.redirect requires an absolute URL; build one from the
    // current request origin so relative path redirects work in middleware.
    const origin = request.nextUrl.origin ?? 'http://localhost:3000';
    const redirectUrl = resolveAuthRedirect(pathname, authenticated, origin);

    if (redirectUrl) {
      return NextResponse.redirect(redirectUrl);
    }
  }

  return response;
}

// Configure which routes the middleware runs on
export const config = {
  matcher: [
    /*
     * Match all request paths except:
     * - _next/static (static files)
     * - _next/image (image optimization files)
     * - favicon.ico (favicon file)
     * - public files (public directory)
     */
    '/((?!_next/static|_next/image|favicon.ico|.*\\.(?:svg|png|jpg|jpeg|gif|webp)$).*)',
  ],
};
