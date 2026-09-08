import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import SimpleAuth from '../components/auth/SimpleAuth';

describe('SimpleAuth', () => {
  const onLogin = jest.fn().mockResolvedValue(undefined);
  const onSignUp = jest.fn().mockResolvedValue(undefined);
  const onResetPassword = jest.fn().mockResolvedValue(undefined);

  beforeEach(() => {
    jest.clearAllMocks();
  });

  const renderAuth = (props = {}) =>
    render(
      <SimpleAuth
        onLogin={onLogin}
        onSignUp={onSignUp}
        onResetPassword={onResetPassword}
        {...props}
      />
    );

  /** Submit the currently visible form (found via the given submit button). */
  const submit = (buttonName: RegExp) => {
    const button = screen.getByRole('button', { name: buttonName });
    const form = button.closest('form');
    expect(form).not.toBeNull();
    fireEvent.submit(form!);
  };

  it('renders the login view by default', () => {
    renderAuth();
    expect(screen.getByText('Welcome Back')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /sign in/i })).toBeInTheDocument();
  });

  it('validates the login form', () => {
    renderAuth();
    submit(/sign in/i);
    expect(screen.getByText('Email is required')).toBeInTheDocument();
    expect(screen.getByText('Password is required')).toBeInTheDocument();
    expect(onLogin).not.toHaveBeenCalled();
  });

  it('rejects invalid email format', () => {
    renderAuth();
    fireEvent.change(screen.getByPlaceholderText('Enter your email'), {
      target: { value: 'not-an-email' },
    });
    fireEvent.change(screen.getByPlaceholderText('Enter your password'), {
      target: { value: 'password123' },
    });
    submit(/sign in/i);
    expect(screen.getByText('Please enter a valid email address')).toBeInTheDocument();
    expect(onLogin).not.toHaveBeenCalled();
  });

  it('rejects short passwords', () => {
    renderAuth();
    fireEvent.change(screen.getByPlaceholderText('Enter your email'), {
      target: { value: 'a@b.com' },
    });
    fireEvent.change(screen.getByPlaceholderText('Enter your password'), {
      target: { value: 'short' },
    });
    submit(/sign in/i);
    expect(screen.getByText('Password must be at least 8 characters')).toBeInTheDocument();
  });

  it('logs in successfully', async () => {
    renderAuth();
    fireEvent.change(screen.getByPlaceholderText('Enter your email'), {
      target: { value: 'a@b.com' },
    });
    fireEvent.change(screen.getByPlaceholderText('Enter your password'), {
      target: { value: 'password123' },
    });
    submit(/sign in/i);
    expect(await screen.findByText('Login successful!')).toBeInTheDocument();
    expect(onLogin).toHaveBeenCalledWith(
      expect.objectContaining({ email: 'a@b.com', password: 'password123', rememberMe: false })
    );
  });

  it('shows an error message when login fails', async () => {
    onLogin.mockRejectedValueOnce(new Error('Invalid credentials'));
    renderAuth();
    fireEvent.change(screen.getByPlaceholderText('Enter your email'), {
      target: { value: 'a@b.com' },
    });
    fireEvent.change(screen.getByPlaceholderText('Enter your password'), {
      target: { value: 'password123' },
    });
    submit(/sign in/i);
    expect(await screen.findByText('Invalid credentials')).toBeInTheDocument();
  });

  it('toggles remember me', async () => {
    renderAuth();
    fireEvent.click(screen.getByLabelText('Remember me'));
    fireEvent.change(screen.getByPlaceholderText('Enter your email'), {
      target: { value: 'a@b.com' },
    });
    fireEvent.change(screen.getByPlaceholderText('Enter your password'), {
      target: { value: 'password123' },
    });
    submit(/sign in/i);
    expect(onLogin).toHaveBeenCalledWith(
      expect.objectContaining({ email: 'a@b.com', password: 'password123', rememberMe: true })
    );
  });

  it('validates and submits the signup form', async () => {
    renderAuth();
    fireEvent.click(screen.getByRole('button', { name: /sign up/i }));
    expect(screen.getByRole('heading', { name: 'Create Account' })).toBeInTheDocument();

    // Validation errors on empty submit
    submit(/create account/i);
    expect(screen.getByText('First name is required')).toBeInTheDocument();
    expect(screen.getByText('You must agree to the terms and conditions')).toBeInTheDocument();

    // Fill the form
    fireEvent.change(screen.getByPlaceholderText('First name'), { target: { value: 'Ada' } });
    fireEvent.change(screen.getByPlaceholderText('Last name'), { target: { value: 'Lovelace' } });
    fireEvent.change(screen.getByPlaceholderText('Enter your email'), {
      target: { value: 'ada@example.com' },
    });
    fireEvent.change(screen.getByPlaceholderText('Create a password'), {
      target: { value: 'password123' },
    });
    fireEvent.change(screen.getByPlaceholderText('Confirm your password'), {
      target: { value: 'password123' },
    });
    fireEvent.click(screen.getByLabelText(/I agree to the Terms/));

    submit(/create account/i);
    expect(await screen.findByText('Account created successfully!')).toBeInTheDocument();
    expect(onSignUp).toHaveBeenCalledWith(
      expect.objectContaining({
        firstName: 'Ada',
        lastName: 'Lovelace',
        email: 'ada@example.com',
        password: 'password123',
      })
    );
  });

  it('rejects mismatched passwords on signup', () => {
    renderAuth();
    fireEvent.click(screen.getByRole('button', { name: /sign up/i }));
    fireEvent.change(screen.getByPlaceholderText('First name'), { target: { value: 'Ada' } });
    fireEvent.change(screen.getByPlaceholderText('Last name'), { target: { value: 'Lovelace' } });
    fireEvent.change(screen.getByPlaceholderText('Enter your email'), {
      target: { value: 'ada@example.com' },
    });
    fireEvent.change(screen.getByPlaceholderText('Create a password'), {
      target: { value: 'password123' },
    });
    fireEvent.change(screen.getByPlaceholderText('Confirm your password'), {
      target: { value: 'different' },
    });
    fireEvent.click(screen.getByLabelText(/I agree to the Terms/));
    submit(/create account/i);
    expect(screen.getByText('Passwords do not match')).toBeInTheDocument();
    expect(onSignUp).not.toHaveBeenCalled();
  });

  it('resets a password via the reset view', async () => {
    renderAuth();
    fireEvent.click(screen.getByRole('button', { name: /forgot password/i }));
    expect(screen.getByText('Reset Password')).toBeInTheDocument();

    submit(/send reset link/i);
    expect(screen.getByText('Email is required')).toBeInTheDocument();

    fireEvent.change(screen.getByPlaceholderText('Enter your email'), {
      target: { value: 'ada@example.com' },
    });
    submit(/send reset link/i);
    expect(await screen.findByText('Password reset link sent!')).toBeInTheDocument();
    expect(onResetPassword).toHaveBeenCalledWith('ada@example.com');
  });

  it('switches views and clears the form', () => {
    renderAuth();
    fireEvent.change(screen.getByPlaceholderText('Enter your email'), {
      target: { value: 'a@b.com' },
    });
    fireEvent.click(screen.getByRole('button', { name: /sign up/i }));
    expect(screen.getByPlaceholderText('First name')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /sign in/i }));
    expect(screen.getByPlaceholderText('Enter your email')).toHaveValue('');
  });
});
