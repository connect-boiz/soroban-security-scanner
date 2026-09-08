import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import CspDashboard from '../components/CspDashboard';

const dashboard = {
  violations_by_directive: [
    { directive: 'script-src', count: 12, trend: 'up' },
    { directive: 'img-src', count: 3, trend: 'down' },
  ],
  top_blocked_uris: [{ blocked_uri: 'https://evil.example/x.js', count: 9 }],
  violations_over_time: [
    { date: '2026-01-01', count: 5 },
    { date: '2026-01-02', count: 8 },
  ],
};

const violations = {
  data: [
    {
      directive: 'script-src',
      blocked_uri: 'https://evil.example/x.js',
      document_uri: 'https://app.example/',
      violated_at: '2026-01-02T10:00:00Z',
    },
  ],
};

describe('CspDashboard', () => {
  const originalFetch = global.fetch;

  afterEach(() => {
    global.fetch = originalFetch;
  });

  it('shows the loading skeleton while fetching', () => {
    global.fetch = jest.fn().mockImplementation(() => new Promise(() => {}));
    const { container } = render(<CspDashboard />);
    expect(container.querySelector('.skeleton')).toBeInTheDocument();
  });

  it('renders the dashboard with violations and trends', async () => {
    global.fetch = jest
      .fn()
      .mockResolvedValueOnce({ json: async () => violations })
      .mockResolvedValueOnce({ json: async () => dashboard });
    render(<CspDashboard />);
    expect(await screen.findByText('CSP violation dashboard')).toBeInTheDocument();
    expect(screen.getByText('1 recent violations')).toBeInTheDocument();
    expect(screen.getByText('Violation trend')).toBeInTheDocument();
    expect(screen.getByText('Directive breakdown')).toBeInTheDocument();
    expect(screen.getAllByText('script-src').length).toBeGreaterThan(0);
    expect(screen.getByText('img-src')).toBeInTheDocument();
    expect(screen.getByText('12')).toBeInTheDocument();
    expect(screen.getByText('2026-01-02')).toBeInTheDocument();
    expect(screen.getByText('https://evil.example/x.js')).toBeInTheDocument();
    expect(screen.getByText('https://app.example/')).toBeInTheDocument();
  });

  it('renders an empty dashboard when there is no data', async () => {
    global.fetch = jest
      .fn()
      .mockResolvedValueOnce({ json: async () => ({}) })
      .mockResolvedValueOnce({ json: async () => ({}) });
    render(<CspDashboard />);
    expect(await screen.findByText('CSP violation dashboard')).toBeInTheDocument();
    expect(screen.getByText('0 recent violations')).toBeInTheDocument();
  });

  it('recovers from a fetch failure', async () => {
    const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => {});
    global.fetch = jest.fn().mockRejectedValue(new Error('boom'));
    render(<CspDashboard />);
    expect(await screen.findByText('CSP violation dashboard')).toBeInTheDocument();
    expect(consoleSpy).toHaveBeenCalled();
    consoleSpy.mockRestore();
  });
});
