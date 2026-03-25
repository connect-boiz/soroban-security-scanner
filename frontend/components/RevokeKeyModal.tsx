'use client';

import { useState } from 'react';
import { X, AlertTriangle, Trash2, Key, Calendar } from 'lucide-react';

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

interface RevokeKeyModalProps {
  apiKey: ApiKey;
  onClose: () => void;
  onRevoke: (keyId: string) => void;
}

export function RevokeKeyModal({ apiKey, onClose, onRevoke }: RevokeKeyModalProps) {
  const [loading, setLoading] = useState(false);
  const [confirmed, setConfirmed] = useState(false);

  const handleRevoke = async () => {
    if (!confirmed) return;
    
    setLoading(true);
    try {
      await onRevoke(apiKey.id);
      onClose();
    } finally {
      setLoading(false);
    }
  };

  const maskKey = (keyPrefix: string) => {
    return `${keyPrefix}••••••••••••••`;
  };

  return (
    <div className="fixed inset-0 bg-gray-500 bg-opacity-75 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg shadow-xl max-w-lg w-full mx-4">
        <div className="flex items-center justify-between p-6 border-b border-gray-200">
          <div className="flex items-center">
            <Trash2 className="h-6 w-6 text-red-600 mr-3" />
            <h2 className="text-xl font-semibold text-gray-900">Revoke API Key</h2>
          </div>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-gray-600"
          >
            <X className="h-6 w-6" />
          </button>
        </div>

        <div className="p-6 space-y-6">
          {/* Warning Message */}
          <div className="bg-red-50 border border-red-200 rounded-md p-4">
            <div className="flex">
              <AlertTriangle className="h-5 w-5 text-red-400 mr-2 flex-shrink-0" />
              <div className="text-sm text-red-800">
                <p className="font-medium mb-2">Warning: This action cannot be undone</p>
                <p className="text-red-700">
                  Revoking this API key will immediately invalidate it. Any applications or services using this key will no longer be able to authenticate with the API.
                </p>
              </div>
            </div>
          </div>

          {/* Key Information */}
          <div className="bg-gray-50 border border-gray-200 rounded-md p-4">
            <div className="text-sm text-gray-600">
              <p className="font-medium mb-2">API Key to Revoke</p>
              <div className="space-y-2">
                <div className="flex items-center">
                  <Key className="h-4 w-4 mr-2 text-gray-400" />
                  <span className="font-medium">Name:</span>
                  <span className="ml-2">{apiKey.name}</span>
                </div>
                
                {apiKey.description && (
                  <div>
                    <span className="font-medium">Description:</span>
                    <span className="ml-2">{apiKey.description}</span>
                  </div>
                )}
                
                <div className="flex items-center">
                  <Key className="h-4 w-4 mr-2 text-gray-400" />
                  <span className="font-medium">Key:</span>
                  <span className="ml-2 font-mono text-sm">{maskKey(apiKey.keyPrefix)}</span>
                </div>
                
                <div className="flex items-center">
                  <Calendar className="h-4 w-4 mr-2 text-gray-400" />
                  <span className="font-medium">Created:</span>
                  <span className="ml-2">{new Date(apiKey.createdAt).toLocaleDateString()}</span>
                </div>
                
                {apiKey.lastUsedAt && (
                  <div className="flex items-center">
                    <Calendar className="h-4 w-4 mr-2 text-gray-400" />
                    <span className="font-medium">Last Used:</span>
                    <span className="ml-2">{new Date(apiKey.lastUsedAt).toLocaleDateString()}</span>
                  </div>
                )}
                
                <div>
                  <span className="font-medium">Permissions:</span>
                  <span className="ml-2">{apiKey.permissions.join(', ')}</span>
                </div>
              </div>
            </div>
          </div>

          {/* Confirmation */}
          <div className="space-y-4">
            <div className="flex items-center">
              <input
                type="checkbox"
                id="confirm-revoke"
                checked={confirmed}
                onChange={(e) => setConfirmed(e.target.checked)}
                className="h-4 w-4 text-red-600 focus:ring-red-500 border-gray-300 rounded"
              />
              <label htmlFor="confirm-revoke" className="ml-2 text-sm text-gray-700">
                I understand that this action cannot be undone and I want to revoke this API key
              </label>
            </div>
            
            {!confirmed && (
              <p className="text-sm text-gray-500">
                Please check the confirmation box to proceed with revoking the API key.
              </p>
            )}
          </div>

          {/* Actions */}
          <div className="flex justify-end space-x-3 pt-4 border-t border-gray-200">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={handleRevoke}
              disabled={!confirmed || loading}
              className="px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-red-600 hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {loading ? 'Revoking...' : 'Revoke API Key'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
