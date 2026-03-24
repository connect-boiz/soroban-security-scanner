'use client';

import React from 'react';
import { CheckCircle, XCircle, AlertTriangle, Info, Copy } from 'lucide-react';
import { ValidationResult } from '@/types/invariant';

interface ValidationPanelProps {
  result: ValidationResult;
}

const ValidationPanel: React.FC<ValidationPanelProps> = ({ result }) => {
  const [copiedConfig, setCopiedConfig] = React.useState(false);

  const handleCopyConfig = async () => {
    if (result.generatedConfig) {
      try {
        await navigator.clipboard.writeText(result.generatedConfig.content);
        setCopiedConfig(true);
        setTimeout(() => setCopiedConfig(false), 2000);
      } catch (error) {
        console.error('Failed to copy configuration:', error);
      }
    }
  };

  const getStatusIcon = (isValid: boolean) => {
    if (isValid) {
      return <CheckCircle className="h-5 w-5 text-green-600" />;
    } else {
      return <XCircle className="h-5 w-5 text-red-600" />;
    }
  };

  const getStatusColor = (isValid: boolean) => {
    if (isValid) {
      return 'bg-green-50 border-green-200 text-green-800';
    } else {
      return 'bg-red-50 border-red-200 text-red-800';
    }
  };

  return (
    <div className="card">
      <div className="p-6">
        {/* Header */}
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold text-gray-900">
            Validation Results
          </h3>
          <div className="flex items-center space-x-2">
            {getStatusIcon(result.isValid)}
            <span className={`px-2 py-1 text-sm font-medium rounded-full border ${getStatusColor(result.isValid)}`}>
              {result.isValid ? 'Valid' : 'Invalid'}
            </span>
          </div>
        </div>

        {/* Errors */}
        {result.errors.length > 0 && (
          <div className="mb-4">
            <h4 className="text-sm font-medium text-red-800 mb-2">
              Errors ({result.errors.length})
            </h4>
            <div className="space-y-2">
              {result.errors.map((error, index) => (
                <div key={index} className="flex items-start space-x-2 p-3 bg-red-50 border border-red-200 rounded-md">
                  <XCircle className="h-4 w-4 text-red-600 mt-0.5 flex-shrink-0" />
                  <p className="text-sm text-red-800">{error}</p>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Warnings */}
        {result.warnings.length > 0 && (
          <div className="mb-4">
            <h4 className="text-sm font-medium text-yellow-800 mb-2">
              Warnings ({result.warnings.length})
            </h4>
            <div className="space-y-2">
              {result.warnings.map((warning, index) => (
                <div key={index} className="flex items-start space-x-2 p-3 bg-yellow-50 border border-yellow-200 rounded-md">
                  <AlertTriangle className="h-4 w-4 text-yellow-600 mt-0.5 flex-shrink-0" />
                  <p className="text-sm text-yellow-800">{warning}</p>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Success Message */}
        {result.isValid && result.errors.length === 0 && result.warnings.length === 0 && (
          <div className="flex items-center space-x-2 p-3 bg-green-50 border border-green-200 rounded-md">
            <CheckCircle className="h-4 w-4 text-green-600 flex-shrink-0" />
            <p className="text-sm text-green-800">
              Rule validation passed successfully! Your invariant rule is syntactically correct and ready to use.
            </p>
          </div>
        )}

        {/* Generated Configuration */}
        {result.generatedConfig && (
          <div className="mt-4">
            <div className="flex items-center justify-between mb-2">
              <h4 className="text-sm font-medium text-gray-900">
                Generated Configuration ({result.generatedConfig.format.toUpperCase()})
              </h4>
              <button
                onClick={handleCopyConfig}
                className="flex items-center space-x-1 text-xs text-primary-600 hover:text-primary-700"
              >
                <Copy className="h-3 w-3" />
                {copiedConfig ? 'Copied!' : 'Copy'}
              </button>
            </div>
            <div className="bg-gray-50 border border-gray-200 rounded-md p-3">
              <pre className="text-xs text-gray-700 font-mono overflow-x-auto whitespace-pre-wrap">
                {result.generatedConfig.content}
              </pre>
            </div>
            
            {/* Configuration Info */}
            <div className="mt-2 flex items-center space-x-2 text-xs text-gray-500">
              <Info className="h-3 w-3" />
              <span>This configuration can be saved to your project or exported for use in the scanner</span>
            </div>
          </div>
        )}

        {/* Validation Tips */}
        <div className="mt-4 p-3 bg-blue-50 border border-blue-200 rounded-md">
          <div className="flex items-start space-x-2">
            <Info className="h-4 w-4 text-blue-600 mt-0.5 flex-shrink-0" />
            <div className="text-sm text-blue-800">
              <p className="font-medium mb-1">Validation Tips:</p>
              <ul className="text-xs space-y-1 ml-4">
                <li>• Ensure all conditions have valid variables and values</li>
                <li>• Check that logical operators make sense for your use case</li>
                <li>• Verify that numeric expressions are mathematically valid</li>
                <li>• Make sure Stellar addresses are properly formatted (starts with 'G')</li>
              </ul>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ValidationPanel;
