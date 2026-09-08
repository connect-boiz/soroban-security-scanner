import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import SignUpForm from '../components/auth/SignUpForm';

describe('SignUpForm', () => {
  const onSignUp = jest.fn().mockResolvedValue(undefined);
  const onSignIn = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
  });

  const renderForm = (props = {}) =>
    render(<SignUpForm onSignUp={onSignUp} onSignIn={onSignIn} {...props} />);

  it('renders the form', () => {
    renderForm();
    expect(screen.getByRole('heading', { name: 'Create Account' })).toBeInTheDocument();
    expect(screen.getByLabelText('First Name')).toBeInTheDocument();
  });

  it('shows validation errors on empty submit', async () => {
    renderForm();
    ['First Name', 'Last Name', 'Email Address', 'Password', 'Confirm Password'].forEach(field => {
      fireEvent.blur(screen.getByLabelText(field));
    });
    fireEvent.click(screen.getByRole('button', { name: /create account/i }));
    expect(await screen.findByText('First name is required')).toBeInTheDocument();
    expect(screen.getByText('Last name is required')).toBeInTheDocument();
    expect(screen.getByText('Email is required')).toBeInTheDocument();
    expect(screen.getByText('Password is required')).toBeInTheDocument();
    expect(screen.getByText('Please confirm your password')).toBeInTheDocument();
    expect(screen.getByText('You must agree to the terms and conditions')).toBeInTheDocument();
    expect(onSignUp).not.toHaveBeenCalled();
  });

  it('validates short names and invalid email on blur', () => {
    renderForm();
    const firstName = screen.getByLabelText('First Name');
    fireEvent.change(firstName, { target: { value: 'A' } });
    fireEvent.blur(firstName);
    expect(screen.getByText('First name must be at least 2 characters')).toBeInTheDocument();

    const email = screen.getByLabelText('Email Address');
    fireEvent.change(email, { target: { value: 'bad' } });
    fireEvent.blur(email);
    expect(screen.getByText('Please enter a valid email address')).toBeInTheDocument();
  });

  it('shows password strength feedback', () => {
    renderForm();
    const password = screen.getByLabelText('Password');
    fireEvent.change(password, { target: { value: 'short' } });
    expect(screen.getByText('Weak')).toBeInTheDocument();
    fireEvent.change(password, { target: { value: 'MediumPass1' } });
    expect(screen.getByText('Medium')).toBeInTheDocument();
    fireEvent.change(password, { target: { value: 'Str0ng!Passw0rd' } });
    expect(screen.getByText('Strong')).toBeInTheDocument();
    fireEvent.change(password, { target: { value: '' } });
    expect(screen.queryByText('Password strength')).not.toBeInTheDocument();
  });

  it('toggles password visibility', () => {
    renderForm();
    const password = screen.getByLabelText('Password') as HTMLInputElement;
    fireEvent.change(password, { target: { value: 'secret123' } });
    expect(password.type).toBe('password');
    const toggleButtons = screen.getAllByRole('button');
    fireEvent.click(toggleButtons.find(b => b.closest('div')?.className.includes('relative'))!);
    expect((screen.getByLabelText('Password') as HTMLInputElement).type).toBe('text');
  });

  it('rejects a weak password on submit', async () => {
    renderForm();
    fireEvent.change(screen.getByLabelText('First Name'), { target: { value: 'Ada' } });
    fireEvent.change(screen.getByLabelText('Last Name'), { target: { value: 'Lovelace' } });
    fireEvent.change(screen.getByLabelText('Email Address'), {
      target: { value: 'ada@example.com' },
    });
    fireEvent.change(screen.getByLabelText('Password'), { target: { value: 'weakpass' } });
    fireEvent.blur(screen.getByLabelText('Password'));
    fireEvent.change(screen.getByLabelText('Confirm Password'), { target: { value: 'weakpass' } });
    fireEvent.click(screen.getByLabelText(/I agree to the/));
    fireEvent.click(screen.getByRole('button', { name: /create account/i }));
    expect(await screen.findByText(/Password is too weak/)).toBeInTheDocument();
    expect(onSignUp).not.toHaveBeenCalled();
  });

  it('submits valid data without confirmPassword/agreeToTerms', async () => {
    renderForm();
    fireEvent.change(screen.getByLabelText('First Name'), { target: { value: 'Ada' } });
    fireEvent.change(screen.getByLabelText('Last Name'), { target: { value: 'Lovelace' } });
    fireEvent.change(screen.getByLabelText('Email Address'), {
      target: { value: 'ada@example.com' },
    });
    fireEvent.change(screen.getByLabelText('Password'), { target: { value: 'Str0ng!Pass' } });
    fireEvent.change(screen.getByLabelText('Confirm Password'), {
      target: { value: 'Str0ng!Pass' },
    });
    fireEvent.click(screen.getByLabelText(/I agree to the/));
    fireEvent.click(screen.getByRole('button', { name: /create account/i }));
    await waitFor(() => {
      expect(onSignUp).toHaveBeenCalledWith({
        firstName: 'Ada',
        lastName: 'Lovelace',
        email: 'ada@example.com',
        password: 'Str0ng!Pass',
      });
    });
  });

  it('clears field errors as the user types', () => {
    renderForm();
    const firstName = screen.getByLabelText('First Name');
    fireEvent.blur(firstName);
    fireEvent.change(firstName, { target: { value: 'A' } });
    fireEvent.blur(firstName);
    expect(screen.getByText('First name must be at least 2 characters')).toBeInTheDocument();
    fireEvent.change(firstName, { target: { value: 'Ada' } });
    expect(screen.queryByText('First name must be at least 2 characters')).not.toBeInTheDocument();
  });

  it('calls onSignIn from the sign-in link', () => {
    renderForm();
    fireEvent.click(screen.getByRole('button', { name: /sign in/i }));
    expect(onSignIn).toHaveBeenCalled();
  });

  it('shows a loading spinner while submitting', () => {
    renderForm({ isLoading: true });
    expect(screen.getByText('Creating account...')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /creating account/i })).toBeDisabled();
  });
});
