import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import LoginForm from '../components/auth/LoginForm';

const submitForm = () => {
  fireEvent.submit(document.querySelector('form')!);
};

describe('LoginForm', () => {
  it('submits valid credentials', async () => {
    const onLogin = jest.fn().mockResolvedValue(undefined);
    render(<LoginForm onLogin={onLogin} onForgotPassword={jest.fn()} onSignUp={jest.fn()} />);
    fireEvent.change(screen.getByLabelText(/email address/i), {
      target: { value: 'user@example.com' },
    });
    fireEvent.change(screen.getByLabelText(/password/i), {
      target: { value: 'password123' },
    });
    fireEvent.click(screen.getByRole('checkbox', { name: /remember me/i }));
    submitForm();
    expect(onLogin).toHaveBeenCalledWith({
      email: 'user@example.com',
      password: 'password123',
      rememberMe: true,
    });
  });

  it('rejects an empty form', () => {
    render(<LoginForm onLogin={jest.fn()} onForgotPassword={jest.fn()} onSignUp={jest.fn()} />);
    // blur first so the fields are marked as touched (errors render only for touched fields)
    fireEvent.blur(screen.getByLabelText(/email address/i));
    fireEvent.blur(screen.getByLabelText(/password/i));
    submitForm();
    expect(screen.getByText('Email is required')).toBeInTheDocument();
    expect(screen.getByText('Password is required')).toBeInTheDocument();
  });

  it('rejects an invalid email and short password', () => {
    render(<LoginForm onLogin={jest.fn()} onForgotPassword={jest.fn()} onSignUp={jest.fn()} />);
    const email = screen.getByLabelText(/email address/i);
    const password = screen.getByLabelText(/password/i);
    fireEvent.change(email, { target: { value: 'not-an-email' } });
    fireEvent.change(password, { target: { value: 'short' } });
    fireEvent.blur(email);
    fireEvent.blur(password);
    submitForm();
    expect(screen.getByText('Please enter a valid email address')).toBeInTheDocument();
    expect(screen.getByText('Password must be at least 8 characters')).toBeInTheDocument();
  });

  it('validates the email on blur', () => {
    render(<LoginForm onLogin={jest.fn()} onForgotPassword={jest.fn()} onSignUp={jest.fn()} />);
    const email = screen.getByLabelText(/email address/i);
    fireEvent.change(email, { target: { value: 'bad-email' } });
    fireEvent.blur(email);
    expect(screen.getByText('Please enter a valid email address')).toBeInTheDocument();
  });

  it('validates the password on blur', () => {
    render(<LoginForm onLogin={jest.fn()} onForgotPassword={jest.fn()} onSignUp={jest.fn()} />);
    const password = screen.getByLabelText(/password/i);
    fireEvent.change(password, { target: { value: 'tiny' } });
    fireEvent.blur(password);
    expect(screen.getByText('Password must be at least 8 characters')).toBeInTheDocument();
  });

  it('toggles password visibility', () => {
    render(<LoginForm onLogin={jest.fn()} onForgotPassword={jest.fn()} onSignUp={jest.fn()} />);
    const password = screen.getByLabelText(/password/i);
    expect(password).toHaveAttribute('type', 'password');
    fireEvent.click(screen.getByRole('button', { name: '' }));
    expect(password).toHaveAttribute('type', 'text');
  });

  it('calls the forgot password and sign up handlers', () => {
    const onForgotPassword = jest.fn();
    const onSignUp = jest.fn();
    render(
      <LoginForm onLogin={jest.fn()} onForgotPassword={onForgotPassword} onSignUp={onSignUp} />
    );
    fireEvent.click(screen.getByRole('button', { name: /forgot password/i }));
    expect(onForgotPassword).toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: /sign up/i }));
    expect(onSignUp).toHaveBeenCalled();
  });

  it('shows the loading state and disables inputs', () => {
    render(
      <LoginForm onLogin={jest.fn()} onForgotPassword={jest.fn()} onSignUp={jest.fn()} isLoading />
    );
    expect(screen.getByText('Signing in...')).toBeInTheDocument();
    expect(screen.getByLabelText(/email address/i)).toBeDisabled();
    expect(screen.getByLabelText(/password/i)).toBeDisabled();
    expect(screen.getByText('Signing in...')).toBeInTheDocument();
  });

  it('shows a green check on valid touched fields', () => {
    render(<LoginForm onLogin={jest.fn()} onForgotPassword={jest.fn()} onSignUp={jest.fn()} />);
    const email = screen.getByLabelText(/email address/i);
    fireEvent.change(email, { target: { value: 'good@example.com' } });
    fireEvent.blur(email);
    // touched + valid + non-empty -> success icon container renders
    const emailRow = email.closest('div.relative')!;
    expect(emailRow.querySelector('svg.text-green-500')).toBeInTheDocument();
  });
});
