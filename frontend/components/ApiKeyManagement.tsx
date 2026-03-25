'use client';

import { useState } from 'react';
import { Plus, Key, Trash2, Edit, Copy, Eye, EyeOff, Calendar, Shield } from 'lucide-react';
import { toast } from 'sonner';
import { GenerateKeyModal } from './GenerateKeyModal';
import { EditKeyModal } from './EditKeyModal';
import { RevokeKeyModal } from './RevokeKeyModal';
import { ShowKeyModal } from './ShowKeyModal';

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

interface ApiKeyManagementProps {
  apiKeys: ApiKey[];
  user: User;
  onGenerateKey: (keyData: any) => Promise<string | null>;
  onRevokeKey: (keyId: string) => Promise<void>;
  onUpdateKey: (keyId: string, updateData: any) => Promise<void>;
  showGenerateModal: boolean;
  setShowGenerateModal: (show: boolean) => void;
}

export function ApiKeyManagement({
  apiKeys,
  user,
  onGenerateKey,
  onRevokeKey,
  onUpdateKey,
  showGenerateModal,
  setShowGenerateModal,
}: ApiKeyManagementProps) {
  const [editingKey, setEditingKey] = useState<ApiKey | null>(null);
  const [revokingKey, setRevokingKey] = useState<ApiKey | null>(null);
  const [newApiKey, setNewApiKey] = useState<string | null>(null);
  const [showKeyModal, setShowKeyModal] = useState(false);

  const handleGenerateKey = async (keyData: any) => {
    const apiKey = await onGenerateKey(keyData);
    if (apiKey) {
      setNewApiKey(apiKey);
      setShowKeyModal(true);
    }
  };

  const maskApiKey = (keyPrefix: string) => {
    return `${keyPrefix}••••••••••••••`;
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const isExpired = (expiresAt?: string) => {
    if (!expiresAt) return false;
    return new Date(expiresAt) < new Date();
  };

  const getStatusBadge = (status: 'active' | 'revoked', expiresAt?: string) => {
    if (status === 'revoked') {
      return (
        <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-red-100 text-red-800">
          Revoked
        </span>
      );
    }
    
    if (isExpired(expiresAt)) {
      return (
        <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-800">
          Expired
        </span>
      );
    }

    return (
      <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800">
        Active
      </span>
    );
  };

  return (
    <div className="space-y-6">
      {/* Header Actions */}
      <div className="flex justify-between items-center">
        <div className="flex items-center space-x-2">
          <Key className="h-5 w-5 text-gray-400" />
          <span className="text-sm text-gray-600">
            {apiKeys.length} API key{apiKeys.length !== 1 ? 's' : ''}
          </span>
        </div>
        
        <button
          onClick={() => setShowGenerateModal(true)}
          className="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
        >
          <Plus className="h-4 w-4 mr-2" />
          Generate New Key
        </button>
      </div>

      {/* API Keys Table */}
      <div className="bg-white shadow overflow-hidden sm:rounded-md">
        <div className="px-4 py-5 sm:px-6">
          <h3 className="text-lg leading-6 font-medium text-gray-900">
            API Keys
          </h3>
          <p className="mt-1 max-w-2xl text-sm text-gray-500">
            Manage your API keys for secure access to the scanner API.
          </p>
        </div>
        
        {apiKeys.length === 0 ? (
          <div className="text-center py-12">
            <Key className="mx-auto h-12 w-12 text-gray-400" />
            <h3 className="mt-2 text-sm font-medium text-gray-900">No API keys</h3>
            <p className="mt-1 text-sm text-gray-500">
              Get started by generating your first API key.
            </p>
            <div className="mt-6">
              <button
                onClick={() => setShowGenerateModal(true)}
                className="inline-flex items-center px-4 py-2 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
              >
                <Plus className="h-4 w-4 mr-2" />
                Generate New Key
              </button>
            </div>
          </div>
        ) : (
          <ul className="divide-y divide-gray-200">
            {apiKeys.map((apiKey) => (
              <li key={apiKey.id}>
                <div className="px-4 py-4 sm:px-6">
                  <div className="flex items-center justify-between">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center space-x-3">
                        <p className="text-sm font-medium text-gray-900 truncate">
                          {apiKey.name}
                        </p>
                        {getStatusBadge(apiKey.status, apiKey.expiresAt)}
                      </div>
                      
                      <div className="mt-2 flex items-center space-x-4 text-sm text-gray-500">
                        <div className="flex items-center">
                          <Key className="h-4 w-4 mr-1" />
                          <span className="font-mono">{maskApiKey(apiKey.keyPrefix)}</span>
                        </div>
                        
                        {apiKey.description && (
                          <div className="truncate">
                            {apiKey.description}
                          </div>
                        )}
                      </div>
                      
                      <div className="mt-2 flex items-center space-x-6 text-xs text-gray-500">
                        <div className="flex items-center">
                          <Calendar className="h-3 w-3 mr-1" />
                          Created {formatDate(apiKey.createdAt)}
                        </div>
                        
                        {apiKey.lastUsedAt && (
                          <div className="flex items-center">
                            <Eye className="h-3 w-3 mr-1" />
                            Last used {formatDate(apiKey.lastUsedAt)}
                          </div>
                        )}
                        
                        {apiKey.expiresAt && (
                          <div className="flex items-center">
                            <Calendar className="h-3 w-3 mr-1" />
                            Expires {formatDate(apiKey.expiresAt)}
                          </div>
                        )}
                        
                        <div className="flex items-center">
                          <Shield className="h-3 w-3 mr-1" />
                          {apiKey.permissions.join(', ')}
                        </div>
                      </div>
                    </div>
                    
                    <div className="flex items-center space-x-2">
                      <button
                        onClick={() => {
                          navigator.clipboard.writeText(maskApiKey(apiKey.keyPrefix));
                          toast.success('API key prefix copied to clipboard');
                        }}
                        className="p-2 text-gray-400 hover:text-gray-600"
                        title="Copy key prefix"
                      >
                        <Copy className="h-4 w-4" />
                      </button>
                      
                      {apiKey.status === 'active' && (
                        <>
                          <button
                            onClick={() => setEditingKey(apiKey)}
                            className="p-2 text-gray-400 hover:text-gray-600"
                            title="Edit API key"
                          >
                            <Edit className="h-4 w-4" />
                          </button>
                          
                          <button
                            onClick={() => setRevokingKey(apiKey)}
                            className="p-2 text-red-400 hover:text-red-600"
                            title="Revoke API key"
                          >
                            <Trash2 className="h-4 w-4" />
                          </button>
                        </>
                      )}
                    </div>
                  </div>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>

      {/* Modals */}
      {showGenerateModal && (
        <GenerateKeyModal
          onClose={() => setShowGenerateModal(false)}
          onGenerate={handleGenerateKey}
        />
      )}

      {editingKey && (
        <EditKeyModal
          apiKey={editingKey}
          onClose={() => setEditingKey(null)}
          onUpdate={onUpdateKey}
        />
      )}

      {revokingKey && (
        <RevokeKeyModal
          apiKey={revokingKey}
          onClose={() => setRevokingKey(null)}
          onRevoke={onRevokeKey}
        />
      )}

      {newApiKey && (
        <ShowKeyModal
          apiKey={newApiKey}
          onClose={() => {
            setShowKeyModal(false);
            setNewApiKey(null);
          }}
        />
      )}
    </div>
  );
}
