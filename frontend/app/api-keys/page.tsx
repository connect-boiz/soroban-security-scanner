'use client';

import { useState, useEffect } from 'react';
import { ApiKeyManagement } from '@/components/ApiKeyManagement';
import { toast } from 'sonner';

interface ApiKey {
  id: string;
  name: string;
  description?: string;
  status: 'active' | 'revoked';
  keyPrefix: string;
  lastUsedAt?: string;
  expiresAt?: string;
  permissions: string[];
  createdAt: string;
  updatedAt: string;
}

interface User {
  id: string;
  email: string;
  firstName?: string;
  lastName?: string;
  role: 'admin' | 'developer' | 'viewer';
}

export default function ApiKeyPage() {
  const [apiKeys, setApiKeys] = useState<ApiKey[]>([]);
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);
  const [showGenerateModal, setShowGenerateModal] = useState(false);

  useEffect(() => {
    fetchUserData();
    fetchApiKeys();
  }, []);

  const fetchUserData = async () => {
    try {
      const token = localStorage.getItem('accessToken');
      if (!token) {
        window.location.href = '/login';
        return;
      }

      const response = await fetch('/api/users/profile', {
        headers: {
          'Authorization': `Bearer ${token}`,
        },
      });

      if (response.ok) {
        const userData = await response.json();
        setUser(userData);
      } else {
        toast.error('Failed to fetch user data');
      }
    } catch (error) {
      toast.error('Error fetching user data');
    }
  };

  const fetchApiKeys = async () => {
    try {
      const token = localStorage.getItem('accessToken');
      if (!token) return;

      const response = await fetch('/api/api-keys', {
        headers: {
          'Authorization': `Bearer ${token}`,
        },
      });

      if (response.ok) {
        const keys = await response.json();
        setApiKeys(keys);
      } else {
        toast.error('Failed to fetch API keys');
      }
    } catch (error) {
      toast.error('Error fetching API keys');
    } finally {
      setLoading(false);
    }
  };

  const handleGenerateKey = async (keyData: { name: string; description?: string; permissions?: string[]; expiresAt?: string }) => {
    try {
      const token = localStorage.getItem('accessToken');
      const response = await fetch('/api/api-keys', {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(keyData),
      });

      if (response.ok) {
        const result = await response.json();
        toast.success('API key generated successfully');
        setShowGenerateModal(false);
        fetchApiKeys();
        
        // Show the raw key in a modal (only once)
        return result.apiKey;
      } else {
        toast.error('Failed to generate API key');
      }
    } catch (error) {
      toast.error('Error generating API key');
    }
    return null;
  };

  const handleRevokeKey = async (keyId: string) => {
    try {
      const token = localStorage.getItem('accessToken');
      const response = await fetch(`/api/api-keys/${keyId}`, {
        method: 'DELETE',
        headers: {
          'Authorization': `Bearer ${token}`,
        },
      });

      if (response.ok) {
        toast.success('API key revoked successfully');
        fetchApiKeys();
      } else {
        toast.error('Failed to revoke API key');
      }
    } catch (error) {
      toast.error('Error revoking API key');
    }
  };

  const handleUpdateKey = async (keyId: string, updateData: { name?: string; description?: string; permissions?: string[]; expiresAt?: string }) => {
    try {
      const token = localStorage.getItem('accessToken');
      const response = await fetch(`/api/api-keys/${keyId}`, {
        method: 'PUT',
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(updateData),
      });

      if (response.ok) {
        toast.success('API key updated successfully');
        fetchApiKeys();
      } else {
        toast.error('Failed to update API key');
      }
    } catch (error) {
      toast.error('Error updating API key');
    }
  };

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto"></div>
          <p className="mt-4 text-gray-600">Loading API Keys...</p>
        </div>
      </div>
    );
  }

  if (!user) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-center">
          <p className="text-red-600">Please log in to access API Key Management</p>
        </div>
      </div>
    );
  }

  if (user.role === 'viewer') {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-center">
          <p className="text-red-600">Access denied. You don't have permission to manage API keys.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <div className="max-w-7xl mx-auto py-6 sm:px-6 lg:px-8">
        <div className="px-4 py-6 sm:px-0">
          <div className="mb-8">
            <h1 className="text-3xl font-bold text-gray-900">API Key Management</h1>
            <p className="mt-2 text-gray-600">
              Manage your API keys for CI/CD pipeline authentication
            </p>
          </div>

          <ApiKeyManagement
            apiKeys={apiKeys}
            user={user}
            onGenerateKey={handleGenerateKey}
            onRevokeKey={handleRevokeKey}
            onUpdateKey={handleUpdateKey}
            showGenerateModal={showGenerateModal}
            setShowGenerateModal={setShowGenerateModal}
          />
        </div>
      </div>
    </div>
  );
}
