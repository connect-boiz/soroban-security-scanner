'use client';

import React, { useState } from 'react';

// Simple Authentication Component - Compatible with existing setup
export interface LoginData {
  email: string;
  password: string;
  rememberMe: boolean;
}

export interface SignUpData {
  firstName: string;
  lastName: string;
  email: string;
  password: string;
  confirmPassword: string;
  agreeToTerms: boolean;
}

export interface AuthProps {
  onLogin: (data: LoginData) => Promise<void>;
  onSignUp: (data: Omit<SignUpData, 'confirmPassword' | 'agreeToTerms'>) => Promise<void>;
  onResetPassword: (email: string) => Promise<void>;
  isLoading?: boolean;
}

export default function SimpleAuth({ onLogin, onSignUp, onResetPassword, isLoading = false }: AuthProps) {
  // State management
  const [currentView, setCurrentView] = useState<'login' | 'signup' | 'reset'>('login');
  const [formData, setFormData] = useState<LoginData | SignUpData>({
    email: '',
    password: '',
    rememberMe: false,
    firstName: '',
    lastName: '',
    confirmPassword: '',
    agreeToTerms: false
  });
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  // Form validation
  const validateEmail = (email: string): boolean => {
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    return emailRegex.test(email);
  };

  const validateLogin = (): boolean => {
    const newErrors: Record<string, string> = {};
    const loginData = formData as LoginData;

    if (!loginData.email) {
      newErrors.email = 'Email is required';
    } else if (!validateEmail(loginData.email)) {
      newErrors.email = 'Please enter a valid email address';
    }

    if (!loginData.password) {
      newErrors.password = 'Password is required';
    } else if (loginData.password.length < 8) {
      newErrors.password = 'Password must be at least 8 characters';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const validateSignUp = (): boolean => {
    const newErrors: Record<string, string> = {};
    const signUpData = formData as SignUpData;

    if (!signUpData.firstName?.trim()) {
      newErrors.firstName = 'First name is required';
    }

    if (!signUpData.lastName?.trim()) {
      newErrors.lastName = 'Last name is required';
    }

    if (!signUpData.email) {
      newErrors.email = 'Email is required';
    } else if (!validateEmail(signUpData.email)) {
      newErrors.email = 'Please enter a valid email address';
    }

    if (!signUpData.password) {
      newErrors.password = 'Password is required';
    } else if (signUpData.password.length < 8) {
      newErrors.password = 'Password must be at least 8 characters';
    }

    if (!signUpData.confirmPassword) {
      newErrors.confirmPassword = 'Please confirm your password';
    } else if (signUpData.password !== signUpData.confirmPassword) {
      newErrors.confirmPassword = 'Passwords do not match';
    }

    if (!signUpData.agreeToTerms) {
      newErrors.agreeToTerms = 'You must agree to the terms and conditions';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const validateReset = (): boolean => {
    const newErrors: Record<string, string> = {};
    const loginData = formData as LoginData;

    if (!loginData.email) {
      newErrors.email = 'Email is required';
    } else if (!validateEmail(loginData.email)) {
      newErrors.email = 'Please enter a valid email address';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  // Form handlers
  const handleInputChange = (field: string, value: string | boolean) => {
    setFormData(prev => ({ ...prev, [field]: value }));
    if (errors[field]) {
      setErrors(prev => ({ ...prev, [field]: '' }));
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setMessage(null);

    try {
      if (currentView === 'login') {
        if (!validateLogin()) return;
        await onLogin(formData as LoginData);
        setMessage({ type: 'success', text: 'Login successful!' });
      } else if (currentView === 'signup') {
        if (!validateSignUp()) return;
        const { confirmPassword, agreeToTerms, ...signUpData } = formData as SignUpData;
        await onSignUp(signUpData);
        setMessage({ type: 'success', text: 'Account created successfully!' });
        setTimeout(() => setCurrentView('login'), 2000);
      } else if (currentView === 'reset') {
        if (!validateReset()) return;
        await onResetPassword((formData as LoginData).email);
        setMessage({ type: 'success', text: 'Password reset link sent!' });
        setTimeout(() => setCurrentView('login'), 2000);
      }
    } catch (error: any) {
      setMessage({ type: 'error', text: error.message || 'An error occurred' });
    }
  };

  const resetForm = () => {
    setFormData({
      email: '',
      password: '',
      rememberMe: false,
      firstName: '',
      lastName: '',
      confirmPassword: '',
      agreeToTerms: false
    });
    setErrors({});
    setMessage(null);
  };

  const switchView = (view: 'login' | 'signup' | 'reset') => {
    resetForm();
    setCurrentView(view);
  };

  // Render methods
  const renderMessage = () => {
    if (!message) return null;
    
    const bgColor = message.type === 'success' ? 'bg-green-50 border-green-200 text-green-800' : 'bg-red-50 border-red-200 text-red-800';
    const icon = message.type === 'success' ? '✓' : '✕';
    
    return (
      <div className={`p-4 rounded-lg border ${bgColor} mb-4 flex items-center`}>
        <span className="mr-2 font-bold">{icon}</span>
        <span>{message.text}</span>
      </div>
    );
  };

  const renderLogin = () => (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-gray-700 mb-1">Email</label>
        <input
          type="email"
          value={(formData as LoginData).email}
          onChange={(e) => handleInputChange('email', e.target.value)}
          className={`w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 ${
            errors.email ? 'border-red-300' : 'border-gray-300'
          }`}
          placeholder="Enter your email"
          disabled={isLoading}
        />
        {errors.email && <p className="text-red-600 text-sm mt-1">{errors.email}</p>}
      </div>

      <div>
        <label className="block text-sm font-medium text-gray-700 mb-1">Password</label>
        <input
          type="password"
          value={(formData as LoginData).password}
          onChange={(e) => handleInputChange('password', e.target.value)}
          className={`w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 ${
            errors.password ? 'border-red-300' : 'border-gray-300'
          }`}
          placeholder="Enter your password"
          disabled={isLoading}
        />
        {errors.password && <p className="text-red-600 text-sm mt-1">{errors.password}</p>}
      </div>

      <div className="flex items-center justify-between">
        <label className="flex items-center">
          <input
            type="checkbox"
            checked={(formData as LoginData).rememberMe}
            onChange={(e) => handleInputChange('rememberMe', e.target.checked)}
            className="mr-2"
            disabled={isLoading}
          />
          <span className="text-sm text-gray-700">Remember me</span>
        </label>
        <button
          type="button"
          onClick={() => switchView('reset')}
          className="text-sm text-blue-600 hover:underline"
          disabled={isLoading}
        >
          Forgot password?
        </button>
      </div>

      <button
        type="submit"
        disabled={isLoading}
        className="w-full bg-blue-600 text-white py-2 px-4 rounded-md hover:bg-blue-700 disabled:opacity-50"
      >
        {isLoading ? 'Signing in...' : 'Sign In'}
      </button>

      <div className="text-center">
        <span className="text-sm text-gray-600">
          Don't have an account?{' '}
          <button
            type="button"
            onClick={() => switchView('signup')}
            className="text-blue-600 hover:underline"
            disabled={isLoading}
          >
            Sign up
          </button>
        </span>
      </div>
    </form>
  );

  const renderSignUp = () => (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">First Name</label>
          <input
            type="text"
            value={(formData as SignUpData).firstName}
            onChange={(e) => handleInputChange('firstName', e.target.value)}
            className={`w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 ${
              errors.firstName ? 'border-red-300' : 'border-gray-300'
            }`}
            placeholder="First name"
            disabled={isLoading}
          />
          {errors.firstName && <p className="text-red-600 text-sm mt-1">{errors.firstName}</p>}
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">Last Name</label>
          <input
            type="text"
            value={(formData as SignUpData).lastName}
            onChange={(e) => handleInputChange('lastName', e.target.value)}
            className={`w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 ${
              errors.lastName ? 'border-red-300' : 'border-gray-300'
            }`}
            placeholder="Last name"
            disabled={isLoading}
          />
          {errors.lastName && <p className="text-red-600 text-sm mt-1">{errors.lastName}</p>}
        </div>
      </div>

      <div>
        <label className="block text-sm font-medium text-gray-700 mb-1">Email</label>
        <input
          type="email"
          value={(formData as SignUpData).email}
          onChange={(e) => handleInputChange('email', e.target.value)}
          className={`w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 ${
            errors.email ? 'border-red-300' : 'border-gray-300'
          }`}
          placeholder="Enter your email"
          disabled={isLoading}
        />
        {errors.email && <p className="text-red-600 text-sm mt-1">{errors.email}</p>}
      </div>

      <div>
        <label className="block text-sm font-medium text-gray-700 mb-1">Password</label>
        <input
          type="password"
          value={(formData as SignUpData).password}
          onChange={(e) => handleInputChange('password', e.target.value)}
          className={`w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 ${
            errors.password ? 'border-red-300' : 'border-gray-300'
          }`}
          placeholder="Create a password"
          disabled={isLoading}
        />
        {errors.password && <p className="text-red-600 text-sm mt-1">{errors.password}</p>}
      </div>

      <div>
        <label className="block text-sm font-medium text-gray-700 mb-1">Confirm Password</label>
        <input
          type="password"
          value={(formData as SignUpData).confirmPassword}
          onChange={(e) => handleInputChange('confirmPassword', e.target.value)}
          className={`w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 ${
            errors.confirmPassword ? 'border-red-300' : 'border-gray-300'
          }`}
          placeholder="Confirm your password"
          disabled={isLoading}
        />
        {errors.confirmPassword && <p className="text-red-600 text-sm mt-1">{errors.confirmPassword}</p>}
      </div>

      <div>
        <label className="flex items-center">
          <input
            type="checkbox"
            checked={(formData as SignUpData).agreeToTerms}
            onChange={(e) => handleInputChange('agreeToTerms', e.target.checked)}
            className="mr-2"
            disabled={isLoading}
          />
          <span className="text-sm text-gray-700">
            I agree to the Terms and Conditions and Privacy Policy
          </span>
        </label>
        {errors.agreeToTerms && <p className="text-red-600 text-sm mt-1">{errors.agreeToTerms}</p>}
      </div>

      <button
        type="submit"
        disabled={isLoading}
        className="w-full bg-blue-600 text-white py-2 px-4 rounded-md hover:bg-blue-700 disabled:opacity-50"
      >
        {isLoading ? 'Creating account...' : 'Create Account'}
      </button>

      <div className="text-center">
        <span className="text-sm text-gray-600">
          Already have an account?{' '}
          <button
            type="button"
            onClick={() => switchView('login')}
            className="text-blue-600 hover:underline"
            disabled={isLoading}
          >
            Sign in
          </button>
        </span>
      </div>
    </form>
  );

  const renderReset = () => (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-gray-700 mb-1">Email</label>
        <input
          type="email"
          value={(formData as LoginData).email}
          onChange={(e) => handleInputChange('email', e.target.value)}
          className={`w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 ${
            errors.email ? 'border-red-300' : 'border-gray-300'
          }`}
          placeholder="Enter your email"
          disabled={isLoading}
        />
        {errors.email && <p className="text-red-600 text-sm mt-1">{errors.email}</p>}
      </div>

      <button
        type="submit"
        disabled={isLoading}
        className="w-full bg-blue-600 text-white py-2 px-4 rounded-md hover:bg-blue-700 disabled:opacity-50"
      >
        {isLoading ? 'Sending...' : 'Send Reset Link'}
      </button>

      <div className="text-center">
        <button
          type="button"
          onClick={() => switchView('login')}
          className="text-sm text-blue-600 hover:underline"
          disabled={isLoading}
        >
          Back to Login
        </button>
      </div>
    </form>
  );

  return (
    <div className="min-h-screen bg-gray-50 flex items-center justify-center p-4">
      <div className="bg-white rounded-lg shadow-md p-6 w-full max-w-md">
        <div className="text-center mb-6">
          <h1 className="text-2xl font-bold text-gray-900 mb-2">
            {currentView === 'login' ? 'Welcome Back' : 
             currentView === 'signup' ? 'Create Account' : 'Reset Password'}
          </h1>
          <p className="text-gray-600 text-sm">
            {currentView === 'login' ? 'Sign in to your account' :
             currentView === 'signup' ? 'Join Soroban Security Scanner' :
             'Enter your email to reset your password'}
          </p>
        </div>

        {renderMessage()}

        {currentView === 'login' && renderLogin()}
        {currentView === 'signup' && renderSignUp()}
        {currentView === 'reset' && renderReset()}
      </div>
    </div>
  );
}
