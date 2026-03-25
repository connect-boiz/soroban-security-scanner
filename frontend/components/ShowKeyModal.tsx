'use client';

import { useState } from 'react';
import { X, Copy, Eye, EyeOff, AlertTriangle, CheckCircle } from 'lucide-react';
import { toast } from 'sonner';

interface ShowKeyModalProps {
  apiKey: string;
  onClose: () => void;
}

export function ShowKeyModal({ apiKey, onClose }: ShowKeyModalProps) {
  const [copied, setCopied] = useState(false);
  const [showKey, setShowKey] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(apiKey);
      setCopied(true);
      toast.success('API key copied to clipboard');
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      toast.error('Failed to copy API key');
    }
  };

  const maskKey = (key: string) => {
    if (key.length <= 12) return key;
    return `${key.substring(0, 12)}${'•'.repeat(key.length - 12)}`;
  };

  return (
    <div className="fixed inset-0 bg-gray-500 bg-opacity-75 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg shadow-xl max-w-lg w-full mx-4">
        <div className="flex items-center justify-between p-6 border-b border-gray-200">
          <div className="flex items-center">
            <CheckCircle className="h-6 w-6 text-green-600 mr-3" />
            <h2 className="text-xl font-semibold text-gray-900">API Key Generated</h2>
          </div>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-gray-600"
          >
            <X className="h-6 w-6" />
          </button>
        </div>

        <div className="p-6 space-y-6">
          {/* Success Message */}
          <div className="bg-green-50 border border-green-200 rounded-md p-4">
            <div className="flex">
              <CheckCircle className="h-5 w-5 text-green-400 mr-2 flex-shrink-0" />
              <div className="text-sm text-green-800">
                <p className="font-medium">API key generated successfully!</p>
                <p className="mt-1">Please save this key securely. It won't be shown again.</p>
              </div>
            </div>
          </div>

          {/* API Key Display */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Your API Key
            </label>
            <div className="relative">
              <div className="flex items-center">
                <input
                  type={showKey ? 'text' : 'password'}
                  value={apiKey}
                  readOnly
                  className="flex-1 pr-12 border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 sm:text-sm font-mono text-sm bg-gray-50"
                />
                <button
                  type="button"
                  onClick={() => setShowKey(!showKey)}
                  className="absolute right-2 p-1 text-gray-400 hover:text-gray-600"
                  title={showKey ? 'Hide key' : 'Show key'}
                >
                  {showKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                </button>
              </div>
              {!showKey && (
                <p className="mt-1 text-xs text-gray-500 font-mono">{maskKey(apiKey)}</p>
              )}
            </div>
          </div>

          {/* Security Warning */}
          <div className="bg-amber-50 border border-amber-200 rounded-md p-4">
            <div className="flex">
              <AlertTriangle className="h-5 w-5 text-amber-400 mr-2 flex-shrink-0" />
              <div className="text-sm text-amber-800">
                <p className="font-medium mb-2">Before you continue:</p>
                <ul className="list-disc list-inside space-y-1 text-amber-700">
                  <li>Copy this API key and store it in a secure location</li>
                  <li>Add it to your environment variables or secret manager</li>
                  <li>Never share this key or commit it to version control</li>
                  <li>This is the only time you'll see the full key</li>
                </ul>
              </div>
            </div>
          </div>

          {/* Actions */}
          <div className="flex justify-end space-x-3">
            <button
              type="button"
              onClick={handleCopy}
              className={`inline-flex items-center px-4 py-2 border rounded-md shadow-sm text-sm font-medium focus:outline-none focus:ring-2 focus:ring-offset-2 ${
                copied
                  ? 'border-green-300 text-green-700 bg-green-50 focus:ring-green-500'
                  : 'border-gray-300 text-gray-700 bg-white hover:bg-gray-50 focus:ring-blue-500'
              }`}
            >
              {copied ? (
                <>
                  <CheckCircle className="h-4 w-4 mr-2" />
                  Copied!
                </>
              ) : (
                <>
                  <Copy className="h-4 w-4 mr-2" />
                  Copy Key
                </>
              )}
            </button>
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
            >
              I've Saved My Key
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
