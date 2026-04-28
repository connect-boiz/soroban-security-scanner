'use client';

import { lazy, Suspense, useState, useEffect } from 'react';
import dynamic from 'next/dynamic';

// Lazy load components for code splitting
const ScannerInterface = dynamic(() => import('../components/ScannerInterface'), {
  loading: () => <div className="skeleton h-96 w-full rounded-lg" />,
  ssr: false
});

const VulnerabilityReport = dynamic(() => import('../components/VulnerabilityReport'), {
  loading: () => <div className="skeleton h-64 w-full rounded-lg" />,
  ssr: false
});

const AnalyticsDashboard = dynamic(() => import('../components/AnalyticsDashboard'), {
  loading: () => <div className="skeleton h-80 w-full rounded-lg" />,
  ssr: false
});

const SettingsPanel = dynamic(() => import('../components/SettingsPanel'), {
  loading: () => <div className="skeleton h-96 w-full rounded-lg" />,
  ssr: false
});

const BalanceDisplay = dynamic(() => import('../components/BalanceDisplay'), {
  loading: () => <div className="skeleton h-96 w-full rounded-lg" />,
  ssr: false
});

export default function HomePage() {
  const [activeTab, setActiveTab] = useState('scanner');
  const [isClient, setIsClient] = useState(false);

  // Ensure client-side rendering for dynamic components
  useEffect(() => {
    setIsClient(true);
  }, []);

  const renderActiveComponent = () => {
    if (!isClient) return <div className="skeleton h-96 w-full rounded-lg" />;

    switch (activeTab) {
      case 'scanner':
        return (
          <Suspense fallback={<div className="skeleton h-96 w-full rounded-lg" />}>
            <ScannerInterface />
          </Suspense>
        );
      case 'report':
        return (
          <Suspense fallback={<div className="skeleton h-64 w-full rounded-lg" />}>
            <VulnerabilityReport />
          </Suspense>
        );
      case 'analytics':
        return (
          <Suspense fallback={<div className="skeleton h-80 w-full rounded-lg" />}>
            <AnalyticsDashboard />
          </Suspense>
        );
      case 'balance':
        return (
          <Suspense fallback={<div className="skeleton h-96 w-full rounded-lg" />}>
            <BalanceDisplay />
          </Suspense>
        );
      case 'settings':
        return (
          <Suspense fallback={<div className="skeleton h-96 w-full rounded-lg" />}>
            <SettingsPanel />
          </Suspense>
        );
      default:
        return null;
    }
  };

  return (
    <div className="min-h-screen bg-gray-50">
      <header className="bg-white shadow-sm border-b">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center h-16">
            <h1 className="text-2xl font-bold text-gray-900">
              Soroban Security Scanner
            </h1>
            <nav className="flex space-x-4">
              {['scanner', 'report', 'analytics', 'balance', 'settings'].map((tab) => (
                <button
                  key={tab}
                  onClick={() => setActiveTab(tab)}
                  className={`btn px-6 py-2 rounded-lg text-sm font-bold transition-all duration-200 border-none ${
                    activeTab === tab
                      ? 'bg-white text-blue-600 shadow-sm'
                      : 'text-gray-500 hover:text-gray-700 bg-transparent'
                  }`}
                >
                  {tab.charAt(0).toUpperCase() + tab.slice(1)}
                </button>
              ))}
            </nav>
          </div>
        </div>
      </header>

      <main className="container mx-auto py-10">
        <div className="max-w-5xl mx-auto space-y-10">
          <div className="flex justify-between items-end">
            <div>
              <h2 className="text-4xl font-bold capitalize">{activeTab}</h2>
              <p className="text-gray-500 mt-2 text-lg">Manage and monitor your smart contract security with precision.</p>
            </div>
            <div className="flex space-x-3">
              <button className="btn btn-secondary">Documentation</button>
              <button className="btn btn-primary">New Scan</button>
            </div>
          </div>
          <div className="animate-fade-in">
            {renderActiveComponent()}
          </div>
        </div>
      </main>
    </div>
  );
}
