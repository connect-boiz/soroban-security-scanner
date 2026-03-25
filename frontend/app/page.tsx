
    'use client';

import { useState, useEffect } from 'react';
import { Shield, Key, Activity, AlertTriangle } from 'lucide-react';
import { toast } from 'sonner';

interface DashboardStats {
  totalScans: number;
  activeApiKeys: number;
  recentActivity: number;
  criticalIssues: number;
}

export default function DashboardPage() {
  const [stats, setStats] = useState<DashboardStats>({
    totalScans: 0,
    activeApiKeys: 0,
    recentActivity: 0,
    criticalIssues: 0,
  });
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchDashboardStats();
  }, []);

  const fetchDashboardStats = async () => {
    try {
      const token = localStorage.getItem('accessToken');
      if (!token) {
        window.location.href = '/login';
        return;
      }

      // Mock data for now - in real implementation, this would call the backend
      const mockStats: DashboardStats = {
        totalScans: 156,
        activeApiKeys: 8,
        recentActivity: 23,
        criticalIssues: 3,
      };

      setStats(mockStats);
    } catch (error) {
      toast.error('Failed to fetch dashboard stats');
    } finally {
      setLoading(false);
    }
  };

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto"></div>
          <p className="mt-4 text-gray-600">Loading dashboard...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <div className="max-w-7xl mx-auto py-6 sm:px-6 lg:px-8">
        <div className="px-4 py-6 sm:px-0">
          <div className="mb-8">
            <h1 className="text-3xl font-bold text-gray-900">Dashboard</h1>
            <p className="mt-2 text-gray-600">
              Overview of your Soroban Security Scanner activity
            </p>
          </div>

          {/* Stats Grid */}
          <div className="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-4">
            <div className="bg-white overflow-hidden shadow rounded-lg">
              <div className="p-5">
                <div className="flex items-center">
                  <div className="flex-shrink-0">
                    <Shield className="h-6 w-6 text-gray-400" />
                  </div>
                  <div className="ml-5 w-0 flex-1">
                    <dl>
                      <dt className="text-sm font-medium text-gray-500 truncate">
                        Total Scans
                      </dt>
                      <dd className="text-lg font-medium text-gray-900">
                        {stats.totalScans}
                      </dd>
                    </dl>
                  </div>
                </div>
              </div>
              <div className="bg-gray-50 px-5 py-3">
                <div className="text-sm">
                  <a href="/scanner" className="font-medium text-blue-700 hover:text-blue-600">
                    View all scans
                  </a>
                </div>
              </div>
            </div>

            <div className="bg-white overflow-hidden shadow rounded-lg">
              <div className="p-5">
                <div className="flex items-center">
                  <div className="flex-shrink-0">
                    <Key className="h-6 w-6 text-gray-400" />
                  </div>
                  <div className="ml-5 w-0 flex-1">
                    <dl>
                      <dt className="text-sm font-medium text-gray-500 truncate">
                        Active API Keys
                      </dt>
                      <dd className="text-lg font-medium text-gray-900">
                        {stats.activeApiKeys}
                      </dd>
                    </dl>
                  </div>
                </div>
              </div>
              <div className="bg-gray-50 px-5 py-3">
                <div className="text-sm">
                  <a href="/api-keys" className="font-medium text-blue-700 hover:text-blue-600">
                    Manage API keys
                  </a>
                </div>
              </div>
            </div>

            <div className="bg-white overflow-hidden shadow rounded-lg">
              <div className="p-5">
                <div className="flex items-center">
                  <div className="flex-shrink-0">
                    <Activity className="h-6 w-6 text-gray-400" />
                  </div>
                  <div className="ml-5 w-0 flex-1">
                    <dl>
                      <dt className="text-sm font-medium text-gray-500 truncate">
                        Recent Activity
                      </dt>
                      <dd className="text-lg font-medium text-gray-900">
                        {stats.recentActivity}
                      </dd>
                    </dl>
                  </div>
                </div>
              </div>
              <div className="bg-gray-50 px-5 py-3">
                <div className="text-sm">
                  <span className="text-gray-500">Last 24 hours</span>
                </div>
              </div>
            </div>

            <div className="bg-white overflow-hidden shadow rounded-lg">
              <div className="p-5">
                <div className="flex items-center">
                  <div className="flex-shrink-0">
                    <AlertTriangle className="h-6 w-6 text-red-400" />
                  </div>
                  <div className="ml-5 w-0 flex-1">
                    <dl>
                      <dt className="text-sm font-medium text-gray-500 truncate">
                        Critical Issues
                      </dt>
                      <dd className="text-lg font-medium text-gray-900">
                        {stats.criticalIssues}
                      </dd>
                    </dl>
                  </div>
                </div>
              </div>
              <div className="bg-gray-50 px-5 py-3">
                <div className="text-sm">
                  <a href="/reports" className="font-medium text-red-700 hover:text-red-600">
                    View reports
                  </a>
                </div>
              </div>
            </div>
          </div>

          {/* Quick Actions */}
          <div className="mt-8">
            <h2 className="text-lg font-medium text-gray-900 mb-4">Quick Actions</h2>
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
              <div className="bg-white p-6 shadow rounded-lg">
                <h3 className="text-base font-medium text-gray-900 mb-2">Run New Scan</h3>
                <p className="text-sm text-gray-500 mb-4">
                  Start a new security scan of your Soroban smart contract
                </p>
                <a
                  href="/scanner"
                  className="inline-flex items-center px-3 py-2 border border-transparent text-sm leading-4 font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
                >
                  Start Scan
                </a>
              </div>

              <div className="bg-white p-6 shadow rounded-lg">
                <h3 className="text-base font-medium text-gray-900 mb-2">Generate API Key</h3>
                <p className="text-sm text-gray-500 mb-4">
                  Create a new API key for CI/CD integration
                </p>
                <a
                  href="/api-keys"
                  className="inline-flex items-center px-3 py-2 border border-transparent text-sm leading-4 font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
                >
                  Generate Key
                </a>
              </div>

              <div className="bg-white p-6 shadow rounded-lg">
                <h3 className="text-base font-medium text-gray-900 mb-2">View Reports</h3>
                <p className="text-sm text-gray-500 mb-4">
                  Check your latest security scan reports
                </p>
                <a
                  href="/reports"
                  className="inline-flex items-center px-3 py-2 border border-gray-300 text-sm leading-4 font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
                >
                  View Reports
                </a>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
