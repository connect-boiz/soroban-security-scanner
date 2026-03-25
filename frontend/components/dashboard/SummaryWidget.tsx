import React from 'react';
import { useDashboardStore } from '../store/dashboardStore';

const getSeverityColor = (severity: string) => {
  switch (severity) {
    case 'critical':
      return 'bg-red-500 text-white';
    case 'high':
      return 'bg-orange-500 text-white';
    case 'medium':
      return 'bg-yellow-500 text-black';
    case 'low':
      return 'bg-blue-500 text-white';
    default:
      return 'bg-gray-500 text-white';
  }
};

const getSeverityBgColor = (severity: string) => {
  switch (severity) {
    case 'critical':
      return 'bg-red-100';
    case 'high':
      return 'bg-orange-100';
    case 'medium':
      return 'bg-yellow-100';
    case 'low':
      return 'bg-blue-100';
    default:
      return 'bg-gray-100';
  }
};

export const SummaryWidget: React.FC = () => {
  const { metrics } = useDashboardStore();

  const severityData = [
    { label: 'Critical', count: metrics.criticalIssues, color: getSeverityColor('critical'), bgColor: getSeverityBgColor('critical') },
    { label: 'High', count: metrics.highIssues, color: getSeverityColor('high'), bgColor: getSeverityBgColor('high') },
    { label: 'Medium', count: metrics.mediumIssues, color: getSeverityColor('medium'), bgColor: getSeverityBgColor('medium') },
    { label: 'Low', count: metrics.lowIssues, color: getSeverityColor('low'), bgColor: getSeverityBgColor('low') },
  ];

  return (
    <div className="bg-white rounded-lg shadow-md p-6">
      <h2 className="text-xl font-bold text-gray-800 mb-4">Vulnerability Summary</h2>
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {severityData.map((severity) => (
          <div key={severity.label} className={`${severity.bgColor} rounded-lg p-4 text-center`}>
            <div className={`text-2xl font-bold ${severity.color} rounded-full w-12 h-12 flex items-center justify-center mx-auto mb-2`}>
              {severity.count}
            </div>
            <div className="text-sm font-medium text-gray-700">{severity.label}</div>
          </div>
        ))}
      </div>
      <div className="mt-6 pt-4 border-t border-gray-200">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
          <div className="text-center">
            <div className="text-lg font-semibold text-gray-800">{metrics.totalScans}</div>
            <div className="text-gray-600">Total Scans</div>
          </div>
          <div className="text-center">
            <div className="text-lg font-semibold text-gray-800">{metrics.passRate.toFixed(1)}%</div>
            <div className="text-gray-600">Pass Rate</div>
          </div>
          <div className="text-center">
            <div className="text-lg font-semibold text-gray-800">{(metrics.averageExecutionTime / 1000).toFixed(2)}s</div>
            <div className="text-gray-600">Avg Execution Time</div>
          </div>
        </div>
      </div>
    </div>
  );
};
