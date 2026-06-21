'use client';

import React from 'react';
import {
  LineChart,
  Line,
  AreaChart,
  Area,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  RadarChart,
  PolarGrid,
  PolarAngleAxis,
  PolarRadiusAxis,
  Radar,
  ComposedChart,
} from 'recharts';
import { PerformanceMetrics, ChartConfig } from '@/types/charts';
import {
  TrendingUp,
  TrendingDown,
  Trophy,
  Target,
  Clock,
  DollarSign,
  Award,
  Activity,
  Zap,
} from 'lucide-react';
import { format } from 'date-fns';

interface PerformanceChartProps {
  data: PerformanceMetrics[];
  config?: Partial<ChartConfig>;
  metrics?: (
    | 'reputation'
    | 'completedBounties'
    | 'totalEarned'
    | 'successRate'
    | 'avgCompletionTime'
  )[];
  className?: string;
}

const METRIC_COLORS = {
  reputation: '#3b82f6', // blue
  completedBounties: '#10b981', // emerald
  totalEarned: '#f59e0b', // amber
  successRate: '#8b5cf6', // violet
  avgCompletionTime: '#ef4444', // red
};

const METRIC_LABELS = {
  reputation: 'Reputation Score',
  completedBounties: 'Completed Bounties',
  totalEarned: 'Total Earned ($)',
  successRate: 'Success Rate (%)',
  avgCompletionTime: 'Avg Completion Time (hrs)',
};

const CustomTooltip = ({ active, payload, label }: any) => {
  if (active && payload && payload.length) {
    return (
      <div className="bg-white p-3 border border-gray-200 rounded-lg shadow-lg">
        <p className="font-semibold text-gray-900 mb-2">
          {format(new Date(label), 'MMM dd, yyyy')}
        </p>
        {payload.map((entry: any, index: number) => (
          <div key={index} className="flex items-center justify-between space-x-4 mb-1">
            <div className="flex items-center space-x-2">
              <div className="w-3 h-3 rounded-full" style={{ backgroundColor: entry.color }} />
              <span className="text-sm text-gray-600">
                {METRIC_LABELS[entry.dataKey as keyof typeof METRIC_LABELS]}
              </span>
            </div>
            <span className="text-sm font-medium text-gray-900">
              {entry.dataKey === 'totalEarned'
                ? `$${entry.value.toLocaleString()}`
                : entry.dataKey === 'successRate'
                  ? `${entry.value.toFixed(1)}%`
                  : entry.dataKey === 'avgCompletionTime'
                    ? `${entry.value.toFixed(1)}h`
                    : entry.value.toLocaleString()}
            </span>
          </div>
        ))}
      </div>
    );
  }
  return null;
};

const RadarTooltip = ({ active, payload }: any) => {
  if (active && payload && payload.length) {
    const data = payload[0].payload;
    return (
      <div className="bg-white p-3 border border-gray-200 rounded-lg shadow-lg">
        <p className="font-semibold text-gray-900 mb-2">
          {format(new Date(data.date), 'MMM dd, yyyy')}
        </p>
        {Object.entries(METRIC_LABELS).map(([key, label]) => (
          <div key={key} className="flex items-center justify-between space-x-4 mb-1">
            <span className="text-sm text-gray-600">{label}</span>
            <span className="text-sm font-medium text-gray-900">
              {key === 'totalEarned'
                ? `$${data[key].toLocaleString()}`
                : key === 'successRate'
                  ? `${data[key].toFixed(1)}%`
                  : key === 'avgCompletionTime'
                    ? `${data[key].toFixed(1)}h`
                    : data[key].toLocaleString()}
            </span>
          </div>
        ))}
      </div>
    );
  }
  return null;
};

