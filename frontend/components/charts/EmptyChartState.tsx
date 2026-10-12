'use client';

import React from 'react';
import { LucideIcon, BarChart2, ArrowRight } from 'lucide-react';
import Link from 'next/link';

export interface EmptyChartStateProps {
  title?: string;
  message: string;
  description?: string;
  icon?: LucideIcon;
  actionText?: string;
  actionHref?: string;
  onAction?: () => void;
  className?: string;
  height?: number | string;
}

export const EmptyChartState: React.FC<EmptyChartStateProps> = ({
  title,
  message,
  description,
  icon: Icon = BarChart2,
  actionText = 'Run a Scan',
  actionHref = '/scan',
  onAction,
  className = '',
  height = 300,
}) => {
  return (
    <div
      role="region"
      aria-label={title ? `${title} empty state` : 'Empty chart state'}
      className={`flex flex-col items-center justify-center text-center p-6 bg-gray-50/60 dark:bg-gray-900/40 rounded-lg border border-dashed border-gray-300 dark:border-gray-700 transition-colors ${className}`}
      style={{ minHeight: typeof height === 'number' ? `${height}px` : height }}
    >
      <div className="w-12 h-12 rounded-full bg-blue-50 dark:bg-blue-950/40 text-blue-600 dark:text-blue-400 flex items-center justify-center mb-3">
        <Icon className="w-6 h-6 text-blue-600 dark:text-blue-400" aria-hidden="true" />
      </div>
      {title && (
        <h4 className="text-base font-semibold text-gray-900 dark:text-gray-100 mb-1">
          {title}
        </h4>
      )}
      <p className="text-sm font-medium text-gray-800 dark:text-gray-200 mb-1 max-w-sm">
        {message}
      </p>
      {description && (
        <p className="text-xs text-gray-600 dark:text-gray-400 mb-4 max-w-xs leading-relaxed">
          {description}
        </p>
      )}
      {(onAction || actionHref) && actionText && (
        <div className="mt-2">
          {onAction ? (
            <button
              onClick={onAction}
              aria-label={actionText}
              className="inline-flex items-center gap-1.5 px-3.5 py-1.5 text-xs font-semibold text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 rounded-md transition-colors shadow-sm"
            >
              <span>{actionText}</span>
              <ArrowRight className="w-3.5 h-3.5" aria-hidden="true" />
            </button>
          ) : (
            <Link
              href={actionHref}
              aria-label={actionText}
              className="inline-flex items-center gap-1.5 px-3.5 py-1.5 text-xs font-semibold text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 rounded-md transition-colors shadow-sm"
            >
              <span>{actionText}</span>
              <ArrowRight className="w-3.5 h-3.5" aria-hidden="true" />
            </Link>
          )}
        </div>
      )}
    </div>
  );
};

export default EmptyChartState;
