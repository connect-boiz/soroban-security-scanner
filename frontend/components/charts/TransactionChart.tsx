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
  PieChart,
  Pie,
  Cell,
} from 'recharts';
import { TransactionData, ChartConfig } from '@/types/charts';
import {
  TrendingUp,
  TrendingDown,
  ArrowUpRight,
  ArrowDownRight,
  Activity,
  DollarSign,
  Clock,
} from 'lucide-react';
import { format, subDays, isAfter, isBefore } from 'date-fns';

interface TransactionChartProps {
  data: TransactionData[];
  config?: Partial<ChartConfig>;
  timeRange?: '24h' | '7d' | '30d' | '90d' | '1y' | 'all';
  className?: string;
}

const DEFAULT_COLORS = {
  deposit: '#10b981', // emerald
  withdrawal: '#ef4444', // red
  reward: '#3b82f6', // blue
  penalty: '#f59e0b', // amber
};

const STATUS_COLORS = {
  completed: '#10b981',
  pending: '#f59e0b',
  failed: '#ef4444',
};

const CustomTooltip = ({ active, payload, label }: any) => {
  if (active && payload && payload.length) {
    return (
      <div className="bg-white p-3 border border-gray-200 rounded-lg shadow-lg">
        <p className="font-semibold text-gray-900 mb-2">
          {format(new Date(label), 'MMM dd, yyyy HH:mm')}
        </p>
        {payload.map((entry: any, index: number) => (
          <div key={index} className="flex items-center justify-between space-x-4">
            <div className="flex items-center space-x-2">
              <div className="w-3 h-3 rounded-full" style={{ backgroundColor: entry.color }} />
              <span className="text-sm text-gray-600 capitalize">{entry.dataKey}</span>
            </div>
            <span className="text-sm font-medium text-gray-900">
              ${entry.value.toLocaleString()}
            </span>
          </div>
        ))}
      </div>
    );
  }
  return null;
};

const TransactionTypeTooltip = ({ active, payload }: any) => {
  if (active && payload && payload.length) {
    const data = payload[0].payload;
    return (
      <div className="bg-white p-3 border border-gray-200 rounded-lg shadow-lg">
        <p className="font-semibold text-gray-900 capitalize">{data.type}</p>
        <p className="text-sm text-gray-600">Count: {data.count}</p>
        <p className="text-sm text-gray-600">Amount: ${data.totalAmount.toLocaleString()}</p>
        <p className="text-sm text-gray-600">Avg: ${(data.totalAmount / data.count).toFixed(2)}</p>
      </div>
    );
  }
  return null;
};

