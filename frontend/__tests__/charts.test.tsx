import React from 'react';
import { render, screen } from '@testing-library/react';
import { subDays } from 'date-fns';
import TransactionChart from '../components/charts/TransactionChart';
import PerformanceChart from '../components/charts/PerformanceChart';
import PortfolioChart from '../components/charts/PortfolioChart';
import { TransactionData, PerformanceMetrics, PortfolioData } from '@/types/charts';

// recharts needs real DOM measurement (ResizeObserver etc.) which jsdom lacks;
// stub the primitives so we exercise our own components' logic instead.
jest.mock('recharts', () => {
  const React = require('react');
  const passthrough = (props: any) => React.createElement('div', props, props.children);
  return {
    LineChart: passthrough,
    Line: passthrough,
    AreaChart: passthrough,
    Area: passthrough,
    BarChart: passthrough,
    Bar: passthrough,
    XAxis: passthrough,
    YAxis: passthrough,
    CartesianGrid: passthrough,
    Tooltip: passthrough,
    Legend: passthrough,
    ResponsiveContainer: ({ children }: any) => React.createElement('div', null, children),
    PieChart: passthrough,
    Pie: passthrough,
    Cell: passthrough,
    RadarChart: passthrough,
    PolarGrid: passthrough,
    PolarAngleAxis: passthrough,
    PolarRadiusAxis: passthrough,
    Radar: passthrough,
    ComposedChart: passthrough,
  };
});

const txData: TransactionData[] = [
  {
    id: '1',
    timestamp: subDays(new Date(), 1),
    amount: 1000,
    type: 'deposit',
    status: 'completed',
  },
  {
    id: '2',
    timestamp: subDays(new Date(), 2),
    amount: 500,
    type: 'withdrawal',
    status: 'completed',
  },
  { id: '3', timestamp: subDays(new Date(), 3), amount: 200, type: 'reward', status: 'pending' },
  { id: '4', timestamp: subDays(new Date(), 4), amount: 50, type: 'penalty', status: 'failed' },
];

const perfData: PerformanceMetrics[] = [
  {
    date: subDays(new Date(), 2),
    reputation: 100,
    completedBounties: 5,
    totalEarned: 500,
    successRate: 80,
    avgCompletionTime: 10,
  },
  {
    date: subDays(new Date(), 1),
    reputation: 120,
    completedBounties: 8,
    totalEarned: 800,
    successRate: 90,
    avgCompletionTime: 8,
  },
];

const portData: PortfolioData[] = [
  { asset: 'XLM', value: 5000, percentage: 50, change: 100, changePercent: 2 },
  { asset: 'BTC', value: 3000, percentage: 30, change: -50, changePercent: -1.6 },
  { asset: 'ETH', value: 2000, percentage: 20, change: 20, changePercent: 1 },
];

describe('TransactionChart', () => {
  it('renders the line chart with aggregate metrics', () => {
    render(<TransactionChart data={txData} />);
    expect(screen.getByText('Transaction Analytics')).toBeInTheDocument();
    expect(screen.getByText('4 transactions')).toBeInTheDocument();
    expect(screen.getByText('Total Volume')).toBeInTheDocument();
    expect(screen.getByText('$1,750')).toBeInTheDocument();
    expect(screen.getByText('50.0%')).toBeInTheDocument();
    expect(screen.getByText('$437.50')).toBeInTheDocument();
  });

  it('renders the area chart variant', () => {
    render(<TransactionChart data={txData} config={{ type: 'area' }} />);
    expect(screen.getByText('$1,750 total')).toBeInTheDocument();
  });

  it('renders the pie chart variant', () => {
    render(<TransactionChart data={txData} config={{ type: 'pie' }} />);
    expect(screen.getByText('Transaction Types')).toBeInTheDocument();
    expect(screen.getAllByText('$1,750').length).toBeGreaterThan(0);
  });

  it('renders the bar chart variant with success rate', () => {
    render(<TransactionChart data={txData} config={{ type: 'bar' }} />);
    expect(screen.getByText('Transaction Status')).toBeInTheDocument();
    expect(screen.getByText('50.0% success rate')).toBeInTheDocument();
  });

  it('filters out transactions older than the time range', () => {
    const oldTx: TransactionData = {
      id: 'old',
      timestamp: subDays(new Date(), 60),
      amount: 9999,
      type: 'deposit',
      status: 'completed',
    };
    render(<TransactionChart data={[...txData, oldTx]} timeRange="7d" />);
    // 9999 excluded from the total
    expect(screen.getByText('$1,750')).toBeInTheDocument();
  });

  it('renders an empty state for an empty data set', () => {
    render(<TransactionChart data={[]} />);
    expect(screen.getByText('0 transactions')).toBeInTheDocument();
    expect(screen.getByText('$0')).toBeInTheDocument();
  });

  it('applies a custom title from config', () => {
    render(<TransactionChart data={txData} config={{ title: 'My Chart' }} />);
    expect(screen.getByText('My Chart')).toBeInTheDocument();
  });
});

