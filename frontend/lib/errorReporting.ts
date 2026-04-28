/**
 * Error reporting service — captures errors and sends them to a reporting endpoint.
 * Designed to be swappable with Sentry, Datadog, etc.
 */

export interface ErrorReport {
  message: string;
  stack?: string;
  componentStack?: string;
  context?: Record<string, unknown>;
  timestamp: string;
  url: string;
  userAgent: string;
}

type ReportingHandler = (report: ErrorReport) => void;

const handlers: ReportingHandler[] = [];

/** Register a custom reporting handler (e.g. Sentry.captureException) */
export function registerErrorHandler(handler: ReportingHandler): void {
  handlers.push(handler);
}

/** Report an error — calls all registered handlers and logs in dev */
export function reportError(
  error: Error,
  componentStack?: string,
  context?: Record<string, unknown>
): void {
  const report: ErrorReport = {
    message: error.message,
    stack: error.stack,
    componentStack,
    context,
    timestamp: new Date().toISOString(),
    url: typeof window !== 'undefined' ? window.location.href : '',
    userAgent: typeof navigator !== 'undefined' ? navigator.userAgent : '',
  };

  if (process.env.NODE_ENV !== 'production') {
    console.group('[ErrorBoundary] Caught error');
    console.error(error);
    if (componentStack) console.error('Component stack:', componentStack);
    if (context) console.error('Context:', context);
    console.groupEnd();
  }

  handlers.forEach((handler) => {
    try {
      handler(report);
    } catch {
      // never let a reporting handler crash the app
    }
  });
}
