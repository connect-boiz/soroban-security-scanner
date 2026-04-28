'use client';

import React, { Component, ErrorInfo, ReactNode } from 'react';
import { reportError } from '@/lib/errorReporting';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface FallbackProps {
  error: Error;
  resetError: () => void;
}

export interface ErrorBoundaryProps {
  children: ReactNode;
  /** Custom fallback UI. Receives the error and a reset callback. */
  fallback?: (props: FallbackProps) => ReactNode;
  /** Called after the error is reported — useful for analytics side-effects */
  onError?: (error: Error, info: ErrorInfo) => void;
  /** Extra context forwarded to the error reporter */
  context?: Record<string, unknown>;
  /** Variant controls the built-in fallback style */
  variant?: 'page' | 'section' | 'inline';
}

interface State {
  hasError: boolean;
  error: Error | null;
}

// ---------------------------------------------------------------------------
// Built-in fallback UIs
// ---------------------------------------------------------------------------

function PageFallback({ error, resetError }: FallbackProps) {
  return (
    <div
      role="alert"
      aria-live="assertive"
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        minHeight: '100vh',
        padding: '2rem',
        textAlign: 'center',
        fontFamily: 'inherit',
      }}
    >
      <svg
        width="64"
        height="64"
        viewBox="0 0 24 24"
        fill="none"
        stroke="#ef4444"
        strokeWidth="1.5"
        aria-hidden="true"
        style={{ marginBottom: '1.5rem' }}
      >
        <circle cx="12" cy="12" r="10" />
        <line x1="12" y1="8" x2="12" y2="12" />
        <line x1="12" y1="16" x2="12.01" y2="16" />
      </svg>
      <h1 style={{ fontSize: '1.5rem', fontWeight: 700, marginBottom: '0.5rem', color: '#111' }}>
        Something went wrong
      </h1>
      <p style={{ color: '#6b7280', marginBottom: '1.5rem', maxWidth: '480px' }}>
        An unexpected error occurred. The issue has been reported and we&apos;re looking into it.
      </p>
      {process.env.NODE_ENV !== 'production' && (
        <pre
          style={{
            background: '#fef2f2',
            border: '1px solid #fecaca',
            borderRadius: '6px',
            padding: '1rem',
            fontSize: '0.75rem',
            color: '#b91c1c',
            maxWidth: '600px',
            overflowX: 'auto',
            textAlign: 'left',
            marginBottom: '1.5rem',
          }}
        >
          {error.message}
        </pre>
      )}
      <button
        onClick={resetError}
        style={{
          padding: '0.625rem 1.5rem',
          background: '#2563eb',
          color: '#fff',
          border: 'none',
          borderRadius: '6px',
          fontSize: '0.875rem',
          fontWeight: 600,
          cursor: 'pointer',
        }}
      >
        Try again
      </button>
    </div>
  );
}

function SectionFallback({ error, resetError }: FallbackProps) {
  return (
    <div
      role="alert"
      aria-live="polite"
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        padding: '2rem',
        background: '#fef2f2',
        border: '1px solid #fecaca',
        borderRadius: '8px',
        textAlign: 'center',
      }}
    >
      <p style={{ color: '#b91c1c', fontWeight: 600, marginBottom: '0.5rem' }}>
        This section failed to load
      </p>
      {process.env.NODE_ENV !== 'production' && (
        <p style={{ fontSize: '0.75rem', color: '#6b7280', marginBottom: '0.75rem' }}>
          {error.message}
        </p>
      )}
      <button
        onClick={resetError}
        style={{
          padding: '0.375rem 1rem',
          background: '#fff',
          color: '#b91c1c',
          border: '1px solid #fecaca',
          borderRadius: '4px',
          fontSize: '0.8rem',
          cursor: 'pointer',
        }}
      >
        Retry
      </button>
    </div>
  );
}

function InlineFallback({ resetError }: FallbackProps) {
  return (
    <span
      role="alert"
      style={{ color: '#ef4444', fontSize: '0.875rem', display: 'inline-flex', gap: '0.5rem', alignItems: 'center' }}
    >
      Failed to render.{' '}
      <button
        onClick={resetError}
        style={{ color: '#2563eb', background: 'none', border: 'none', cursor: 'pointer', textDecoration: 'underline', fontSize: 'inherit' }}
      >
        Retry
      </button>
    </span>
  );
}

const FALLBACK_MAP = {
  page: PageFallback,
  section: SectionFallback,
  inline: InlineFallback,
} as const;

// ---------------------------------------------------------------------------
// ErrorBoundary class component
// ---------------------------------------------------------------------------

export class ErrorBoundary extends Component<ErrorBoundaryProps, State> {
  static defaultProps: Partial<ErrorBoundaryProps> = {
    variant: 'section',
  };

  state: State = { hasError: false, error: null };

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    reportError(error, info.componentStack ?? undefined, this.props.context);
    this.props.onError?.(error, info);
  }

  resetError = (): void => {
    this.setState({ hasError: false, error: null });
  };

  render(): ReactNode {
    const { hasError, error } = this.state;
    const { children, fallback, variant = 'section' } = this.props;

    if (!hasError || !error) return children;

    if (fallback) return fallback({ error, resetError: this.resetError });

    const FallbackComponent = FALLBACK_MAP[variant];
    return <FallbackComponent error={error} resetError={this.resetError} />;
  }
}

// ---------------------------------------------------------------------------
// Convenience wrappers
// ---------------------------------------------------------------------------

/** Full-page error boundary — wraps entire routes */
export function PageErrorBoundary({ children, ...props }: Omit<ErrorBoundaryProps, 'variant'>) {
  return (
    <ErrorBoundary variant="page" {...props}>
      {children}
    </ErrorBoundary>
  );
}

/** Section-level boundary — wraps individual panels/cards */
export function SectionErrorBoundary({ children, ...props }: Omit<ErrorBoundaryProps, 'variant'>) {
  return (
    <ErrorBoundary variant="section" {...props}>
      {children}
    </ErrorBoundary>
  );
}

/** Inline boundary — for small widgets inside text/lists */
export function InlineErrorBoundary({ children, ...props }: Omit<ErrorBoundaryProps, 'variant'>) {
  return (
    <ErrorBoundary variant="inline" {...props}>
      {children}
    </ErrorBoundary>
  );
}

export default ErrorBoundary;
