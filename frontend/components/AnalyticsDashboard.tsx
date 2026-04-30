'use client';

import React, { useState } from 'react';
import { PortfolioChart, TransactionChart, PerformanceChart } from '@/components/charts';
import TransactionHistory from './TransactionHistory';
import { PortfolioData, TransactionData, PerformanceMetrics, ChartFilter } from '@/types/charts';
import { 
  BarChart3, 
  TrendingUp, 
  PieChart, 
  Activity,
  Filter,
  Calendar,
  Download,
  RefreshCw
} from 'lucide-react';

interface AnalyticsDashboardProps {
  className?: string;
}

const mockPortfolioData: PortfolioData[] = [
  { asset: 'XLM', value: 15000, percentage: 35.7, change: 250, changePercent: 1.7 },
  { asset: 'USDC', value: 12000, percentage: 28.6, change: 0, changePercent: 0 },
  { asset: 'ETH', value: 8000, percentage: 19.0, change: -300, changePercent: -3.6 },
  { asset: 'BTC', value: 5000, percentage: 11.9, change: 150, changePercent: 3.1 },
  { asset: 'Other', value: 2000, percentage: 4.8, change: 50, changePercent: 2.6 }
];

const mockTransactionData: TransactionData[] = [
  {
    id: '1',
    timestamp: new Date('2024-01-15T10:30:00'),
    amount: 1500,
    type: 'reward',
    status: 'completed',
    from: 'bounty-contract',
    to: 'user-wallet'
  },
  {
    id: '2',
    timestamp: new Date('2024-01-14T15:45:00'),
    amount: 800,
    type: 'deposit',
    status: 'completed',
    from: 'user-wallet',
    to: 'platform'
  },
  {
    id: '3',
    timestamp: new Date('2024-01-13T09:20:00'),
    amount: 500,
    type: 'withdrawal',
    status: 'completed',
    from: 'platform',
    to: 'user-wallet'
  },
  {
    id: '4',
    timestamp: new Date('2024-01-12T14:10:00'),
    amount: 2000,
    type: 'reward',
    status: 'completed',
    from: 'bounty-contract',
    to: 'user-wallet'
  },
  {
    id: '5',
    timestamp: new Date('2024-01-11T11:00:00'),
    amount: 300,
    type: 'penalty',
    status: 'completed',
    from: 'user-wallet',
    to: 'platform'
  }
];

const mockPerformanceData: PerformanceMetrics[] = [
  {
    date: new Date('2024-01-01'),
    reputation: 1200,
    completedBounties: 15,
    totalEarned: 8500,
    successRate: 85.5,
    avgCompletionTime: 24.5
  },
  {
    date: new Date('2024-01-08'),
    reputation: 1350,
    completedBounties: 18,
    totalEarned: 10200,
    successRate: 87.2,
    avgCompletionTime: 22.1
  },
  {
    date: new Date('2024-01-15'),
    reputation: 1580,
    completedBounties: 22,
    totalEarned: 13100,
    successRate: 89.1,
    avgCompletionTime: 20.3
  },
  {
    date: new Date('2024-01-22'),
    reputation: 1720,
    completedBounties: 26,
    totalEarned: 15800,
    successRate: 90.5,
    avgCompletionTime: 18.7
  },
  {
    date: new Date('2024-01-29'),
    reputation: 1950,
    completedBounties: 31,
    totalEarned: 19200,
    successRate: 92.3,
    avgCompletionTime: 16.2
  }
];

