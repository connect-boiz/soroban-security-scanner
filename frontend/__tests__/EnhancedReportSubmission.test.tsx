import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { EnhancedReportSubmission } from '../components/EnhancedReportSubmission';
import { Bounty } from '@/types/bounty';

const bounty: Bounty = {
  id: 'b1',
  title: 'Scanner Contract Audit',
  reward: '5000',
  rewardAmount: 5000,
  difficulty: 'High',
  firstToFind: true,
};

const validFindings =
  'This vulnerability allows an attacker to bypass access controls. ' +
  'Impact: funds can be drained from the escrow contract. ' +
  'Mitigation: implement checks-effects-interactions and reentrancy guards.';

const validPublicKey = 'G' + 'A'.repeat(55);

const fillForm = () => {
  fireEvent.change(screen.getByLabelText(/security findings/i), {
    target: { value: validFindings },
  });
  fireEvent.change(screen.getByLabelText(/proof of concept/i), {
    target: { value: '```rust\nlet x = 1;\n```' },
  });
  fireEvent.change(screen.getByLabelText(/affected files/i), {
    target: { value: 'src/auth.js, lib/crypto.ts' },
  });
  fireEvent.change(screen.getByLabelText(/reproduction steps/i), {
    target: {
      value:
        '1. Deploy the contract\n2. Call the vulnerable function with crafted input\n3. Observe the drained balance',
    },
  });
  fireEvent.change(screen.getByLabelText(/owner.s public key/i), {
    target: { value: validPublicKey },
  });
};

describe('EnhancedReportSubmission', () => {
  beforeEach(() => {
    // Make the async public-key validation deterministic (always passes)
    jest.spyOn(Math, 'random').mockReturnValue(0.5);
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  it('renders the bounty header with reward and difficulty', () => {
    render(<EnhancedReportSubmission bounty={bounty} onSubmit={jest.fn()} onCancel={jest.fn()} />);
    expect(screen.getByText('Submit Security Report')).toBeInTheDocument();
    expect(screen.getByText('Scanner Contract Audit')).toBeInTheDocument();
    expect(screen.getByText(/5000 XLM/)).toBeInTheDocument();
    expect(screen.getAllByText('High').length).toBeGreaterThan(0);
    expect(screen.getByText(/first-to-find bonus/i)).toBeInTheDocument();
  });

  it('updates the estimated reward when a severity is selected', () => {
    render(<EnhancedReportSubmission bounty={bounty} onSubmit={jest.fn()} onCancel={jest.fn()} />);
    expect(screen.getByText('3000 XLM')).toBeInTheDocument(); // Medium = 60%
    fireEvent.click(screen.getByRole('button', { name: /critical/i }));
    expect(screen.getAllByText('5000 XLM').length).toBe(2); // header + estimated (100%)
    fireEvent.click(screen.getByRole('button', { name: /low/i }));
    expect(screen.getByText('1500 XLM')).toBeInTheDocument(); // Low = 30%
  });

  it('shows validation errors on blur for invalid fields', async () => {
    render(<EnhancedReportSubmission bounty={bounty} onSubmit={jest.fn()} onCancel={jest.fn()} />);
    const findings = screen.getByLabelText(/security findings/i);
    fireEvent.change(findings, { target: { value: 'too short' } });
    fireEvent.blur(findings);
    expect(await screen.findByText('Please fix the following errors:')).toBeInTheDocument();
    expect(screen.getAllByText(/at least 100 characters/i).length).toBeGreaterThan(0);

    const keyField = screen.getByLabelText(/owner.s public key/i);
    fireEvent.change(keyField, { target: { value: 'not-a-valid-key' } });
    fireEvent.blur(keyField);
    // appears in both the field error and the summary after the 500ms async rule
    const stellarErrors = await screen.findAllByText(
      /valid Stellar public key/i,
      {},
      { timeout: 3000 }
    );
    expect(stellarErrors.length).toBeGreaterThan(0);
  });

  it('encrypts the findings and submits the report', async () => {
    const onSubmit = jest.fn();
    render(<EnhancedReportSubmission bounty={bounty} onSubmit={onSubmit} onCancel={jest.fn()} />);
    fillForm();
    fireEvent.click(screen.getByRole('button', { name: /encrypt & preview/i }));

    expect(await screen.findByText('Findings Encrypted Successfully')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /submit report/i })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /submit report/i }));
    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));

    const submission = onSubmit.mock.calls[0][0];
    expect(submission).toMatchObject({
      bountyId: 'b1',
      severity: 'Medium',
      findings: validFindings,
      status: 'Pending',
    });
    expect(submission.encryptedFindings).toBeTruthy();
  });

  it('toggles the encrypted data preview', async () => {
    render(<EnhancedReportSubmission bounty={bounty} onSubmit={jest.fn()} onCancel={jest.fn()} />);
    fillForm();
    fireEvent.click(screen.getByRole('button', { name: /encrypt & preview/i }));
    expect(await screen.findByText('Findings Encrypted Successfully')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /hide encrypted data/i }));
    // The whole preview block (including the toggle) unmounts when hidden
    expect(screen.queryByText('Findings Encrypted Successfully')).not.toBeInTheDocument();
  });

  it('cancels the form via the cancel button', () => {
    const onCancel = jest.fn();
    render(<EnhancedReportSubmission bounty={bounty} onSubmit={jest.fn()} onCancel={onCancel} />);
    fireEvent.click(screen.getByRole('button', { name: /cancel/i }));
    expect(onCancel).toHaveBeenCalled();
  });

  it('shows the findings character counter', () => {
    render(<EnhancedReportSubmission bounty={bounty} onSubmit={jest.fn()} onCancel={jest.fn()} />);
    expect(screen.getByText(/0\/5000 characters/)).toBeInTheDocument();
  });
});
