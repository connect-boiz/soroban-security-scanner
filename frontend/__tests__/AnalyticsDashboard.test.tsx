import React from 'react';
import { render, screen, fireEvent, act } from '@testing-library/react';
import AnalyticsDashboard from '../components/AnalyticsDashboard';

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

describe('AnalyticsDashboard', () => {
  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it('shows the loading state then the security overview', async () => {
    render(<AnalyticsDashboard />);
    expect(screen.getByText('Loading analytics data...')).toBeInTheDocument();
    await act(async () => {
      jest.advanceTimersByTime(1100);
    });
    expect(screen.getByText('Analytics Dashboard')).toBeInTheDocument();
    expect(screen.getByText('1,250')).toBeInTheDocument(); // total scans
    expect(screen.getByText('342')).toBeInTheDocument(); // vulnerabilities
    expect(screen.getByText('27.4%')).toBeInTheDocument(); // detection rate
    expect(screen.getByText('Severity Distribution')).toBeInTheDocument();
    expect(screen.getByText('Scan Activity Trends')).toBeInTheDocument();
  });

  it('switches to the marketplace view with charts', async () => {
    render(<AnalyticsDashboard />);
    await act(async () => {
      jest.advanceTimersByTime(1100);
    });
    fireEvent.click(screen.getByRole('button', { name: /marketplace/i }));
    expect(screen.getByText('Marketplace Summary')).toBeInTheDocument();
    expect(screen.getByText('Total Portfolio')).toBeInTheDocument();
    expect(screen.getByText('Portfolio Distribution')).toBeInTheDocument();
    expect(screen.getByText('Transaction Volume')).toBeInTheDocument();
    expect(screen.getAllByText('Performance Trends').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Overall Performance').length).toBeGreaterThan(0);
  });

  it('changes the time filter and refreshes', async () => {
    render(<AnalyticsDashboard />);
    await act(async () => {
      jest.advanceTimersByTime(1100);
    });
    fireEvent.change(screen.getByDisplayValue('Last 30 days'), {
      target: { value: '7d' },
    });
    expect((screen.getByDisplayValue('Last 7 days') as HTMLSelectElement).value).toBe('7d');

    fireEvent.click(screen.getByRole('button', { name: /refresh/i }));
    await act(async () => {
      jest.advanceTimersByTime(1100);
    });
    expect(screen.getByRole('button', { name: /refresh/i })).toBeEnabled();
  });

  it('renders the export button', async () => {
    render(<AnalyticsDashboard />);
    await act(async () => {
      jest.advanceTimersByTime(1100);
    });
    expect(screen.getByRole('button', { name: /export/i })).toBeInTheDocument();
  });

  it('correctly evaluates hasAnyData utility', () => {
    const { hasAnyData } = require('../components/AnalyticsDashboard');
    expect(hasAnyData(null, null, null, null)).toBe(false);
    expect(
      hasAnyData(
        {
          totalScans: 0,
          vulnerabilitiesFound: 0,
          contractsAnalyzed: 0,
          averageScanTime: 0,
          severityBreakdown: { critical: 0, high: 0, medium: 0, low: 0 },
          weeklyData: [],
        },
        [],
        [],
        []
      )
    ).toBe(false);

    expect(
      hasAnyData(
        {
          totalScans: 10,
          vulnerabilitiesFound: 1,
          contractsAnalyzed: 5,
          averageScanTime: 1,
          severityBreakdown: { critical: 0, high: 0, medium: 0, low: 0 },
          weeklyData: [],
        },
        [],
        [],
        []
      )
    ).toBe(true);

    expect(hasAnyData(null, [{ asset: 'XLM', value: 100, percentage: 100, change: 0, changePercent: 0 }], [], [])).toBe(true);
  });

  it('renders a single Welcome to Analytics empty state with link to scan page when all data sources are empty', async () => {
    render(
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
    );
    await act(async () => {
      jest.advanceTimersByTime(1100);
    });

    expect(screen.getByText('Welcome to Analytics')).toBeInTheDocument();
    expect(
      screen.getByText(/No scan data for this period — run your first scan to populate your security and marketplace analytics here/i)
    ).toBeInTheDocument();
    expect(screen.getByRole('region', { name: /Welcome to Analytics empty state/i })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /Run your first scan/i })).toHaveAttribute('href', '/scan');
  });
});
