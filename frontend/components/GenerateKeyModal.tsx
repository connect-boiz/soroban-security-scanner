'use client';

import { useState } from 'react';
import { X, Key, Shield, Calendar, AlertTriangle } from 'lucide-react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';

const generateKeySchema = z.object({
  name: z.string().min(3, 'Name must be at least 3 characters').max(100, 'Name must be less than 100 characters'),
  description: z.string().max(500, 'Description must be less than 500 characters').optional(),
  permissions: z.array(z.string()).min(1, 'At least one permission is required'),
  expiresAt: z.string().optional(),
});

type GenerateKeyFormData = z.infer<typeof generateKeySchema>;

interface GenerateKeyModalProps {
  onClose: () => void;
  onGenerate: (keyData: GenerateKeyFormData) => void;
}

const availablePermissions = [
  { id: 'scan:read', label: 'Read Scans', description: 'View scan results and reports' },
  { id: 'scan:write', label: 'Create Scans', description: 'Create and run new scans' },
  { id: 'scan:delete', label: 'Delete Scans', description: 'Delete scan results' },
  { id: 'user:read', label: 'Read Users', description: 'View user information (Admin only)' },
  { id: 'api-key:read', label: 'Read API Keys', description: 'View API key information' },
];

export function GenerateKeyModal({ onClose, onGenerate }: GenerateKeyModalProps) {
  const [loading, setLoading] = useState(false);
  
  const {
    register,
    handleSubmit,
    formState: { errors },
    watch,
    setValue,
  } = useForm<GenerateKeyFormData>({
    resolver: zodResolver(generateKeySchema),
    defaultValues: {
      name: '',
      description: '',
      permissions: ['scan:read', 'scan:write'],
      expiresAt: '',
    },
  });

  const selectedPermissions = watch('permissions');

  const handlePermissionToggle = (permissionId: string) => {
    const currentPermissions = selectedPermissions || [];
    if (currentPermissions.includes(permissionId)) {
      setValue('permissions', currentPermissions.filter(p => p !== permissionId));
    } else {
      setValue('permissions', [...currentPermissions, permissionId]);
    }
  };

  const onSubmit = async (data: GenerateKeyFormData) => {
    setLoading(true);
    try {
      await onGenerate(data);
      onClose();
    } finally {
      setLoading(false);
    }
  };

  const getMinDate = () => {
    const tomorrow = new Date();
    tomorrow.setDate(tomorrow.getDate() + 1);
    return tomorrow.toISOString().split('T')[0];
  };

  return (
    <div className="fixed inset-0 bg-gray-500 bg-opacity-75 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg shadow-xl max-w-2xl w-full mx-4 max-h-[90vh] overflow-y-auto">
        <div className="flex items-center justify-between p-6 border-b border-gray-200">
          <div className="flex items-center">
            <Key className="h-6 w-6 text-blue-600 mr-3" />
            <h2 className="text-xl font-semibold text-gray-900">Generate New API Key</h2>
          </div>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-gray-600"
          >
            <X className="h-6 w-6" />
          </button>
        </div>

        <form onSubmit={handleSubmit(onSubmit)} className="p-6 space-y-6">
          {/* Key Information */}
          <div className="space-y-4">
            <div>
              <label htmlFor="name" className="block text-sm font-medium text-gray-700">
                Key Name *
              </label>
              <input
                type="text"
                id="name"
                {...register('name')}
                className="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
                placeholder="e.g., Production CI/CD Pipeline"
              />
              {errors.name && (
                <p className="mt-1 text-sm text-red-600">{errors.name.message}</p>
              )}
            </div>

            <div>
              <label htmlFor="description" className="block text-sm font-medium text-gray-700">
                Description
              </label>
              <textarea
                id="description"
                rows={3}
                {...register('description')}
                className="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
                placeholder="Optional description of what this API key is used for"
              />
              {errors.description && (
                <p className="mt-1 text-sm text-red-600">{errors.description.message}</p>
              )}
            </div>

            <div>
              <label htmlFor="expiresAt" className="block text-sm font-medium text-gray-700">
                Expiration Date
              </label>
              <input
                type="date"
                id="expiresAt"
                {...register('expiresAt')}
                min={getMinDate()}
                className="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
              />
              <p className="mt-1 text-sm text-gray-500">
                Leave empty for no expiration. Keys will expire at the end of the selected date.
              </p>
              {errors.expiresAt && (
                <p className="mt-1 text-sm text-red-600">{errors.expiresAt.message}</p>
              )}
            </div>
          </div>

          {/* Permissions */}
          <div>
            <div className="flex items-center mb-4">
              <Shield className="h-5 w-5 text-gray-400 mr-2" />
              <label className="block text-sm font-medium text-gray-700">
                Permissions *
              </label>
            </div>
            
            <div className="space-y-3">
              {availablePermissions.map((permission) => (
                <div key={permission.id} className="flex items-start">
                  <div className="flex items-center h-5">
                    <input
                      type="checkbox"
                      id={permission.id}
                      checked={selectedPermissions.includes(permission.id)}
                      onChange={() => handlePermissionToggle(permission.id)}
                      className="focus:ring-blue-500 h-4 w-4 text-blue-600 border-gray-300 rounded"
                    />
                  </div>
                  <div className="ml-3 text-sm">
                    <label htmlFor={permission.id} className="font-medium text-gray-700">
                      {permission.label}
                    </label>
                    <p className="text-gray-500">{permission.description}</p>
                  </div>
                </div>
              ))}
            </div>
            
            {errors.permissions && (
              <p className="mt-1 text-sm text-red-600">{errors.permissions.message}</p>
            )}
          </div>

          {/* Security Warning */}
          <div className="bg-amber-50 border border-amber-200 rounded-md p-4">
            <div className="flex">
              <AlertTriangle className="h-5 w-5 text-amber-400 mr-2 flex-shrink-0" />
              <div className="text-sm text-amber-800">
                <p className="font-medium mb-1">Important Security Notice</p>
                <ul className="list-disc list-inside space-y-1 text-amber-700">
                  <li>The API key will be shown only once after generation</li>
                  <li>Store it securely in your environment variables or secret manager</li>
                  <li>Never commit API keys to version control</li>
                  <li>Rotate keys regularly for security</li>
                </ul>
              </div>
            </div>
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
              type="submit"
              disabled={loading}
              className="px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {loading ? 'Generating...' : 'Generate API Key'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
