'use client';

import React, { useState } from 'react';
import LoginForm from './LoginForm';
import SignUpForm from './SignUpForm';
import PasswordResetForm from './PasswordResetForm';
import MultiFactorAuth from './MultiFactorAuth';
import { AlertCircle, CheckCircle } from 'lucide-react';

type AuthView = 'login' | 'signup' | 'reset-password' | 'mfa';

interface AuthContainerProps {
  onAuthSuccess?: (user: any) => void;
  initialView?: AuthView;
}

export default function AuthContainer({ onAuthSuccess, initialView = 'login' }: AuthContainerProps) {
  const [currentView, setCurrentView] = useState<AuthView>(initialView);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [userEmail, setUserEmail] = useState('');
  const [userPhone, setUserPhone] = useState('');

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

  // Mock authentication functions - replace with actual API calls
  const mockLogin = async (credentials: { email: string; password: string; rememberMe: boolean }) => {
    // Simulate API delay
    await new Promise(resolve => setTimeout(resolve, 1500));
    
    // Mock validation
    if (credentials.email === 'demo@example.com' && credentials.password === 'password123') {
      setUserEmail(credentials.email);
      return { user: { email: credentials.email, name: 'Demo User' }, requiresMfa: true };
    }
    throw new Error('Invalid email or password');
  };

  const mockSignUp = async (userData: { firstName: string; lastName: string; email: string; password: string }) => {
    // Simulate API delay
    await new Promise(resolve => setTimeout(resolve, 2000));
    
    // Mock validation
    if (userData.email === 'existing@example.com') {
      throw new Error('An account with this email already exists');
    }
    
    setUserEmail(userData.email);
    return { user: { email: userData.email, name: `${userData.firstName} ${userData.lastName}` } };
  };

  const mockResetPassword = async (email: string) => {
    // Simulate API delay
    await new Promise(resolve => setTimeout(resolve, 1000));
    
    // Mock validation
    if (email === 'notfound@example.com') {
      throw new Error('No account found with this email address');
    }
    
    return { success: true };
  };

  const mockVerifyMfa = async (code: string, method: string) => {
    // Simulate API delay
    await new Promise(resolve => setTimeout(resolve, 1000));
    
    // Mock validation
    if (code === '123456') {
      return { verified: true };
    }
    throw new Error('Invalid verification code');
  };

  const mockResendMfaCode = async (method: string) => {
    // Simulate API delay
    await new Promise(resolve => setTimeout(resolve, 500));
    return { success: true };
  };

  const handleLogin = async (credentials: { email: string; password: string; rememberMe: boolean }) => {
    clearMessages();
    setIsLoading(true);
    
    try {
      const result = await mockLogin(credentials);
      
      if (result.requiresMfa) {
        setCurrentView('mfa');
        handleSuccess('Login successful! Please complete two-factor authentication.');
      } else {
        handleSuccess('Login successful!');
        onAuthSuccess?.(result.user);
      }
    } catch (err) {
      handleError(err instanceof Error ? err.message : 'Login failed');
    } finally {
      setIsLoading(false);
    }
  };

  const handleSignUp = async (userData: { firstName: string; lastName: string; email: string; password: string }) => {
    clearMessages();
    setIsLoading(true);
    
    try {
      const result = await mockSignUp(userData);
      handleSuccess('Account created successfully! Please check your email to verify your account.');
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
      await mockResetPassword(email);
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
      const result = await mockVerifyMfa(code, method);
      handleSuccess('Authentication successful!');
      onAuthSuccess?.({ email: userEmail, verified: true });
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
      await mockResendMfaCode(method);
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
