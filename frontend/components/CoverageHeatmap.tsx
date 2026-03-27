'use client';

import { useState, useEffect, useCallback } from 'react';
import Editor from '@monaco-editor/react';
import { 
  ChartBarIcon, 
  ExclamationTriangleIcon,
  CheckCircleIcon,
  InformationCircleIcon 
} from '@heroicons/react/24/outline';

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

interface CoverageHeatmapProps {
  fileContent: string;
  coverageData: CoverageData;
  fileName: string;
  onLineClick?: (lineNumber: number, fuzzerInputs: FuzzerInput[]) => void;
}

export default function CoverageHeatmap({ 
  fileContent, 
  coverageData, 
  fileName, 
  onLineClick 
}: CoverageHeatmapProps) {
  const [selectedLine, setSelectedLine] = useState<number | null>(null);
  const [fuzzerInputs, setFuzzerInputs] = useState<FuzzerInput[]>([]);
  const [loadingInputs, setLoadingInputs] = useState(false);

  const calculateLineCoverage = useCallback((lineNumber: number): 'covered' | 'partial' | 'not-hit' => {
    const hitCount = coverageData.lines_hit[lineNumber] || 0;
    const branches = coverageData.branches_hit[lineNumber];
    
    if (hitCount === 0) return 'not-hit';
    if (branches && branches.length > 1 && branches.some(b => !b)) return 'partial';
    return 'covered';
  }, [coverageData]);

  const calculateCoveragePercentage = useCallback(() => {
    const totalLines = fileContent.split('\n').length;
    const coveredLines = Object.keys(coverageData.lines_hit).length;
    return totalLines > 0 ? Math.round((coveredLines / totalLines) * 100) : 0;
  }, [coverageData, fileContent]);

  const calculateBranchCoverage = useCallback(() => {
    let totalBranches = 0;
    let coveredBranches = 0;
    
    Object.values(coverageData.branches_hit).forEach(branches => {
      totalBranches += branches.length;
      coveredBranches += branches.filter(b => b).length;
    });
    
    return totalBranches > 0 ? Math.round((coveredBranches / totalBranches) * 100) : 0;
  }, [coverageData]);

  const getLineDecoration = useCallback((lineNumber: number) => {
    const coverage = calculateLineCoverage(lineNumber);
    const hitCount = coverageData.lines_hit[lineNumber] || 0;
    
    const colors = {
      'covered': { bg: 'rgba(34, 197, 94, 0.1)', border: '#22c55e' },
      'partial': { bg: 'rgba(250, 204, 21, 0.1)', border: '#facc15' },
      'not-hit': { bg: 'rgba(239, 68, 68, 0.1)', border: '#ef4444' }
    };
    
    return {
      range: {
        startLineNumber: lineNumber,
        startColumn: 1,
        endLineNumber: lineNumber,
        endColumn: 1000
      },
      options: {
        isWholeLine: true,
        className: `coverage-line-${coverage}`,
        afterContentClassName: `coverage-gutter-${coverage}`,
        afterContent: hitCount.toString(),
        minimap: {
          color: colors[coverage].border,
          position: 1
        },
        hoverMessage: { 
          value: `Line ${lineNumber}: ${coverage.replace('-', ' ')} (hit ${hitCount} times)` 
        }
      }
    };
  }, [calculateLineCoverage, coverageData]);

  const handleLineClick = useCallback(async (lineNumber: number) => {
    setSelectedLine(lineNumber);
    
    if (onLineClick) {
      setLoadingInputs(true);
      try {
        // Mock fuzzer inputs for now - in real implementation, fetch from backend
        const mockInputs: FuzzerInput[] = [
          {
            values: [42, true, "test"],
            iteration: 1,
            timestamp: new Date().toISOString()
          },
          {
            values: [100, false, "another"],
            iteration: 2,
            timestamp: new Date().toISOString()
          }
        ];
        
        setFuzzerInputs(mockInputs);
        onLineClick(lineNumber, mockInputs);
      } catch (error) {
        console.error('Failed to fetch fuzzer inputs:', error);
      } finally {
        setLoadingInputs(false);
      }
    }
  }, [onLineClick]);

  const getCoverageColor = (coverage: string) => {
    switch (coverage) {
      case 'covered': return 'text-green-600 bg-green-50 border-green-200';
      case 'partial': return 'text-yellow-600 bg-yellow-50 border-yellow-200';
      case 'not-hit': return 'text-red-600 bg-red-50 border-red-200';
      default: return 'text-gray-600 bg-gray-50 border-gray-200';
    }
  };

  const lineCoveragePercentage = calculateCoveragePercentage();
  const branchCoveragePercentage = calculateBranchCoverage();

  return (
    <div className="flex flex-col h-full">
      {/* Coverage Summary Header */}
      <div className="border-b border-gray-200 p-4 bg-gray-50">
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-lg font-semibold text-gray-900">Code Coverage Heatmap</h3>
          <span className="text-sm text-gray-500">{fileName}</span>
        </div>
        
        <div className="grid grid-cols-3 gap-4 text-sm">
          <div className="flex items-center space-x-2">
            <ChartBarIcon className="h-4 w-4 text-blue-600" />
            <span className="font-medium">Line Coverage:</span>
            <span className={`px-2 py-1 rounded-full text-xs font-medium ${
              lineCoveragePercentage >= 80 ? 'bg-green-100 text-green-800' :
              lineCoveragePercentage >= 60 ? 'bg-yellow-100 text-yellow-800' :
              'bg-red-100 text-red-800'
            }`}>
              {lineCoveragePercentage}%
            </span>
          </div>
          
          <div className="flex items-center space-x-2">
            <ChartBarIcon className="h-4 w-4 text-purple-600" />
            <span className="font-medium">Branch Coverage:</span>
            <span className={`px-2 py-1 rounded-full text-xs font-medium ${
              branchCoveragePercentage >= 80 ? 'bg-green-100 text-green-800' :
              branchCoveragePercentage >= 60 ? 'bg-yellow-100 text-yellow-800' :
              'bg-red-100 text-red-800'
            }`}>
              {branchCoveragePercentage}%
            </span>
          </div>
          
          <div className="flex items-center space-x-2">
            <CheckCircleIcon className="h-4 w-4 text-green-600" />
            <span className="font-medium">Functions Hit:</span>
            <span className="px-2 py-1 bg-blue-100 text-blue-800 rounded-full text-xs font-medium">
              {Object.keys(coverageData.functions_hit).length}
            </span>
          </div>
        </div>
        
        {/* Low Coverage Alert */}
        {lineCoveragePercentage < 60 && (
          <div className="mt-3 p-2 bg-red-50 border border-red-200 rounded-lg flex items-center space-x-2">
            <ExclamationTriangleIcon className="h-4 w-4 text-red-600" />
            <span className="text-sm text-red-800">
              Low coverage alert! Critical business logic may not be adequately tested.
            </span>
          </div>
        )}
      </div>

      {/* Legend */}
      <div className="border-b border-gray-200 p-3 bg-white">
        <div className="flex items-center space-x-6 text-sm">
          <div className="flex items-center space-x-2">
            <div className="w-4 h-4 bg-green-100 border border-green-500 rounded"></div>
            <span>Covered</span>
          </div>
          <div className="flex items-center space-x-2">
            <div className="w-4 h-4 bg-yellow-100 border border-yellow-500 rounded"></div>
            <span>Partial</span>
          </div>
          <div className="flex items-center space-x-2">
            <div className="w-4 h-4 bg-red-100 border border-red-500 rounded"></div>
            <span>Not Hit</span>
          </div>
        </div>
      </div>

      <div className="flex-1 flex">
        {/* Code Editor */}
        <div className="flex-1">
          <Editor
            height="100%"
            language="rust"
            value={fileContent}
            theme="vs-dark"
            options={{
              readOnly: true,
              minimap: { enabled: true },
              fontSize: 14,
              lineNumbers: 'on',
              renderLineHighlight: 'line',
              scrollBeyondLastLine: false,
              automaticLayout: true,
              glyphMargin: true,
              folding: false,
              lineDecorationsWidth: 20,
              lineNumbersMinChars: 3,
            }}
            onMount={(editor, monaco) => {
              // Add custom styles for coverage lines
              monaco.editor.defineTheme('coverage-dark', {
                base: 'vs-dark',
                inherit: true,
                rules: [
                  { token: 'coverage-line-covered', foreground: '22c55e', background: 'rgba(34, 197, 94, 0.1)' },
                  { token: 'coverage-line-partial', foreground: 'facc15', background: 'rgba(250, 204, 21, 0.1)' },
                  { token: 'coverage-line-not-hit', foreground: 'ef4444', background: 'rgba(239, 68, 68, 0.1)' },
                ],
                colors: {}
              });
              
              // Handle line clicks
              editor.onMouseDown((e) => {
                if (e.target.position) {
                  const lineNumber = e.target.position.lineNumber;
                  handleLineClick(lineNumber);
                }
              });
            }}
            decorations={Object.keys(coverageData.lines_hit).map(Number).map(getLineDecoration)}
          />
        </div>

        {/* Fuzzer Inputs Panel */}
        {selectedLine && (
          <div className="w-80 border-l border-gray-200 bg-white overflow-y-auto">
            <div className="p-4 border-b border-gray-200">
              <h4 className="font-semibold text-gray-900">Line {selectedLine} Details</h4>
              <p className="text-sm text-gray-500">Fuzzer inputs that triggered this path</p>
            </div>
            
            {loadingInputs ? (
              <div className="p-4 text-center">
                <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600 mx-auto"></div>
                <p className="text-sm text-gray-500 mt-2">Loading fuzzer inputs...</p>
              </div>
            ) : fuzzerInputs.length > 0 ? (
              <div className="p-4 space-y-3">
                {fuzzerInputs.map((input, index) => (
                  <div key={index} className="p-3 bg-gray-50 rounded-lg border border-gray-200">
                    <div className="flex items-center justify-between mb-2">
                      <span className="text-sm font-medium text-gray-900">Iteration {input.iteration}</span>
                      <span className="text-xs text-gray-500">
                        {new Date(input.timestamp).toLocaleTimeString()}
                      </span>
                    </div>
                    <div className="text-xs text-gray-600">
                      <strong>Input values:</strong> [{input.values.join(', ')}]
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <div className="p-4 text-center text-gray-500">
                <InformationCircleIcon className="h-8 w-8 mx-auto mb-2 text-gray-400" />
                <p className="text-sm">No fuzzer inputs recorded for this line</p>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
