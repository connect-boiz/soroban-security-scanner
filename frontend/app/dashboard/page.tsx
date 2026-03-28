import React, { useEffect } from 'react';
import { useDashboardStore } from '../store/dashboardStore';
import { SummaryWidget } from '../components/dashboard/SummaryWidget';
import { VulnerabilityTrendsChart } from '../components/dashboard/VulnerabilityTrendsChart';
import { RecentScansTable } from '../components/dashboard/RecentScansTable';
import { DatePicker } from '../components/dashboard/DatePicker';
import { ContractHealthScores } from '../components/dashboard/ContractHealthScores';

export default function Dashboard() {
  const { fetchDashboardData, isLoading } = useDashboardStore();

  useEffect(() => {
    fetchDashboardData();
  }, [fetchDashboardData]);

  if (isLoading) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-indigo-600 mx-auto"></div>
          <p className="mt-4 text-gray-600">Loading dashboard data...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="mb-8">
          <div className="flex flex-col sm:flex-row sm:justify-between sm:items-center gap-4">
            <div>
              <h1 className="text-3xl font-bold text-gray-900">Security Scan Dashboard</h1>
              <p className="mt-2 text-gray-600">
                Monitor your smart contract security metrics and vulnerability trends
              </p>
            </div>
            <DatePicker />
          </div>
        </div>

        {/* Summary Cards Row */}
        <div className="mb-8">
          <SummaryWidget />
        </div>

        {/* Charts Grid */}
        <div className="grid grid-cols-1 xl:grid-cols-2 gap-6 mb-8">
          <div className="xl:col-span-2">
            <VulnerabilityTrendsChart />
          </div>
          <div className="xl:col-span-2">
            <ContractHealthScores />
          </div>
        </div>

        {/* Recent Scans Table */}
        <div className="grid grid-cols-1 gap-6">
          <RecentScansTable />
        </div>
      </div>
    </div>
  );
}
