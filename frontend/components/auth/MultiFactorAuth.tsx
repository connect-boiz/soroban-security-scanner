'use client';

import React, { useState, FormEvent, useEffect } from 'react';
import { Shield, ArrowLeft, AlertCircle, CheckCircle, Smartphone, Mail, Key, Clock } from 'lucide-react';

type MfaMethod = 'totp' | 'sms' | 'email';

interface MfaFormData {
  code: string;
}

interface MultiFactorAuthProps {
  onVerify: (code: string, method: MfaMethod) => Promise<void>;
  onBack: () => void;
  onResendCode: (method: MfaMethod) => Promise<void>;
  selectedMethod?: MfaMethod;
  isLoading?: boolean;
  userEmail?: string;
  userPhone?: string;
}

export default function MultiFactorAuth({
  onVerify,
  onBack,
  onResendCode,
  selectedMethod = 'totp',
  isLoading = false,
  userEmail = '',
  userPhone = ''
}: MultiFactorAuthProps) {
  const [method, setMethod] = useState<MfaMethod>(selectedMethod);
  const [code, setCode] = useState('');
  const [errors, setErrors] = useState<{ code?: string }>({});
  const [touched, setTouched] = useState(false);
  const [resendTimer, setResendTimer] = useState(0);
  const [isResending, setIsResending] = useState(false);

  // Countdown timer for resend button
  useEffect(() => {
    let interval: NodeJS.Timeout;
    if (resendTimer > 0) {
      interval = setInterval(() => {
        setResendTimer(prev => prev - 1);
      }, 1000);
    }
    return () => clearInterval(interval);
  }, [resendTimer]);

  const validateCode = (code: string): boolean => {
    // TOTP codes are typically 6 digits, SMS can vary
    const minLength = method === 'totp' ? 6 : 4;
    const maxLength = method === 'totp' ? 6 : 8;
    const regex = method === 'totp' ? /^\d{6}$/ : /^\d{4,8}$/;
    
    return code.length >= minLength && code.length <= maxLength && regex.test(code);
  };

  const validateForm = (): boolean => {
    const newErrors: { code?: string } = {};

    if (!code) {
      newErrors.code = 'Verification code is required';
    } else if (!validateCode(code)) {
      newErrors.code = method === 'totp' 
        ? 'Please enter a valid 6-digit code'
        : 'Please enter a valid verification code';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleCodeChange = (value: string) => {
    // Only allow digits
    const digitsOnly = value.replace(/\D/g, '');
    setCode(digitsOnly);
    
    // Clear error when user starts typing
    if (errors.code) {
      setErrors({});
    }
  };

  const handleCodeBlur = () => {
    setTouched(true);
    
    // Validate code on blur
    if (code && !validateCode(code)) {
      setErrors({ 
        code: method === 'totp' 
          ? 'Please enter a valid 6-digit code'
          : 'Please enter a valid verification code'
      });
    }
  };

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    
    if (!validateForm()) {
      return;
    }

    try {
      await onVerify(code, method);
    } catch (error) {
      // Error handling is managed by parent component
    }
  };

  const handleResendCode = async (newMethod: MfaMethod) => {
    if (resendTimer > 0 || isResending) return;

    setIsResending(true);
    try {
      await onResendCode(newMethod);
      setResendTimer(60); // 60 seconds cooldown
      setCode('');
      setErrors({});
      setTouched(false);
    } catch (error) {
      // Error handling is managed by parent component
    } finally {
      setIsResending(false);
    }
  };

  const handleMethodChange = (newMethod: MfaMethod) => {
    setMethod(newMethod);
    setCode('');
    setErrors({});
    setTouched(false);
    setResendTimer(0);
  };

  const formatMethodText = (method: MfaMethod): string => {
    switch (method) {
      case 'totp': return 'Authenticator App';
      case 'sms': return 'SMS';
      case 'email': return 'Email';
      default: return 'Authentication';
    }
  };

  const getMethodIcon = (method: MfaMethod) => {
    switch (method) {
      case 'totp': return <Key className="h-5 w-5" />;
      case 'sms': return <Smartphone className="h-5 w-5" />;
      case 'email': return <Mail className="h-5 w-5" />;
      default: return <Shield className="h-5 w-5" />;
    }
  };

  const getMethodDescription = (method: MfaMethod): string => {
    switch (method) {
      case 'totp': return 'Enter the 6-digit code from your authenticator app';
      case 'sms': return `Enter the code sent to ${userPhone || 'your phone'}`;
      case 'email': return `Enter the code sent to ${userEmail || 'your email'}`;
      default: return 'Enter your verification code';
    }
  };

  return (
    <div className="w-full max-w-md mx-auto">
      <div className="bg-white rounded-xl shadow-lg border border-gray-200 p-8">
        {/* Back Button */}
        <button
          onClick={onBack}
          className="flex items-center text-gray-600 hover:text-gray-900 mb-6 transition-optimized"
          disabled={isLoading}
        >
          <ArrowLeft className="h-4 w-4 mr-2" />
          Back
        </button>

        <div className="text-center mb-8">
          <div className="mx-auto flex items-center justify-center h-12 w-12 rounded-full bg-blue-100 mb-4">
            <Shield className="h-6 w-6 text-blue-600" />
          </div>
          <h2 className="text-2xl font-bold text-gray-900 mb-2">Two-Factor Authentication</h2>
          <p className="text-gray-600 text-sm">
            Enter your verification code to complete the sign in process
          </p>
        </div>

        {/* Method Selection */}
        <div className="mb-6">
          <label className="block text-sm font-medium text-gray-700 mb-3">
            Verification Method
          </label>
          <div className="grid grid-cols-3 gap-2">
            {(['totp', 'sms', 'email'] as MfaMethod[]).map((m) => (
              <button
                key={m}
                type="button"
                onClick={() => handleMethodChange(m)}
                className={`flex flex-col items-center justify-center p-3 rounded-lg border-2 transition-optimized ${
                  method === m
                    ? 'border-blue-500 bg-blue-50 text-blue-700'
                    : 'border-gray-200 hover:border-gray-300 text-gray-600'
                }`}
                disabled={isLoading}
              >
                {getMethodIcon(m)}
                <span className="text-xs mt-1 font-medium">{formatMethodText(m)}</span>
              </button>
            ))}
          </div>
        </div>

        {/* Method Description */}
        <div className="mb-6 p-3 bg-gray-50 rounded-lg">
          <p className="text-sm text-gray-700 flex items-center">
            {getMethodIcon(method)}
            <span className="ml-2">{getMethodDescription(method)}</span>
          </p>
        </div>

        <form onSubmit={handleSubmit} className="space-y-6">
          {/* Code Input */}
          <div>
            <label htmlFor="code" className="block text-sm font-medium text-gray-700 mb-2">
              Verification Code
            </label>
            <div className="relative">
              <input
                id="code"
                type="text"
                value={code}
                onChange={(e) => handleCodeChange(e.target.value)}
                onBlur={handleCodeBlur}
                className={`block w-full px-4 py-3 border rounded-lg text-gray-900 placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent text-center text-lg font-mono tracking-widest transition-optimized ${
                  errors.code && touched
                    ? 'border-red-300 bg-red-50'
                    : touched && !errors.code
                    ? 'border-green-300 bg-green-50'
                    : 'border-gray-300'
                }`}
                placeholder={method === 'totp' ? '000000' : '0000'}
                maxLength={method === 'totp' ? 6 : 8}
                disabled={isLoading}
                autoComplete="one-time-code"
              />
              {errors.code && touched && (
                <div className="absolute inset-y-0 right-0 pr-3 flex items-center">
                  <AlertCircle className="h-5 w-5 text-red-500" />
                </div>
              )}
              {touched && !errors.code && code.length === (method === 'totp' ? 6 : 6) && (
                <div className="absolute inset-y-0 right-0 pr-3 flex items-center">
                  <CheckCircle className="h-5 w-5 text-green-500" />
                </div>
              )}
            </div>
            {errors.code && touched && (
              <p className="mt-1 text-sm text-red-600">{errors.code}</p>
            )}
          </div>

          {/* Resend Code */}
          <div className="flex items-center justify-between">
            <div className="flex items-center text-sm text-gray-600">
              <Clock className="h-4 w-4 mr-1" />
              {resendTimer > 0 ? (
                <span>Resend code in {resendTimer}s</span>
              ) : (
                <span>Didn't receive the code?</span>
              )}
            </div>
            <button
              type="button"
              onClick={() => handleResendCode(method)}
              disabled={resendTimer > 0 || isResending || isLoading}
              className="text-sm text-blue-600 hover:text-blue-500 font-medium disabled:opacity-50 disabled:cursor-not-allowed transition-optimized"
            >
              {isResending ? 'Sending...' : 'Resend'}
            </button>
          </div>

          {/* Submit Button */}
          <button
            type="submit"
            disabled={isLoading || code.length === 0}
            className="w-full flex justify-center py-3 px-4 border border-transparent rounded-lg shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed transition-optimized"
          >
            {isLoading ? (
              <div className="flex items-center">
                <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white mr-2"></div>
                Verifying...
              </div>
            ) : (
              'Verify Code'
            )}
          </button>
        </form>

        {/* Trust Device Option */}
        <div className="mt-6">
          <label className="flex items-center">
            <input
              type="checkbox"
              className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
              disabled={isLoading}
            />
            <span className="ml-2 block text-sm text-gray-700">
              Trust this device for 30 days
            </span>
          </label>
        </div>

        {/* Help Link */}
        <div className="mt-6 text-center">
          <p className="text-sm text-gray-600">
            Having trouble?{' '}
            <a href="#" className="font-medium text-blue-600 hover:text-blue-500">
              Contact support
            </a>
          </p>
        </div>
      </div>
    </div>
  );
}
