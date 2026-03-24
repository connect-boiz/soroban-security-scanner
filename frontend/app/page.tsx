'use client';

import { useState } from 'react';
import { BountyBoard } from '@/components/BountyBoard';
import { ReportSubmission } from '@/components/ReportSubmission';
import { Leaderboard } from '@/components/Leaderboard';
import { WalletConnect, BountyDeposit } from '@/components/WalletConnect';
import { NotificationCenter } from '@/components/Notifications';
import { DisputeForm, DisputeStatus } from '@/components/Dispute';
import { Bounty } from '@/types/bounty';
import { BountySubmission as BountySubmissionType } from '@/types/bounty';
import { DisputeData } from '@/components/Dispute';
import { 
  Shield, 
  Search, 
  Trophy, 
  Wallet, 
  Bell,
  Menu,
  X,
  Home,
  FileText,
  Settings
} from 'lucide-react';

type View = 'bounties' | 'leaderboard' | 'wallet' | 'settings';

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
      case 'wallet':
        return (
          <div className="max-w-4xl mx-auto p-6 space-y-6">
            <WalletConnect />
            <BountyDeposit 
              contractAddress="GC123..." 
              bountyId="bounty_123"
              onSuccess={(txHash) => console.log('Deposit successful:', txHash)}
            />
          </div>
        );
      case 'settings':
        return (
          <div className="max-w-2xl mx-auto p-6">
            <div className="card">
              <h2 className="text-2xl font-bold text-gray-900 mb-6">Settings</h2>
              <p className="text-gray-600">Settings functionality coming soon...</p>
            </div>
          </div>
        );
      default:
        return <BountyBoard onBountySelect={handleBountySelect} />;
    }
  };

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Header */}
      <header className="bg-white shadow-sm border-b border-gray-200">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center h-16">
            {/* Logo */}
            <div className="flex items-center">
              <Shield className="h-8 w-8 text-primary-600 mr-3" />
              <h1 className="text-xl font-bold text-gray-900">
                Soroban Security Scanner
              </h1>
            </div>

            {/* Desktop Navigation */}
            <nav className="hidden md:flex space-x-8">
              {navigation.map((item) => {
                const Icon = item.icon;
                return (
                  <button
                    key={item.view}
                    onClick={() => setCurrentView(item.view)}
                    className={`flex items-center px-3 py-2 text-sm font-medium rounded-md transition-colors ${
                      currentView === item.view
                        ? 'text-primary-600 bg-primary-50'
                        : 'text-gray-700 hover:text-gray-900 hover:bg-gray-50'
                    }`}
                  >
                    <Icon className="h-4 w-4 mr-2" />
                    {item.name}
                  </button>
                );
              })}
            </nav>

            {/* Right side items */}
            <div className="flex items-center space-x-4">
              <NotificationCenter />
              
              {/* Mobile menu button */}
              <button
                onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
                className="md:hidden p-2 text-gray-700 hover:text-gray-900"
              >
                {isMobileMenuOpen ? (
                  <X className="h-6 w-6" />
                ) : (
                  <Menu className="h-6 w-6" />
                )}
              </button>
            </div>
          </div>
        </div>

        {/* Mobile Navigation */}
        {isMobileMenuOpen && (
          <div className="md:hidden border-t border-gray-200">
            <div className="px-2 pt-2 pb-3 space-y-1">
              {navigation.map((item) => {
                const Icon = item.icon;
                return (
                  <button
                    key={item.view}
                    onClick={() => {
                      setCurrentView(item.view);
                      setIsMobileMenuOpen(false);
                    }}
                    className={`flex items-center w-full px-3 py-2 text-base font-medium rounded-md transition-colors ${
                      currentView === item.view
                        ? 'text-primary-600 bg-primary-50'
                        : 'text-gray-700 hover:text-gray-900 hover:bg-gray-50'
                    }`}
                  >
                    <Icon className="h-5 w-5 mr-3" />
                    {item.name}
                  </button>
                );
              })}
            </div>
          </div>
        )}
      </header>

      {/* Main Content */}
      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {renderMainContent()}
      </main>

      {/* Footer */}
      <footer className="bg-white border-t border-gray-200 mt-auto">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
          <div className="text-center text-gray-500">
            <p>&copy; 2024 Soroban Security Scanner. All rights reserved.</p>
            <p className="text-sm mt-2">
              Building a more secure Stellar ecosystem, one bounty at a time.
            </p>
          </div>
        </div>
      </footer>
    </div>
  );
}
