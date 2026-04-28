'use client';

import { useState, useMemo, useCallback, useEffect } from 'react';
import { LoadingOverlay, SkeletonCard, ProgressBar, LoadingSpinner } from './ui';

interface AnalyticsData {
  totalScans: number;
  vulnerabilitiesFound: number;
  contractsAnalyzed: number;
  averageScanTime: number;
  severityBreakdown: {
    critical: number;
    high: number;
    medium: number;
    low: number;
  };
  weeklyData: {
    week: string;
    scans: number;
    vulnerabilities: number;
  }[];
}

const mockAnalyticsData: AnalyticsData = {
  totalScans: 1250,
  vulnerabilitiesFound: 342,
  contractsAnalyzed: 890,
  averageScanTime: 2.4,
  severityBreakdown: {
    critical: 45,
    high: 128,
    medium: 134,
    low: 35
  },
  weeklyData: [
    { week: 'Week 1', scans: 180, vulnerabilities: 48 },
    { week: 'Week 2', scans: 220, vulnerabilities: 62 },
    { week: 'Week 3', scans: 195, vulnerabilities: 51 },
    { week: 'Week 4', scans: 240, vulnerabilities: 71 },
    { week: 'Week 5', scans: 210, vulnerabilities: 60 },
    { week: 'Week 6', scans: 205, vulnerabilities: 50 }
  ]
};

