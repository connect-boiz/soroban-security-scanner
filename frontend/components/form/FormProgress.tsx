'use client';

import React from 'react';
import { CheckCircle, Circle } from 'lucide-react';

export interface FormProgressProps {
  steps: Array<{
    name: string;
    title: string;
    isValid?: boolean;
    isInvalid?: boolean;
    isVisited?: boolean;
  }>;
  currentStep: number;
  className?: string;
}

export function FormProgress({ steps, currentStep, className = '' }: FormProgressProps) {
  return (
    <nav aria-label="Form progress" className={`flex items-center justify-between ${className}`}>
      {/* Visually hidden live region for step change announcements */}
      <div className="sr-only" aria-live="polite" aria-atomic="true">
        {`Step ${currentStep + 1} of ${steps.length}: ${steps[currentStep]?.title || ''}`}
      </div>
      <ol className="flex items-center justify-between w-full list-none p-0 m-0">
        {steps.map((step, index) => {
          const isCompleted = index < currentStep;
          const isCurrent = index === currentStep;
          const isInvalid = step.isInvalid && step.isVisited;
          const isValid = step.isValid && step.isVisited;

          return (
            <React.Fragment key={step.name}>
              <li className="flex items-center">
                <div
                  aria-current={isCurrent ? 'step' : undefined}
                  aria-label={`${step.title}${isCompleted ? ' (completed)' : isCurrent ? ' (current)' : ''}`}
                  className={`
                    flex items-center justify-center w-8 h-8 rounded-full border-2 transition-colors
                    ${
                      isCompleted
                        ? 'bg-green-500 border-green-500'
                        : isCurrent
                          ? 'bg-primary-500 border-primary-500'
                          : isInvalid
                            ? 'bg-red-500 border-red-500'
                            : isValid
                              ? 'bg-green-500 border-green-500'
                              : 'bg-white border-gray-300'
                    }
                  `}
                >
                  {isCompleted || isValid ? (
                    <CheckCircle className="w-5 h-5 text-white" aria-hidden="true" />
                  ) : (
                    <Circle
                      className={`w-5 h-5 ${isCurrent ? 'text-white' : 'text-gray-400'}`}
                      aria-hidden="true"
                    />
                  )}
                </div>
                <span
                  className={`
                    ml-2 text-sm font-medium
                    ${
                      isCurrent
                        ? 'text-primary-600'
                        : isCompleted || isValid
                          ? 'text-green-600'
                          : isInvalid
                            ? 'text-red-600'
                            : 'text-gray-500'
                    }
                  `}
                >
                  {step.title}
                </span>
              </li>
              {index < steps.length - 1 && (
                <li
                  aria-hidden="true"
                  className={`flex-1 h-0.5 mx-4 transition-colors ${isCompleted ? 'bg-green-500' : 'bg-gray-300'}`}
                />
              )}
            </React.Fragment>
          );
        })}
      </ol>
    </nav>
  );
}
