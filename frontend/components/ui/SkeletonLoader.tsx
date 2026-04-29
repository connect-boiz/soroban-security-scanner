'use client';

import React from 'react';

interface SkeletonLoaderProps {
  type?: 'card' | 'table' | 'list' | 'chart' | 'form' | 'modal';
  lines?: number;
  avatar?: boolean;
  button?: boolean;
  height?: string;
  width?: string;
  className?: string;
  animated?: boolean;
}

export const SkeletonLoader: React.FC<SkeletonLoaderProps> = ({
  type = 'card',
  lines = 3,
  avatar = false,
  button = false,
  height = 'h-4',
  width = 'w-full',
  className = '',
  animated = true
}) => {
  const skeletonClass = animated ? 'animate-pulse' : '';
  const baseClass = `bg-gray-200 rounded ${skeletonClass}`;

  const renderCardSkeleton = () => (
    <div className={`p-4 border border-gray-200 rounded-lg ${className}`}>
      {avatar && (
        <div className={`w-12 h-12 ${baseClass} rounded-full mb-4`} />
      )}
      <div className="space-y-3">
        <div className={`h-6 ${baseClass} w-3/4`} />
        {Array.from({ length: lines - 1 }, (_, i) => (
          <div key={i} className={`h-4 ${baseClass} w-${i % 3 === 0 ? 'full' : i % 3 === 1 ? '5/6' : '4/6'}`} />
        ))}
      </div>
      {button && (
        <div className={`h-10 ${baseClass} w-1/3 mt-4`} />
      )}
    </div>
  );

  const renderTableSkeleton = () => (
    <div className={`w-full ${className}`}>
      <div className="border border-gray-200 rounded-lg overflow-hidden">
        <div className="bg-gray-50 p-4 border-b">
          <div className="h-6 bg-gray-200 rounded animate-pulse w-1/4" />
        </div>
        <div className="divide-y divide-gray-200">
          {Array.from({ length: lines }, (_, i) => (
            <div key={i} className="p-4">
              <div className="grid grid-cols-4 gap-4">
                {Array.from({ length: 4 }, (_, j) => (
                  <div key={j} className="h-4 bg-gray-200 rounded animate-pulse" />
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );

  const renderListSkeleton = () => (
    <div className={`space-y-3 ${className}`}>
      {Array.from({ length: lines }, (_, i) => (
        <div key={i} className="flex items-center space-x-3 p-3 border border-gray-200 rounded-lg">
          {avatar && (
            <div className="w-8 h-8 bg-gray-200 rounded-full animate-pulse" />
          )}
          <div className="flex-1 space-y-2">
            <div className="h-4 bg-gray-200 rounded animate-pulse w-3/4" />
            <div className="h-3 bg-gray-200 rounded animate-pulse w-1/2" />
          </div>
          <div className="h-6 bg-gray-200 rounded animate-pulse w-16" />
        </div>
      ))}
    </div>
  );

  const renderChartSkeleton = () => (
    <div className={`p-6 border border-gray-200 rounded-lg ${className}`}>
      <div className="h-6 bg-gray-200 rounded animate-pulse w-1/3 mb-4" />
      <div className="h-48 bg-gray-200 rounded animate-pulse mb-4" />
      <div className="flex justify-between">
        {Array.from({ length: 5 }, (_, i) => (
          <div key={i} className="h-8 bg-gray-200 rounded animate-pulse w-12" />
        ))}
      </div>
    </div>
  );

  const renderFormSkeleton = () => (
    <div className={`space-y-4 ${className}`}>
      {Array.from({ length: lines }, (_, i) => (
        <div key={i} className="space-y-2">
          <div className="h-4 bg-gray-200 rounded animate-pulse w-1/4" />
          <div className="h-10 bg-gray-200 rounded animate-pulse" />
        </div>
      ))}
      <div className="h-10 bg-gray-200 rounded animate-pulse w-1/3 mt-6" />
    </div>
  );

  const renderModalSkeleton = () => (
    <div className={`p-6 border border-gray-200 rounded-lg max-w-md w-full ${className}`}>
      <div className="flex items-center justify-between mb-4">
        <div className="h-6 bg-gray-200 rounded animate-pulse w-1/3" />
        <div className="h-6 w-6 bg-gray-200 rounded animate-pulse" />
      </div>
      <div className="space-y-4">
        {Array.from({ length: lines }, (_, i) => (
          <div key={i} className="space-y-2">
            <div className="h-4 bg-gray-200 rounded animate-pulse w-1/4" />
            <div className="h-4 bg-gray-200 rounded animate-pulse w-full" />
          </div>
        ))}
      </div>
      <div className="flex justify-end space-x-3 mt-6">
        <div className="h-10 bg-gray-200 rounded animate-pulse w-20" />
        <div className="h-10 bg-gray-200 rounded animate-pulse w-20" />
      </div>
    </div>
  );

  switch (type) {
    case 'card':
      return renderCardSkeleton();
    case 'table':
      return renderTableSkeleton();
    case 'list':
      return renderListSkeleton();
    case 'chart':
      return renderChartSkeleton();
    case 'form':
      return renderFormSkeleton();
    case 'modal':
      return renderModalSkeleton();
    default:
      return renderCardSkeleton();
  }
};

// Enhanced skeleton components for specific use cases
export const SkeletonCard: React.FC<Omit<SkeletonLoaderProps, 'type'>> = (props) => (
  <SkeletonLoader {...props} type="card" />
);

export const SkeletonTable: React.FC<Omit<SkeletonLoaderProps, 'type'>> = (props) => (
  <SkeletonLoader {...props} type="table" />
);

export const SkeletonList: React.FC<Omit<SkeletonLoaderProps, 'type'>> = (props) => (
  <SkeletonLoader {...props} type="list" />
);

export const SkeletonChart: React.FC<Omit<SkeletonLoaderProps, 'type'>> = (props) => (
  <SkeletonLoader {...props} type="chart" />
);

export const SkeletonForm: React.FC<Omit<SkeletonLoaderProps, 'type'>> = (props) => (
  <SkeletonLoader {...props} type="form" />
);

export const SkeletonModal: React.FC<Omit<SkeletonLoaderProps, 'type'>> = (props) => (
  <SkeletonLoader {...props} type="modal" />
);

export default SkeletonLoader;
