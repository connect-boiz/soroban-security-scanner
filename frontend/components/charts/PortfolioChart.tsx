'use client';

import React from 'react';
import {
  PieChart,
  Pie,
  Cell,
  ResponsiveContainer,
  Legend,
  Tooltip,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  LineChart,
  Line,
  Area,
  AreaChart
} from 'recharts';
import { PortfolioData, ChartConfig, ChartDataPoint } from '@/types/charts';
import { TrendingUp, TrendingDown, DollarSign, PieChartIcon } from 'lucide-react';

interface PortfolioChartProps {
  data: PortfolioData[];
  config?: Partial<ChartConfig>;
  className?: string;
}

const DEFAULT_COLORS = [
  '#3b82f6', // blue
  '#10b981', // emerald
  '#f59e0b', // amber
  '#ef4444', // red
  '#8b5cf6', // violet
  '#ec4899', // pink
  '#06b6d4', // cyan
  '#84cc16', // lime
];

const CustomTooltip = ({ active, payload }: any) => {
  if (active && payload && payload.length) {
    const data = payload[0].payload;
    return (
      <div className="bg-white p-3 border border-gray-200 rounded-lg shadow-lg">
        <p className="font-semibold text-gray-900">{data.asset}</p>
        <p className="text-sm text-gray-600">Value: ${data.value.toLocaleString()}</p>
        <p className="text-sm text-gray-600">Percentage: {data.percentage.toFixed(1)}%</p>
        <p className={`text-sm font-medium ${
          data.change >= 0 ? 'text-green-600' : 'text-red-600'
        }`}>
          Change: {data.change >= 0 ? '+' : ''}{data.changePercent.toFixed(2)}%
        </p>
      </div>
    );
  }
  return null;
};

const CustomLabel = ({ cx, cy, midAngle, innerRadius, outerRadius, percent }: any) => {
  const RADIAN = Math.PI / 180;
  const radius = innerRadius + (outerRadius - innerRadius) * 0.5;
  const x = cx + radius * Math.cos(-midAngle * RADIAN);
  const y = cy + radius * Math.sin(-midAngle * RADIAN);

  if (percent < 0.05) return null; // Don't show label for small slices

  return (
    <text 
      x={x} 
      y={y} 
      fill="white" 
      textAnchor={x > cx ? 'start' : 'end'} 
      dominantBaseline="central"
      className="font-semibold text-sm"
    >
      {`${(percent * 100).toFixed(0)}%`}
    </text>
  );
};

