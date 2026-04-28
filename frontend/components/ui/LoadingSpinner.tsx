'use client';

import React from 'react';

interface LoadingSpinnerProps {
  size?: 'sm' | 'md' | 'lg';
  color?: 'blue' | 'green' | 'red' | 'yellow' | 'purple' | 'gray';
  className?: string;
  text?: string;
}

export const LoadingSpinner: React.FC<LoadingSpinnerProps> = ({
  size = 'md',
  color = 'blue',
  className = '',
  text
}) => {
  const sizeClasses = {
    sm: 'w-4 h-4',
    md: 'w-8 h-8',
    lg: 'w-12 h-12'
  };
  
  const colorClasses = {
    blue: 'border-blue-500',
    green: 'border-green-500',
    red: 'border-red-500',
    yellow: 'border-yellow-500',
    purple: 'border-purple-500',
    gray: 'border-gray-500'
  };
  
  return (
    <div className={`flex items-center justify-center ${className}`}>
      <div className="relative">
        <div
          className={`${sizeClasses[size]} ${colorClasses[color]} border-4 border-t-transparent rounded-full animate-spin`}
        />
      </div>
      {text && (
        <span className="ml-3 text-sm text-gray-600">{text}</span>
      )}
    </div>
  );
};

interface LoadingDotsProps {
  className?: string;
  text?: string;
}

export const LoadingDots: React.FC<LoadingDotsProps> = ({
  className = '',
  text = 'Loading'
}) => {
  return (
    <div className={`flex items-center space-x-1 ${className}`}>
      <span className="text-gray-600">{text}</span>
      <div className="flex space-x-1">
        <div className="w-1 h-1 bg-gray-600 rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
        <div className="w-1 h-1 bg-gray-600 rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
        <div className="w-1 h-1 bg-gray-600 rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
      </div>
    </div>
  );
};

interface LoadingOverlayProps {
  isLoading: boolean;
  text?: string;
  spinner?: boolean;
  backdrop?: boolean;
  children: React.ReactNode;
}

export const LoadingOverlay: React.FC<LoadingOverlayProps> = ({
  isLoading,
  text = 'Loading...',
  spinner = true,
  backdrop = true,
  children
}) => {
  return (
    <div className="relative">
      {children}
      {isLoading && (
        <div className={`absolute inset-0 flex items-center justify-center ${
          backdrop ? 'bg-white bg-opacity-75' : ''
        } z-10`}>
          {spinner ? (
            <LoadingSpinner text={text} />
          ) : (
            <LoadingDots text={text} />
          )}
        </div>
      )}
    </div>
  );
};

export default LoadingSpinner;
