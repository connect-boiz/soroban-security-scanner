'use client';

import React, { useState } from 'react';
import LoginForm from './LoginForm';
import SignUpForm from './SignUpForm';
import PasswordResetForm from './PasswordResetForm';
import MultiFactorAuth from './MultiFactorAuth';
import { AlertCircle, CheckCircle } from 'lucide-react';
import { persistSession } from '../../lib/auth/session';
import {
  login,
  signUp,
  resetPassword,
  sendMfaCode,
  resendMfaCode,
  verifyMfa,
  getPendingMfaCode,
} from '../../lib/auth/authService';

type AuthView = 'login' | 'signup' | 'reset-password' | 'mfa';

interface AuthContainerProps {
  onAuthSuccess?: (user: any) => void;
  initialView?: AuthView;
}

export default function AuthContainer({
  onAuthSuccess,
  initialView = 'login',
}: AuthContainerProps) {
  const [currentView, setCurrentView] = useState<AuthView>(initialView);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [userEmail, setUserEmail] = useState('');
  const [userPhone, setUserPhone] = useState('');
  const [rememberMe, setRememberMe] = useState(false);
  // The one-time code the mock MFA flow "sent", shown only in demo mode.
  const [demoMfaCode, setDemoMfaCode] = useState<string | null>(null);

  const clearMessages = () => {
    setError(null);
    setSuccess(null);
  };

  const handleError = (message: string) => {
    setError(message);
    setSuccess(null);
  };

  const handleSuccess = (message: string) => {
    setSuccess(message);
    setError(null);
  };

  const handleLogin = async (credentials: {
    email: string;
    password: string;
    rememberMe: boolean;
  }) => {
    clearMessages();
    setIsLoading(true);
    setRememberMe(credentials.rememberMe);

    try {
      const result = await login(credentials);

      if (result.requiresMfa) {
        setUserEmail(result.user.email);
        // Simulate the backend sending a one-time code, then surface it
        // so the demo flow can be completed.
        await sendMfaCode('totp');
        setDemoMfaCode(getPendingMfaCode());
        setCurrentView('mfa');
        handleSuccess('Login successful! Please complete two-factor authentication.');
      } else {
        await persistSession(result.user, credentials.rememberMe);
        handleSuccess('Login successful!');
        onAuthSuccess?.(result.user);
      }
    } catch (err) {
      handleError(err instanceof Error ? err.message : 'Login failed');
    } finally {
      setIsLoading(false);
    }
  };

  const handleSignUp = async (userData: {
    firstName: string;
    lastName: string;
    email: string;
    password: string;
  }) => {
    clearMessages();
    setIsLoading(true);

    try {
      await signUp(userData);
      handleSuccess(
        'Account created successfully! Please check your email to verify your account.'
      );
      // Optionally redirect to login or show verification screen
      setTimeout(() => setCurrentView('login'), 2000);
    } catch (err) {
      handleError(err instanceof Error ? err.message : 'Sign up failed');
    } finally {
      setIsLoading(false);
    }
  };

  const handleResetPassword = async (email: string) => {
    clearMessages();
    setIsLoading(true);

    try {
      await resetPassword(email);
      handleSuccess('Password reset link sent successfully!');
    } catch (err) {
      handleError(err instanceof Error ? err.message : 'Password reset failed');
    } finally {
      setIsLoading(false);
    }
  };

  const handleMfaVerify = async (code: string, method: string) => {
    clearMessages();
    setIsLoading(true);

    try {
      await verifyMfa(code, method);
      const user = { email: userEmail, verified: true };
      await persistSession(user, rememberMe);
      setDemoMfaCode(null);
      handleSuccess('Authentication successful!');
      onAuthSuccess?.(user);
    } catch (err) {
      handleError(err instanceof Error ? err.message : 'Verification failed');
    } finally {
      setIsLoading(false);
    }
  };

  const handleResendCode = async (method: string) => {
    clearMessages();
    setIsLoading(true);

    try {
      await resendMfaCode(method);
      setDemoMfaCode(getPendingMfaCode());
      handleSuccess(`New code sent via ${method}`);
    } catch (err) {
      handleError(err instanceof Error ? err.message : 'Failed to resend code');
    } finally {
      setIsLoading(false);
    }
  };

  const renderCurrentView = () => {
    switch (currentView) {
      case 'login':
        return (
          <LoginForm
            onLogin={handleLogin}
            onForgotPassword={() => setCurrentView('reset-password')}
            onSignUp={() => setCurrentView('signup')}
            isLoading={isLoading}
          />
        );

      case 'signup':
        return (
          <SignUpForm
            onSignUp={handleSignUp}
            onSignIn={() => setCurrentView('login')}
            isLoading={isLoading}
          />
        );

      case 'reset-password':
        return (
          <PasswordResetForm
            onResetPassword={handleResetPassword}
            onBackToLogin={() => setCurrentView('login')}
            isLoading={isLoading}
          />
        );

      case 'mfa':
        return (
          <MultiFactorAuth
            onVerify={handleMfaVerify}
            onBack={() => setCurrentView('login')}
            onResendCode={handleResendCode}
            userEmail={userEmail}
            userPhone={userPhone}
            demoCode={demoMfaCode}
            isLoading={isLoading}
          />
        );

      default:
        return null;
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100 flex items-center justify-center p-4">
      <div className="w-full max-w-md">
        {/* Error/Success Messages */}
        {error && (
          <div className="mb-4 p-4 bg-red-50 border border-red-200 rounded-lg flex items-center">
            <AlertCircle className="h-5 w-5 text-red-600 mr-2 flex-shrink-0" />
            <p className="text-sm text-red-700">{error}</p>
          </div>
        )}

        {success && (
          <div className="mb-4 p-4 bg-green-50 border border-green-200 rounded-lg flex items-center">
            <CheckCircle className="h-5 w-5 text-green-600 mr-2 flex-shrink-0" />
            <p className="text-sm text-green-700">{success}</p>
          </div>
        )}

        {/* Current Auth View */}
        {renderCurrentView()}

        {/* Footer */}
        <div className="mt-8 text-center">
          <p className="text-xs text-gray-500">
            By continuing, you agree to our{' '}
            <a href="#" className="text-blue-600 hover:text-blue-500">
              Terms of Service
            </a>{' '}
            and{' '}
            <a href="#" className="text-blue-600 hover:text-blue-500">
              Privacy Policy
            </a>
          </p>
        </div>
      </div>
    </div>
  );
}
