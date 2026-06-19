// Test file for MultiSigWizard component
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import MultiSigWizard from './MultiSigWizard';

// Mock the utils
jest.mock('../utils/multisig', () => ({
  validateMultiSigConfig: jest.fn(() => ({ isValid: true, errors: [], warnings: [] })),
  calculateTotalWeight: jest.fn(() => 10),
  calculateSignersNeeded: jest.fn(() => 2),
  getThresholdRecommendations: jest.fn(() => ({ conservative: 8, standard: 7, flexible: 6 })),
  analyzeSecurity: jest.fn(() => ({ score: 85, risks: [], recommendations: [] })),
  formatDuration: jest.fn(() => '1h 0m'),
  truncatePublicKey: jest.fn(() => 'GABC...1234'),
  generateSignerId: jest.fn(() => '12345'),
  isValidPublicKey: jest.fn(() => true),
}));

describe('MultiSigWizard', () => {
  const mockOnConfigCreate = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
  });

  test('renders wizard with initial step', () => {
    render(<MultiSigWizard onConfigCreate={mockOnConfigCreate} />);

    expect(screen.getByText('Multi-Signature Wallet Creator')).toBeInTheDocument();
    expect(screen.getAllByText('Basic Information').length).toBeGreaterThan(0);
    expect(
      screen.getByText('Provide basic information about your multi-signature wallet')
    ).toBeInTheDocument();
  });

  test('shows progress steps', () => {
    render(<MultiSigWizard onConfigCreate={mockOnConfigCreate} />);

    expect(screen.getAllByText('Basic Information').length).toBeGreaterThan(0);
    expect(screen.getByText('Configure Signers')).toBeInTheDocument();
    expect(screen.getByText('Set Threshold')).toBeInTheDocument();
    expect(screen.getByText('Advanced Settings')).toBeInTheDocument();
    expect(screen.getByText('Preview & Create')).toBeInTheDocument();
  });

  test('validates wallet name', async () => {
    render(<MultiSigWizard onConfigCreate={mockOnConfigCreate} />);

    const nextButton = screen.getByText('Next →');
    fireEvent.click(nextButton);

    await waitFor(() => {
      expect(screen.getAllByText('Basic Information').length).toBeGreaterThan(0);
    });
  });

  test('allows navigation through steps', async () => {
    render(<MultiSigWizard onConfigCreate={mockOnConfigCreate} />);

    // Fill in basic info
    const nameInput = screen.getByLabelText('Wallet Name *');
    fireEvent.change(nameInput, { target: { value: 'Test Wallet' } });

    const nextButton = screen.getByText('Next →');
    fireEvent.click(nextButton);

    await waitFor(() => {
      expect(screen.getAllByText('Configure Signers').length).toBeGreaterThan(0);
    });
  });

  test('adds and removes signers', async () => {
    render(<MultiSigWizard onConfigCreate={mockOnConfigCreate} />);

    // Navigate to signers step
    const nameInput = screen.getByLabelText('Wallet Name *');
    fireEvent.change(nameInput, { target: { value: 'Test Wallet' } });

    const nextButton = screen.getByText('Next →');
    fireEvent.click(nextButton);

    await waitFor(() => {
      expect(screen.getByText('Signers (0)')).toBeInTheDocument();
    });

    // Add signer
    const addSignerButton = screen.getByText('+ Add Signer');
    fireEvent.click(addSignerButton);

    await waitFor(() => {
      expect(screen.getByText('Signers (1)')).toBeInTheDocument();
    });
  });

  test('renders with provided initial configuration', async () => {
    const mockConfig = {
      name: 'Test Wallet',
      description: 'Test description',
      signers: [],
      threshold: 2,
      timeLock: 3600,
      network: 'testnet' as const,
    };

    render(<MultiSigWizard onConfigCreate={mockOnConfigCreate} initialConfig={mockConfig} />);

    await waitFor(() => {
      expect(screen.getByLabelText('Wallet Name *')).toBeInTheDocument();
    });
  });
});