describe('PerformanceChart', () => {
  it('renders the line chart with current and previous metric deltas', () => {
    render(<PerformanceChart data={perfData} />);
    expect(screen.getByText('Performance Metrics')).toBeInTheDocument();
    expect(screen.getByText('2 data points')).toBeInTheDocument();
    expect(screen.getByText('Reputation')).toBeInTheDocument();
    expect(screen.getByText('120')).toBeInTheDocument();
    expect(screen.getByText('$800')).toBeInTheDocument();
    expect(screen.getByText('90.0%')).toBeInTheDocument();
    expect(screen.getAllByText('+20.0%').length).toBeGreaterThan(0);
  });

  it('renders the area chart variant', () => {
    render(<PerformanceChart data={perfData} config={{ type: 'area' }} />);
    expect(screen.getByText('Growth Overview')).toBeInTheDocument();
  });

  it('renders the radar chart variant', () => {
    render(
      <PerformanceChart
        data={perfData}
        config={{ type: 'radar' }}
        metrics={[
          'reputation',
          'completedBounties',
          'totalEarned',
          'successRate',
          'avgCompletionTime',
        ]}
      />
    );
    expect(screen.getByText('Overall Performance')).toBeInTheDocument();
  });

  it('renders the bar chart variant', () => {
    render(<PerformanceChart data={perfData} config={{ type: 'bar' }} />);
    expect(screen.getByText('Monthly Comparison')).toBeInTheDocument();
  });

  it('renders a negative delta in red', () => {
    const declining: PerformanceMetrics[] = [
      { ...perfData[0], reputation: 120, completedBounties: 8 },
      { ...perfData[1], reputation: 100, completedBounties: 5 },
    ];
    render(<PerformanceChart data={declining} />);
    expect(screen.getByText('-16.7%')).toBeInTheDocument();
    expect(screen.getByText('-37.5%')).toBeInTheDocument();
  });

  it('renders no metric cards when data is empty', () => {
    render(<PerformanceChart data={[]} />);
    expect(screen.getByText('0 data points')).toBeInTheDocument();
    expect(screen.queryByText('Reputation')).not.toBeInTheDocument();
  });
});

describe('PortfolioChart', () => {
  it('renders the pie chart with totals', () => {
    render(<PortfolioChart data={portData} />);
    expect(screen.getByText('Portfolio Distribution')).toBeInTheDocument();
    expect(screen.getByText('Total Value')).toBeInTheDocument();
    expect(screen.getByText('$10,000')).toBeInTheDocument();
    expect(screen.getAllByText('+0.70%').length).toBeGreaterThan(0);
  });

  it('renders a negative 24h change', () => {
    const declining = portData.map(d => ({
      ...d,
      change: -d.change,
      changePercent: -d.changePercent,
    }));
    render(<PortfolioChart data={declining} />);
    expect(screen.getAllByText('-0.70%').length).toBeGreaterThan(0);
  });

  it('renders the bar chart variant', () => {
    render(<PortfolioChart data={portData} config={{ type: 'bar' }} />);
    expect(screen.getAllByText('$10,000').length).toBeGreaterThan(0);
  });

  it('renders the area chart variant with cumulative values', () => {
    render(<PortfolioChart data={portData} config={{ type: 'area' }} />);
    expect(screen.getAllByText('$10,000').length).toBeGreaterThan(0);
  });

  it('renders an empty portfolio', () => {
    render(<PortfolioChart data={[]} />);
    expect(screen.getByText('$0')).toBeInTheDocument();
  });
});
