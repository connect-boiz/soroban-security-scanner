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

const TransactionHistory = dynamic(() => import('../components/TransactionHistory'), {
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
      case 'history':
        return (
          <Suspense fallback={<div className="skeleton h-96 w-full rounded-lg" />}>
            <TransactionHistory />
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
    <div className="min-h-screen bg-gray-50 pb-20">
      <header className="bg-white shadow-sm border-b sticky top-0 z-50">
        <div className="container mx-auto">
          <div className="flex justify-between items-center h-20">
            <div className="flex items-center space-x-3">
              <div className="w-10 h-10 bg-blue-600 rounded-xl flex-center shadow-lg">
                <span className="text-white font-bold text-xl">S</span>
              </div>
              <h1 className="text-xl font-bold text-gray-900 tracking-tight">
                Soroban<span className="text-blue-600">Scan</span>
              </h1>
            </div>
            <nav className="flex bg-gray-100 p-1 rounded-xl shadow-inner">
              {['scanner', 'report', 'analytics', 'history', 'settings'].map((tab) => (
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
