import React from 'react';
import { useDashboardStore } from '../store/dashboardStore';

const getSeverityColor = (severity: string) => {
  switch (severity) {
    case 'critical':
      return 'bg-red-600 text-white border-red-700';
    case 'high':
      return 'bg-orange-600 text-white border-orange-700';
    case 'medium':
      return 'bg-yellow-500 text-gray-900 border-yellow-600';
    case 'low':
      return 'bg-blue-600 text-white border-blue-700';
    default:
      return 'bg-gray-500 text-white border-gray-600';
  }
};

const getSeverityBgColor = (severity: string) => {
  switch (severity) {
    case 'critical':
      return 'bg-red-50 border-red-200';
    case 'high':
      return 'bg-orange-50 border-orange-200';
    case 'medium':
      return 'bg-yellow-50 border-yellow-200';
    case 'low':
      return 'bg-blue-50 border-blue-200';
    default:
      return 'bg-gray-50 border-gray-200';
  }
};

const getSeverityTextColor = (severity: string) => {
  switch (severity) {
    case 'critical':
      return 'text-red-700';
    case 'high':
      return 'text-orange-700';
    case 'medium':
      return 'text-yellow-700';
    case 'low':
      return 'text-blue-700';
    default:
      return 'text-gray-700';
  }
};

export const SummaryWidget: React.FC = () => {
  const { metrics } = useDashboardStore();

  const severityData = [
    { label: 'Critical', count: metrics.criticalIssues, color: getSeverityColor('critical'), bgColor: getSeverityBgColor('critical'), textColor: getSeverityTextColor('critical') },
    { label: 'High', count: metrics.highIssues, color: getSeverityColor('high'), bgColor: getSeverityBgColor('high'), textColor: getSeverityTextColor('high') },
    { label: 'Medium', count: metrics.mediumIssues, color: getSeverityColor('medium'), bgColor: getSeverityBgColor('medium'), textColor: getSeverityTextColor('medium') },
    { label: 'Low', count: metrics.lowIssues, color: getSeverityColor('low'), bgColor: getSeverityBgColor('low'), textColor: getSeverityTextColor('low') },
  ];

  return (
    <div className="bg-white rounded-lg shadow-md p-6 border border-gray-200">
      <h2 className="text-xl font-bold text-gray-800 mb-6">Vulnerability Summary</h2>
      
      {/* Severity Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
        {severityData.map((severity) => (
          <div 
            key={severity.label} 
            className={`${severity.bgColor} border rounded-lg p-4 text-center transition-all hover:shadow-md focus-within:ring-2 focus-within:ring-offset-2 focus-within:ring-indigo-500`}
            role="article"
            aria-label={`${severity.label} vulnerabilities: ${severity.count}`}
          >
            <div className={`text-3xl font-bold ${severity.color} rounded-full w-16 h-16 flex items-center justify-center mx-auto mb-3 border-2`} 
                 aria-hidden="true">
              {severity.count}
            </div>
            <div className={`text-sm font-semibold ${severity.textColor}`}>{severity.label}</div>
            <div className="text-xs text-gray-500 mt-1">Issues</div>
          </div>
        ))}
      </div>
      
      {/* Summary Statistics */}
      <div className="pt-6 border-t border-gray-200">
        <h3 className="text-lg font-semibold text-gray-800 mb-4">Scan Statistics</h3>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          <div className="text-center p-4 bg-gray-50 rounded-lg">
            <div className="text-2xl font-bold text-gray-800 mb-1">{metrics.totalScans}</div>
            <div className="text-sm text-gray-600 font-medium">Total Scans</div>
          </div>
          <div className="text-center p-4 bg-gray-50 rounded-lg">
            <div className="text-2xl font-bold text-green-600 mb-1">{metrics.passRate.toFixed(1)}%</div>
            <div className="text-sm text-gray-600 font-medium">Pass Rate</div>
          </div>
          <div className="text-center p-4 bg-gray-50 rounded-lg">
            <div className="text-2xl font-bold text-blue-600 mb-1">{(metrics.averageExecutionTime / 1000).toFixed(2)}s</div>
            <div className="text-sm text-gray-600 font-medium">Avg Execution Time</div>
          </div>
        </div>
      </div>
    </div>
  );
};