export default function AnalyticsDashboard() {
  const [selectedMetric, setSelectedMetric] = useState<'overview' | 'trends' | 'severity'>('overview');
  const [isLoading, setIsLoading] = useState(false);
  const [isGeneratingData, setIsGeneratingData] = useState(false);

  const metrics = useMemo(() => ({
    vulnerabilityRate: ((mockAnalyticsData.vulnerabilitiesFound / mockAnalyticsData.totalScans) * 100).toFixed(1),
    contractsPerScan: (mockAnalyticsData.contractsAnalyzed / mockAnalyticsData.totalScans).toFixed(2),
    totalIssues: Object.values(mockAnalyticsData.severityBreakdown).reduce((a, b) => a + b, 0)
  }), []);

  const handleMetricChange = useCallback(async (metric: 'overview' | 'trends' | 'severity') => {
    setIsGeneratingData(true);
    setSelectedMetric(metric);
    
    // Simulate data loading for different metrics
    await new Promise(resolve => setTimeout(resolve, 600));
    
    setIsGeneratingData(false);
  }, []);

  // Simulate initial data loading
  useEffect(() => {
    const loadAnalytics = async () => {
      setIsLoading(true);
      await new Promise(resolve => setTimeout(resolve, 1500));
      setIsLoading(false);
    };
    
    loadAnalytics();
  }, []);

  return (
    <div className="space-y-8 animate-fade-in">
      {/* Top Stats Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <div className="card border-l-4 border-l-blue-600">
          <p className="text-xs font-bold text-gray-500 uppercase tracking-wider">Total Scans</p>
          <div className="flex items-end justify-between mt-2">
            <h3 className="text-3xl font-bold">{mockAnalyticsData.totalScans.toLocaleString()}</h3>
            <span className="text-green-500 text-sm font-bold">↑ 12%</span>
          </div>
        </div>

        <div className="card border-l-4 border-l-red-600">
          <p className="text-xs font-bold text-gray-500 uppercase tracking-wider">Vulnerabilities</p>
          <div className="flex items-end justify-between mt-2">
            <h3 className="text-3xl font-bold">{mockAnalyticsData.vulnerabilitiesFound}</h3>
            <span className="text-red-500 text-sm font-bold">↑ 5%</span>
          </div>
        </div>

        <div className="card border-l-4 border-l-green-600">
          <p className="text-xs font-bold text-gray-500 uppercase tracking-wider">Detection Rate</p>
          <div className="flex items-end justify-between mt-2">
            <h3 className="text-3xl font-bold">{metrics.vulnerabilityRate}%</h3>
            <span className="text-gray-400 text-sm font-bold">Stable</span>
          </div>
        </div>

        <div className="card border-l-4 border-l-purple-600">
          <p className="text-xs font-bold text-gray-500 uppercase tracking-wider">Avg Scan Time</p>
          <div className="flex items-end justify-between mt-2">
            <h3 className="text-3xl font-bold">{mockAnalyticsData.averageScanTime}s</h3>
            <span className="text-green-500 text-sm font-bold">↓ 0.2s</span>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        {/* Main Chart Widget */}
        <div className="card lg:col-span-2">
          <div className="flex justify-between items-center mb-6">
            <h3 className="text-lg font-bold">Scan Activity Trends</h3>
            <div className="flex bg-gray-100 p-1 rounded-lg">
              {['7d', '30d', '90d'].map(range => (
                <button 
                  key={range}
                  onClick={() => setSelectedRange(range)}
                  className={`px-3 py-1 text-xs font-bold rounded-md transition-all ${selectedRange === range ? 'bg-white shadow-sm text-blue-600' : 'text-gray-500'}`}
                >
                  {range.toUpperCase()}
                </button>
              ))}
            </div>
          </div>
          
          <div className="space-y-4 h-[300px] flex flex-col justify-end">
            <div className="flex items-end justify-between gap-2 h-full">
              {mockAnalyticsData.weeklyData.map((week, i) => (
                <div key={i} className="flex-1 flex flex-col items-center group">
                  <div className="relative w-full flex items-end justify-center gap-1 h-full pb-2">
                    <div 
                      className="bg-blue-100 group-hover:bg-blue-200 transition-all rounded-t-sm"
                      style={{ height: `${(week.scans / 250) * 100}%`, width: '15px' }}
                    />
                    <div 
                      className="bg-blue-600 group-hover:bg-blue-700 transition-all rounded-t-sm"
                      style={{ height: `${(week.vulnerabilities / 100) * 100}%`, width: '15px' }}
                    />
                  </div>
                  <span className="text-[10px] font-bold text-gray-400 mt-2 uppercase">{week.week.replace('Week ', 'W')}</span>
                </div>
              ))}
            </div>
            <div className="flex items-center space-x-6 pt-4 border-t border-border">
              <div className="flex items-center space-x-2">
                <div className="w-3 h-3 bg-blue-100 rounded-sm" />
                <span className="text-xs font-bold text-gray-500">Total Scans</span>
              </div>
              <div className="flex items-center space-x-2">
                <div className="w-3 h-3 bg-blue-600 rounded-sm" />
                <span className="text-xs font-bold text-gray-500">Vulnerabilities</span>
              </div>
            </div>
          </div>
        </div>

        {/* Severity Breakdown Widget */}
        <div className="card">
          <h3 className="text-lg font-bold mb-6">Severity Distribution</h3>
          <div className="space-y-6">
            {Object.entries(severityData).map(([severity, count]) => {
              const percentage = (count / total) * 100;
              const colors = {
                critical: 'bg-red-600',
                high: 'bg-orange-500',
                medium: 'bg-yellow-500',
                low: 'bg-green-500'
              };
              
              return (
                <div key={severity} className="space-y-2">
                  <div className="flex justify-between text-xs font-bold uppercase tracking-wider">
                    <span className="text-gray-500">{severity}</span>
                    <span className="text-gray-900">{count} ({percentage.toFixed(0)}%)</span>
                  </div>
                  <div className="h-2 bg-gray-100 rounded-full overflow-hidden">
                    <div 
                      className={`${colors[severity as keyof typeof colors]} h-full transition-all duration-1000 ease-out`}
                      style={{ width: `${percentage}%` }}
                    />
                  </div>
                </div>
              );
            })}
          </div>
          <div className="mt-8 p-4 bg-blue-50 rounded-xl border border-blue-100">
            <p className="text-xs text-blue-700 leading-relaxed font-medium">
              <strong>Tip:</strong> Critical vulnerabilities have increased by 2% this week. We recommend reviewing the recent scan history.
            </p>
          </div>
        </div>
      </div>
    );
  };

  return (
    <LoadingOverlay isLoading={isLoading} text="Loading analytics data...">
      <div className="bg-white rounded-lg shadow-md p-6 space-y-6">
        <div className="flex items-center justify-between">
          <h2 className="text-xl font-semibold text-gray-900">Analytics Dashboard</h2>
          
          <div className="flex space-x-2">
            {(['overview', 'trends', 'severity'] as const).map((metric) => (
              <button
                key={metric}
                onClick={() => handleMetricChange(metric)}
                disabled={isGeneratingData}
                className={`px-4 py-2 rounded-md text-sm font-medium transition-optimized ${
                  selectedMetric === metric
                    ? 'bg-blue-600 text-white'
                    : 'bg-gray-100 text-gray-600 hover:bg-gray-200 disabled:opacity-50'
                }`}
              >
                {metric.charAt(0).toUpperCase() + metric.slice(1)}
              </button>
            ))}
          </div>
        </div>

        <div>
          {isGeneratingData ? (
            <div className="space-y-6">
              <div className="flex items-center justify-center py-12">
                <LoadingSpinner size="lg" text="Generating analytics..." />
              </div>
              {selectedMetric === 'overview' && (
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
                  <SkeletonCard lines={3} avatar={false} button={false} height="h-24" />
                  <SkeletonCard lines={3} avatar={false} button={false} height="h-24" />
                  <SkeletonCard lines={3} avatar={false} button={false} height="h-24" />
                  <SkeletonCard lines={3} avatar={false} button={false} height="h-24" />
                </div>
              )}
              {selectedMetric === 'trends' && (
                <SkeletonCard lines={8} avatar={false} button={false} />
              )}
              {selectedMetric === 'severity' && (
                <div className="space-y-6">
                  <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
                    <SkeletonCard lines={3} avatar={false} button={false} height="h-20" />
                    <SkeletonCard lines={3} avatar={false} button={false} height="h-20" />
                    <SkeletonCard lines={3} avatar={false} button={false} height="h-20" />
                    <SkeletonCard lines={3} avatar={false} button={false} height="h-20" />
                  </div>
                  <SkeletonCard lines={6} avatar={false} button={false} />
                </div>
              )}
            </div>
          ) : (
            <>
              {selectedMetric === 'overview' && renderOverview()}
              {selectedMetric === 'trends' && renderTrends()}
              {selectedMetric === 'severity' && renderSeverity()}
            </>
          )}
        </div>
      </div>
    </LoadingOverlay>
  );
}
