'use client';

import { useState } from 'react';
import { CloudArrowUpIcon, DocumentTextIcon, CheckCircleIcon } from '@heroicons/react/24/outline';
import ScanProgress from './ScanProgress';

export default function ScannerInterface() {
  const [contractCode, setContractCode] = useState('');
  const [isScanning, setIsScanning] = useState(false);
  const [scanResults, setScanResults] = useState(null);
  const [currentScanId, setCurrentScanId] = useState<string | null>(null);

  const handleScan = async () => {
    if (!contractCode.trim()) return;
    
    setIsScanning(true);
    setScanResults(null);
    
    try {
      // Start scan and get scan ID
      const response = await fetch('/api/scan', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ 
          code: contractCode,
          options: {
            deepAnalysis: true,
            checkInvariants: true
          }
        }),
      });
      
      if (!response.ok) {
        throw new Error('Failed to start scan');
      }
      
      const data = await response.json();
      setCurrentScanId(data.scanId);
    } catch (error) {
      console.error('Scan failed:', error);
      setIsScanning(false);
      setCurrentScanId(null);
    }
  };

  const handleScanComplete = (results: any) => {
    setScanResults(results);
    setIsScanning(false);
    setCurrentScanId(null);
  };

  const handleScanError = (error: string) => {
    console.error('Scan error:', error);
    setIsScanning(false);
    setCurrentScanId(null);
  };

  const handleReset = () => {
    setContractCode('');
    setScanResults(null);
    setCurrentScanId(null);
    setIsScanning(false);
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
    <div className="space-y-6">
      {/* Main Scanner Interface */}
      {!isScanning && !currentScanId && (
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
                disabled={!contractCode.trim()}
                className="btn-primary disabled:opacity-50 disabled:cursor-not-allowed"
              >
                Start Scan
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Scan Progress Component */}
      {isScanning && currentScanId && (
        <ScanProgress
          scanId={currentScanId}
          onComplete={handleScanComplete}
          onError={handleScanError}
        />
      )}

      {/* Scan Results */}
      {scanResults && (
        <div className="card">
          <div className="mb-6">
            <div className="flex items-center justify-between">
              <h3 className="text-lg font-semibold text-gray-900">Scan Results</h3>
              <button
                onClick={handleReset}
                className="btn-secondary"
              >
                New Scan
              </button>
            </div>
          </div>
          
          <div className="space-y-4">
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div className="bg-gray-50 p-4 rounded-md">
                <h4 className="font-medium text-gray-900 mb-1">Total Vulnerabilities</h4>
                <p className="text-2xl font-bold text-gray-900">
                  {scanResults.vulnerabilities?.length || 0}
                </p>
              </div>
              <div className="bg-gray-50 p-4 rounded-md">
                <h4 className="font-medium text-gray-900 mb-1">Risk Score</h4>
                <p className="text-2xl font-bold text-gray-900">
                  {scanResults.metrics?.riskScore || 0}
                </p>
              </div>
              <div className="bg-gray-50 p-4 rounded-md">
                <h4 className="font-medium text-gray-900 mb-1">Scan Time</h4>
                <p className="text-2xl font-bold text-gray-900">
                  {scanResults.scanTime ? `${(scanResults.scanTime / 1000).toFixed(1)}s` : 'N/A'}
                </p>
              </div>
            </div>

            {scanResults.vulnerabilities && scanResults.vulnerabilities.length > 0 && (
              <div className="mt-6">
                <h4 className="font-medium text-gray-900 mb-3">Vulnerabilities Found</h4>
                <div className="space-y-3">
                  {scanResults.vulnerabilities.map((vuln: any, index: number) => (
                    <div key={index} className="border border-gray-200 rounded-md p-4">
                      <div className="flex items-start justify-between">
                        <div>
                          <h5 className="font-medium text-gray-900">{vuln.title}</h5>
                          <p className="text-sm text-gray-600 mt-1">{vuln.description}</p>
                          <div className="flex items-center space-x-2 mt-2">
                            <span className={`px-2 py-1 text-xs font-medium rounded-md ${
                              vuln.severity === 'critical' ? 'bg-red-100 text-red-800' :
                              vuln.severity === 'high' ? 'bg-orange-100 text-orange-800' :
                              vuln.severity === 'medium' ? 'bg-yellow-100 text-yellow-800' :
                              'bg-gray-100 text-gray-800'
                            }`}>
                              {vuln.severity.toUpperCase()}
                            </span>
                            <span className="text-xs text-gray-500">{vuln.type}</span>
                          </div>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {(!scanResults.vulnerabilities || scanResults.vulnerabilities.length === 0) && (
              <div className="mt-6 p-4 bg-green-50 border border-green-200 rounded-md">
                <div className="flex items-center space-x-3">
                  <CheckCircleIcon className="h-5 w-5 text-green-600" />
                  <div>
                    <h4 className="font-semibold text-green-800">No Vulnerabilities Found</h4>
                    <p className="text-green-600 text-sm">
                      Your contract passed all security checks! 🎉
                    </p>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
