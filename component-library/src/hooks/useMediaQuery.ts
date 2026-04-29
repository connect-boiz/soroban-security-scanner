import { useState, useEffect } from 'react';
import { breakpoints, Breakpoint, mediaQuery } from '../utils/breakpoints';

/**
 * Returns true when the viewport matches the given CSS media query string.
 * SSR-safe: defaults to false on the server.
 */
export function useMediaQuery(query: string): boolean {
  const [matches, setMatches] = useState(false);

  useEffect(() => {
    if (typeof window === 'undefined') return;
    const mql = window.matchMedia(query);
    setMatches(mql.matches);
    const handler = (e: MediaQueryListEvent) => setMatches(e.matches);
    mql.addEventListener('change', handler);
    return () => mql.removeEventListener('change', handler);
  }, [query]);

  return matches;
}

/**
 * Convenience hook — returns true when viewport is at least the given breakpoint.
 *
 * @example
 * const isTablet = useBreakpoint('md'); // true when width >= 768px
 */
export function useBreakpoint(bp: Breakpoint): boolean {
  return useMediaQuery(mediaQuery(bp));
}

/**
 * Returns the current active breakpoint name based on viewport width.
 * Useful for conditional rendering without multiple hooks.
 */
export function useActiveBreakpoint(): Breakpoint | 'xs' {
  const isSm = useMediaQuery(mediaQuery('sm'));
  const isMd = useMediaQuery(mediaQuery('md'));
  const isLg = useMediaQuery(mediaQuery('lg'));
  const isXl = useMediaQuery(mediaQuery('xl'));

  if (isXl) return 'xl';
  if (isLg) return 'lg';
  if (isMd) return 'md';
  if (isSm) return 'sm';
  return 'xs';
}

/** Returns true when the primary input is a touch device. */
export function useIsTouchDevice(): boolean {
  return useMediaQuery('(pointer: coarse)');
}

export { breakpoints };
