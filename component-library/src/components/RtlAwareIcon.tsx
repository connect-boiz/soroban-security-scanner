import React, { forwardRef } from 'react';
import { useRTL } from '../hooks/useRTL';

export interface RtlAwareIconProps extends React.HTMLAttributes<HTMLElement> {
  children?: React.ReactNode;
  icon?: React.ReactNode;
  as?: React.ElementType;
  dir?: 'ltr' | 'rtl';
  mirror?: boolean;
  className?: string;
  style?: React.CSSProperties;
}

/**
 * RtlAwareIcon
 * Mirrors directional icons (such as arrows, back/forward buttons, chevrons)
 * horizontally when rendered in a right-to-left layout.
 */
export const RtlAwareIcon = forwardRef<HTMLElement, RtlAwareIconProps>(
  (
    {
      children,
      icon,
      as: Component = 'span',
      dir: overrideDir,
      mirror = true,
      className = '',
      style,
      'aria-hidden': ariaHidden = true,
      ...rest
    },
    ref
  ) => {
    let contextDir: 'ltr' | 'rtl' = 'ltr';
    try {
      const rtlHook = useRTL();
      contextDir = rtlHook.direction;
    } catch {
      contextDir = typeof document !== 'undefined' && document.documentElement.dir === 'rtl' ? 'rtl' : 'ltr';
    }

    const effectiveDir = overrideDir || contextDir;
    const isRtl = effectiveDir === 'rtl';
    const shouldMirror = isRtl && mirror;

    const computedStyle: React.CSSProperties = {
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      ...(shouldMirror ? { transform: 'scaleX(-1)' } : {}),
      ...style,
    };

    const combinedClassName = [
      'rtl-aware-icon',
      shouldMirror ? 'mirrored' : '',
      className,
    ].filter(Boolean).join(' ');

    return (
      <Component
        ref={ref}
        aria-hidden={ariaHidden}
        data-direction={effectiveDir}
        data-mirrored={shouldMirror ? 'true' : 'false'}
        className={combinedClassName}
        style={computedStyle}
        {...rest}
      >
        {icon || children}
      </Component>
    );
  }
);

RtlAwareIcon.displayName = 'RtlAwareIcon';

export default RtlAwareIcon;
