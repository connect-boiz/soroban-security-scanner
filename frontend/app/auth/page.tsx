'use client';

import React from 'react';
import AuthContainer from '../../components/auth/AuthContainer';
import { PageErrorBoundary } from '../../components/ui/ErrorBoundary';

export default function AuthPage() {
  const handleAuthSuccess = (user: any) => {
    console.log('Authentication successful:', user);
  };

  return (
    <PageErrorBoundary context={{ page: 'auth' }}>
      <div className="min-h-screen">
        <AuthContainer onAuthSuccess={handleAuthSuccess} />
      </div>
    </PageErrorBoundary>
  );
}
