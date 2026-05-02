'use client';

import { Suspense, useState, useEffect } from 'react';
import dynamic from 'next/dynamic';
import { SectionErrorBoundary } from '@/components/ui/ErrorBoundary';
import { 
  Search, 
  Trophy, 
  BarChart3, 
  Wallet, 
  Settings, 
  Activity, 
  Shield, 
  Zap, 
  Clock, 
  Plus,
  HelpCircle,
  FileText,
  ShieldCheck,
  Users
} from 'lucide-react';

// Help components
import HelpPanel from '../components/help/HelpPanel';
import GuidedTour from '../components/help/GuidedTour';
import { useHelpStore } from '../lib/store/helpStore';

// Types
type View = 'bounties' | 'leaderboard' | 'wallet' | 'analytics' | 'settings' | 'scanner' | 'report' | 'time-travel' | 'batch' | 'balance' | 'multisig';

interface Bounty {
  id: string;
  title: string;
  reward: string;
}

interface BountySubmissionType {
  id: string;
  bountyId: string;
  findings: string;
}

interface DisputeData {
  id: string;
  submissionId: string;
  reason: string;
}

// Dynamic components with skeletons
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

const BalanceDisplay = dynamic(() => import('../components/BalanceDisplay'), {
  loading: () => <div className="skeleton h-96 w-full rounded-lg" />,
  ssr: false
});

const SettingsPanel = dynamic(() => import('../components/SettingsPanel'), {
  loading: () => <div className="skeleton h-96 w-full rounded-lg" />,
  ssr: false
});

const MultiSigWizard = dynamic(() => import('../components/MultiSigWizard'), {
  loading: () => <div className="skeleton h-96 w-full rounded-lg" />,
  ssr: false
});

// Mock components for missing files to avoid build breakage
const BountyBoard = ({ onBountySelect }: { onBountySelect: (bounty: Bounty) => void }) => (
  <div className="p-8 text-center bg-white rounded-xl border border-dashed border-gray-300">
    <Search className="h-12 w-12 text-gray-400 mx-auto mb-4" />
    <h3 className="text-xl font-bold">Bounty Board</h3>
    <p className="text-gray-500">Bounty marketplace features are coming soon.</p>
  </div>
);

const Leaderboard = () => (
  <div className="p-8 text-center bg-white rounded-xl border border-dashed border-gray-300">
    <Trophy className="h-12 w-12 text-gray-400 mx-auto mb-4" />
    <h3 className="text-xl font-bold">Leaderboard</h3>
    <p className="text-gray-500">Researcher rankings will be displayed here.</p>
  </div>
);

const ReportSubmission = ({ bounty, onSubmit, onCancel }: any) => (
  <div className="p-8 bg-white rounded-xl border border-gray-200">
    <h3 className="text-xl font-bold mb-4">Submit Report for {bounty.title}</h3>
    <div className="flex gap-4">
      <button onClick={() => onSubmit({ id: '1', findings: 'test' })} className="btn btn-primary">Submit</button>
      <button onClick={onCancel} className="btn btn-secondary">Cancel</button>
    </div>
  </div>
);

const DisputeForm = ({ submission, onSubmitDispute, onCancel }: any) => (
  <div className="p-8 bg-white rounded-xl border border-gray-200">
    <h3 className="text-xl font-bold mb-4">Dispute Submission {submission.id}</h3>
    <div className="flex gap-4">
      <button onClick={() => onSubmitDispute({ id: '1', reason: 'test' })} className="btn btn-primary">Submit Dispute</button>
      <button onClick={onCancel} className="btn btn-secondary">Cancel</button>
    </div>
  </div>
);

export default function App() {
  const [activeTab, setActiveTab] = useState<View>('scanner');
  const [selectedBounty, setSelectedBounty] = useState<Bounty | null>(null);
  const [showReportForm, setShowReportForm] = useState(false);
  const [showDisputeForm, setShowDisputeForm] = useState(false);
  const [selectedSubmission, setSelectedSubmission] = useState<BountySubmissionType | null>(null);
  const [disputes, setDisputes] = useState<DisputeData[]>([]);
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);
  const [isClient, setIsClient] = useState(false);

  const { 
    setActiveTour, 
    helpPanelTopic, 
    setHelpPanelTopic 
  } = useHelpStore();

  useEffect(() => {
    setIsClient(true);
  }, []);

  const handleBountySelect = (bounty: Bounty) => {
    setSelectedBounty(bounty);
    setShowReportForm(true);
  };

  const handleReportSubmit = (submission: BountySubmissionType) => {
    console.log('Report submitted:', submission);
    setShowReportForm(false);
    setSelectedBounty(null);
  };

  const handleDisputeSubmit = (disputeData: DisputeData) => {
    setDisputes([...disputes, disputeData]);
    setShowDisputeForm(false);
    setSelectedSubmission(null);
  };

  const navigation = [
    { name: 'Bounty Board', view: 'bounties' as View, icon: Search },
    { name: 'Scanner', view: 'scanner' as View, icon: Shield },
    { name: 'Leaderboard', view: 'leaderboard' as View, icon: Trophy },
    { name: 'Analytics', view: 'analytics' as View, icon: BarChart3 },
    { name: 'Wallet', view: 'balance' as View, icon: Wallet },
    { name: 'Settings', view: 'settings' as View, icon: Settings },
  ];

  const renderActiveComponent = () => {
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

    switch (activeTab) {
      case 'bounties':
        return <BountyBoard />;
      case 'leaderboard':
        return <Leaderboard />;
      case 'scanner':
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
      case 'multisig':
        return (
          <Suspense fallback={<div className="skeleton h-96 w-full rounded-lg" />}>
            <MultiSigWizard />
          </Suspense>
        );
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

  if (!isClient) return null;

  return (
    <div className="min-h-screen bg-gray-50">
      <header className="bg-white shadow-sm border-b">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center h-16">
            <h1 className="text-2xl font-bold text-gray-900 flex items-center gap-2">
              <ShieldCheck className="h-8 w-8 text-blue-600" />
              Soroban Security Scanner
            </h1>
            <nav className="hidden md:flex space-x-4">
              {navigation.map((item) => (
                <button
                  key={item.view}
                  onClick={() => setActiveTab(item.view)}
                  className={`flex items-center gap-2 px-3 py-2 rounded-lg text-sm font-bold transition-all duration-200 ${
                    activeTab === item.view
                      ? 'bg-blue-50 text-blue-600 shadow-sm'
                      : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'
                  }`}
                >
                  <item.icon className="h-4 w-4" />
                  {item.name}
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
                <HelpCircle className="h-5 w-5" />
                Documentation
              </button>
              {['scanner', 'report', 'time-travel'].includes(activeTab) && (
                <button 
                  onClick={() => setActiveTour(activeTab === 'scanner' ? 'scan' : activeTab === 'report' ? 'vulnerability' : 'time-travel')}
                  className="btn btn-secondary flex items-center gap-2 border-blue-200 text-blue-600 bg-blue-50 hover:bg-blue-100"
                >
                  <Activity className="h-5 w-5" />
                  Take a Tour
                </button>
              )}
              <button className="btn btn-primary flex items-center gap-2">
                <Plus className="h-5 w-5" />
                New Scan
              </button>
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
          <HelpCircle className="h-7 w-7" />
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
