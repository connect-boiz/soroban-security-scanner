'use client';

import React from 'react';

interface SkeletonCardProps {
  className?: string;
  height?: string;
  width?: string;
  lines?: number;
  avatar?: boolean;
  button?: boolean;
}

export const SkeletonCard: React.FC<SkeletonCardProps> = ({
  className = '',
  height = 'h-48',
  width = 'w-full',
  lines = 3,
  avatar = false,
  button = false
}) => {
  return (
    <div className={`bg-white rounded-lg shadow-md p-6 border border-gray-200 ${className} ${width}`}>
      {avatar && (
        <div className="flex items-center space-x-3 mb-4">
          <div className="skeleton w-12 h-12 rounded-full" />
          <div className="flex-1">
            <div className="skeleton h-4 w-32 rounded mb-2" />
            <div className="skeleton h-3 w-24 rounded" />
          </div>
        </div>
      )}
      
      <div className="space-y-3">
        {Array.from({ length: lines }).map((_, index) => (
          <div
            key={index}
            className={`skeleton h-4 rounded ${
              index === lines - 1 ? 'w-3/4' : 'w-full'
            }`}
          />
        ))}
      </div>
      
      {button && (
        <div className="mt-4">
          <div className="skeleton h-10 w-full rounded-md" />
        </div>
      )}
    </div>
  );
};

export default SkeletonCard;
