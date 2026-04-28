/**
 * Responsive design tokens and breakpoint utilities.
 * Mobile-first: styles apply from the given breakpoint and up.
 */

export const breakpoints = {
  sm: 640,   // small phones and up
  md: 768,   // tablets and up
  lg: 1024,  // small desktops and up
  xl: 1280,  // large desktops and up
} as const;

export type Breakpoint = keyof typeof breakpoints;

/** Returns a CSS min-width media query string for the given breakpoint. */
export function mediaQuery(bp: Breakpoint): string {
  return `(min-width: ${breakpoints[bp]}px)`;
}

/**
 * Minimum touch target size (px) per WCAG 2.5.5 / Apple HIG.
 * Ensures interactive elements are comfortably tappable on mobile.
 */
export const MIN_TOUCH_TARGET = 44;

/**
 * Spacing scale (px) used for consistent padding/margin across breakpoints.
 */
export const spacing = {
  xs: 4,
  sm: 8,
  md: 16,
  lg: 24,
  xl: 32,
  xxl: 48,
} as const;
