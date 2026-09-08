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
});
