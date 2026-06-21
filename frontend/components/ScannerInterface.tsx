'use client';

import { useState, useCallback, useMemo } from 'react';
import LazyImage from './LazyImage';
import {
  LoadingOverlay,
  ProgressBar,
  SkeletonCard,
  EnhancedProgressBar,
  MultiStepProgress,
} from './ui';
import HelpIcon from './help/HelpIcon';
import { HELP_CONTENT } from '../lib/help-content';
import FileUploadZone from './FileUploadZone';

type InputMode = 'paste' | 'upload';

interface ScanResult {
  vulnerabilities: string[];
  severity: 'low' | 'medium' | 'high' | 'critical';
  contractAddress: string;
}

export default function ScannerInterface() {
  const [inputMode, setInputMode] = useState<InputMode>('paste');
  const [contractCode, setContractCode] = useState('');
  const [uploadedFiles, setUploadedFiles] = useState<File[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [scanResults, setScanResults] = useState<ScanResult | null>(null);
  const [scanProgress, setScanProgress] = useState(0);
  const [scanStage, setScanStage] = useState('');

  const canScan = inputMode === 'paste' ? contractCode.trim().length > 0 : uploadedFiles.length > 0;

  const handleScan = useCallback(async () => {
    if (!canScan) return;

    setIsScanning(true);
    setScanProgress(0);
    setScanResults(null);
    setScanStage('Initializing scan...');

    const stages = [
      { name: 'Validating contract code...', duration: 500, progress: 20 },
      { name: 'Analyzing bytecode...', duration: 800, progress: 40 },
      { name: 'Checking for vulnerabilities...', duration: 1200, progress: 70 },
      { name: 'Generating report...', duration: 500, progress: 90 },
      { name: 'Finalizing results...', duration: 300, progress: 100 },
    ];

    for (const stage of stages) {
      setScanStage(stage.name);
      await new Promise(resolve => setTimeout(resolve, stage.duration));
      setScanProgress(stage.progress);
    }

    setScanResults({
      vulnerabilities: [
        'Potential reentrancy vulnerability detected',
        'Missing input validation in function transfer',
        'Unprotected external call',
      ],
      severity: 'high',
      contractAddress: '0x1234...5678',
    });

    setIsScanning(false);
    setScanStage('');
    setScanProgress(0);
  }, [canScan]);

  const severityColors = useMemo(
    () => ({
      low: 'bg-green-100 text-green-800',
      medium: 'bg-yellow-100 text-yellow-800',
      high: 'bg-red-100 text-red-800',
      critical: 'bg-purple-100 text-purple-800',
    }),
    []
  );

  return (
    <LoadingOverlay isLoading={isScanning && scanProgress === 0} text="Initializing scanner...">
      <div className="bg-white rounded-lg shadow-md p-6 space-y-6">
        <div className="flex items-center space-x-4">
          <LazyImage
            src="/scanner-icon.svg"
            alt="Scanner Icon"
            className="w-12 h-12"
            width={48}
            height={48}
          />
          <h2 className="text-xl font-semibold text-gray-900">Contract Scanner</h2>
        </div>

        {/* Input mode toggle */}
        <div className="flex rounded-lg border border-gray-200 overflow-hidden w-fit">
          {(['paste', 'upload'] as InputMode[]).map(mode => (
            <button
              key={mode}
              onClick={() => setInputMode(mode)}
              disabled={isScanning}
              className={`px-5 py-2 text-sm font-medium transition-colors ${
                inputMode === mode
                  ? 'bg-blue-600 text-white'
                  : 'bg-white text-gray-600 hover:bg-gray-50'
              }`}
            >
              {mode === 'paste' ? '📋 Paste Code' : '📁 Upload Files'}
            </button>
          ))}
        </div>

        <div className="space-y-4">
          {inputMode === 'paste' ? (
            <div>
              <label
                htmlFor="contract-code"
                className="flex items-center gap-1 text-sm font-medium text-gray-700 mb-2"
              >
                Contract Code
                <HelpIcon content={HELP_CONTENT.scan.contractId} label="Contract ID" />
              </label>
              <textarea
                id="contract-code"
                value={contractCode}
                onChange={e => setContractCode(e.target.value)}
                className="w-full h-32 p-3 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500 transition-optimized"
                placeholder="Paste your Soroban contract code here..."
                disabled={isScanning}
              />
            </div>
          ) : (
            <div>
              <label className="flex items-center gap-1 text-sm font-medium text-gray-700 mb-2">
                Upload Files
                <HelpIcon content={HELP_CONTENT.scan.contractId} label="Contract Files" />
              </label>
              <FileUploadZone
                allowedTypes={['.rs', '.wasm', '.toml']}
                maxSizeMB={10}
                maxFiles={5}
                onFilesReady={files => setUploadedFiles(files)}
              />
            </div>
          )}

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label
                htmlFor="vulnerability-types"
                className="flex items-center gap-1 text-sm font-medium text-gray-700 mb-2"
              >
                Vulnerability Types
                <HelpIcon
                  content={HELP_CONTENT.scan.vulnerabilityTypes}
                  label="Vulnerability Types"
                />
              </label>
              <select
                id="vulnerability-types"
                className="w-full p-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500"
                disabled={isScanning}
              >
                <option>All Categories</option>
                <option>Arithmetic Overflows</option>
                <option>Reentrancy</option>
                <option>Access Control</option>
                <option>Logic Flaws</option>
              </select>
            </div>

            <div>
              <label
                htmlFor="scan-depth"
                className="flex items-center gap-1 text-sm font-medium text-gray-700 mb-2"
              >
                Scan Depth
                <HelpIcon content={HELP_CONTENT.scan.scanDepth} label="Scan Depth" />
              </label>
              <select
                id="scan-depth"
                className="w-full p-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500"
                disabled={isScanning}
              >
                <option>Basic (Fast)</option>
                <option>Deep (Symbolic)</option>
                <option>Exhaustive (Formal)</option>
              </select>
            </div>
          </div>

          <button
            id="submit-scan-btn"
            onClick={handleScan}
            disabled={isScanning || !canScan}
            className="w-full bg-blue-600 text-white py-2 px-4 rounded-md hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed transition-optimized font-bold"
          >
            {isScanning
              ? 'Scanning…'
              : inputMode === 'upload' && uploadedFiles.length > 0
                ? `Scan ${uploadedFiles.length} File${uploadedFiles.length !== 1 ? 's' : ''}`
                : 'Scan Contract'}
          </button>
        </div>

        {/* Enhanced Progress Section */}
        {isScanning && scanProgress > 0 && (
          <div className="space-y-6 border-t pt-6">
            <div className="flex items-center justify-between">
              <h3 className="text-lg font-medium text-gray-900">Scan Progress</h3>
              <span className="text-sm text-gray-600">{scanProgress}%</span>
            </div>

            {/* Multi-step progress indicator */}
            <div className="space-y-4">
              <MultiStepProgress
                steps={[
                  {
                    name: 'Validation',
                    completed: scanProgress >= 20,
                    current: scanProgress > 0 && scanProgress < 20,
                  },
                  {
                    name: 'Analysis',
                    completed: scanProgress >= 40,
                    current: scanProgress >= 20 && scanProgress < 40,
                  },
                  {
                    name: 'Vulnerability Check',
                    completed: scanProgress >= 70,
                    current: scanProgress >= 40 && scanProgress < 70,
                  },
                  {
                    name: 'Report Generation',
                    completed: scanProgress >= 90,
                    current: scanProgress >= 70 && scanProgress < 90,
                  },
                  {
                    name: 'Finalization',
                    completed: scanProgress >= 100,
                    current: scanProgress >= 90 && scanProgress < 100,
                  },
                ]}
              />
            </div>

            {/* Enhanced progress bar with stages */}
            <EnhancedProgressBar
              value={scanProgress}
              color="blue"
              showLabel={true}
              showPercentage={true}
              animated={true}
              striped={true}
              stages={[
                { name: 'Contract Validation', value: 20, completed: scanProgress >= 20 },
                { name: 'Bytecode Analysis', value: 40, completed: scanProgress >= 40 },
                { name: 'Vulnerability Detection', value: 70, completed: scanProgress >= 70 },
                { name: 'Report Generation', value: 90, completed: scanProgress >= 90 },
                { name: 'Finalization', value: 100, completed: scanProgress >= 100 },
              ]}
            />

            <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
              <div className="flex items-center space-x-3">
                <div className="w-4 h-4 bg-blue-500 rounded-full animate-pulse" />
                <p className="text-sm text-blue-700 font-medium">{scanStage}</p>
              </div>
            </div>
          </div>
        )}

        {scanResults && (
          <div className="border-t pt-6 space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="text-lg font-medium text-gray-900">Scan Results</h3>
              <span
                className={`px-3 py-1 rounded-full text-sm font-medium ${severityColors[scanResults.severity]}`}
              >
                {scanResults.severity.toUpperCase()}
              </span>
            </div>

            <div className="space-y-2">
              <p className="text-sm text-gray-600">
                Contract:{' '}
                <code className="bg-gray-100 px-2 py-1 rounded">{scanResults.contractAddress}</code>
              </p>

              <div className="space-y-2">
                <h4 className="text-sm font-medium text-gray-700">Detected Issues:</h4>
                <ul className="space-y-1">
                  {scanResults.vulnerabilities.map((vulnerability, index) => (
                    <li key={index} className="flex items-start space-x-2 text-sm text-gray-600">
                      <span className="text-red-500 mt-0.5">•</span>
                      <span>{vulnerability}</span>
                    </li>
                  ))}
                </ul>
              </div>
            </div>
          </div>
        )}

        {/* Enhanced skeleton for loading results */}
        {isScanning && scanProgress > 70 && (
          <div className="border-t pt-6">
            <div className="space-y-4">
              <div className="flex items-center space-x-3">
                <div className="w-6 h-6 bg-gray-200 rounded-full animate-pulse" />
                <div className="h-5 bg-gray-200 rounded animate-pulse w-48" />
              </div>
              <div className="bg-gray-50 border border-gray-200 rounded-lg p-4">
                <div className="space-y-3">
                  <div className="h-4 bg-gray-200 rounded animate-pulse w-32" />
                  <div className="space-y-2">
                    <div className="h-3 bg-gray-200 rounded animate-pulse w-full" />
                    <div className="h-3 bg-gray-200 rounded animate-pulse w-5/6" />
                    <div className="h-3 bg-gray-200 rounded animate-pulse w-4/5" />
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </LoadingOverlay>
  );
}
