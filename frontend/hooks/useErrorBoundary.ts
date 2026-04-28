'use client';

import { useState, useCallback } from 'react';

/**
 * useErrorBoundary — lets functional components imperatively trigger
 * the nearest error boundary (e.g. after a failed async operation).
 *
 * Usage:
 *   const { throwError } = useErrorBoundary();
 *   try { await fetchData(); } catch (e) { throwError(e as Error); }
 */
export function useErrorBoundary() {
  const [, setState] = useState<unknown>(null);

  const throwError = useCallback((error: Error) => {
    // Throwing inside setState causes React to propagate to the nearest boundary
    setState(() => {
      throw error;
    });
  }, []);

  return { throwError };
}
