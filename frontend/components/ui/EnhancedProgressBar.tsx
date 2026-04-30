'use client';

import React, { useState, useEffect } from 'react';

interface EnhancedProgressBarProps {
  value: number;
  max?: number;
  size?: 'sm' | 'md' | 'lg';
  color?: 'blue' | 'green' | 'red' | 'yellow' | 'purple' | 'gray';
  showLabel?: boolean;
  showPercentage?: boolean;
  animated?: boolean;
  striped?: boolean;
  indeterminate?: boolean;
  className?: string;
  label?: string;
  stages?: Array<{ name: string; value: number; completed?: boolean }>;
}

export const EnhancedProgressBar: React.FC<EnhancedProgressBarProps> = ({
  value,
  max = 100,
  size = 'md',
  color = 'blue',
  showLabel = true,
  showPercentage = true,
  animated = true,
  striped = false,
  indeterminate = false,
  className = '',
  label,
  stages
}) => {
  const [currentValue, setCurrentValue] = useState(0);
  const [completedStages, setCompletedStages] = useState<Set<number>>(new Set());

  useEffect(() => {
    if (!indeterminate) {
      const timer = setTimeout(() => {
        setCurrentValue(value);
      }, 100);
      return () => clearTimeout(timer);
    }
  }, [value, indeterminate]);

  useEffect(() => {
    if (stages) {
      const newCompletedStages = new Set<number>();
      stages.forEach((stage, index) => {
        if (value >= stage.value) {
          newCompletedStages.add(index);
        }
      });
      setCompletedStages(newCompletedStages);
    }
  }, [value, stages]);

  const sizeClasses = {
    sm: 'h-2',
    md: 'h-4',
    lg: 'h-6'
  };

  const colorClasses = {
    blue: 'bg-blue-500',
    green: 'bg-green-500',
    red: 'bg-red-500',
    yellow: 'bg-yellow-500',
    purple: 'bg-purple-500',
    gray: 'bg-gray-500'
  };

  const bgColorClasses = {
    blue: 'bg-blue-100',
    green: 'bg-green-100',
    red: 'bg-red-100',
    yellow: 'bg-yellow-100',
    purple: 'bg-purple-100',
    gray: 'bg-gray-100'
  };

  const textColorClasses = {
    blue: 'text-blue-700',
    green: 'text-green-700',
    red: 'text-red-700',
    yellow: 'text-yellow-700',
    purple: 'text-purple-700',
    gray: 'text-gray-700'
  };

  const progressPercentage = Math.min((currentValue / max) * 100, 100);

  const renderProgressBar = () => {
    if (indeterminate) {
      return (
        <div className={`${sizeClasses[size]} ${bgColorClasses[color]} rounded-full overflow-hidden`}>
          <div 
            className={`h-full ${colorClasses[color]} ${animated ? 'animate-pulse' : ''}`}
            style={{
              width: '30%',
              animation: 'indeterminate 2s ease-in-out infinite'
            }}
          />
        </div>
      );
    }

    return (
      <div className={`${sizeClasses[size]} ${bgColorClasses[color]} rounded-full overflow-hidden`}>
        <div
          className={`h-full ${colorClasses[color]} transition-all duration-500 ease-out ${
            striped ? 'bg-stripes' : ''
          } ${animated ? 'animate-pulse' : ''}`}
          style={{ 
            width: `${progressPercentage}%`,
            backgroundImage: striped ? 'repeating-linear-gradient(45deg, transparent, transparent 10px, rgba(255,255,255,.1) 10px, rgba(255,255,255,.1) 20px)' : undefined
          }}
        />
      </div>
    );
  };

  const renderStages = () => {
    if (!stages) return null;

    return (
      <div className="mt-2 space-y-1">
        {stages.map((stage, index) => (
          <div key={index} className="flex items-center space-x-2 text-xs">
            <div className={`w-3 h-3 rounded-full ${
              completedStages.has(index) ? colorClasses[color] : 'bg-gray-300'
            }`} />
            <span className={`${
              completedStages.has(index) ? textColorClasses[color] : 'text-gray-500'
            } font-medium`}>
              {stage.name}
            </span>
            <span className="text-gray-400">
              {stage.value}%
            </span>
          </div>
        ))}
      </div>
    );
  };

  return (
    <div className={`space-y-2 ${className}`}>
      {(label || showLabel) && (
        <div className="flex items-center justify-between">
          {label && (
            <span className={`text-sm font-medium ${textColorClasses[color]}`}>
              {label}
            </span>
          )}
          {showPercentage && !indeterminate && (
            <span className={`text-sm ${textColorClasses[color]}`}>
              {Math.round(progressPercentage)}%
            </span>
          )}
        </div>
      )}
      
      {renderProgressBar()}
      {renderStages()}
    </div>
  );
};

