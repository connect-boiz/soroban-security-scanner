'use client';

import React, { useState, useMemo, useCallback, useEffect } from 'react';
import { PortfolioChart, TransactionChart, PerformanceChart } from '@/components/charts';
import { PortfolioData, TransactionData, PerformanceMetrics, ChartFilter } from '@/types/charts';
import { LoadingOverlay, SkeletonCard, ProgressBar, LoadingSpinner } from './ui';
import { 
  BarChart3, 
  TrendingUp, 
  PieChart, 
  Activity,
  Filter,
  Calendar,
  Download,
  RefreshCw,
  Shield,
  Zap,
  Clock,
  AlertTriangle
} from 'lucide-react';

interface AnalyticsDashboardProps {
  className?: string;
}

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
  }
];

const AnalyticsDashboard: React.FC<AnalyticsDashboardProps> = ({ className = '' }) => {
  const [dashboardView, setDashboardView] = useState<'security' | 'marketplace'>('security');
  const [selectedMetric, setSelectedMetric] = useState<'overview' | 'trends' | 'severity'>('overview');
  const [timeFilter, setTimeFilter] = useState<ChartFilter['timeRange']>('30d');
  const [isLoading, setIsLoading] = useState(false);
  const [refreshing, setRefreshing] = useState(false);

  const metrics = useMemo(() => ({
    vulnerabilityRate: ((mockAnalyticsData.vulnerabilitiesFound / mockAnalyticsData.totalScans) * 100).toFixed(1),
    contractsPerScan: (mockAnalyticsData.contractsAnalyzed / mockAnalyticsData.totalScans).toFixed(2),
    totalIssues: Object.values(mockAnalyticsData.severityBreakdown).reduce((a, b) => a + b, 0)
  }), []);

  useEffect(() => {
    const loadAnalytics = async () => {
      setIsLoading(true);
      await new Promise(resolve => setTimeout(resolve, 1000));
      setIsLoading(false);
    };
    loadAnalytics();
  }, []);

  const handleRefresh = async () => {
    setRefreshing(true);
    await new Promise(resolve => setTimeout(resolve, 1000));
    setRefreshing(false);
  };

  const renderSecurityOverview = () => (
    <div className="space-y-6 animate-fade-in">
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <div className="card border-l-4 border-l-blue-600 bg-white p-6 rounded-lg shadow-sm">
          <p className="text-xs font-bold text-gray-500 uppercase tracking-wider">Total Scans</p>
          <div className="flex items-end justify-between mt-2">
            <h3 className="text-3xl font-bold">{mockAnalyticsData.totalScans.toLocaleString()}</h3>
            <span className="text-green-500 text-sm font-bold">↑ 12%</span>
          </div>
        </div>
        <div className="card border-l-4 border-l-red-600 bg-white p-6 rounded-lg shadow-sm">
          <p className="text-xs font-bold text-gray-500 uppercase tracking-wider">Vulnerabilities</p>
          <div className="flex items-end justify-between mt-2">
            <h3 className="text-3xl font-bold">{mockAnalyticsData.vulnerabilitiesFound}</h3>
            <span className="text-red-500 text-sm font-bold">↑ 5%</span>
          </div>
        </div>
        <div className="card border-l-4 border-l-green-600 bg-white p-6 rounded-lg shadow-sm">
          <p className="text-xs font-bold text-gray-500 uppercase tracking-wider">Detection Rate</p>
          <div className="flex items-end justify-between mt-2">
            <h3 className="text-3xl font-bold">{metrics.vulnerabilityRate}%</h3>
            <span className="text-gray-400 text-sm font-bold">Stable</span>
          </div>
        </div>
        <div className="card border-l-4 border-l-purple-600 bg-white p-6 rounded-lg shadow-sm">
          <p className="text-xs font-bold text-gray-500 uppercase tracking-wider">Avg Scan Time</p>
          <div className="flex items-end justify-between mt-2">
            <h3 className="text-3xl font-bold">{mockAnalyticsData.averageScanTime}s</h3>
            <span className="text-green-500 text-sm font-bold">↓ 0.2s</span>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="card lg:col-span-2 bg-white p-6 rounded-lg shadow-sm">
          <h3 className="text-lg font-bold mb-6">Scan Activity Trends</h3>
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
          </div>
        </div>

        <div className="card bg-white p-6 rounded-lg shadow-sm">
          <h3 className="text-lg font-bold mb-6">Severity Distribution</h3>
          <div className="space-y-6">
            {Object.entries(mockAnalyticsData.severityBreakdown).map(([severity, count]) => {
              const total = metrics.totalIssues;
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
        </div>
      </div>
    </div>
  );

  const renderMarketplaceAnalytics = () => (
    <div className="space-y-6 animate-fade-in">
      {/* Summary Stats */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">Marketplace Summary</h3>
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
            <p className="text-sm text-gray-600">Completed Bounties</p>
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

      {/* Charts Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <PortfolioChart
          data={mockPortfolioData}
          config={{ type: 'pie', title: 'Portfolio Distribution', showLegend: true, height: 350 }}
        />
        <TransactionChart
          data={mockTransactionData}
          config={{ type: 'area', title: 'Transaction Volume', showLegend: true, height: 350 }}
          timeRange={timeFilter}
        />
        <PerformanceChart
          data={mockPerformanceData}
          config={{ type: 'line', title: 'Performance Trends', showLegend: true, height: 350 }}
          metrics={['reputation', 'completedBounties', 'totalEarned']}
        />
        <PerformanceChart
          data={mockPerformanceData}
          config={{ type: 'radar', title: 'Overall Performance', showLegend: false, height: 350 }}
          metrics={['reputation', 'completedBounties', 'totalEarned', 'successRate', 'avgCompletionTime']}
        />
      </div>
    </div>
  );

  return (
    <LoadingOverlay isLoading={isLoading} text="Loading analytics data...">
      <div className={`space-y-6 ${className}`}>
        {/* Header */}
        <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center space-y-4 sm:space-y-0">
          <div>
            <h2 className="text-2xl font-bold text-gray-900">Analytics Dashboard</h2>
            <div className="flex mt-2 p-1 bg-gray-100 rounded-lg w-fit">
              <button
                onClick={() => setDashboardView('security')}
                className={`px-4 py-1.5 text-sm font-bold rounded-md transition-all ${
                  dashboardView === 'security' ? 'bg-white shadow-sm text-blue-600' : 'text-gray-500'
                }`}
              >
                Security
              </button>
              <button
                onClick={() => setDashboardView('marketplace')}
                className={`px-4 py-1.5 text-sm font-bold rounded-md transition-all ${
                  dashboardView === 'marketplace' ? 'bg-white shadow-sm text-blue-600' : 'text-gray-500'
                }`}
              >
                Marketplace
              </button>
            </div>
          </div>
          
          <div className="flex items-center space-x-3">
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
            
            <button
              onClick={handleRefresh}
              disabled={refreshing}
              className="flex items-center space-x-2 bg-white border border-gray-200 rounded-lg px-3 py-2 hover:bg-gray-50 transition-colors disabled:opacity-50"
            >
              <RefreshCw className={`w-4 h-4 text-gray-500 ${refreshing ? 'animate-spin' : ''}`} />
              <span className="text-sm text-gray-700">Refresh</span>
            </button>
            
            <button className="flex items-center space-x-2 bg-blue-600 text-white rounded-lg px-3 py-2 hover:bg-blue-700 transition-colors">
              <Download className="w-4 h-4" />
              <span className="text-sm">Export</span>
            </button>
          </div>
        </div>

        {/* Content */}
        <div>
          {dashboardView === 'security' ? renderSecurityOverview() : renderMarketplaceAnalytics()}
        </div>
      </div>
    </LoadingOverlay>
  );
};

export default AnalyticsDashboard;
