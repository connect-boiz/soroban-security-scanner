import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import App from '../app/page';
import AuthPage from '../app/auth/page';
import SimpleAuthPage from '../app/auth/simple-page';
import NotificationsPage from '../app/notifications/page';
import { login, signUp, resetPassword } from '../lib/auth/authService';
import { persistSession } from '../lib/auth/session';
import { useHelpStore } from '../lib/store/helpStore';

// Mock every dynamically-imported view so the dashboard test focuses on
// navigation and chrome rather than the views themselves (covered separately).
jest.mock('../components/ScannerInterface', () => () => (
  <div data-testid="view-scanner">Scanner</div>
));
jest.mock('../components/VulnerabilityReport', () => () => (
  <div data-testid="view-report">Report</div>
));
jest.mock('../components/AnalyticsDashboard', () => () => (
  <div data-testid="view-analytics">Analytics</div>
));
jest.mock('../components/TimeTravelDebugger', () => () => (
  <div data-testid="view-timetravel">Time Travel</div>
));
jest.mock('../components/BatchOperations', () => () => <div data-testid="view-batch">Batch</div>);
jest.mock('../components/BalanceDisplay', () => () => (
  <div data-testid="view-balance">Balance</div>
));
jest.mock('../components/SettingsPanel', () => () => (
  <div data-testid="view-settings">Settings</div>
));
jest.mock('../components/MultiSigWizard', () => () => (
  <div data-testid="view-multisig">MultiSig</div>
));
jest.mock('../components/CspDashboard', () => () => <div data-testid="view-csp">CSP</div>);

jest.mock('../lib/auth/authService', () => ({
  login: jest.fn(),
  signUp: jest.fn(),
  resetPassword: jest.fn(),
}));

jest.mock('../lib/auth/session', () => ({
  persistSession: jest.fn(),
}));

const loginMock = login as jest.Mock;
const persistSessionMock = persistSession as jest.Mock;

describe('app dashboard', () => {
  beforeEach(() => {
    useHelpStore.setState({ activeTour: null, completedTours: [], helpPanelTopic: null });
    localStorage.clear();
  });

  it('renders the scanner view by default', async () => {
    render(<App />);
    expect(screen.getByText('Soroban Security Scanner')).toBeInTheDocument();
    expect(await screen.findByTestId('view-scanner')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /new scan/i })).toBeInTheDocument();
  });

  it('navigates between views via the header nav', async () => {
    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: /bounty board/i }));
    expect(screen.getByText('Bounty marketplace features are coming soon.')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /leaderboard/i }));
    expect(screen.getByText('Researcher rankings will be displayed here.')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /analytics/i }));
    expect(await screen.findByTestId('view-analytics')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /wallet/i }));
    expect(await screen.findByTestId('view-balance')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /csp/i }));
    expect(await screen.findByTestId('view-csp')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /settings/i }));
    expect(await screen.findByTestId('view-settings')).toBeInTheDocument();
  });

  it('opens the help panel from the Documentation button', () => {
    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: /documentation/i }));
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('starts the guided tour from the Take a Tour button', () => {
    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: /take a tour/i }));
    expect(screen.getByTestId('mock-joyride').getAttribute('data-run')).toBe('true');
  });

  it('opens help from the floating help button', () => {
    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: /get help/i }));
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });
});

describe('app auth page', () => {
  it('renders the auth container with the login form', () => {
    render(<AuthPage />);
    expect(screen.getByText('Welcome Back')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /sign in/i })).toBeInTheDocument();
  });
});

describe('app simple auth page', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    loginMock.mockResolvedValue({ user: { email: 'a@b.com' }, requiresMfa: false });
  });

  it('logs in and persists the session', async () => {
    render(<SimpleAuthPage />);
    fireEvent.change(screen.getByPlaceholderText('Enter your email'), {
      target: { value: 'a@b.com' },
    });
    fireEvent.change(screen.getByPlaceholderText('Enter your password'), {
      target: { value: 'password123' },
    });
    fireEvent.submit(screen.getByRole('button', { name: /sign in/i }).closest('form')!);
    await waitFor(() => expect(loginMock).toHaveBeenCalled());
    expect(persistSessionMock).toHaveBeenCalledWith({ email: 'a@b.com' }, expect.any(Boolean));
  });
});

describe('app notifications page', () => {
  it('renders the notifications page with the empty state and preferences panel', () => {
    render(<NotificationsPage />);
    expect(screen.getByRole('heading', { level: 1, name: /notifications/i })).toBeInTheDocument();
    expect(screen.getByText('No notifications')).toBeInTheDocument();
    expect(screen.getByText('Notification Channels')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /notifications/i })).toBeInTheDocument();
  });
});
