import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { PerformanceAlertBanner } from '../components/PerformanceAlertBanner';

const alerts = [
  {
    metric: 'LCP',
    value: 4.2,
    threshold: 2.5,
    severity: 'Critical',
    message: 'Slow',
    timestamp: 't',
  },
  {
    metric: 'CLS',
    value: 0.3,
    threshold: 0.1,
    severity: 'Warning',
    message: 'Janky',
    timestamp: 't',
  },
];

describe('PerformanceAlertBanner', () => {
  const originalFetch = global.fetch;

  afterEach(() => {
    global.fetch = originalFetch;
  });

  it('renders nothing when there are no alerts', async () => {
    global.fetch = jest.fn().mockResolvedValue({ ok: true, json: async () => ({ alerts: [] }) });
    const { container } = render(<PerformanceAlertBanner />);
    await waitFor(() => expect(global.fetch).toHaveBeenCalled());
    expect(container.firstChild).toBeNull();
  });

  it('shows critical alerts with a red banner', async () => {
    global.fetch = jest.fn().mockResolvedValue({ ok: true, json: async () => ({ alerts }) });
    render(<PerformanceAlertBanner />);
    expect(await screen.findByRole('alert')).toBeInTheDocument();
    expect(screen.getByText(/Performance Alert: 1 critical, 1 warning/)).toBeInTheDocument();
    expect(screen.getByText(/LCP, CLS/)).toBeInTheDocument();
    expect(screen.getByRole('alert').className).toContain('bg-red-600');
  });

  it('shows a yellow banner for warnings only', async () => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ alerts: [alerts[1]] }),
    });
    render(<PerformanceAlertBanner />);
    expect(await screen.findByRole('alert')).toBeInTheDocument();
    expect(screen.getByText(/1 warning/)).toBeInTheDocument();
    expect(screen.getByRole('alert').className).toContain('bg-yellow-500');
  });

  it('dismisses the banner', async () => {
    global.fetch = jest.fn().mockResolvedValue({ ok: true, json: async () => ({ alerts }) });
    render(<PerformanceAlertBanner />);
    expect(await screen.findByRole('alert')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /dismiss alert banner/i }));
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('renders nothing when the fetch fails', async () => {
    global.fetch = jest.fn().mockRejectedValue(new Error('network down'));
    const { container } = render(<PerformanceAlertBanner />);
    await waitFor(() => expect(global.fetch).toHaveBeenCalled());
    expect(container.firstChild).toBeNull();
  });
});
