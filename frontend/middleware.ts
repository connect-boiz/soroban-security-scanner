import { NextResponse } from 'next/server';
import type { NextRequest } from 'next/server';

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
  // Edge runtime has no Node 'crypto' module — use the Web Crypto API.
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return btoa(String.fromCharCode(...bytes));
}

/**
 * Build Content Security Policy header value
 *
 * @param nonce - Cryptographic nonce for inline scripts
 * @param isProduction - Whether to exclude development-only connection sources
 */
function buildCSP(nonce: string, isProduction: boolean): string {
  const connectSources = [
    "'self'",
    'https://horizon.stellar.org', // Mainnet
    'https://horizon-testnet.stellar.org', // Testnet
    'https://horizon-futurenet.stellar.org', // Futurenet
  ];

  if (!isProduction) {
    connectSources.push('ws://localhost:*');
  }

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

    // API connections: self + the exact Stellar Horizon endpoints used by the app.
    // Local WebSocket access is development-only.
    'connect-src': connectSources,

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
      isProduction
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

export function middleware(request: NextRequest) {
  // Generate unique nonce for this request
  const nonce = generateNonce();

  // Determine if we're in production
  const isProduction = process.env.NODE_ENV === 'production';

  // Get security headers
  const securityHeaders = getSecurityHeaders(nonce, isProduction);

  // Forward nonce on the incoming request so Server Components can read it via
  // headers().get('x-nonce') in layout.tsx and apply it to inline scripts.
  const requestHeaders = new Headers(request.headers);
  requestHeaders.set('x-nonce', nonce);

  // Next.js parses the CSP header on the request to auto-apply nonces to
  // framework scripts during server-side rendering.
  const cspHeaderKey = isProduction
    ? 'Content-Security-Policy'
    : 'Content-Security-Policy-Report-Only';
  requestHeaders.set(cspHeaderKey, securityHeaders[cspHeaderKey]);

  const response = NextResponse.next({
    request: {
      headers: requestHeaders,
    },
  });

  // Apply all security headers
  Object.entries(securityHeaders).forEach(([key, value]) => {
    response.headers.set(key, value);
  });

  // Also expose nonce on the response for integration tests and debugging
  response.headers.set('x-nonce', nonce);

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
