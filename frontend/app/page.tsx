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

const TimeTravelDebugger = dynamic(() => import('../components/TimeTravelDebugger'), {
  loading: () => <div className="skeleton h-96 w-full rounded-lg" />,
  ssr: false
});

const BatchOperations = dynamic(() => import('../components/BatchOperations'), {
  loading: () => <div className="skeleton h-96 w-full rounded-lg" />,
  ssr: false
});

const SettingsPanel = dynamic(() => import('../components/SettingsPanel'), {
  loading: () => <div className="skeleton h-96 w-full rounded-lg" />,
  ssr: false
});

// Help components
import HelpPanel from '../components/help/HelpPanel';
import GuidedTour from '../components/help/GuidedTour';
import { useHelpStore } from '../lib/store/helpStore';

const BalanceDisplay = dynamic(() => import('../components/BalanceDisplay'), {
  loading: () => <div className="skeleton h-96 w-full rounded-lg" />,
  ssr: false
});

export default function HomePage() {
  const [activeTab, setActiveTab] = useState('scanner');
  const [isClient, setIsClient] = useState(false);

  const { 
    setActiveTour, 
    helpPanelTopic, 
    setHelpPanelTopic 
  } = useHelpStore();

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
      case 'time-travel':
        return (
          <Suspense fallback={<div className="skeleton h-96 w-full rounded-lg" />}>
            <TimeTravelDebugger />
          </Suspense>
        );
      case 'batch':
        return (
          <Suspense fallback={<div className="skeleton h-96 w-full rounded-lg" />}>
            <BatchOperations />
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
              {['scanner', 'report', 'time-travel', 'batch', 'analytics', 'balance', 'settings'].map((tab) => (
                <button
                  key={tab}
                  onClick={() => setActiveTab(tab)}
                  className={`btn px-6 py-2 rounded-lg text-sm font-bold transition-all duration-200 border-none ${
                    activeTab === tab
                      ? 'bg-white text-blue-600 shadow-sm'
                      : 'text-gray-500 hover:text-gray-700 bg-transparent'
                  }`}
                >
                  {tab.split('-').map(word => word.charAt(0).toUpperCase() + word.slice(1)).join(' ')}
                </button>
              ))}
            </nav>
          </div>
        </div>
      </header>

      <main className="container mx-auto py-10 pb-24">
        <div className="max-w-5xl mx-auto space-y-10">
          <div className="flex justify-between items-end">
            <div>
              <h2 className="text-4xl font-bold capitalize">{activeTab.replace('-', ' ')}</h2>
              <p className="text-gray-500 mt-2 text-lg">Manage and monitor your smart contract security with precision.</p>
            </div>
            <div className="flex space-x-3">
              <button 
                onClick={() => setHelpPanelTopic(activeTab as any)}
                className="btn btn-secondary flex items-center gap-2"
              >
                <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
                </svg>
                Documentation
              </button>
              {['scanner', 'report', 'time-travel'].includes(activeTab) && (
                <button 
                  onClick={() => setActiveTour(activeTab === 'scanner' ? 'scan' : activeTab === 'report' ? 'vulnerability' : 'time-travel')}
                  className="btn btn-secondary flex items-center gap-2 border-blue-200 text-blue-600 bg-blue-50 hover:bg-blue-100"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 5l7 7-7 7M5 5l7 7-7 7" />
                  </svg>
                  Take a Tour
                </button>
              )}
              <button className="btn btn-primary">New Scan</button>
            </div>
          </div>
          <div className="animate-fade-in">
            {renderActiveComponent()}
          </div>
        </div>
      </main>

      {/* Floating Help Button */}
      <div className="fixed bottom-8 right-8 z-[90]">
        <button
          onClick={() => setHelpPanelTopic(activeTab as any)}
          className="group relative flex items-center justify-center w-14 h-14 bg-[#0f172a] text-white rounded-full shadow-2xl hover:scale-110 transition-all duration-300 focus:outline-none focus:ring-4 focus:ring-blue-500/20"
          aria-label="Get help"
        >
          <svg xmlns="http://www.w3.org/2000/svg" className="h-7 w-7" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <span className="absolute right-full mr-4 px-3 py-1.5 bg-white text-[#0f172a] text-sm font-bold rounded-xl shadow-xl opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap pointer-events-none border border-gray-100">
            Need help with {activeTab.replace('-', ' ')}?
          </span>
        </button>
      </div>

      {/* Help Overlay Components */}
      <HelpPanel topic={helpPanelTopic} onClose={() => setHelpPanelTopic(null)} />
      {activeTab === 'scanner' && <GuidedTour tourId="scan" />}
      {activeTab === 'report' && <GuidedTour tourId="vulnerability" />}
      {activeTab === 'time-travel' && <GuidedTour tourId="time-travel" />}
    </div>
  );
}
