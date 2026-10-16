'use client';

import React, { useState } from 'react';
import SimpleAuth, { LoginData, SignUpData } from '../../components/auth/SimpleAuth';
import { persistSession } from '../../lib/auth/session';
import { login, signUp, resetPassword } from '../../lib/auth/authService';

export default function SimpleAuthPage() {
  const [isLoading, setIsLoading] = useState(false);

  const handleLogin = async (data: LoginData) => {
    setIsLoading(true);

    try {
      const result = await login(data);
      await persistSession(result.user, data.rememberMe);
      // redirect to dashboard happens via the middleware route guard
    } finally {
      setIsLoading(false);
    }
  };

  const handleSignUp = async (data: Omit<SignUpData, 'confirmPassword' | 'agreeToTerms'>) => {
    setIsLoading(true);

    try {
      await signUp(data);
    } finally {
      setIsLoading(false);
    }
  };

  const handleResetPassword = async (email: string) => {
    setIsLoading(true);

    try {
      await resetPassword(email);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="min-h-screen">
      <SimpleAuth
        onLogin={handleLogin}
        onSignUp={handleSignUp}
        onResetPassword={handleResetPassword}
        isLoading={isLoading}
      />
    </div>
  );
}