// Multi-step progress indicator
interface MultiStepProgressProps {
  steps: Array<{ name: string; completed?: boolean; current?: boolean }>;
  className?: string;
}

export const MultiStepProgress: React.FC<MultiStepProgressProps> = ({
  steps,
  className = ''
}) => {
  return (
    <div className={`space-y-2 ${className}`}>
      <div className="flex items-center space-x-2">
        {steps.map((step, index) => (
          <React.Fragment key={index}>
            <div className={`flex items-center justify-center w-8 h-8 rounded-full border-2 ${
              step.completed 
                ? 'bg-blue-500 border-blue-500 text-white' 
                : step.current 
                  ? 'border-blue-500 text-blue-500'
                  : 'border-gray-300 text-gray-400'
            }`}>
              {step.completed ? (
                <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                </svg>
              ) : (
                <span className="text-sm font-medium">{index + 1}</span>
              )}
            </div>
            {index < steps.length - 1 && (
              <div className={`flex-1 h-1 ${
                step.completed ? 'bg-blue-500' : 'bg-gray-300'
              }`} />
            )}
          </React.Fragment>
        ))}
      </div>
      
      <div className="flex justify-between">
        {steps.map((step, index) => (
          <div key={index} className="flex-1 text-center">
            <span className={`text-xs ${
              step.completed || step.current ? 'text-blue-700 font-medium' : 'text-gray-500'
            }`}>
              {step.name}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
};

// Circular progress indicator
interface CircularProgressProps {
  value: number;
  max?: number;
  size?: number;
  strokeWidth?: number;
  color?: string;
  backgroundColor?: string;
  showPercentage?: boolean;
  className?: string;
}

export const CircularProgress: React.FC<CircularProgressProps> = ({
  value,
  max = 100,
  size = 120,
  strokeWidth = 8,
  color = '#3B82F6',
  backgroundColor = '#E5E7EB',
  showPercentage = true,
  className = ''
}) => {
  const [currentValue, setCurrentValue] = useState(0);

  useEffect(() => {
    const timer = setTimeout(() => {
      setCurrentValue(value);
    }, 100);
    return () => clearTimeout(timer);
  }, [value]);

  const radius = (size - strokeWidth) / 2;
  const circumference = radius * 2 * Math.PI;
  const progressPercentage = Math.min((currentValue / max) * 100, 100);
  const strokeDashoffset = circumference - (progressPercentage / 100) * circumference;

  return (
    <div className={`relative inline-flex items-center justify-center ${className}`}>
      <svg
        width={size}
        height={size}
        className="transform -rotate-90"
      >
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          stroke={backgroundColor}
          strokeWidth={strokeWidth}
          fill="none"
        />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          stroke={color}
          strokeWidth={strokeWidth}
          fill="none"
          strokeDasharray={circumference}
          strokeDashoffset={strokeDashoffset}
          className="transition-all duration-500 ease-out"
          strokeLinecap="round"
        />
      </svg>
      {showPercentage && (
        <div className="absolute inset-0 flex items-center justify-center">
          <span className="text-lg font-semibold text-gray-700">
            {Math.round(progressPercentage)}%
          </span>
        </div>
      )}
    </div>
  );
};

export default EnhancedProgressBar;
