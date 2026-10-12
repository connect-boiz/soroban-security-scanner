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
      persistSession(result.user, data.rememberMe);
      // In a real app, redirect to dashboard
    } catch (err) {
      // Prevent account enumeration by always providing generic auth failure
      throw new Error('Invalid email or password');
    } finally {
      setIsLoading(false);
    }
  };

  const handleSignUp = async (data: Omit<SignUpData, 'confirmPassword' | 'agreeToTerms'>) => {
    setIsLoading(true);

    try {
      await signUp(data);
    } catch (err) {
      // Prevent account enumeration by returning generic registration message
      throw new Error(
        'Unable to complete registration. If you already have an account, please sign in or reset your password.'
      );
    } finally {
      setIsLoading(false);
    }
  };

  const handleResetPassword = async (email: string) => {
    setIsLoading(true);

    try {
      await resetPassword(email);
    } catch (err) {
      // Prevent account enumeration: do not propagate account-absence errors
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