export const AnalyticsDashboard: React.FC<AnalyticsDashboardProps> = ({ className = '' }) => {
  const [timeFilter, setTimeFilter] = useState<ChartFilter['timeRange']>('30d');
  const [refreshing, setRefreshing] = useState(false);

  const handleRefresh = async () => {
    setRefreshing(true);
    // Simulate API call
    await new Promise(resolve => setTimeout(resolve, 1000));
    setRefreshing(false);
  };

  const handleExport = () => {
    // Export functionality
    console.log('Exporting analytics data...');
  };

  return (
    <div className={`space-y-6 ${className}`}>
      {/* Header */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center space-y-4 sm:space-y-0">
        <div>
          <h2 className="text-2xl font-bold text-gray-900">Analytics Dashboard</h2>
          <p className="text-gray-600 mt-1">Monitor your portfolio, transactions, and performance</p>
        </div>
        
        <div className="flex items-center space-x-3">
          {/* Time Filter */}
          <div className="flex items-center space-x-2 bg-white border border-gray-200 rounded-lg px-3 py-2">
            <Calendar className="w-4 h-4 text-gray-500" />
            <select
              value={timeFilter}
              onChange={(e) => setTimeFilter(e.target.value as ChartFilter['timeRange'])}
              className="text-sm text-gray-700 bg-transparent border-none focus:outline-none focus:ring-0"
            >
              <option value="24h">Last 24 hours</option>
              <option value="7d">Last 7 days</option>
              <option value="30d">Last 30 days</option>
              <option value="90d">Last 90 days</option>
              <option value="all">All time</option>
            </select>
          </div>
          
          {/* Action Buttons */}
          <button
            onClick={handleRefresh}
            disabled={refreshing}
            className="flex items-center space-x-2 bg-white border border-gray-200 rounded-lg px-3 py-2 hover:bg-gray-50 transition-colors disabled:opacity-50"
          >
            <RefreshCw className={`w-4 h-4 text-gray-500 ${refreshing ? 'animate-spin' : ''}`} />
            <span className="text-sm text-gray-700">Refresh</span>
          </button>
          
          <button
            onClick={handleExport}
            className="flex items-center space-x-2 bg-blue-600 text-white rounded-lg px-3 py-2 hover:bg-blue-700 transition-colors"
          >
            <Download className="w-4 h-4" />
            <span className="text-sm">Export</span>
          </button>
        </div>
      </div>

      {/* Charts Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Portfolio Chart */}
        <PortfolioChart
          data={mockPortfolioData}
          config={{
            type: 'pie',
            title: 'Portfolio Distribution',
            showLegend: true,
            height: 350
          }}
        />

        {/* Transaction Chart */}
        <TransactionChart
          data={mockTransactionData}
          config={{
            type: 'area',
            title: 'Transaction Volume',
            showLegend: true,
            height: 350
          }}
          timeRange={timeFilter}
        />

        {/* Performance Chart - Line */}
        <PerformanceChart
          data={mockPerformanceData}
          config={{
            type: 'line',
            title: 'Performance Trends',
            showLegend: true,
            height: 350
          }}
          metrics={['reputation', 'completedBounties', 'totalEarned']}
        />

        {/* Performance Chart - Radar */}
        <PerformanceChart
          data={mockPerformanceData}
          config={{
            type: 'radar',
            title: 'Overall Performance',
            showLegend: false,
            height: 350
          }}
          metrics={['reputation', 'completedBounties', 'totalEarned', 'successRate', 'avgCompletionTime']}
        />
      </div>

      {/* Additional Charts Row */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Portfolio Bar Chart */}
        <PortfolioChart
          data={mockPortfolioData}
          config={{
            type: 'bar',
            title: 'Asset Values',
            showLegend: false,
            height: 300
          }}
        />

        {/* Transaction Pie Chart */}
        <TransactionChart
          data={mockTransactionData}
          config={{
            type: 'pie',
            title: 'Transaction Types',
            showLegend: true,
            height: 300
          }}
          timeRange={timeFilter}
        />

        {/* Performance Bar Chart */}
        <PerformanceChart
          data={mockPerformanceData}
          config={{
            type: 'bar',
            title: 'Monthly Bounties',
            showLegend: true,
            height: 300
          }}
          metrics={['completedBounties']}
        />
      </div>

      {/* Summary Stats */}
      <div className="bg-white rounded-lg border border-gray-200 p-6">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">Quick Stats</h3>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="text-center">
            <div className="flex items-center justify-center w-12 h-12 bg-blue-100 rounded-lg mx-auto mb-2">
              <TrendingUp className="w-6 h-6 text-blue-600" />
            </div>
            <p className="text-2xl font-bold text-gray-900">$42,000</p>
            <p className="text-sm text-gray-600">Total Portfolio</p>
          </div>
          
          <div className="text-center">
            <div className="flex items-center justify-center w-12 h-12 bg-green-100 rounded-lg mx-auto mb-2">
              <Activity className="w-6 h-6 text-green-600" />
            </div>
            <p className="text-2xl font-bold text-gray-900">1,950</p>
            <p className="text-sm text-gray-600">Reputation</p>
          </div>
          
          <div className="text-center">
            <div className="flex items-center justify-center w-12 h-12 bg-purple-100 rounded-lg mx-auto mb-2">
              <BarChart3 className="w-6 h-6 text-purple-600" />
            </div>
            <p className="text-2xl font-bold text-gray-900">31</p>
            <p className="text-sm text-gray-600">Completed</p>
          </div>
          
          <div className="text-center">
            <div className="flex items-center justify-center w-12 h-12 bg-amber-100 rounded-lg mx-auto mb-2">
              <PieChart className="w-6 h-6 text-amber-600" />
            </div>
            <p className="text-2xl font-bold text-gray-900">92.3%</p>
            <p className="text-sm text-gray-600">Success Rate</p>
          </div>
        </div>
      </div>

      <div className="space-y-2">
        <h3 className="text-lg font-semibold text-gray-900">Advanced Search and Filtering</h3>
        <p className="text-sm text-gray-600">
          Explore scan history with autocomplete, faceted filters, and saved search presets.
        </p>
        <TransactionHistory />
      </div>
    </div>
  );
};

export default AnalyticsDashboard;
