import React from 'react';
import { render, screen, fireEvent, act } from '@testing-library/react';
import AuthContainer from '../components/auth/AuthContainer';
import {
  login,
  signUp,
  resetPassword,
  sendMfaCode,
  getPendingMfaCode,
} from '../lib/auth/authService';
import { persistSession } from '../lib/auth/session';

jest.mock('../lib/auth/authService', () => ({
  login: jest.fn(),
  signUp: jest.fn(),
  resetPassword: jest.fn(),
  sendMfaCode: jest.fn(),
  resendMfaCode: jest.fn(),
  verifyMfa: jest.fn(),
  getPendingMfaCode: jest.fn(),
}));

jest.mock('../lib/auth/session', () => ({
  persistSession: jest.fn(),
}));

const loginMock = login as jest.Mock;
const signUpMock = signUp as jest.Mock;
const resetPasswordMock = resetPassword as jest.Mock;
const sendMfaCodeMock = sendMfaCode as jest.Mock;
const getPendingMfaCodeMock = getPendingMfaCode as jest.Mock;
const persistSessionMock = persistSession as jest.Mock;

const fillLogin = () => {
  fireEvent.change(screen.getByLabelText(/email address/i), {
    target: { value: 'user@example.com' },
  });
  fireEvent.change(screen.getByLabelText(/password/i), {
    target: { value: 'password123' },
  });
};

const submitForm = () => {
  const form = document.querySelector('form')!;
  fireEvent.submit(form);
};

describe('AuthContainer', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    loginMock.mockResolvedValue({ requiresMfa: false, user: { email: 'user@example.com' } });
    signUpMock.mockResolvedValue({});
    resetPasswordMock.mockResolvedValue({});
    sendMfaCodeMock.mockResolvedValue({});
    getPendingMfaCodeMock.mockReturnValue('123456');
  });

  it('renders the login form by default', () => {
    render(<AuthContainer />);
    expect(screen.getByText('Welcome Back')).toBeInTheDocument();
  });

  it('logs in successfully without MFA and calls onAuthSuccess', async () => {
    const onAuthSuccess = jest.fn();
    render(<AuthContainer onAuthSuccess={onAuthSuccess} />);
    fillLogin();
    fireEvent.click(screen.getByRole('checkbox', { name: /remember me/i }));
    submitForm();
    expect(await screen.findByText('Login successful!')).toBeInTheDocument();
    expect(loginMock).toHaveBeenCalledWith({
      email: 'user@example.com',
      password: 'password123',
      rememberMe: true,
    });
    expect(persistSessionMock).toHaveBeenCalled();
    expect(onAuthSuccess).toHaveBeenCalledWith({ email: 'user@example.com' });
  });

  it('switches to MFA when login requires it', async () => {
    loginMock.mockResolvedValue({
      requiresMfa: true,
      user: { email: 'user@example.com' },
    });
    render(<AuthContainer />);
    fillLogin();
    submitForm();
    expect(await screen.findByText('Two-Factor Authentication')).toBeInTheDocument();
    expect(screen.getByText('123456')).toBeInTheDocument();
    expect(sendMfaCodeMock).toHaveBeenCalledWith('totp');
  });

  it('shows the login error when credentials are rejected', async () => {
    loginMock.mockRejectedValue(new Error('Invalid credentials'));
    render(<AuthContainer />);
    fillLogin();
    submitForm();
    expect(await screen.findByText('Invalid credentials')).toBeInTheDocument();
  });

  it('switches to signup and creates an account', async () => {
    render(<AuthContainer />);
    fireEvent.click(screen.getByRole('button', { name: /sign up/i }));
    expect((await screen.findAllByText('Create Account')).length).toBeGreaterThan(0);
    fireEvent.change(screen.getByLabelText(/first name/i), { target: { value: 'Ada' } });
    fireEvent.change(screen.getByLabelText(/last name/i), { target: { value: 'Lovelace' } });
    fireEvent.change(screen.getByLabelText(/email address/i), {
      target: { value: 'ada@example.com' },
    });
    fireEvent.change(screen.getByLabelText(/^password/i), { target: { value: 'Password123!' } });
    fireEvent.change(screen.getByLabelText(/confirm password/i), {
      target: { value: 'Password123!' },
    });
    fireEvent.click(screen.getByRole('checkbox', { name: /terms/i }));
    submitForm();
    expect(await screen.findByText(/Account created successfully/i)).toBeInTheDocument();
    expect(signUpMock).toHaveBeenCalledWith(expect.objectContaining({ email: 'ada@example.com' }));
  });

  it('navigates to the reset password view and sends a reset link', async () => {
    render(<AuthContainer />);
    fireEvent.click(screen.getByRole('button', { name: /forgot password/i }));
    expect(await screen.findByText('Reset Password')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText(/email address/i), {
      target: { value: 'ada@example.com' },
    });
    submitForm();
    expect(await screen.findByText('Password reset link sent successfully!')).toBeInTheDocument();
    expect(resetPasswordMock).toHaveBeenCalledWith('ada@example.com');
  });

  it('completes MFA verification with the demo code', async () => {
    const onAuthSuccess = jest.fn();
    const { verifyMfa } = require('../lib/auth/authService');
    verifyMfa.mockResolvedValue({});
    loginMock.mockResolvedValue({
      requiresMfa: true,
      user: { email: 'user@example.com' },
    });
    render(<AuthContainer onAuthSuccess={onAuthSuccess} initialView="mfa" />);
    expect(screen.getByText('Two-Factor Authentication')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText(/verification code/i), {
      target: { value: '123456' },
    });
    submitForm();
    expect(await screen.findByText('Authentication successful!')).toBeInTheDocument();
    expect(verifyMfa).toHaveBeenCalledWith('123456', 'totp');
    expect(onAuthSuccess).toHaveBeenCalledWith({ email: '', verified: true });
  });

  it('shows the terms of service footer', () => {
    render(<AuthContainer />);
    expect(screen.getByText(/Terms of Service/i)).toBeInTheDocument();
    expect(screen.getByText(/Privacy Policy/i)).toBeInTheDocument();
  });
});
