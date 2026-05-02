import React from 'react';
import Tooltip from './Tooltip';

interface HelpIconProps {
  content: string | React.ReactNode;
  label?: string;
}

/**
 * HelpIcon trigger component
 * A small question mark icon that shows a tooltip on hover/focus
 */
const HelpIcon: React.FC<HelpIconProps> = ({ content, label = 'information' }) => {
  return (
    <Tooltip content={content}>
      <button
        type="button"
        className="inline-flex items-center justify-center w-4 h-4 text-gray-400 hover:text-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500 rounded-full transition-colors"
        aria-label={`Help: ${label}`}
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.5"
          strokeLinecap="round"
          strokeLinejoin="round"
          className="w-full h-full"
        >
          <circle cx="12" cy="12" r="10" />
          <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3" />
          <line x1="12" y1="17" x2="12.01" y2="17" />
        </svg>
      </button>
    </Tooltip>
  );
};

export default HelpIcon;
