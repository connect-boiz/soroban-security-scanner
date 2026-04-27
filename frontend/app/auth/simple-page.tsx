'use client';

import React, { useState } from 'react';
import SimpleAuth, { LoginData, SignUpData } from '../../components/auth/SimpleAuth';

export default function SimpleAuthPage() {
  const [isLoading, setIsLoading] = useState(false);

  // Mock authentication functions
  const handleLogin = async (data: LoginData) => {
    setIsLoading(true);
    console.log('Login attempt:', data);
    
    // Simulate API call
    await new Promise(resolve => setTimeout(resolve, 1500));
    
    // Mock validation
    if (data.email === 'demo@example.com' && data.password === 'password123') {
      console.log('Login successful');
      // In a real app, redirect to dashboard
    } else {
      throw new Error('Invalid email or password');
    }
  };

  const handleSignUp = async (data: Omit<SignUpData, 'confirmPassword' | 'agreeToTerms'>) => {
    setIsLoading(true);
    console.log('Sign up attempt:', data);
    
    // Simulate API call
    await new Promise(resolve => setTimeout(resolve, 2000));
    
    // Mock validation
    if (data.email === 'existing@example.com') {
      throw new Error('An account with this email already exists');
    }
    
    console.log('Sign up successful');
  };

  const handleResetPassword = async (email: string) => {
    setIsLoading(true);
    console.log('Password reset attempt:', email);
    
    // Simulate API call
    await new Promise(resolve => setTimeout(resolve, 1000));
    
    // Mock validation
    if (email === 'notfound@example.com') {
      throw new Error('No account found with this email address');
    }
    
    console.log('Password reset email sent');
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