export const TransactionChart: React.FC<TransactionChartProps> = ({
  data,
  config = {},
  timeRange = '30d',
  className = '',
}) => {
  const {
    type = 'line',
    title = 'Transaction Analytics',
    showLegend = true,
    showGrid = true,
    height = 300,
    responsive = true,
  } = config;

  // Filter data based on time range
  const getFilteredData = () => {
    if (timeRange === 'all') return data;

    const now = new Date();
    const daysAgo = (
      {
        '24h': 1,
        '7d': 7,
        '30d': 30,
        '90d': 90,
      } as Record<string, number>
    )[timeRange];

    const cutoffDate = subDays(now, daysAgo!);
    return data.filter(tx => isAfter(tx.timestamp, cutoffDate));
  };

  const filteredData = getFilteredData();

  // Process data for different chart types
  const getTimeSeriesData = () => {
    const groupedByDate = filteredData.reduce((acc: any, tx) => {
      const dateKey = format(tx.timestamp, 'yyyy-MM-dd');
      if (!acc[dateKey]) {
        acc[dateKey] = {
          date: dateKey,
          timestamp: tx.timestamp,
          total: 0,
          deposit: 0,
          withdrawal: 0,
          reward: 0,
          penalty: 0,
          count: 0,
        };
      }

      acc[dateKey][tx.type] += tx.amount;
      acc[dateKey].total += tx.amount;
      acc[dateKey].count += 1;

      return acc;
    }, {});

    return Object.values(groupedByDate).sort(
      (a: any, b: any) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
    );
  };

  const getTypeDistribution = () => {
    const distribution = filteredData.reduce((acc: any, tx) => {
      if (!acc[tx.type]) {
        acc[tx.type] = {
          name: tx.type,
          type: tx.type,
          count: 0,
          totalAmount: 0,
        };
      }
      acc[tx.type].count += 1;
      acc[tx.type].totalAmount += tx.amount;
      return acc;
    }, {});

    return Object.values(distribution);
  };

  const getStatusDistribution = () => {
    const distribution = filteredData.reduce((acc: any, tx) => {
      if (!acc[tx.status]) {
        acc[tx.status] = {
          status: tx.status,
          count: 0,
          totalAmount: 0,
        };
      }
      acc[tx.status].count += 1;
      acc[tx.status].totalAmount += tx.amount;
      return acc;
    }, {});

    return Object.values(distribution);
  };

  const timeSeriesData = getTimeSeriesData();
  const typeDistribution = getTypeDistribution();
  const statusDistribution = getStatusDistribution();

  // Calculate metrics
  const totalVolume = filteredData.reduce((sum, tx) => sum + tx.amount, 0);
  const totalTransactions = filteredData.length;
  const avgTransactionSize = totalTransactions > 0 ? totalVolume / totalTransactions : 0;
  const successfulTransactions = filteredData.filter(tx => tx.status === 'completed').length;
  const successRate =
    totalTransactions > 0 ? (successfulTransactions / totalTransactions) * 100 : 0;

  const renderLineChart = () => (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold text-gray-900">{title}</h3>
        <div className="flex items-center space-x-2">
          <Activity className="w-4 h-4 text-gray-600" />
          <span className="text-sm font-medium text-gray-900">
            {totalTransactions} transactions
          </span>
        </div>
      </div>

      <ResponsiveContainer width="100%" height={height}>
        <LineChart data={timeSeriesData}>
          {showGrid && <CartesianGrid strokeDasharray="3 3" className="opacity-30" />}
          <XAxis
            dataKey="date"
            tick={{ fontSize: 12 }}
            className="text-gray-600"
            tickFormatter={value => format(new Date(value), 'MMM dd')}
          />
          <YAxis
            tick={{ fontSize: 12 }}
            className="text-gray-600"
            tickFormatter={value => `$${(value / 1000).toFixed(0)}k`}
          />
          <Tooltip content={<CustomTooltip />} />
          {showLegend && <Legend />}
          <Line
            type="monotone"
            dataKey="total"
            stroke="#3b82f6"
            strokeWidth={2}
            dot={{ r: 4 }}
            name="Total Volume"
          />
          <Line
            type="monotone"
            dataKey="deposit"
            stroke="#10b981"
            strokeWidth={2}
            dot={{ r: 3 }}
            name="Deposits"
          />
          <Line
            type="monotone"
            dataKey="reward"
            stroke="#3b82f6"
            strokeWidth={2}
            dot={{ r: 3 }}
            name="Rewards"
          />
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
          <span className="text-sm font-medium text-gray-900">
            ${totalVolume.toLocaleString()} total
          </span>
        </div>
      </div>

      <ResponsiveContainer width="100%" height={height}>
        <AreaChart data={timeSeriesData}>
          {showGrid && <CartesianGrid strokeDasharray="3 3" className="opacity-30" />}
          <XAxis
            dataKey="date"
            tick={{ fontSize: 12 }}
            className="text-gray-600"
            tickFormatter={value => format(new Date(value), 'MMM dd')}
          />
          <YAxis
            tick={{ fontSize: 12 }}
            className="text-gray-600"
            tickFormatter={value => `$${(value / 1000).toFixed(0)}k`}
          />
          <Tooltip content={<CustomTooltip />} />
          {showLegend && <Legend />}
          <Area
            type="monotone"
            dataKey="deposit"
            stackId="1"
            stroke={DEFAULT_COLORS.deposit}
            fill={DEFAULT_COLORS.deposit}
            fillOpacity={0.6}
            name="Deposits"
          />
          <Area
            type="monotone"
            dataKey="reward"
            stackId="1"
            stroke={DEFAULT_COLORS.reward}
            fill={DEFAULT_COLORS.reward}
            fillOpacity={0.6}
            name="Rewards"
          />
          <Area
            type="monotone"
            dataKey="withdrawal"
            stackId="1"
            stroke={DEFAULT_COLORS.withdrawal}
            fill={DEFAULT_COLORS.withdrawal}
            fillOpacity={0.6}
            name="Withdrawals"
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );

  const renderPieChart = () => (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold text-gray-900">Transaction Types</h3>
        <div className="flex items-center space-x-2">
          <DollarSign className="w-4 h-4 text-gray-600" />
          <span className="text-sm font-medium text-gray-900">${totalVolume.toLocaleString()}</span>
        </div>
      </div>

      <ResponsiveContainer width="100%" height={height}>
        <PieChart>
          <Pie
            data={typeDistribution}
            cx="50%"
            cy="50%"
            labelLine={false}
            label={({ name, percent }: any) => `${name} ${(percent! * 100).toFixed(0)}%`}
            outerRadius={80}
            fill="#8884d8"
            dataKey="totalAmount"
          >
            {typeDistribution.map((entry: any, index: number) => (
              <Cell
                key={`cell-${index}`}
                fill={DEFAULT_COLORS[entry.type as keyof typeof DEFAULT_COLORS]}
              />
            ))}
          </Pie>
          <Tooltip content={<TransactionTypeTooltip />} />
          {showLegend && (
            <Legend
              verticalAlign="bottom"
              height={36}
              formatter={(value, entry: any) => (
                <span className="text-sm text-gray-700 capitalize">
                  {value} (${entry.payload.totalAmount.toLocaleString()})
                </span>
              )}
            />
          )}
        </PieChart>
      </ResponsiveContainer>
    </div>
  );

  const renderBarChart = () => (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold text-gray-900">Transaction Status</h3>
        <div className="flex items-center space-x-2">
          <Clock className="w-4 h-4 text-gray-600" />
          <span className="text-sm font-medium text-gray-900">
            {successRate.toFixed(1)}% success rate
          </span>
        </div>
      </div>

      <ResponsiveContainer width="100%" height={height}>
        <BarChart data={statusDistribution}>
          {showGrid && <CartesianGrid strokeDasharray="3 3" className="opacity-30" />}
          <XAxis
            dataKey="status"
            tick={{ fontSize: 12 }}
            className="text-gray-600"
            tickFormatter={value => value.charAt(0).toUpperCase() + value.slice(1)}
          />
          <YAxis tick={{ fontSize: 12 }} className="text-gray-600" />
          <Tooltip
            content={({ active, payload }: any) => {
              if (active && payload && payload.length) {
                const data = payload[0].payload;
                return (
                  <div className="bg-white p-3 border border-gray-200 rounded-lg shadow-lg">
                    <p className="font-semibold text-gray-900 capitalize">{data.status}</p>
                    <p className="text-sm text-gray-600">Count: {data.count}</p>
                    <p className="text-sm text-gray-600">
                      Amount: ${data.totalAmount.toLocaleString()}
                    </p>
                  </div>
                );
              }
              return null;
            }}
          />
          <Bar dataKey="count" radius={[8, 8, 0, 0]}>
            {statusDistribution.map((entry: any, index: number) => (
              <Cell
                key={`cell-${index}`}
                fill={STATUS_COLORS[entry.status as keyof typeof STATUS_COLORS]}
              />
            ))}
          </Bar>
        </BarChart>
      </ResponsiveContainer>
    </div>
  );

  return (
    <div className={`bg-white rounded-lg border border-gray-200 p-6 ${className}`}>
      {type === 'line' && renderLineChart()}
      {type === 'area' && renderAreaChart()}
      {type === 'pie' && renderPieChart()}
      {type === 'bar' && renderBarChart()}

      <div className="mt-6 grid grid-cols-2 md:grid-cols-4 gap-4">
        <div className="bg-gray-50 rounded-lg p-3">
          <p className="text-sm text-gray-600">Total Volume</p>
          <p className="text-lg font-semibold text-gray-900">${totalVolume.toLocaleString()}</p>
        </div>
        <div className="bg-gray-50 rounded-lg p-3">
          <p className="text-sm text-gray-600">Transactions</p>
          <p className="text-lg font-semibold text-gray-900">{totalTransactions}</p>
        </div>
        <div className="bg-gray-50 rounded-lg p-3">
          <p className="text-sm text-gray-600">Avg Size</p>
          <p className="text-lg font-semibold text-gray-900">${avgTransactionSize.toFixed(2)}</p>
        </div>
        <div className="bg-gray-50 rounded-lg p-3">
          <p className="text-sm text-gray-600">Success Rate</p>
          <p className="text-lg font-semibold text-green-600">{successRate.toFixed(1)}%</p>
        </div>
      </div>
    </div>
  );
};

export default TransactionChart;
