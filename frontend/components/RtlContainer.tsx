'use client';

import React, { forwardRef } from 'react';
import { useTextDirection } from '@/hooks/useTextDirection';

export interface RtlContainerProps extends React.HTMLAttributes<HTMLElement> {
  children?: React.ReactNode;
  as?: React.ElementType;
  dir?: 'ltr' | 'rtl';
  swapFlexDirection?: boolean;
  className?: string;
  style?: React.CSSProperties;
}

/**
 * RtlContainer
 * Layout primitive wrapper that enforces text direction and automatically
 * reverses flex directions when operating in right-to-left layout mode.
 */
export const RtlContainer = forwardRef<HTMLElement, RtlContainerProps>(
  (
    {
      children,
      as: Component = 'div',
      dir: overrideDir,
      swapFlexDirection = true,
      className = '',
      style,
      ...rest
    },
    ref
  ) => {
    const activeDir = useTextDirection();
    const effectiveDir = overrideDir || activeDir;
    const isRtl = effectiveDir === 'rtl';

    // Reverse row flex direction if requested in RTL
    let resolvedClassName = className;
    if (swapFlexDirection && isRtl) {
      if (/\bflex-row\b/.test(resolvedClassName)) {
        resolvedClassName = resolvedClassName.replace(/\bflex-row\b/g, 'flex-row-reverse');
      } else if (
        /\bflex\b/.test(resolvedClassName) &&
        !/\bflex-col\b/.test(resolvedClassName) &&
        !/\bflex-row-reverse\b/.test(resolvedClassName)
      ) {
        resolvedClassName = `${resolvedClassName} flex-row-reverse`.trim();
      }
    }

    const computedStyle: React.CSSProperties = {
      direction: effectiveDir,
      ...style,
    };

    return (
      <Component
        ref={ref}
        dir={effectiveDir}
        data-direction={effectiveDir}
        data-is-rtl={isRtl ? 'true' : 'false'}
        className={resolvedClassName}
        style={computedStyle}
        {...rest}
      >
        {children}
      </Component>
    );
  }
);

RtlContainer.displayName = 'RtlContainer';

export default RtlContainer;
