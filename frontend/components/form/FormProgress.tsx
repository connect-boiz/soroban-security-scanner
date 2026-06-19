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
    <div className={`flex items-center justify-between ${className}`}>
      {steps.map((step, index) => {
        const isCompleted = index < currentStep;
        const isCurrent = index === currentStep;
        const isInvalid = step.isInvalid && step.isVisited;
        const isValid = step.isValid && step.isVisited;

        return (
          <React.Fragment key={step.name}>
            <div className="flex items-center">
              <div
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
                  <CheckCircle className="w-5 h-5 text-white" />
                ) : (
                  <Circle className={`w-5 h-5 ${isCurrent ? 'text-white' : 'text-gray-400'}`} />
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
            </div>
            {index < steps.length - 1 && (
              <div
                className={`
                  flex-1 h-0.5 mx-4 transition-colors
                  ${isCompleted ? 'bg-green-500' : 'bg-gray-300'}
                `}
              />
            )}
          </React.Fragment>
        );
      })}
    </div>
  );
}
