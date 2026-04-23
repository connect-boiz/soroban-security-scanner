'use client';

import { useState } from 'react';
import VulnerabilityReportViewer from '../../components/VulnerabilityReportViewer';

// Mock SARIF data for demonstration
const mockSarifData = {
  version: '2.1.0',
  $schema: 'https://json.schemastore.org/sarif-2.1.0',
  runs: [{
    tool: {
      driver: {
        name: 'Soroban Security Scanner',
        version: '1.0.0',
        rules: [
          {
            id: 'CWE-284',
            name: 'Missing Access Control',
            shortDescription: {
              text: 'Missing Access Control'
            },
            fullDescription: {
              text: 'Public function lacks access control checks'
            },
            defaultConfiguration: {
              level: 'error'
            },
            help: {
              text: 'Add require_auth() or similar access control checks'
            },
            properties: {
              category: 'Access Control',
              tags: ['access-control']
            }
          },
          {
            id: 'CWE-400',
            name: 'Potential Infinite Mint',
            shortDescription: {
              text: 'Potential Infinite Mint'
            },
            fullDescription: {
              text: 'Token minting function may lack proper limits'
            },
            defaultConfiguration: {
              level: 'warning'
            },
            help: {
              text: 'Implement total supply limits and minting controls'
            },
            properties: {
              category: 'Token Economics',
              tags: ['token-economics']
            }
          }
        ]
      }
    },
    results: [
      {
        ruleId: 'CWE-284',
        level: 'error',
        message: {
          text: 'Public function lacks access control checks'
        },
        locations: [{
          physicalLocation: {
            artifactLocation: {
              uri: 'src/contract.rs'
            },
            region: {
              startLine: 15,
              startColumn: 1,
              endLine: 15,
              endColumn: 25
            }
          }
        }]
      },
      {
        ruleId: 'CWE-400',
        level: 'warning',
        message: {
          text: 'Token minting function may lack proper limits'
        },
        locations: [{
          physicalLocation: {
            artifactLocation: {
              uri: 'src/token.rs'
            },
            region: {
              startLine: 32,
              startColumn: 5,
              endLine: 32,
              endColumn: 30
            }
          }
        }]
      }
    ],
    artifacts: [{
      location: {
        uri: 'src/contract.rs'
      },
      length: 1000,
      mimeType: 'text/x-rust'
    }]
  }]
};

// Mock coverage data
const mockCoverageData = {
  lines_hit: {
    1: 10, 2: 10, 3: 10, 4: 10, 5: 10, 6: 10, 7: 10, 8: 10, 9: 10, 10: 10,
    11: 8, 12: 8, 13: 8, 14: 8, 15: 8, 16: 8, 17: 8, 18: 8, 19: 8, 20: 8,
    21: 5, 22: 5, 23: 5, 24: 5, 25: 5, 26: 5, 27: 5, 28: 5, 29: 5, 30: 5,
    31: 0, 32: 0, 33: 0, 34: 0, 35: 0, 36: 0, 37: 0, 38: 0, 39: 0, 40: 0,
  },
  branches_hit: {
    15: [true, false], // Partial coverage - only true path tested
    25: [true, true],  // Full coverage - both paths tested
    35: [false, false], // No coverage - neither path tested
  },
  functions_hit: {
    "new": 10,
    "mint": 8,
    "transfer": 5,
    "get_balance": 0, // Critical function not tested
    "approve": 3,
  },
  total_instructions: 1000000,
  executed_instructions: 450000,
};

