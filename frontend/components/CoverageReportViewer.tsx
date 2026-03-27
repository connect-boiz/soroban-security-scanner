'use client';

import { useState, useEffect, useCallback } from 'react';
import { 
  DocumentArrowDownIcon, 
  ChartBarIcon,
  ExclamationTriangleIcon,
  CheckCircleIcon,
  XMarkIcon,
  ArrowDownTrayIcon
} from '@heroicons/react/24/outline';
import CoverageHeatmap from './CoverageHeatmap';

interface CoverageData {
  lines_hit: Record<number, number>;
  branches_hit: Record<number, boolean[]>;
  functions_hit: Record<string, number>;
  total_instructions: number;
  executed_instructions: number;
}

interface FuzzerInput {
  values: any[];
  iteration: number;
  timestamp: string;
}

interface VulnerabilityData {
  id: string;
  title: string;
  severity: 'critical' | 'high' | 'medium' | 'low';
  location: {
    file: string;
    line: number;
  };
  found_at_line: number;
  coverage_at_discovery?: 'covered' | 'partial' | 'not-hit';
}

interface CoverageReportViewerProps {
  scanId: string;
  coverageData: CoverageData;
  vulnerabilities: VulnerabilityData[];
  fileContents: Record<string, string>;
  onClose?: () => void;
}