export const PortfolioChart: React.FC<PortfolioChartProps> = ({
  data,
  config = {},
  className = ''
}) => {
  const {
    type = 'pie',
    title = 'Portfolio Distribution',
    showLegend = true,
    showGrid = true,
    height = 300,
    colors = DEFAULT_COLORS,
    responsive = true
  } = config;

  const totalValue = data.reduce((sum, item) => sum + item.value, 0);
  const totalChange = data.reduce((sum, item) => sum + item.change, 0);
  const totalChangePercent = (totalChange / totalValue) * 100;

  const renderPieChart = () => (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold text-gray-900">{title}</h3>
        <div className="flex items-center space-x-2">
          {totalChangePercent >= 0 ? (
            <TrendingUp className="w-4 h-4 text-green-600" />
          ) : (
            <TrendingDown className="w-4 h-4 text-red-600" />
          )}
          <span className={`text-sm font-medium ${
            totalChangePercent >= 0 ? 'text-green-600' : 'text-red-600'
          }`}>
            {totalChangePercent >= 0 ? '+' : ''}{totalChangePercent.toFixed(2)}%
          </span>
        </div>
      </div>
      
      <ResponsiveContainer width="100%" height={height}>
        <PieChart>
          <Pie
            data={data}
            cx="50%"
            cy="50%"
            labelLine={false}
            label={CustomLabel}
            outerRadius={80}
            fill="#8884d8"
            dataKey="value"
          >
            {data.map((entry, index) => (
              <Cell key={`cell-${index}`} fill={colors[index % colors.length]} />
            ))}
          </Pie>
          <Tooltip content={<CustomTooltip />} />
          {showLegend && (
            <Legend 
              verticalAlign="bottom" 
              height={36}
              formatter={(value, entry: any) => (
                <span className="text-sm text-gray-700">
                  {value} ({entry.payload.percentage.toFixed(1)}%)
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
        <h3 className="text-lg font-semibold text-gray-900">{title}</h3>
        <div className="flex items-center space-x-2">
          <DollarSign className="w-4 h-4 text-gray-600" />
          <span className="text-sm font-medium text-gray-900">
            ${totalValue.toLocaleString()}
          </span>
        </div>
      </div>
      
      <ResponsiveContainer width="100%" height={height}>
        <BarChart data={data}>
          {showGrid && <CartesianGrid strokeDasharray="3 3" className="opacity-30" />}
          <XAxis 
            dataKey="asset" 
            tick={{ fontSize: 12 }}
            className="text-gray-600"
          />
          <YAxis 
            tick={{ fontSize: 12 }}
            className="text-gray-600"
            tickFormatter={(value) => `$${(value / 1000).toFixed(0)}k`}
          />
          <Tooltip content={<CustomTooltip />} />
          <Bar dataKey="value" radius={[8, 8, 0, 0]}>
            {data.map((entry, index) => (
              <Cell key={`cell-${index}`} fill={colors[index % colors.length]} />
            ))}
          </Bar>
        </BarChart>
      </ResponsiveContainer>
    </div>
  );

  const renderAreaChart = () => {
    const cumulativeData = data.reduce((acc: any[], item, index) => {
      const cumulativeValue = acc.length > 0 
        ? acc[acc.length - 1].cumulativeValue + item.value 
        : item.value;
      return [...acc, { ...item, cumulativeValue }];
    }, []);

    return (
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h3 className="text-lg font-semibold text-gray-900">{title}</h3>
          <div className="flex items-center space-x-2">
            <PieChartIcon className="w-4 h-4 text-gray-600" />
            <span className="text-sm font-medium text-gray-900">
              ${totalValue.toLocaleString()}
            </span>
          </div>
        </div>
        
        <ResponsiveContainer width="100%" height={height}>
          <AreaChart data={cumulativeData}>
            {showGrid && <CartesianGrid strokeDasharray="3 3" className="opacity-30" />}
            <XAxis 
              dataKey="asset" 
              tick={{ fontSize: 12 }}
              className="text-gray-600"
            />
            <YAxis 
              tick={{ fontSize: 12 }}
              className="text-gray-600"
              tickFormatter={(value) => `$${(value / 1000).toFixed(0)}k`}
            />
            <Tooltip content={<CustomTooltip />} />
            <Area 
              type="monotone" 
              dataKey="cumulativeValue" 
              stroke={colors[0]} 
              fill={colors[0]}
              fillOpacity={0.6}
              strokeWidth={2}
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>
    );
  };

  return (
    <div className={`bg-white rounded-lg border border-gray-200 p-6 ${className}`}>
      {type === 'pie' && renderPieChart()}
      {type === 'bar' && renderBarChart()}
      {type === 'area' && renderAreaChart()}
      
      <div className="mt-6 grid grid-cols-2 gap-4">
        <div className="bg-gray-50 rounded-lg p-3">
          <p className="text-sm text-gray-600">Total Value</p>
          <p className="text-lg font-semibold text-gray-900">
            ${totalValue.toLocaleString()}
          </p>
        </div>
        <div className="bg-gray-50 rounded-lg p-3">
          <p className="text-sm text-gray-600">24h Change</p>
          <p className={`text-lg font-semibold ${
            totalChangePercent >= 0 ? 'text-green-600' : 'text-red-600'
          }`}>
            {totalChangePercent >= 0 ? '+' : ''}{totalChangePercent.toFixed(2)}%
          </p>
        </div>
      </div>
    </div>
  );
};

export default PortfolioChart;
