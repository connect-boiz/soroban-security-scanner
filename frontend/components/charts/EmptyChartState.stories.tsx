import React from 'react';
import type { Meta, StoryObj } from '@storybook/react';
import { PortfolioChart } from './PortfolioChart';
import { TransactionChart } from './TransactionChart';
import { PerformanceChart } from './PerformanceChart';
import { EmptyChartState } from './EmptyChartState';
import AnalyticsDashboard from '../AnalyticsDashboard';
import { PieChartIcon, Trophy, Activity } from 'lucide-react';

const meta: Meta<typeof EmptyChartState> = {
  title: 'Components/Charts/EmptyStates',
  component: EmptyChartState,
  parameters: {
    layout: 'padded',
  },
};

export default meta;
type Story = StoryObj<typeof EmptyChartState>;

export const StandaloneDefault: Story = {
  args: {
    title: 'Custom Empty State',
    message: 'No chart data available for this view',
    description: 'Execute a smart contract scan to view analytics.',
    actionText: 'Run a Scan',
    actionHref: '/scan',
  },
};

export const StandalonePortfolio: Story = {
  args: {
    title: 'Portfolio Distribution',
    message: 'No portfolio assets yet',
    description: 'Connect a wallet or run a scan to populate your portfolio distribution.',
    icon: PieChartIcon,
    actionText: 'Run a Scan',
    actionHref: '/scan',
  },
};

export const StandalonePerformance: Story = {
  args: {
    title: 'Performance Trends',
    message: 'No performance metrics yet',
    description: 'Complete scans and audits to start tracking your security performance metrics.',
    icon: Trophy,
    actionText: 'Start Scanning',
    actionHref: '/scan',
  },
};

export const StandaloneTransaction: Story = {
  args: {
    title: 'Transaction Volume',
    message: 'No transactions yet',
    description: 'Run vulnerability scans or execute contract interactions to see transaction history.',
    icon: Activity,
    actionText: 'Run a Scan',
    actionHref: '/scan',
  },
};

export const EmptyPortfolioComponent = () => (
  <div className="max-w-xl mx-auto">
    <PortfolioChart data={[]} />
  </div>
);

export const EmptyTransactionComponent = () => (
  <div className="max-w-xl mx-auto">
    <TransactionChart data={[]} />
  </div>
);

export const EmptyPerformanceComponent = () => (
  <div className="max-w-xl mx-auto">
    <PerformanceChart data={[]} />
  </div>
);

export const EmptyDashboardWelcome = () => (
  <div className="max-w-4xl mx-auto">
    <AnalyticsDashboard
      portfolioData={[]}
      transactionData={[]}
      performanceData={[]}
      analyticsData={{
        totalScans: 0,
        vulnerabilitiesFound: 0,
        contractsAnalyzed: 0,
        averageScanTime: 0,
        severityBreakdown: { critical: 0, high: 0, medium: 0, low: 0 },
        weeklyData: [],
      }}
    />
  </div>
);
