import React from 'react';
import { render, screen, fireEvent, act } from '@testing-library/react';
import MultiFactorAuth from '../components/auth/MultiFactorAuth';

const submitForm = () => {
  fireEvent.submit(document.querySelector('form')!);
};

describe('MultiFactorAuth', () => {
  it('renders the TOTP method by default', () => {
    render(<MultiFactorAuth onVerify={jest.fn()} onBack={jest.fn()} onResendCode={jest.fn()} />);
    expect(screen.getByText('Two-Factor Authentication')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('000000')).toBeInTheDocument();
    expect(
      screen.getByText(/Enter the 6-digit code from your authenticator app/)
    ).toBeInTheDocument();
  });

  it('shows the demo one-time code when provided', () => {
    render(
      <MultiFactorAuth
        onVerify={jest.fn()}
        onBack={jest.fn()}
        onResendCode={jest.fn()}
        demoCode="654321"
      />
    );
    expect(screen.getByText('654321')).toBeInTheDocument();
  });

  it('submits a valid code with the selected method', async () => {
    const onVerify = jest.fn().mockResolvedValue(undefined);
    render(<MultiFactorAuth onVerify={onVerify} onBack={jest.fn()} onResendCode={jest.fn()} />);
    fireEvent.change(screen.getByLabelText(/verification code/i), {
      target: { value: '123456' },
    });
    submitForm();
    expect(onVerify).toHaveBeenCalledWith('123456', 'totp');
  });

  it('rejects an empty or malformed code', () => {
    render(<MultiFactorAuth onVerify={jest.fn()} onBack={jest.fn()} onResendCode={jest.fn()} />);
    fireEvent.blur(screen.getByLabelText(/verification code/i));
    submitForm();
    expect(screen.getByText('Verification code is required')).toBeInTheDocument();
  });

  it('rejects a non-6-digit TOTP code', () => {
    render(<MultiFactorAuth onVerify={jest.fn()} onBack={jest.fn()} onResendCode={jest.fn()} />);
    const input = screen.getByLabelText(/verification code/i);
    fireEvent.change(input, { target: { value: '12345' } });
    fireEvent.blur(input);
    submitForm();
    expect(screen.getByText('Please enter a valid 6-digit code')).toBeInTheDocument();
  });

  it('filters non-digit characters from the code', () => {
    render(<MultiFactorAuth onVerify={jest.fn()} onBack={jest.fn()} onResendCode={jest.fn()} />);
    const input = screen.getByLabelText(/verification code/i) as HTMLInputElement;
    fireEvent.change(input, { target: { value: '12a34b56' } });
    expect(input.value).toBe('123456');
  });

  it('switches to the SMS method', () => {
    render(
      <MultiFactorAuth
        onVerify={jest.fn()}
        onBack={jest.fn()}
        onResendCode={jest.fn()}
        userPhone="+1234567890"
      />
    );
    fireEvent.click(screen.getByRole('button', { name: /sms/i }));
    expect(screen.getByPlaceholderText('0000')).toBeInTheDocument();
    expect(screen.getByText(/Enter the code sent to \+1234567890/)).toBeInTheDocument();
  });

  it('switches to the email method', () => {
    render(
      <MultiFactorAuth
        onVerify={jest.fn()}
        onBack={jest.fn()}
        onResendCode={jest.fn()}
        userEmail="user@example.com"
      />
    );
    fireEvent.click(screen.getByRole('button', { name: /email/i }));
    expect(screen.getByText(/Enter the code sent to user@example.com/)).toBeInTheDocument();
  });

  it('resends the code and starts a cooldown', async () => {
    jest.useFakeTimers();
    const onResendCode = jest.fn().mockResolvedValue(undefined);
    render(<MultiFactorAuth onVerify={jest.fn()} onBack={jest.fn()} onResendCode={onResendCode} />);
    fireEvent.click(screen.getByRole('button', { name: /resend/i }));
    await act(async () => {
      await Promise.resolve();
    });
    expect(onResendCode).toHaveBeenCalledWith('totp');
    expect(screen.getByText(/Resend code in 60s/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /resend/i })).toBeDisabled();
    jest.useRealTimers();
  });

  it('calls onBack when the back button is clicked', () => {
    const onBack = jest.fn();
    render(<MultiFactorAuth onVerify={jest.fn()} onBack={onBack} onResendCode={jest.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: /back/i }));
    expect(onBack).toHaveBeenCalled();
  });

  it('renders the trust device option', () => {
    render(<MultiFactorAuth onVerify={jest.fn()} onBack={jest.fn()} onResendCode={jest.fn()} />);
    expect(screen.getByText(/Trust this device for 30 days/)).toBeInTheDocument();
  });
});