export default function ReportViewerPage() {
  const [showViewer, setShowViewer] = useState(false);
  const [scanId] = useState('demo-scan-123');

  return (
    <div className="min-h-screen bg-gray-50">
      <div className="container mx-auto px-4 py-8">
        <div className="text-center mb-8">
          <h1 className="text-4xl font-bold text-gray-900 mb-4">
            Interactive Vulnerability Report Viewer
          </h1>
          <p className="text-lg text-gray-600 max-w-3xl mx-auto">
            A comprehensive security report interface that highlights specific lines of vulnerable code, 
            explains the exploit, and offers actionable remediation steps.
          </p>
        </div>

        <div className="bg-white rounded-xl shadow-lg p-8 max-w-4xl mx-auto">
          <h2 className="text-2xl font-semibold text-gray-900 mb-6">Features</h2>
          
          <div className="grid md:grid-cols-2 gap-6 mb-8">
            <div className="space-y-4">
              <h3 className="text-lg font-medium text-gray-800">🔍 Code Analysis</h3>
              <ul className="space-y-2 text-gray-600">
                <li className="flex items-start">
                  <span className="text-green-500 mr-2">✓</span>
                  Parse SARIF output to map vulnerabilities
                </li>
                <li className="flex items-start">
                  <span className="text-green-500 mr-2">✓</span>
                  Monaco Editor integration for code display
                </li>
                <li className="flex items-start">
                  <span className="text-green-500 mr-2">✓</span>
                  Vulnerable line highlighting
                </li>
                <li className="flex items-start">
                  <span className="text-green-500 mr-2">✓</span>
                  Interactive code navigation
                </li>
              </ul>
            </div>

            <div className="space-y-4">
              <h3 className="text-lg font-medium text-gray-800">📊 Report Management</h3>
              <ul className="space-y-2 text-gray-600">
                <li className="flex items-start">
                  <span className="text-green-500 mr-2">✓</span>
                  Detailed vulnerability explanations
                </li>
                <li className="flex items-start">
                  <span className="text-green-500 mr-2">✓</span>
                  Acknowledge/False Positive management
                </li>
                <li className="flex items-start">
                  <span className="text-green-500 mr-2">✓</span>
                  PDF export functionality
                </li>
                <li className="flex items-start">
                  <span className="text-green-500 mr-2">✓</span>
                  Deep-linking for sharing
                </li>
              </ul>
            </div>
          </div>

          <div className="border-t pt-6">
            <h3 className="text-lg font-medium text-gray-800 mb-4">Demo Scan Information</h3>
            <div className="bg-gray-50 rounded-lg p-4 space-y-2">
              <p className="text-sm text-gray-600">
                <strong>Scan ID:</strong> {scanId}
              </p>
              <p className="text-sm text-gray-600">
                <strong>Vulnerabilities Found:</strong> 2 (1 Critical, 1 High)
              </p>
              <p className="text-sm text-gray-600">
                <strong>Files Analyzed:</strong> src/contract.rs, src/token.rs
              </p>
              <p className="text-sm text-gray-600">
                <strong>Scan Date:</strong> {new Date().toLocaleDateString()}
              </p>
            </div>
          </div>

          <div className="mt-8 text-center">
            <button
              onClick={() => setShowViewer(true)}
              className="inline-flex items-center px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-lg transition-colors"
            >
              Launch Interactive Report Viewer
            </button>
          </div>
        </div>

        {/* Additional Information */}
        <div className="mt-12 grid md:grid-cols-3 gap-6">
          <div className="bg-white rounded-lg shadow p-6">
            <div className="text-blue-600 text-2xl mb-3">🛡️</div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2">Security First</h3>
            <p className="text-gray-600 text-sm">
              Comprehensive vulnerability detection with CWE references and detailed remediation guidance.
            </p>
          </div>

          <div className="bg-white rounded-lg shadow p-6">
            <div className="text-green-600 text-2xl mb-3">📋</div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2">Compliance Ready</h3>
            <p className="text-gray-600 text-sm">
              Export professional PDF reports for audits, compliance, and team collaboration.
            </p>
          </div>

          <div className="bg-white rounded-lg shadow p-6">
            <div className="text-purple-600 text-2xl mb-3">🔗</div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2">Team Collaboration</h3>
            <p className="text-gray-600 text-sm">
              Share specific findings with deep-links and manage risk acceptance across your team.
            </p>
          </div>
        </div>
      </div>

      {/* Interactive Report Viewer Modal */}
      {showViewer && (
        <VulnerabilityReportViewer
          scanId={scanId}
          sarifData={mockSarifData}
          coverageData={mockCoverageData}
          onClose={() => setShowViewer(false)}
        />
      )}
    </div>
  );
}
