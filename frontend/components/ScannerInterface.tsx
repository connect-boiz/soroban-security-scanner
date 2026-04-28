'use client';

import { useState, useCallback, useMemo } from 'react';
import LazyImage from './LazyImage';

interface ScanResult {
  vulnerabilities: string[];
  severity: 'low' | 'medium' | 'high' | 'critical';
  contractAddress: string;
}

export default function ScannerInterface() {
  const [contractCode, setContractCode] = useState('');
  const [isScanning, setIsScanning] = useState(false);
  const [scanResults, setScanResults] = useState<ScanResult | null>(null);

  // Memoize the scan function to prevent unnecessary re-renders
  const handleScan = useCallback(async () => {
    if (!contractCode.trim()) return;

    setIsScanning(true);
    
    // Simulate API call with delay
    await new Promise(resolve => setTimeout(resolve, 2000));
    
    // Mock scan results
    setScanResults({
      vulnerabilities: [
        'Potential reentrancy vulnerability detected',
        'Missing input validation in function transfer',
        'Unprotected external call'
      ],
      severity: 'high',
      contractAddress: '0x1234...5678'
    });
    
    setIsScanning(false);
  }, [contractCode]);

  // Memoize severity color mapping
  const severityColors = useMemo(() => ({
    low: 'bg-green-100 text-green-800',
    medium: 'bg-yellow-100 text-yellow-800',
    high: 'bg-red-100 text-red-800',
    critical: 'bg-purple-100 text-purple-800'
  }), []);

  return (
    <div className="card animate-fade-in space-y-6">
      <div className="flex items-center space-x-4">
        <LazyImage
          src="/scanner-icon.png"
          alt="Scanner Icon"
          className="w-12 h-12 rounded-lg"
          width={48}
          height={48}
        />
        <h2 className="text-xl font-bold">
          Contract Scanner
        </h2>
      </div>

      <div className="space-y-4">
        <div>
          <label htmlFor="contract-code" className="block text-sm font-medium text-gray-700 mb-2">
            Contract Code
          </label>
          <textarea
            id="contract-code"
            value={contractCode}
            onChange={(e) => setContractCode(e.target.value)}
            className="w-full h-32 p-3 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500 transition-optimized"
            placeholder="Paste your Soroban contract code here..."
          />
        </div>

        <button
          onClick={handleScan}
          disabled={isScanning || !contractCode.trim()}
          className="w-full bg-blue-600 text-white py-2 px-4 rounded-md hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed transition-optimized"
        >
          {isScanning ? 'Scanning...' : 'Scan Contract'}
        </button>
      </div>

      {scanResults && (
        <div className="border-t pt-6 space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-lg font-medium text-gray-900">Scan Results</h3>
            <span className={`px-3 py-1 rounded-full text-sm font-medium ${severityColors[scanResults.severity]}`}>
              {scanResults.severity.toUpperCase()}
            </span>
          </div>

          <div className="space-y-2">
            <p className="text-sm text-gray-600">
              Contract: <code className="bg-gray-100 px-2 py-1 rounded">{scanResults.contractAddress}</code>
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
    </div>
  );
}
