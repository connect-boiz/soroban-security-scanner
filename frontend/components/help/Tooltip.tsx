import React, { useState, useRef, useEffect } from 'react';

interface TooltipProps {
  content: string | React.ReactNode;
  children: React.ReactNode;
  placement?: 'top' | 'bottom' | 'left' | 'right'; // default: 'top'
  maxWidth?: number; // default: 280px
  delayShow?: number; // default: 300ms
}

/**
 * Tooltip component for inline field/feature explanations
 * Accessible and customizable tooltip without external library dependencies
 */
const Tooltip: React.FC<TooltipProps> = ({
  content,
  children,
  placement = 'top',
  maxWidth = 280,
  delayShow = 300,
}) => {
  const [isVisible, setIsVisible] = useState(false);
  const timeoutRef = useRef<NodeJS.Timeout | null>(null);
  const tooltipId = React.useId();

  const showTooltip = () => {
    timeoutRef.current = setTimeout(() => {
      setIsVisible(true);
    }, delayShow);
  };

  const hideTooltip = () => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }
    setIsVisible(false);
  };

  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, []);

  // Map placement to Tailwind classes
  // Note: Using absolute positioning relative to the container
  const placementClasses = {
    top: 'bottom-full left-1/2 -translate-x-1/2 mb-2',
    bottom: 'top-full left-1/2 -translate-x-1/2 mt-2',
    left: 'right-full top-1/2 -translate-y-1/2 mr-2',
    right: 'left-full top-1/2 -translate-y-1/2 ml-2',
  };

  const arrowClasses = {
    top: 'top-full left-1/2 -translate-x-1/2 border-t-[#0f172a]',
    bottom: 'bottom-full left-1/2 -translate-x-1/2 border-b-[#0f172a]',
    left: 'left-full top-1/2 -translate-y-1/2 border-l-[#0f172a]',
    right: 'right-full top-1/2 -translate-y-1/2 border-r-[#0f172a]',
  };

  return (
    <div
      className="relative inline-block"
      onMouseEnter={showTooltip}
      onMouseLeave={hideTooltip}
      onFocus={showTooltip}
      onBlur={hideTooltip}
    >
      <div aria-describedby={isVisible ? tooltipId : undefined} tabIndex={0}>
        {children}
      </div>
      {isVisible && (
        <div
          id={tooltipId}
          role="tooltip"
          className={`absolute z-50 px-3 py-2 text-xs font-medium text-white bg-[#0f172a] rounded-lg shadow-sm whitespace-normal break-words animate-fade-in ${placementClasses[placement]}`}
          style={{ maxWidth: `${maxWidth}px`, backgroundColor: '#0f172a' }}
        >
          {content}
          <div className={`absolute border-4 border-transparent ${arrowClasses[placement]}`} />
        </div>
      )}
    </div>
  );
};

export default Tooltip;
