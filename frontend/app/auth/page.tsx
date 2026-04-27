'use client';

import React from 'react';
import AuthContainer from '../../components/auth/AuthContainer';

export default function AuthPage() {
  const handleAuthSuccess = (user: any) => {
    console.log('Authentication successful:', user);
    // In a real app, you would redirect to dashboard or update app state
    // For demo purposes, we'll just log the success
  };

  return (
    <div className="min-h-screen">
      <AuthContainer onAuthSuccess={handleAuthSuccess} />
    </div>
  );
}