export const PerformanceChart: React.FC<PerformanceChartProps> = ({
  data,
  config = {},
  metrics = ['reputation', 'completedBounties', 'totalEarned', 'successRate'],
  className = '',
}) => {
  const {
    type = 'line',
    title = 'Performance Metrics',
    showLegend = true,
    showGrid = true,
    height = 300,
    responsive = true,
  } = config;

  // Normalize data for radar chart (scale all metrics to 0-100)
  const getNormalizedData = () => {
    if (data.length === 0) return [];

    const maxValues = {
      reputation: Math.max(...data.map(d => d.reputation)),
      completedBounties: Math.max(...data.map(d => d.completedBounties)),
      totalEarned: Math.max(...data.map(d => d.totalEarned)),
      successRate: 100, // Already percentage
      avgCompletionTime: Math.max(...data.map(d => d.avgCompletionTime)),
    };

    return data.map(item => ({
      ...item,
      reputationNormalized: (item.reputation / maxValues.reputation) * 100,
      completedBountiesNormalized: (item.completedBounties / maxValues.completedBounties) * 100,
      totalEarnedNormalized: (item.totalEarned / maxValues.totalEarned) * 100,
      successRateNormalized: item.successRate,
      avgCompletionTimeNormalized:
        100 - (item.avgCompletionTime / maxValues.avgCompletionTime) * 100, // Invert time (lower is better)
    }));
  };

  // Calculate current metrics
  const currentMetrics = data.length > 0 ? data[data.length - 1] : null;
  const previousMetrics = data.length > 1 ? data[data.length - 2] : null;

  const calculateChange = (current: number, previous: number) => {
    if (!previous) return 0;
    return ((current - previous) / previous) * 100;
  };

  const renderLineChart = () => (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold text-gray-900">{title}</h3>
        <div className="flex items-center space-x-2">
          <Activity className="w-4 h-4 text-gray-600" />
          <span className="text-sm font-medium text-gray-900">{data.length} data points</span>
        </div>
      </div>

      <ResponsiveContainer width="100%" height={height}>
        <LineChart data={data}>
          {showGrid && <CartesianGrid strokeDasharray="3 3" className="opacity-30" />}
          <XAxis
            dataKey="date"
            tick={{ fontSize: 12 }}
            className="text-gray-600"
            tickFormatter={value => format(new Date(value), 'MMM dd')}
          />
          <YAxis yAxisId="left" tick={{ fontSize: 12 }} className="text-gray-600" />
          <YAxis
            yAxisId="right"
            orientation="right"
            tick={{ fontSize: 12 }}
            className="text-gray-600"
          />
          <Tooltip content={<CustomTooltip />} />
          {showLegend && <Legend />}

          {metrics.includes('reputation') && (
            <Line
              yAxisId="left"
              type="monotone"
              dataKey="reputation"
              stroke={METRIC_COLORS.reputation}
              strokeWidth={2}
              dot={{ r: 4 }}
              name="Reputation"
            />
          )}

          {metrics.includes('completedBounties') && (
            <Line
              yAxisId="left"
              type="monotone"
              dataKey="completedBounties"
              stroke={METRIC_COLORS.completedBounties}
              strokeWidth={2}
              dot={{ r: 4 }}
              name="Completed"
            />
          )}

          {metrics.includes('totalEarned') && (
            <Line
              yAxisId="right"
              type="monotone"
              dataKey="totalEarned"
              stroke={METRIC_COLORS.totalEarned}
              strokeWidth={2}
              dot={{ r: 4 }}
              name="Earned ($)"
            />
          )}

          {metrics.includes('successRate') && (
            <Line
              yAxisId="left"
              type="monotone"
              dataKey="successRate"
              stroke={METRIC_COLORS.successRate}
              strokeWidth={2}
              dot={{ r: 4 }}
              name="Success %"
            />
          )}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );

  const renderAreaChart = () => (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold text-gray-900">{title}</h3>
        <div className="flex items-center space-x-2">
          <TrendingUp className="w-4 h-4 text-gray-600" />
          <span className="text-sm font-medium text-gray-900">Growth Overview</span>
        </div>
      </div>

      <ResponsiveContainer width="100%" height={height}>
        <AreaChart data={data}>
          {showGrid && <CartesianGrid strokeDasharray="3 3" className="opacity-30" />}
          <XAxis
            dataKey="date"
            tick={{ fontSize: 12 }}
            className="text-gray-600"
            tickFormatter={value => format(new Date(value), 'MMM dd')}
          />
          <YAxis tick={{ fontSize: 12 }} className="text-gray-600" />
          <Tooltip content={<CustomTooltip />} />
          {showLegend && <Legend />}

          {metrics.includes('totalEarned') && (
            <Area
              type="monotone"
              dataKey="totalEarned"
              stroke={METRIC_COLORS.totalEarned}
              fill={METRIC_COLORS.totalEarned}
              fillOpacity={0.6}
              name="Total Earned"
            />
          )}

          {metrics.includes('completedBounties') && (
            <Area
              type="monotone"
              dataKey="completedBounties"
              stroke={METRIC_COLORS.completedBounties}
              fill={METRIC_COLORS.completedBounties}
              fillOpacity={0.6}
              name="Completed Bounties"
            />
          )}
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );

  const renderRadarChart = () => {
    const normalizedData = getNormalizedData();
    const latestData = normalizedData[normalizedData.length - 1];

    const radarData = metrics.map(metric => ({
      metric: METRIC_LABELS[metric].replace(/\s*\([^)]*\)/, ''), // Remove parentheses
      value: latestData ? latestData[`${metric}Normalized`] : 0,
      fullMark: 100,
    }));

    return (
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h3 className="text-lg font-semibold text-gray-900">{title}</h3>
          <div className="flex items-center space-x-2">
            <Target className="w-4 h-4 text-gray-600" />
            <span className="text-sm font-medium text-gray-900">Overall Performance</span>
          </div>
        </div>

        <ResponsiveContainer width="100%" height={height}>
          <RadarChart data={radarData}>
            <PolarGrid strokeDasharray="3 3" className="opacity-30" />
            <PolarAngleAxis dataKey="metric" tick={{ fontSize: 11 }} className="text-gray-600" />
            <PolarRadiusAxis
              angle={90}
              domain={[0, 100]}
              tick={{ fontSize: 10 }}
              className="text-gray-600"
            />
            <Radar
              name="Performance"
              dataKey="value"
              stroke="#3b82f6"
              fill="#3b82f6"
              fillOpacity={0.6}
              strokeWidth={2}
            />
            <Tooltip content={<RadarTooltip />} />
          </RadarChart>
        </ResponsiveContainer>
      </div>
    );
  };

  const renderBarChart = () => (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold text-gray-900">{title}</h3>
        <div className="flex items-center space-x-2">
          <Trophy className="w-4 h-4 text-gray-600" />
          <span className="text-sm font-medium text-gray-900">Monthly Comparison</span>
        </div>
      </div>

      <ResponsiveContainer width="100%" height={height}>
        <BarChart data={data}>
          {showGrid && <CartesianGrid strokeDasharray="3 3" className="opacity-30" />}
          <XAxis
            dataKey="date"
            tick={{ fontSize: 12 }}
            className="text-gray-600"
            tickFormatter={value => format(new Date(value), 'MMM dd')}
          />
          <YAxis tick={{ fontSize: 12 }} className="text-gray-600" />
          <Tooltip content={<CustomTooltip />} />
          {showLegend && <Legend />}

          {metrics.includes('completedBounties') && (
            <Bar
              dataKey="completedBounties"
              fill={METRIC_COLORS.completedBounties}
              name="Completed"
            />
          )}

          {metrics.includes('reputation') && (
            <Bar dataKey="reputation" fill={METRIC_COLORS.reputation} name="Reputation" />
          )}
        </BarChart>
      </ResponsiveContainer>
    </div>
  );

  return (
    <div className={`bg-white rounded-lg border border-gray-200 p-6 ${className}`}>
      {type === 'line' && renderLineChart()}
      {type === 'area' && renderAreaChart()}
      {type === 'radar' && renderRadarChart()}
      {type === 'bar' && renderBarChart()}

      {currentMetrics && (
        <div className="mt-6 grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="bg-gray-50 rounded-lg p-3">
            <p className="text-sm text-gray-600">Reputation</p>
            <div className="flex items-center space-x-2">
              <p className="text-lg font-semibold text-gray-900">
                {currentMetrics.reputation.toLocaleString()}
              </p>
              {previousMetrics && (
                <span
                  className={`text-xs font-medium ${
                    calculateChange(currentMetrics.reputation, previousMetrics.reputation) >= 0
                      ? 'text-green-600'
                      : 'text-red-600'
                  }`}
                >
                  {calculateChange(currentMetrics.reputation, previousMetrics.reputation) >= 0
                    ? '+'
                    : ''}
                  {calculateChange(currentMetrics.reputation, previousMetrics.reputation).toFixed(
                    1
                  )}
                  %
                </span>
              )}
            </div>
          </div>

          <div className="bg-gray-50 rounded-lg p-3">
            <p className="text-sm text-gray-600">Completed</p>
            <div className="flex items-center space-x-2">
              <p className="text-lg font-semibold text-gray-900">
                {currentMetrics.completedBounties}
              </p>
              {previousMetrics && (
                <span
                  className={`text-xs font-medium ${
                    calculateChange(
                      currentMetrics.completedBounties,
                      previousMetrics.completedBounties
                    ) >= 0
                      ? 'text-green-600'
                      : 'text-red-600'
                  }`}
                >
                  {calculateChange(
                    currentMetrics.completedBounties,
                    previousMetrics.completedBounties
                  ) >= 0
                    ? '+'
                    : ''}
                  {calculateChange(
                    currentMetrics.completedBounties,
                    previousMetrics.completedBounties
                  ).toFixed(1)}
                  %
                </span>
              )}
            </div>
          </div>

          <div className="bg-gray-50 rounded-lg p-3">
            <p className="text-sm text-gray-600">Total Earned</p>
            <div className="flex items-center space-x-2">
              <p className="text-lg font-semibold text-gray-900">
                ${currentMetrics.totalEarned.toLocaleString()}
              </p>
              {previousMetrics && (
                <span
                  className={`text-xs font-medium ${
                    calculateChange(currentMetrics.totalEarned, previousMetrics.totalEarned) >= 0
                      ? 'text-green-600'
                      : 'text-red-600'
                  }`}
                >
                  {calculateChange(currentMetrics.totalEarned, previousMetrics.totalEarned) >= 0
                    ? '+'
                    : ''}
                  {calculateChange(currentMetrics.totalEarned, previousMetrics.totalEarned).toFixed(
                    1
                  )}
                  %
                </span>
              )}
            </div>
          </div>

          <div className="bg-gray-50 rounded-lg p-3">
            <p className="text-sm text-gray-600">Success Rate</p>
            <div className="flex items-center space-x-2">
              <p className="text-lg font-semibold text-gray-900">
                {currentMetrics.successRate.toFixed(1)}%
              </p>
              {previousMetrics && (
                <span
                  className={`text-xs font-medium ${
                    calculateChange(currentMetrics.successRate, previousMetrics.successRate) >= 0
                      ? 'text-green-600'
                      : 'text-red-600'
                  }`}
                >
                  {calculateChange(currentMetrics.successRate, previousMetrics.successRate) >= 0
                    ? '+'
                    : ''}
                  {calculateChange(currentMetrics.successRate, previousMetrics.successRate).toFixed(
                    1
                  )}
                  %
                </span>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default PerformanceChart;
