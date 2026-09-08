import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import PasswordResetForm from '../components/auth/PasswordResetForm';

const submitForm = () => {
  fireEvent.submit(document.querySelector('form')!);
};

describe('PasswordResetForm', () => {
  it('submits a valid email and shows the sent confirmation', async () => {
    const onResetPassword = jest.fn().mockResolvedValue(undefined);
    render(<PasswordResetForm onResetPassword={onResetPassword} onBackToLogin={jest.fn()} />);
    fireEvent.change(screen.getByLabelText(/email address/i), {
      target: { value: 'user@example.com' },
    });
    submitForm();
    expect(onResetPassword).toHaveBeenCalledWith('user@example.com');
    expect(await screen.findByText('Reset Link Sent')).toBeInTheDocument();
    expect(screen.getByText(/We've sent a password reset link/i)).toBeInTheDocument();
  });

  it('rejects an empty form', () => {
    render(<PasswordResetForm onResetPassword={jest.fn()} onBackToLogin={jest.fn()} />);
    fireEvent.blur(screen.getByLabelText(/email address/i));
    submitForm();
    expect(screen.getByText('Email is required')).toBeInTheDocument();
  });

  it('rejects an invalid email', () => {
    render(<PasswordResetForm onResetPassword={jest.fn()} onBackToLogin={jest.fn()} />);
    const email = screen.getByLabelText(/email address/i);
    fireEvent.change(email, { target: { value: 'nope' } });
    fireEvent.blur(email);
    expect(screen.getByText('Please enter a valid email address')).toBeInTheDocument();
  });

  it('returns to the form via Try another email', async () => {
    const onResetPassword = jest.fn().mockResolvedValue(undefined);
    render(<PasswordResetForm onResetPassword={onResetPassword} onBackToLogin={jest.fn()} />);
    fireEvent.change(screen.getByLabelText(/email address/i), {
      target: { value: 'user@example.com' },
    });
    submitForm();
    expect(await screen.findByText('Reset Link Sent')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /try another email/i }));
    expect(screen.getByText('Reset Password')).toBeInTheDocument();
  });

  it('calls onBackToLogin from both back buttons', () => {
    const onBackToLogin = jest.fn();
    render(<PasswordResetForm onResetPassword={jest.fn()} onBackToLogin={onBackToLogin} />);
    fireEvent.click(screen.getByRole('button', { name: /back to login/i }));
    expect(onBackToLogin).toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: /sign in/i }));
    expect(onBackToLogin).toHaveBeenCalledTimes(2);
  });

  it('shows the loading state', () => {
    render(<PasswordResetForm onResetPassword={jest.fn()} onBackToLogin={jest.fn()} isLoading />);
    expect(screen.getByText('Sending reset link...')).toBeInTheDocument();
    expect(screen.getByLabelText(/email address/i)).toBeDisabled();
  });
});
