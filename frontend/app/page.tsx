'use client';

import { Suspense, useState, useEffect } from 'react';
import dynamic from 'next/dynamic';
import { SectionErrorBoundary } from '@/components/ui/ErrorBoundary';

type View = 'bounties' | 'leaderboard' | 'wallet' | 'analytics' | 'settings';

export default function App() {
  const [currentView, setCurrentView] = useState<View>('bounties');
  const [selectedBounty, setSelectedBounty] = useState<Bounty | null>(null);
  const [showReportForm, setShowReportForm] = useState(false);
  const [showDisputeForm, setShowDisputeForm] = useState(false);
  const [selectedSubmission, setSelectedSubmission] = useState<BountySubmissionType | null>(null);
  const [disputes, setDisputes] = useState<DisputeData[]>([]);
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);

  const handleBountySelect = (bounty: Bounty) => {
    setSelectedBounty(bounty);
    setShowReportForm(true);
  };

  const handleReportSubmit = (submission: BountySubmissionType) => {
    console.log('Report submitted:', submission);
    setShowReportForm(false);
    setSelectedBounty(null);
    // In a real app, this would submit to the backend
  };

  const handleDisputeSubmit = (disputeData: DisputeData) => {
    setDisputes([...disputes, disputeData]);
    setShowDisputeForm(false);
    setSelectedSubmission(null);
    // In a real app, this would submit to the backend
  };

  const navigation = [
    { name: 'Bounty Board', view: 'bounties' as View, icon: Search },
    { name: 'Leaderboard', view: 'leaderboard' as View, icon: Trophy },
    { name: 'Analytics', view: 'analytics' as View, icon: BarChart3 },
    { name: 'Wallet', view: 'wallet' as View, icon: Wallet },
    { name: 'Settings', view: 'settings' as View, icon: Settings },
  ];

  const renderMainContent = () => {
    if (showReportForm && selectedBounty) {
      return (
        <ReportSubmission
          bounty={selectedBounty}
          onSubmit={handleReportSubmit}
          onCancel={() => {
            setShowReportForm(false);
            setSelectedBounty(null);
          }}
        />
      );
    }

    if (showDisputeForm && selectedSubmission) {
      return (
        <DisputeForm
          submission={selectedSubmission}
          onSubmitDispute={handleDisputeSubmit}
          onCancel={() => {
            setShowDisputeForm(false);
            setSelectedSubmission(null);
          }}
        />
      );
    }

    switch (currentView) {
      case 'bounties':
        return <BountyBoard onBountySelect={handleBountySelect} />;
      case 'leaderboard':
        return <Leaderboard />;
      case 'analytics':
        return <AnalyticsDashboard />;
      case 'wallet':
        return (
          <SectionErrorBoundary context={{ tab: 'scanner' }}>
            <Suspense fallback={<div className="skeleton h-96 w-full rounded-lg" />}>
              <ScannerInterface />
            </Suspense>
          </SectionErrorBoundary>
        );
      case 'report':
        return (
          <SectionErrorBoundary context={{ tab: 'report' }}>
            <Suspense fallback={<div className="skeleton h-64 w-full rounded-lg" />}>
              <VulnerabilityReport />
            </Suspense>
          </SectionErrorBoundary>
        );
      case 'analytics':
        return (
          <SectionErrorBoundary context={{ tab: 'analytics' }}>
            <Suspense fallback={<div className="skeleton h-80 w-full rounded-lg" />}>
              <AnalyticsDashboard />
            </Suspense>
          </SectionErrorBoundary>
        );
      case 'multisig':
        return (
          <Suspense fallback={<div className="skeleton h-96 w-full rounded-lg" />}>
            <MultiSigWizard />
      case 'balance':
        return (
          <SectionErrorBoundary context={{ tab: 'balance' }}>
            <Suspense fallback={<div className="skeleton h-96 w-full rounded-lg" />}>
              <BalanceDisplay />
            </Suspense>
          </SectionErrorBoundary>
        );
      case 'settings':
        return (
          <SectionErrorBoundary context={{ tab: 'settings' }}>
            <Suspense fallback={<div className="skeleton h-96 w-full rounded-lg" />}>
              <SettingsPanel />
            </Suspense>
          </SectionErrorBoundary>
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
              {['scanner', 'report', 'analytics', 'multisig', 'settings'].map((tab) => (
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
                  {tab === 'multisig' ? 'Multi-Sig' : tab.charAt(0).toUpperCase() + tab.slice(1)}
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
