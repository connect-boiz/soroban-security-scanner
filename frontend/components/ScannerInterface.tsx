'use client';

import { useState } from 'react';
import { CloudArrowUpIcon, DocumentTextIcon } from '@heroicons/react/24/outline';

export default function ScannerInterface() {
  const [contractCode, setContractCode] = useState('');
  const [isScanning, setIsScanning] = useState(false);
  const [scanResults, setScanResults] = useState(null);

  const handleScan = async () => {
    if (!contractCode.trim()) return;
    
    setIsScanning(true);
    try {
      // TODO: Implement actual scan API call
      const response = await fetch('/api/scan', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ code: contractCode }),
      });
      const results = await response.json();
      setScanResults(results);
    } catch (error) {
      console.error('Scan failed:', error);
    } finally {
      setIsScanning(false);
    }
  };

  const handleFileUpload = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) {
      const reader = new FileReader();
      reader.onload = (e) => {
        setContractCode(e.target?.result as string);
      };
      reader.readAsText(file);
    }
  };

  return (
    <div className="card">
      <div className="mb-6">
        <h2 className="text-xl font-semibold text-gray-900 mb-2">
          Contract Scanner
        </h2>
        <p className="text-gray-600">
          Upload or paste your Soroban contract code for security analysis
        </p>
      </div>

      <div className="space-y-4">
        <div className="flex items-center space-x-4">
          <label className="btn-secondary cursor-pointer inline-flex items-center">
            <CloudArrowUpIcon className="h-5 w-5 mr-2" />
            Upload Contract
            <input
              type="file"
              accept=".rs,.txt"
              onChange={handleFileUpload}
              className="hidden"
            />
          </label>
          <span className="text-sm text-gray-500">or paste code below</span>
        </div>

        <div>
          <label htmlFor="contract-code" className="block text-sm font-medium text-gray-700 mb-2">
            Contract Code
          </label>
          <textarea
            id="contract-code"
            value={contractCode}
            onChange={(e) => setContractCode(e.target.value)}
            placeholder="Paste your Soroban contract code here..."
            className="input-field h-64 font-mono text-sm"
          />
        </div>

        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-4">
            <label className="flex items-center">
              <input type="checkbox" className="mr-2" defaultChecked />
              <span className="text-sm text-gray-600">Deep analysis</span>
            </label>
            <label className="flex items-center">
              <input type="checkbox" className="mr-2" defaultChecked />
              <span className="text-sm text-gray-600">Check invariants</span>
            </label>
          </div>
          
          <button
            onClick={handleScan}
            disabled={!contractCode.trim() || isScanning}
            className="btn-primary disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isScanning ? 'Scanning...' : 'Start Scan'}
          </button>
        </div>

        {scanResults && (
          <div className="mt-6 p-4 bg-gray-50 rounded-md">
            <h3 className="font-medium text-gray-900 mb-2">Scan Results</h3>
            <pre className="text-sm text-gray-600 overflow-x-auto">
              {JSON.stringify(scanResults, null, 2)}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}
