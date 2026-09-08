import React from 'react';
import { render, screen, fireEvent, act } from '@testing-library/react';
import PerformanceMonitor from '../components/PerformanceMonitor';
import { StorePerformanceMonitor } from '@/store/storeDebug';

jest.mock('@/store/storeDebug', () => ({
  StorePerformanceMonitor: {
    getInstance: jest.fn(),
  },
}));

const getInstanceMock = StorePerformanceMonitor.getInstance as jest.Mock;

const makeMetrics = () => ({
  scanContract: { count: 3, avgTime: 2.5, totalTime: 7.5 },
  submitReport: { count: 1, avgTime: 8.2, totalTime: 8.2 },
});

describe('PerformanceMonitor', () => {
  const originalEnv = process.env.NODE_ENV;

  let monitorMock: { getMetrics: jest.Mock; reset: jest.Mock; logMetrics: jest.Mock };

  beforeEach(() => {
    jest.useFakeTimers();
    Object.defineProperty(process.env, 'NODE_ENV', { value: 'development' });
    monitorMock = {
      getMetrics: jest.fn().mockReturnValue({}),
      reset: jest.fn(),
      logMetrics: jest.fn(),
    };
    getInstanceMock.mockReturnValue(monitorMock);
  });

  afterEach(() => {
    jest.useRealTimers();
    Object.defineProperty(process.env, 'NODE_ENV', { value: originalEnv });
  });

  it('renders nothing outside development', () => {
    Object.defineProperty(process.env, 'NODE_ENV', { value: 'production' });
    const { container } = render(<PerformanceMonitor />);
    expect(container.firstChild).toBeNull();
  });

  it('shows metrics after the polling interval', () => {
    monitorMock.getMetrics.mockReturnValue(makeMetrics());
    render(<PerformanceMonitor />);
    act(() => {
      jest.advanceTimersByTime(1000);
    });
    expect(screen.getByText('Performance Monitor')).toBeInTheDocument();
    expect(screen.getByText('4')).toBeInTheDocument(); // total actions
    expect(screen.getByText('scanContract')).toBeInTheDocument();
    expect(screen.getByText('3x')).toBeInTheDocument();
    expect(screen.getByText('Avg: 2.50ms')).toBeInTheDocument();
    expect(screen.getByText('Total: 7.50ms')).toBeInTheDocument();
  });

  it('shows zeroed summary when there are no metrics', () => {
    render(<PerformanceMonitor />);
    act(() => {
      jest.advanceTimersByTime(1000);
    });
    expect(screen.getByText('0')).toBeInTheDocument();
    expect(screen.getByText('0.0ms')).toBeInTheDocument();
  });

  it('resets metrics when Reset is clicked', () => {
    monitorMock.getMetrics.mockReturnValue(makeMetrics());
    render(<PerformanceMonitor />);
    act(() => {
      jest.advanceTimersByTime(1000);
    });
    fireEvent.click(screen.getByRole('button', { name: /reset/i }));
    expect(monitorMock.reset).toHaveBeenCalled();
    expect(screen.queryByText('scanContract')).not.toBeInTheDocument();
  });

  it('logs metrics when Log is clicked', () => {
    monitorMock.getMetrics.mockReturnValue(makeMetrics());
    render(<PerformanceMonitor />);
    act(() => {
      jest.advanceTimersByTime(1000);
    });
    fireEvent.click(screen.getByRole('button', { name: /log/i }));
    expect(monitorMock.logMetrics).toHaveBeenCalled();
  });

  it('hides the panel when the close button is clicked', () => {
    render(<PerformanceMonitor />);
    act(() => {
      jest.advanceTimersByTime(1000);
    });
    const closeButton = screen.getAllByRole('button').find(b => b.textContent === '×');
    fireEvent.click(closeButton!);
    expect(screen.queryByText('Performance Monitor')).not.toBeInTheDocument();
  });
});
