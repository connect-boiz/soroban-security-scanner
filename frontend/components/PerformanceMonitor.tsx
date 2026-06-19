'use client';

import { useState, useEffect } from 'react';
import { Monitor, Activity, Clock, TrendingUp } from 'lucide-react';
import { StorePerformanceMonitor } from '@/store/storeDebug';

interface PerformanceMetrics {
  [actionName: string]: {
    count: number;
    avgTime: number;
    totalTime: number;
  };
}

const PerformanceMonitor: React.FC = () => {
  const [metrics, setMetrics] = useState<PerformanceMetrics>({});
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    // Only show in development
    if (process.env.NODE_ENV === 'development') {
      setIsVisible(true);

      const interval = setInterval(() => {
        const monitor = StorePerformanceMonitor.getInstance();
        setMetrics(monitor.getMetrics());
      }, 1000);

      return () => clearInterval(interval);
    }
  }, []);

  if (!isVisible) return null;

  const handleReset = () => {
    StorePerformanceMonitor.getInstance().reset();
    setMetrics({});
  };

  const handleLogMetrics = () => {
    StorePerformanceMonitor.getInstance().logMetrics();
  };

  const totalActions = Object.values(metrics).reduce((sum, stat) => sum + stat.count, 0);
  const avgActionTime =
    totalActions > 0
      ? Object.values(metrics).reduce((sum, stat) => sum + stat.avgTime, 0) /
        Object.keys(metrics).length
      : 0;

  return (
    <div className="fixed bottom-4 right-4 bg-gray-900 text-white p-4 rounded-lg shadow-xl max-w-sm max-h-96 overflow-y-auto z-50">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center space-x-2">
          <Monitor className="h-5 w-5 text-green-400" />
          <h3 className="text-sm font-semibold">Performance Monitor</h3>
        </div>
        <button
          onClick={() => setIsVisible(false)}
          className="text-gray-400 hover:text-white text-xs"
        >
          ×
        </button>
      </div>

      {/* Summary Stats */}
      <div className="grid grid-cols-2 gap-2 mb-3 text-xs">
        <div className="bg-gray-800 p-2 rounded">
          <div className="flex items-center space-x-1 text-blue-400">
            <Activity className="h-3 w-3" />
            <span>Total Actions</span>
          </div>
          <div className="text-lg font-bold">{totalActions}</div>
        </div>
        <div className="bg-gray-800 p-2 rounded">
          <div className="flex items-center space-x-1 text-yellow-400">
            <Clock className="h-3 w-3" />
            <span>Avg Time</span>
          </div>
          <div className="text-lg font-bold">{avgActionTime.toFixed(1)}ms</div>
        </div>
      </div>

      {/* Detailed Metrics */}
      <div className="space-y-2 mb-3">
        {Object.entries(metrics).map(([action, stats]) => (
          <div key={action} className="bg-gray-800 p-2 rounded text-xs">
            <div className="flex justify-between items-center mb-1">
              <span className="font-medium text-gray-300">{action}</span>
              <span className="text-green-400">{stats.count}x</span>
            </div>
            <div className="flex justify-between text-gray-400">
              <span>Avg: {stats.avgTime.toFixed(2)}ms</span>
              <span>Total: {stats.totalTime.toFixed(2)}ms</span>
            </div>
            {/* Performance indicator */}
            <div className="mt-1 h-1 bg-gray-700 rounded-full overflow-hidden">
              <div
                className={`h-full ${
                  stats.avgTime < 1
                    ? 'bg-green-500'
                    : stats.avgTime < 5
                      ? 'bg-yellow-500'
                      : 'bg-red-500'
                }`}
                style={{ width: `${Math.min(stats.avgTime * 10, 100)}%` }}
              />
            </div>
          </div>
        ))}
      </div>

      {/* Actions */}
      <div className="flex space-x-2">
        <button
          onClick={handleLogMetrics}
          className="flex-1 bg-blue-600 hover:bg-blue-700 px-2 py-1 rounded text-xs flex items-center justify-center space-x-1"
        >
          <TrendingUp className="h-3 w-3" />
          <span>Log</span>
        </button>
        <button
          onClick={handleReset}
          className="flex-1 bg-red-600 hover:bg-red-700 px-2 py-1 rounded text-xs"
        >
          Reset
        </button>
      </div>
    </div>
  );
};

export default PerformanceMonitor;
