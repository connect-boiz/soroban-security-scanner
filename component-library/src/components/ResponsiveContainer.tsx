import { CSSProperties, ReactNode } from 'react';
import { useActiveBreakpoint } from '../hooks/useMediaQuery';
import { spacing } from '../utils/breakpoints';

export interface ResponsiveContainerProps {
  children: ReactNode;
  className?: string;
  /**
   * Maximum content width in px. Defaults to 1280 (xl breakpoint).
   */
  maxWidth?: number;
  /**
   * Horizontal padding adapts to the current breakpoint unless overridden.
   */
  paddingX?: number;
}

/**
 * A full-width wrapper that centers content and applies responsive horizontal
 * padding. Use this as the outermost layout wrapper on every page.
 *
 * Padding scale:
 *  - xs  (< 640px)  : 16px
 *  - sm  (≥ 640px)  : 24px
 *  - md  (≥ 768px)  : 32px
 *  - lg+ (≥ 1024px) : 48px
 */
export function ResponsiveContainer({
  children,
  className,
  maxWidth = 1280,
  paddingX,
}: ResponsiveContainerProps) {
  const bp = useActiveBreakpoint();

  const defaultPaddingX =
    paddingX ??
    (bp === 'xs' || bp === 'sm'
      ? spacing.md
      : bp === 'md'
      ? spacing.lg
      : spacing.xxl);

  const style: CSSProperties = {
    width: '100%',
    maxWidth,
    marginLeft: 'auto',
    marginRight: 'auto',
    paddingLeft: defaultPaddingX,
    paddingRight: defaultPaddingX,
    boxSizing: 'border-box',
  };

  return (
    <div style={style} className={className}>
      {children}
    </div>
  );
}

export default ResponsiveContainer;
