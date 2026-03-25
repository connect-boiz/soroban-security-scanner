'use client';

import { useState, useEffect } from 'react';
import { X, Key, Shield, Calendar } from 'lucide-react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';

const editKeySchema = z.object({
  name: z.string().min(3, 'Name must be at least 3 characters').max(100, 'Name must be less than 100 characters'),
  description: z.string().max(500, 'Description must be less than 500 characters').optional(),
  permissions: z.array(z.string()).min(1, 'At least one permission is required'),
  expiresAt: z.string().optional(),
});

type EditKeyFormData = z.infer<typeof editKeySchema>;

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

interface EditKeyModalProps {
  apiKey: ApiKey;
  onClose: () => void;
  onUpdate: (keyId: string, updateData: EditKeyFormData) => void;
}

const availablePermissions = [
  { id: 'scan:read', label: 'Read Scans', description: 'View scan results and reports' },
  { id: 'scan:write', label: 'Create Scans', description: 'Create and run new scans' },
  { id: 'scan:delete', label: 'Delete Scans', description: 'Delete scan results' },
  { id: 'user:read', label: 'Read Users', description: 'View user information (Admin only)' },
  { id: 'api-key:read', label: 'Read API Keys', description: 'View API key information' },
];

export function EditKeyModal({ apiKey, onClose, onUpdate }: EditKeyModalProps) {
  const [loading, setLoading] = useState(false);
  
  const {
    register,
    handleSubmit,
    formState: { errors },
    watch,
    setValue,
  } = useForm<EditKeyFormData>({
    resolver: zodResolver(editKeySchema),
    defaultValues: {
      name: apiKey.name,
      description: apiKey.description || '',
      permissions: apiKey.permissions,
      expiresAt: apiKey.expiresAt ? new Date(apiKey.expiresAt).toISOString().split('T')[0] : '',
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

  const onSubmit = async (data: EditKeyFormData) => {
    setLoading(true);
    try {
      await onUpdate(apiKey.id, data);
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
            <h2 className="text-xl font-semibold text-gray-900">Edit API Key</h2>
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

          {/* Key Info */}
          <div className="bg-gray-50 border border-gray-200 rounded-md p-4">
            <div className="text-sm text-gray-600">
              <p className="font-medium mb-2">Key Information</p>
              <div className="space-y-1">
                <p><span className="font-medium">Key ID:</span> {apiKey.id}</p>
                <p><span className="font-medium">Key Prefix:</span> {apiKey.keyPrefix}••••••••••••••</p>
                <p><span className="font-medium">Status:</span> {apiKey.status}</p>
                <p><span className="font-medium">Created:</span> {new Date(apiKey.createdAt).toLocaleDateString()}</p>
                {apiKey.lastUsedAt && (
                  <p><span className="font-medium">Last Used:</span> {new Date(apiKey.lastUsedAt).toLocaleDateString()}</p>
                )}
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
              {loading ? 'Updating...' : 'Update API Key'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
