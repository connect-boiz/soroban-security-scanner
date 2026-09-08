import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import MultiSigWizard from './MultiSigWizard';

const VALID_KEY = 'G' + 'A'.repeat(55);
const VALID_KEY2 = 'G' + 'B'.repeat(55);

const fillName = (value = 'Test Wallet') => {
  fireEvent.change(screen.getByLabelText('Wallet Name *'), { target: { value } });
};

const clickNext = () => fireEvent.click(screen.getByRole('button', { name: /next/i }));

const goToSigners = (name = 'Test Wallet') => {
  fillName(name);
  clickNext();
};

const getLastSignerCard = () => {
  const headings = screen.getAllByText(/^Signer \d+$/);
  return headings[headings.length - 1].closest('div.border') as HTMLElement;
};

const fillLastSigner = (name: string, publicKey = VALID_KEY, weight?: number) => {
  const card = getLastSignerCard();
  const inputs = card.querySelectorAll('input');
  fireEvent.change(inputs[0], { target: { value: name } });
  if (weight !== undefined) {
    fireEvent.change(inputs[1], { target: { value: String(weight) } });
  }
  fireEvent.change(inputs[2], { target: { value: publicKey } });
};

const addValidSigner = (name: string, publicKey = VALID_KEY, weight = 1) => {
  fireEvent.click(screen.getByRole('button', { name: /\+ add signer/i }));
  fillLastSigner(name, publicKey, weight);
};

