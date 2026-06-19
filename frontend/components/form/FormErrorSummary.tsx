'use client';

import React from 'react';
import { AlertCircle, AlertTriangle } from 'lucide-react';

export interface FormErrorSummaryProps {
  errors: Record<string, string>;
  touched: Record<string, boolean>;
  title?: string;
  className?: string;
  showOnlyTouched?: boolean;
}

export function FormErrorSummary({
  errors,
  touched,
  title = 'Please fix the following errors:',
  className = '',
  showOnlyTouched = true,
}: FormErrorSummaryProps) {
  const filteredErrors = Object.entries(errors).filter(([field, error]) => {
    if (!error) return false;
    if (showOnlyTouched && !touched[field]) return false;
    return true;
  });

  if (filteredErrors.length === 0) return null;

  return (
    <div className={`bg-red-50 border border-red-200 rounded-lg p-4 ${className}`}>
      <div className="flex items-start">
        <div className="flex-shrink-0">
          <AlertTriangle className="h-5 w-5 text-red-600 mt-0.5" />
        </div>
        <div className="ml-3 flex-1">
          <h3 className="text-sm font-medium text-red-800">{title}</h3>
          <div className="mt-2 text-sm text-red-700">
            <ul className="list-disc list-inside space-y-1">
              {filteredErrors.map(([field, error]) => (
                <li key={field}>
                  <span className="font-medium capitalize">
                    {field.replace(/([A-Z])/g, ' $1').trim()}:
                  </span>{' '}
                  {error}
                </li>
              ))}
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
}
