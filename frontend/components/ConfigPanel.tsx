'use client';

import React from 'react';
import { X, Download, Upload, Copy, Eye, EyeOff, FileText, Code } from 'lucide-react';
import { useInvariantStore } from '@/store/invariantStore';

interface ConfigPanelProps {
  onClose: () => void;
  onExport: (format: 'json' | 'yaml') => void;
}

const ConfigPanel: React.FC<ConfigPanelProps> = ({ onClose, onExport }) => {
  const { builderState, generateConfig } = useInvariantStore();
  const [format, setFormat] = React.useState<'json' | 'yaml'>('json');
  const [showPreview, setShowPreview] = React.useState(true);
  const [copiedConfig, setCopiedConfig] = React.useState(false);

  const generatedConfig = React.useMemo(() => {
    return generateConfig(format);
  }, [builderState, format, generateConfig]);

  const handleCopyConfig = async () => {
    try {
      await navigator.clipboard.writeText(generatedConfig);
      setCopiedConfig(true);
      setTimeout(() => setCopiedConfig(false), 2000);
    } catch (error) {
      console.error('Failed to copy configuration:', error);
    }
  };

  const handleImportConfig = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) {
      const reader = new FileReader();
      reader.onload = (e) => {
        try {
          const content = e.target?.result as string;
          // TODO: Parse and import configuration
          console.log('Imported config:', content);
        } catch (error) {
          console.error('Failed to import configuration:', error);
        }
      };
      reader.readAsText(file);
    }
  };

  const getLanguageIcon = (format: string) => {
    return format === 'json' ? <Code className="h-4 w-4" /> : <FileText className="h-4 w-4" />;
  };

  const getSyntaxHighlight = (content: string, format: string) => {
    // Simple syntax highlighting for preview
    if (format === 'json') {
      return content
        .replace(/"([^"]+)":/g, '<span class="text-blue-600">"$1"</span>:')
        .replace(/: "([^"]+)"/g, ': <span class="text-green-600">"$1"</span>')
        .replace(/: (\d+)/g, ': <span class="text-orange-600">$1</span>')
        .replace(/: (true|false)/g, ': <span class="text-purple-600">$1</span>');
    } else {
      return content
        .replace(/^(\w+):/gm, '<span class="text-blue-600">$1</span>:')
        .replace(/: "([^"]+)"/g, ': <span class="text-green-600">"$1"</span>')
        .replace(/: (\d+)/g, ': <span class="text-orange-600">$1</span>');
    }
  };

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg shadow-xl max-w-4xl w-full max-h-[90vh] overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-gray-200">
          <div className="flex items-center space-x-3">
            <Code className="h-6 w-6 text-primary-600" />
            <h2 className="text-xl font-semibold text-gray-900">
              Configuration Panel
            </h2>
          </div>
          <button
            onClick={onClose}
            className="p-2 text-gray-400 hover:text-gray-600 transition-colors"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        <div className="p-6">
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            {/* Left Panel - Controls */}
            <div className="space-y-6">
              {/* Format Selection */}
              <div>
                <h3 className="text-lg font-medium text-gray-900 mb-3">
                  Export Format
                </h3>
                <div className="flex space-x-2">
                  <button
                    onClick={() => setFormat('json')}
                    className={`flex-1 flex items-center justify-center space-x-2 px-4 py-2 rounded-md border transition-colors ${
                      format === 'json'
                        ? 'bg-primary-600 text-white border-primary-600'
                        : 'bg-white text-gray-700 border-gray-300 hover:bg-gray-50'
                    }`}
                  >
                    {getLanguageIcon('json')}
                    <span>JSON</span>
                  </button>
                  <button
                    onClick={() => setFormat('yaml')}
                    className={`flex-1 flex items-center justify-center space-x-2 px-4 py-2 rounded-md border transition-colors ${
                      format === 'yaml'
                        ? 'bg-primary-600 text-white border-primary-600'
                        : 'bg-white text-gray-700 border-gray-300 hover:bg-gray-50'
                    }`}
                  >
                    {getLanguageIcon('yaml')}
                    <span>YAML</span>
                  </button>
                </div>
              </div>

              {/* Export Actions */}
              <div>
                <h3 className="text-lg font-medium text-gray-900 mb-3">
                  Export Actions
                </h3>
                <div className="space-y-3">
                  <button
                    onClick={() => onExport(format)}
                    className="w-full btn-primary flex items-center justify-center space-x-2"
                  >
                    <Download className="h-4 w-4" />
                    <span>Download {format.toUpperCase()}</span>
                  </button>
                  
                  <button
                    onClick={handleCopyConfig}
                    className="w-full btn-secondary flex items-center justify-center space-x-2"
                  >
                    <Copy className="h-4 w-4" />
                    <span>{copiedConfig ? 'Copied!' : 'Copy to Clipboard'}</span>
                  </button>

                  <div className="relative">
                    <input
                      type="file"
                      accept=".json,.yaml,.yml"
                      onChange={handleImportConfig}
                      className="hidden"
                      id="import-config"
                    />
                    <label
                      htmlFor="import-config"
                      className="w-full btn-secondary flex items-center justify-center space-x-2 cursor-pointer"
                    >
                      <Upload className="h-4 w-4" />
                      <span>Import Configuration</span>
                    </label>
                  </div>
                </div>
              </div>

              {/* Rule Statistics */}
              <div>
                <h3 className="text-lg font-medium text-gray-900 mb-3">
                  Rule Statistics
                </h3>
                <div className="bg-gray-50 rounded-lg p-4 space-y-2">
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-600">Conditions:</span>
                    <span className="font-medium">{builderState.conditions.length}</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-600">Logic Operator:</span>
                    <span className="font-medium">{builderState.logicOperator}</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-600">Format:</span>
                    <span className="font-medium">{format.toUpperCase()}</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-600">Size:</span>
                    <span className="font-medium">{generatedConfig.length} characters</span>
                  </div>
                </div>
              </div>

              {/* Preview Toggle */}
              <div>
                <button
                  onClick={() => setShowPreview(!showPreview)}
                  className="w-full btn-secondary flex items-center justify-center space-x-2"
                >
                  {showPreview ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                  <span>{showPreview ? 'Hide' : 'Show'} Preview</span>
                </button>
              </div>
            </div>

            {/* Right Panel - Configuration Preview */}
            <div>
              <h3 className="text-lg font-medium text-gray-900 mb-3">
                Configuration Preview
              </h3>
              <div className="bg-gray-50 border border-gray-200 rounded-lg">
                <div className="flex items-center justify-between px-4 py-2 border-b border-gray-200">
                  <div className="flex items-center space-x-2">
                    {getLanguageIcon(format)}
                    <span className="text-sm font-medium text-gray-700">
                      {format.toUpperCase()} Configuration
                    </span>
                  </div>
                  <button
                    onClick={handleCopyConfig}
                    className="text-xs text-primary-600 hover:text-primary-700"
                  >
                    {copiedConfig ? 'Copied!' : 'Copy'}
                  </button>
                </div>
                
                {showPreview && (
                  <div className="p-4 max-h-96 overflow-y-auto">
                    <pre className="text-xs text-gray-700 font-mono whitespace-pre-wrap">
                      <div
                        dangerouslySetInnerHTML={{
                          __html: getSyntaxHighlight(generatedConfig, format)
                        }}
                      />
                    </pre>
                  </div>
                )}
              </div>
            </div>
          </div>

          {/* Usage Instructions */}
          <div className="mt-6 p-4 bg-blue-50 border border-blue-200 rounded-lg">
            <div className="flex items-start space-x-2">
              <div className="w-5 h-5 text-blue-600 mt-0.5 flex-shrink-0">
                <svg fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clipRule="evenodd" />
                </svg>
              </div>
              <div className="text-sm text-blue-800">
                <p className="font-medium mb-1">Usage Instructions:</p>
                <ul className="space-y-1 text-xs">
                  <li>• Export the configuration to use with the Soroban Security Scanner</li>
                  <li>• JSON format is recommended for programmatic integration</li>
                  <li>• YAML format is more human-readable for documentation</li>
                  <li>• Import existing configurations to modify or extend them</li>
                </ul>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ConfigPanel;
