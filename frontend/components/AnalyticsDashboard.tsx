'use client';

import { useState, useMemo, useCallback } from 'react';

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

  // Memoize calculations for performance
  const metrics = useMemo(() => ({
    vulnerabilityRate: ((mockAnalyticsData.vulnerabilitiesFound / mockAnalyticsData.totalScans) * 100).toFixed(1),
    contractsPerScan: (mockAnalyticsData.contractsAnalyzed / mockAnalyticsData.totalScans).toFixed(2),
    totalIssues: Object.values(mockAnalyticsData.severityBreakdown).reduce((a, b) => a + b, 0)
  }), []);

  const handleMetricChange = useCallback((metric: 'overview' | 'trends' | 'severity') => {
    setSelectedMetric(metric);
  }, []);

  const renderOverview = () => (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      <div className="bg-blue-50 p-4 rounded-lg border border-blue-200">
        <h3 className="text-sm font-medium text-blue-800 mb-1">Total Scans</h3>
        <p className="text-2xl font-bold text-blue-900">{mockAnalyticsData.totalScans.toLocaleString()}</p>
        <p className="text-xs text-blue-600 mt-1">Last 30 days</p>
      </div>
      
      <div className="bg-red-50 p-4 rounded-lg border border-red-200">
        <h3 className="text-sm font-medium text-red-800 mb-1">Vulnerabilities Found</h3>
        <p className="text-2xl font-bold text-red-900">{mockAnalyticsData.vulnerabilitiesFound}</p>
        <p className="text-xs text-red-600 mt-1">{metrics.vulnerabilityRate}% detection rate</p>
      </div>
      
      <div className="bg-green-50 p-4 rounded-lg border border-green-200">
        <h3 className="text-sm font-medium text-green-800 mb-1">Contracts Analyzed</h3>
        <p className="text-2xl font-bold text-green-900">{mockAnalyticsData.contractsAnalyzed.toLocaleString()}</p>
        <p className="text-xs text-green-600 mt-1">{metrics.contractsPerScan} contracts/scan</p>
      </div>
      
      <div className="bg-purple-50 p-4 rounded-lg border border-purple-200">
        <h3 className="text-sm font-medium text-purple-800 mb-1">Avg Scan Time</h3>
        <p className="text-2xl font-bold text-purple-900">{mockAnalyticsData.averageScanTime}s</p>
        <p className="text-xs text-purple-600 mt-1">Per contract</p>
      </div>
    </div>
  );

  const renderTrends = () => (
    <div className="space-y-6">
      <div className="bg-white p-4 rounded-lg border">
        <h3 className="text-lg font-medium text-gray-900 mb-4">Weekly Scan Trends</h3>
        <div className="space-y-3">
          {mockAnalyticsData.weeklyData.map((week, index) => (
            <div key={week.week} className="flex items-center justify-between">
              <span className="text-sm text-gray-600 w-16">{week.week}</span>
              <div className="flex-1 mx-4">
                <div className="flex items-center space-x-2">
                  <div className="flex-1 bg-gray-200 rounded-full h-6 relative overflow-hidden">
                    <div 
                      className="bg-blue-500 h-full rounded-full transition-all duration-500 ease-out"
                      style={{ width: `${(week.scans / 250) * 100}%` }}
                    />
                  </div>
                  <span className="text-xs font-medium text-gray-700 w-12 text-right">
                    {week.scans}
                  </span>
                </div>
                <div className="flex items-center space-x-2 mt-1">
                  <div className="flex-1 bg-gray-100 rounded-full h-4 relative overflow-hidden">
                    <div 
                      className="bg-red-400 h-full rounded-full transition-all duration-500 ease-out"
                      style={{ width: `${(week.vulnerabilities / 80) * 100}%` }}
                    />
                  </div>
                  <span className="text-xs text-gray-600 w-12 text-right">
                    {week.vulnerabilities}
                  </span>
                </div>
              </div>
            </div>
          ))}
        </div>
        <div className="flex items-center space-x-4 mt-4 text-xs">
          <div className="flex items-center space-x-1">
            <div className="w-3 h-3 bg-blue-500 rounded-full" />
            <span className="text-gray-600">Scans</span>
          </div>
          <div className="flex items-center space-x-1">
            <div className="w-3 h-3 bg-red-400 rounded-full" />
            <span className="text-gray-600">Vulnerabilities</span>
          </div>
        </div>
      </div>
    </div>
  );

  const renderSeverity = () => {
    const total = metrics.totalIssues;
    const data = mockAnalyticsData.severityBreakdown;
    
    return (
      <div className="space-y-6">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          <div className="bg-red-50 p-4 rounded-lg border border-red-200">
            <h3 className="text-sm font-medium text-red-800 mb-1">Critical</h3>
            <p className="text-2xl font-bold text-red-900">{data.critical}</p>
            <p className="text-xs text-red-600 mt-1">{((data.critical / total) * 100).toFixed(1)}% of total</p>
          </div>
          
          <div className="bg-orange-50 p-4 rounded-lg border border-orange-200">
            <h3 className="text-sm font-medium text-orange-800 mb-1">High</h3>
            <p className="text-2xl font-bold text-orange-900">{data.high}</p>
            <p className="text-xs text-orange-600 mt-1">{((data.high / total) * 100).toFixed(1)}% of total</p>
          </div>
          
          <div className="bg-yellow-50 p-4 rounded-lg border border-yellow-200">
            <h3 className="text-sm font-medium text-yellow-800 mb-1">Medium</h3>
            <p className="text-2xl font-bold text-yellow-900">{data.medium}</p>
            <p className="text-xs text-yellow-600 mt-1">{((data.medium / total) * 100).toFixed(1)}% of total</p>
          </div>
          
          <div className="bg-green-50 p-4 rounded-lg border border-green-200">
            <h3 className="text-sm font-medium text-green-800 mb-1">Low</h3>
            <p className="text-2xl font-bold text-green-900">{data.low}</p>
            <p className="text-xs text-green-600 mt-1">{((data.low / total) * 100).toFixed(1)}% of total</p>
          </div>
        </div>
        
        <div className="bg-white p-4 rounded-lg border">
          <h3 className="text-lg font-medium text-gray-900 mb-4">Severity Distribution</h3>
          <div className="space-y-2">
            {Object.entries(data).map(([severity, count]) => {
              const percentage = (count / total) * 100;
              const colors = {
                critical: 'bg-red-500',
                high: 'bg-orange-500',
                medium: 'bg-yellow-500',
                low: 'bg-green-500'
              };
              
              return (
                <div key={severity} className="flex items-center space-x-3">
                  <span className="text-sm font-medium text-gray-700 w-16 capitalize">
                    {severity}
                  </span>
                  <div className="flex-1 bg-gray-200 rounded-full h-6 relative overflow-hidden">
                    <div 
                      className={`${colors[severity as keyof typeof colors]} h-full rounded-full transition-all duration-500 ease-out`}
                      style={{ width: `${percentage}%` }}
                    />
                  </div>
                  <span className="text-sm font-medium text-gray-700 w-12 text-right">
                    {count}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    );
  };

  return (
    <div className="bg-white rounded-lg shadow-md p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold text-gray-900">Analytics Dashboard</h2>
        
        <div className="flex space-x-2">
          {(['overview', 'trends', 'severity'] as const).map((metric) => (
            <button
              key={metric}
              onClick={() => handleMetricChange(metric)}
              className={`px-4 py-2 rounded-md text-sm font-medium transition-optimized ${
                selectedMetric === metric
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
              }`}
            >
              {metric.charAt(0).toUpperCase() + metric.slice(1)}
            </button>
          ))}
        </div>
      </div>

      <div>
        {selectedMetric === 'overview' && renderOverview()}
        {selectedMetric === 'trends' && renderTrends()}
        {selectedMetric === 'severity' && renderSeverity()}
      </div>
    </div>
  );
}