export default function CoverageReportViewer({ 
  scanId, 
  coverageData, 
  vulnerabilities, 
  fileContents,
  onClose 
}: CoverageReportViewerProps) {
  const [selectedFile, setSelectedFile] = useState<string>('');
  const [activeTab, setActiveTab] = useState<'heatmap' | 'correlation'>('heatmap');
  const [showExportModal, setShowExportModal] = useState(false);

  useEffect(() => {
    const files = Object.keys(fileContents);
    if (files.length > 0 && !selectedFile) {
      setSelectedFile(files[0]);
    }
  }, [fileContents, selectedFile]);

  const calculateOverallCoverage = useCallback(() => {
    const totalLines = Object.values(fileContents).reduce((sum, content) => 
      sum + content.split('\n').length, 0);
    const coveredLines = Object.keys(coverageData.lines_hit).length;
    return totalLines > 0 ? Math.round((coveredLines / totalLines) * 100) : 0;
  }, [coverageData, fileContents]);

  const calculateBranchCoverage = useCallback(() => {
    let totalBranches = 0;
    let coveredBranches = 0;
    
    Object.values(coverageData.branches_hit).forEach(branches => {
      totalBranches += branches.length;
      coveredBranches += branches.filter(b => b).length;
    });
    
    return totalBranches > 0 ? Math.round((coveredBranches / totalBranches) * 100) : 0;
  }, [coverageData]);

  const getVulnerabilityCoverageCorrelation = useCallback(() => {
    return vulnerabilities.map(vuln => {
      const lineCoverage = coverageData.lines_hit[vuln.found_at_line] || 0;
      const branches = coverageData.branches_hit[vuln.found_at_line];
      
      let coverageStatus: 'covered' | 'partial' | 'not-hit' = 'not-hit';
      if (lineCoverage > 0) {
        if (branches && branches.length > 1 && branches.some(b => !b)) {
          coverageStatus = 'partial';
        } else {
          coverageStatus = 'covered';
        }
      }
      
      return {
        ...vuln,
        coverage_at_discovery: coverageStatus,
        hit_count: lineCoverage
      };
    });
  }, [coverageData, vulnerabilities]);

  const exportCoverageReport = useCallback(() => {
    setShowExportModal(true);
    
    const report = {
      scan_id: scanId,
      timestamp: new Date().toISOString(),
      overall_coverage: calculateOverallCoverage(),
      branch_coverage: calculateBranchCoverage(),
      coverage_data: coverageData,
      vulnerability_correlation: getVulnerabilityCoverageCorrelation(),
      recommendations: generateCoverageRecommendations()
    };
    
    // Create and download JSON report
    const blob = new Blob([JSON.stringify(report, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `coverage-report-${scanId}.json`;
    a.click();
    URL.revokeObjectURL(url);
    
    setTimeout(() => setShowExportModal(false), 2000);
  }, [scanId, coverageData, calculateOverallCoverage, calculateBranchCoverage]);

  const generateCoverageRecommendations = useCallback(() => {
    const overallCoverage = calculateOverallCoverage();
    const branchCoverage = calculateBranchCoverage();
    const recommendations = [];
    
    if (overallCoverage < 60) {
      recommendations.push({
        type: 'critical',
        message: 'Overall line coverage is below 60%. Consider adding more comprehensive test cases.',
        priority: 'high'
      });
    }
    
    if (branchCoverage < 50) {
      recommendations.push({
        type: 'branch',
        message: 'Branch coverage is below 50%. Test both true and false paths in conditional statements.',
        priority: 'medium'
      });
    }
    
    const criticalFunctions = Object.entries(coverageData.functions_hit)
      .filter(([_, hits]) => hits === 0)
      .map(([name]) => name);
    
    if (criticalFunctions.length > 0) {
      recommendations.push({
        type: 'function',
        message: `Critical functions not tested: ${criticalFunctions.join(', ')}`,
        priority: 'high'
      });
    }
    
    return recommendations;
  }, [calculateOverallCoverage, calculateBranchCoverage, coverageData]);

  const overallCoverage = calculateOverallCoverage();
  const branchCoverage = calculateBranchCoverage();
  const correlatedVulnerabilities = getVulnerabilityCoverageCorrelation();

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white rounded-xl shadow-2xl w-full h-full max-w-7xl max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="border-b border-gray-200 px-6 py-4 flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold text-gray-900">Coverage Report Viewer</h1>
            <p className="text-sm text-gray-500 mt-1">Scan ID: {scanId}</p>
          </div>
          <div className="flex items-center space-x-4">
            <button
              onClick={exportCoverageReport}
              className="flex items-center space-x-2 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors"
            >
              <ArrowDownTrayIcon className="h-4 w-4" />
              <span>Export Report</span>
            </button>
            {onClose && (
              <button
                onClick={onClose}
                className="p-2 hover:bg-gray-100 rounded-lg transition-colors"
              >
                <XMarkIcon className="h-5 w-5" />
              </button>
            )}
          </div>
        </div>

        {/* Summary Stats */}
        <div className="border-b border-gray-200 px-6 py-4 bg-gray-50">
          <div className="grid grid-cols-4 gap-4">
            <div className="text-center">
              <div className={`text-2xl font-bold ${
                overallCoverage >= 80 ? 'text-green-600' :
                overallCoverage >= 60 ? 'text-yellow-600' : 'text-red-600'
              }`}>
                {overallCoverage}%
              </div>
              <div className="text-sm text-gray-500">Line Coverage</div>
            </div>
            <div className="text-center">
              <div className={`text-2xl font-bold ${
                branchCoverage >= 80 ? 'text-green-600' :
                branchCoverage >= 60 ? 'text-yellow-600' : 'text-red-600'
              }`}>
                {branchCoverage}%
              </div>
              <div className="text-sm text-gray-500">Branch Coverage</div>
            </div>
            <div className="text-center">
              <div className="text-2xl font-bold text-blue-600">
                {Object.keys(coverageData.functions_hit).length}
              </div>
              <div className="text-sm text-gray-500">Functions Hit</div>
            </div>
            <div className="text-center">
              <div className="text-2xl font-bold text-purple-600">
                {vulnerabilities.length}
              </div>
              <div className="text-sm text-gray-500">Vulnerabilities Found</div>
            </div>
          </div>
        </div>

        {/* Tabs */}
        <div className="border-b border-gray-200">
          <div className="flex">
            <button
              onClick={() => setActiveTab('heatmap')}
              className={`px-6 py-3 font-medium text-sm ${
                activeTab === 'heatmap' 
                  ? 'border-b-2 border-blue-500 text-blue-600' 
                  : 'text-gray-500 hover:text-gray-700'
              }`}
            >
              <ChartBarIcon className="h-4 w-4 inline mr-2" />
              Coverage Heatmap
            </button>
            <button
              onClick={() => setActiveTab('correlation')}
              className={`px-6 py-3 font-medium text-sm ${
                activeTab === 'correlation' 
                  ? 'border-b-2 border-blue-500 text-blue-600' 
                  : 'text-gray-500 hover:text-gray-700'
              }`}
            >
              <ExclamationTriangleIcon className="h-4 w-4 inline mr-2" />
              Vulnerability Correlation
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-hidden">
          {activeTab === 'heatmap' && (
            <div className="h-full flex flex-col">
              {/* File Selector */}
              <div className="border-b border-gray-200 p-4 bg-white">
                <label className="block text-sm font-medium text-gray-700 mb-2">Select File:</label>
                <select
                  value={selectedFile}
                  onChange={(e) => setSelectedFile(e.target.value)}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                >
                  {Object.keys(fileContents).map(file => (
                    <option key={file} value={file}>{file}</option>
                  ))}
                </select>
              </div>
              
              {/* Heatmap */}
              {selectedFile && fileContents[selectedFile] && (
                <div className="flex-1">
                  <CoverageHeatmap
                    fileContent={fileContents[selectedFile]}
                    coverageData={coverageData}
                    fileName={selectedFile}
                  />
                </div>
              )}
            </div>
          )}

          {activeTab === 'correlation' && (
            <div className="p-6 overflow-y-auto h-full">
              <h3 className="text-lg font-semibold text-gray-900 mb-4">Coverage vs Vulnerability Correlation</h3>
              
              {/* Correlation Analysis */}
              <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-6">
                <h4 className="font-medium text-blue-900 mb-2">Analysis Summary</h4>
                <div className="grid grid-cols-2 gap-4 text-sm text-blue-800">
                  <div>
                    <strong>Vulnerabilities in Covered Code:</strong> {correlatedVulnerabilities.filter(v => v.coverage_at_discovery === 'covered').length}
                  </div>
                  <div>
                    <strong>Vulnerabilities in Partial Coverage:</strong> {correlatedVulnerabilities.filter(v => v.coverage_at_discovery === 'partial').length}
                  </div>
                  <div>
                    <strong>Vulnerabilities in Uncovered Code:</strong> {correlatedVulnerabilities.filter(v => v.coverage_at_discovery === 'not-hit').length}
                  </div>
                  <div>
                    <strong>Average Hits per Vulnerable Line:</strong> {
                      Math.round(correlatedVulnerabilities.reduce((sum, v) => sum + (v.hit_count || 0), 0) / correlatedVulnerabilities.length)
                    }
                  </div>
                </div>
              </div>

              {/* Vulnerability List with Coverage Info */}
              <div className="space-y-3">
                {correlatedVulnerabilities.map(vuln => (
                  <div key={vuln.id} className="border border-gray-200 rounded-lg p-4">
                    <div className="flex items-start justify-between">
                      <div className="flex-1">
                        <h4 className="font-medium text-gray-900">{vuln.title}</h4>
                        <p className="text-sm text-gray-500 mt-1">
                          {vuln.location.file}:{vuln.location.line} (Line {vuln.found_at_line})
                        </p>
                      </div>
                      <div className="flex items-center space-x-2">
                        <span className={`px-2 py-1 text-xs font-medium rounded-full border ${
                          vuln.severity === 'critical' ? 'bg-red-100 text-red-800 border-red-200' :
                          vuln.severity === 'high' ? 'bg-orange-100 text-orange-800 border-orange-200' :
                          vuln.severity === 'medium' ? 'bg-yellow-100 text-yellow-800 border-yellow-200' :
                          'bg-blue-100 text-blue-800 border-blue-200'
                        }`}>
                          {vuln.severity.toUpperCase()}
                        </span>
                        <span className={`px-2 py-1 text-xs font-medium rounded-full ${
                          vuln.coverage_at_discovery === 'covered' ? 'bg-green-100 text-green-800' :
                          vuln.coverage_at_discovery === 'partial' ? 'bg-yellow-100 text-yellow-800' :
                          'bg-red-100 text-red-800'
                        }`}>
                          {vuln.coverage_at_discovery?.replace('-', ' ')}
                        </span>
                      </div>
                    </div>
                    <div className="mt-2 text-sm text-gray-600">
                      <strong>Hit Count:</strong> {vuln.hit_count || 0} times
                    </div>
                  </div>
                ))}
              </div>

              {/* Recommendations */}
              {generateCoverageRecommendations().length > 0 && (
                <div className="mt-6">
                  <h4 className="font-medium text-gray-900 mb-3">Coverage Recommendations</h4>
                  <div className="space-y-2">
                    {generateCoverageRecommendations().map((rec, index) => (
                      <div key={index} className={`p-3 rounded-lg border ${
                        rec.priority === 'high' ? 'bg-red-50 border-red-200' :
                        rec.priority === 'medium' ? 'bg-yellow-50 border-yellow-200' :
                        'bg-blue-50 border-blue-200'
                      }`}>
                        <div className="flex items-start space-x-2">
                          {rec.priority === 'high' ? (
                            <ExclamationTriangleIcon className="h-4 w-4 text-red-600 mt-0.5" />
                          ) : (
                            <CheckCircleIcon className="h-4 w-4 text-blue-600 mt-0.5" />
                          )}
                          <p className={`text-sm ${
                            rec.priority === 'high' ? 'text-red-800' :
                            rec.priority === 'medium' ? 'text-yellow-800' :
                            'text-blue-800'
                          }`}>
                            {rec.message}
                          </p>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Export Modal */}
      {showExportModal && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-60">
          <div className="bg-white rounded-lg p-6 max-w-sm">
            <h3 className="text-lg font-semibold mb-4">Exporting Coverage Report...</h3>
            <div className="flex items-center justify-center">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
