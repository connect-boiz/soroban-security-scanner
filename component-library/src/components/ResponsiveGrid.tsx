import { CSSProperties, ReactNode } from 'react';
import { useActiveBreakpoint } from '../hooks/useMediaQuery';
import { spacing } from '../utils/breakpoints';

export interface ResponsiveGridProps {
  children: ReactNode;
  className?: string;
  /**
   * Number of columns at each breakpoint.
   * Defaults: xs=1, sm=2, md=2, lg=3, xl=4
   */
  columns?: Partial<Record<'xs' | 'sm' | 'md' | 'lg' | 'xl', number>>;
  /** Gap between grid cells in px. Defaults to spacing.md (16px). */
  gap?: number;
}

const DEFAULT_COLUMNS: Record<'xs' | 'sm' | 'md' | 'lg' | 'xl', number> = {
  xs: 1,
  sm: 2,
  md: 2,
  lg: 3,
  xl: 4,
};

/**
 * A responsive CSS grid that automatically adjusts column count based on
 * the current viewport breakpoint.
 */
export function ResponsiveGrid({
  children,
  className,
  columns,
  gap = spacing.md,
}: ResponsiveGridProps) {
  const bp = useActiveBreakpoint();
  const cols = { ...DEFAULT_COLUMNS, ...columns };
  const currentCols = cols[bp];

  const style: CSSProperties = {
    display: 'grid',
    gridTemplateColumns: `repeat(${currentCols}, minmax(0, 1fr))`,
    gap,
    width: '100%',
    boxSizing: 'border-box',
  };

  return (
    <div style={style} className={className}>
      {children}
    </div>
  );
}

export default ResponsiveGrid;
