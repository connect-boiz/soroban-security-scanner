import React, { useEffect } from 'react';
import { useDashboardStore } from '../store/dashboardStore';
import { SummaryWidget } from '../components/dashboard/SummaryWidget';
import { VulnerabilityTrendsChart } from '../components/dashboard/VulnerabilityTrendsChart';
import { RecentScansTable } from '../components/dashboard/RecentScansTable';
import { DatePicker } from '../components/dashboard/DatePicker';

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
          <div className="flex justify-between items-center">
            <h1 className="text-3xl font-bold text-gray-900">Security Scan Dashboard</h1>
            <DatePicker />
          </div>
          <p className="mt-2 text-gray-600">
            Monitor your smart contract security metrics and vulnerability trends
          </p>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
          <SummaryWidget />
          <VulnerabilityTrendsChart />
        </div>

        <div className="grid grid-cols-1 gap-6">
          <RecentScansTable />
        </div>
      </div>
    </div>
  );
}