describe('MultiSigWizard', () => {
  it('renders the wizard with all steps and initial validation error', () => {
    render(<MultiSigWizard />);
    expect(screen.getByText('Multi-Signature Wallet Creator')).toBeInTheDocument();
    expect(screen.getAllByText('Basic Information').length).toBeGreaterThan(0);
    expect(screen.getByText('Configure Signers')).toBeInTheDocument();
    expect(screen.getByText('Set Threshold')).toBeInTheDocument();
    expect(screen.getByText('Advanced Settings')).toBeInTheDocument();
    expect(screen.getByText('Preview & Create')).toBeInTheDocument();
    expect(screen.getByText('⚠️ Wallet name is required')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /next/i })).toBeDisabled();
    expect(screen.getByRole('button', { name: /previous/i })).toBeDisabled();
  });

  it('validates the wallet name length', () => {
    render(<MultiSigWizard />);
    fillName('AB');
    expect(screen.getByText('⚠️ Wallet name must be at least 3 characters')).toBeInTheDocument();
    fillName('X'.repeat(51));
    expect(screen.getByText('⚠️ Wallet name must be less than 50 characters')).toBeInTheDocument();
  });

  it('warns when the description is very long', () => {
    render(<MultiSigWizard />);
    fillName('Valid Name');
    fireEvent.change(screen.getByLabelText('Description'), {
      target: { value: 'd'.repeat(201) },
    });
    expect(
      screen.getByText('⚡ Description is quite long, consider keeping it concise')
    ).toBeInTheDocument();
  });

  it('navigates to the signers step with a valid name and network selection', () => {
    render(<MultiSigWizard />);
    fillName('My Wallet');
    fireEvent.change(screen.getByLabelText('Network'), { target: { value: 'futurenet' } });
    clickNext();
    expect(screen.getByText('Signers (0)')).toBeInTheDocument();
    expect(screen.getByText('No signers added yet')).toBeInTheDocument();
    expect(screen.getByText('⚠️ At least one signer is required')).toBeInTheDocument();
    expect(
      screen.getByText('⚡ Consider adding multiple signers for better security')
    ).toBeInTheDocument();
  });

  it('adds and removes signers', () => {
    render(<MultiSigWizard />);
    goToSigners();
    addValidSigner('Alice');
    expect(screen.getByText('Signers (1)')).toBeInTheDocument();
    // A single signer cannot be removed
    expect(screen.queryByRole('button', { name: /remove/i })).not.toBeInTheDocument();
    addValidSigner('Bob');
    expect(screen.getByText('Signers (2)')).toBeInTheDocument();
    fireEvent.click(screen.getAllByRole('button', { name: /remove/i })[0]);
    expect(screen.getByText('Signers (1)')).toBeInTheDocument();
  });

  it('rejects a signer with an empty name and missing public key', () => {
    render(<MultiSigWizard />);
    goToSigners();
    addValidSigner('Alice');
    // Clear the name and public key of the last signer
    const card = getLastSignerCard();
    const inputs = card.querySelectorAll('input');
    fireEvent.change(inputs[0], { target: { value: '' } });
    fireEvent.change(inputs[2], { target: { value: '' } });
    expect(screen.getByText('⚠️ Signer 1 name is required')).toBeInTheDocument();
    expect(screen.getByText('⚠️ Signer 1 public key is required')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /next/i })).toBeDisabled();
  });

  it('rejects duplicate signer names and public keys', () => {
    render(<MultiSigWizard />);
    goToSigners();
    addValidSigner('Alice');
    addValidSigner('Alice');
    expect(screen.getByText('⚠️ Duplicate signer name: Alice')).toBeInTheDocument();
    expect(
      screen.getByText(
        '⚠️ Duplicate public key: GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA'
      )
    ).toBeInTheDocument();
  });

  it('rejects an invalid public key format', () => {
    render(<MultiSigWizard />);
    goToSigners();
    addValidSigner('Alice', 'not-a-key');
    expect(screen.getByText('⚠️ Invalid public key format for Alice')).toBeInTheDocument();
  });

  it('rejects a signer weight outside the allowed range', () => {
    render(<MultiSigWizard />);
    goToSigners();
    addValidSigner('Alice', VALID_KEY, 101);
    expect(
      screen.getByText('⚠️ Signer Alice weight must be between 1 and 100')
    ).toBeInTheDocument();
  });

  it('supports switching signature schemes', () => {
    render(<MultiSigWizard />);
    goToSigners();
    fireEvent.click(screen.getByRole('button', { name: /\+ add signer/i }));
    const card = getLastSignerCard();
    fireEvent.change(card.querySelector('select')!, { target: { value: 'secp256k1' } });
    const inputs = card.querySelectorAll('input');
    fireEvent.change(inputs[0], { target: { value: 'Carol' } });
    fireEvent.change(inputs[2], {
      target: { value: 'a'.repeat(66) },
    });
    expect(screen.queryByText(/Invalid public key format/i)).not.toBeInTheDocument();
  });

  it('walks through threshold presets with validation warnings', () => {
    render(<MultiSigWizard />);
    goToSigners();
    addValidSigner('Alice');
    addValidSigner('Bob', VALID_KEY2);
    clickNext();
    expect(screen.getByText('Signature Threshold')).toBeInTheDocument();
    expect(screen.getByText('1 of 2')).toBeInTheDocument();
    // Threshold of 1 with multiple signers warns
    expect(
      screen.getByText('⚡ Threshold of 1 with multiple signers - any single signer can approve')
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /100% \(unanimous\)/i }));
    expect(
      screen.getByText('⚡ Threshold equals total weight - all signers must approve')
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /67% \(supermajority\)/i }));
    expect(screen.getByText('2 of 2')).toBeInTheDocument();
    expect(screen.getByText('Signers Needed:')).toBeInTheDocument();
  });

  it('supports advanced time lock presets and warnings', () => {
    render(<MultiSigWizard />);
    goToSigners();
    addValidSigner('Alice');
    addValidSigner('Bob', VALID_KEY2);
    clickNext();
    fireEvent.click(screen.getByRole('button', { name: /next/i }));
    expect(screen.getByText('Time Lock (seconds)')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /1 hour/i }));
    fireEvent.click(screen.getByRole('button', { name: /1 week/i }));
    expect((screen.getByLabelText('Time Lock (seconds)') as HTMLInputElement).value).toBe('604800');
    // Over 30 days -> warning
    fireEvent.change(screen.getByLabelText('Time Lock (seconds)'), {
      target: { value: String(86400 * 31) },
    });
    expect(screen.getByText('⚡ Time lock is very long (over 30 days)')).toBeInTheDocument();
    // Negative -> error
    fireEvent.change(screen.getByLabelText('Time Lock (seconds)'), {
      target: { value: '-5' },
    });
    expect(screen.getByText('⚠️ Time lock cannot be negative')).toBeInTheDocument();
  });

  it('reaches the preview step and shows the configuration summary', () => {
    const consoleSpy = jest.spyOn(console, 'log').mockImplementation(() => {});
    render(<MultiSigWizard />);
    goToSigners();
    addValidSigner('Alice');
    addValidSigner('Bob', VALID_KEY2);
    clickNext();
    fireEvent.click(screen.getByRole('button', { name: /100% \(unanimous\)/i }));
    clickNext();
    clickNext();

    expect(screen.getByText('🎉 Ready to Create')).toBeInTheDocument();
    expect(screen.getByText('Test Wallet')).toBeInTheDocument();
    expect(screen.getAllByText(/testnet/i).length).toBeGreaterThan(0);
    expect(screen.getByText('None')).toBeInTheDocument(); // time lock
    expect(screen.getByText('Alice')).toBeInTheDocument();

    const createButton = screen.getByRole('button', { name: /create wallet/i });
    expect(createButton).toBeEnabled();
    fireEvent.click(createButton);
    expect(consoleSpy).toHaveBeenCalledWith(
      'Creating multi-sig wallet:',
      expect.objectContaining({ name: 'Test Wallet', signers: expect.any(Array) })
    );
    consoleSpy.mockRestore();
  });

  it('navigates backwards through steps', () => {
    render(<MultiSigWizard />);
    goToSigners();
    fireEvent.click(screen.getByRole('button', { name: /previous/i }));
    expect(
      screen.getByText('Provide basic information about your multi-signature wallet')
    ).toBeInTheDocument();
  });

  it('allows clicking completed steps and blocks future steps while invalid', () => {
    render(<MultiSigWizard />);
    // While the form is invalid, future step tabs are disabled
    expect(screen.getByRole('button', { name: /configure signers/i })).toBeDisabled();
    goToSigners();
    // Past step is clickable
    fireEvent.click(screen.getByRole('button', { name: /basic information/i }));
    expect(
      screen.getByText('Provide basic information about your multi-signature wallet')
    ).toBeInTheDocument();
  });
});
