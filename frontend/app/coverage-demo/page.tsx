'use client';

import { useState } from 'react';
import CoverageReportViewer from '../../components/CoverageReportViewer';

// Mock coverage data for demonstration
const mockCoverageData = {
  lines_hit: {
    1: 10, 2: 10, 3: 10, 4: 10, 5: 10, 6: 10, 7: 10, 8: 10, 9: 10, 10: 10,
    11: 8, 12: 8, 13: 8, 14: 8, 15: 8, 16: 8, 17: 8, 18: 8, 19: 8, 20: 8,
    21: 5, 22: 5, 23: 5, 24: 5, 25: 5, 26: 5, 27: 5, 28: 5, 29: 5, 30: 5,
    31: 0, 32: 0, 33: 0, 34: 0, 35: 0, 36: 0, 37: 0, 38: 0, 39: 0, 40: 0,
    41: 3, 42: 3, 43: 3, 44: 3, 45: 3, 46: 3, 47: 3, 48: 3, 49: 3, 50: 3,
  },
  branches_hit: {
    15: [true, false], // Partial coverage - only true path tested
    25: [true, true],  // Full coverage - both paths tested
    35: [false, false], // No coverage - neither path tested
    45: [true, false], // Partial coverage
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

// Mock file contents
const mockFileContents = {
  "src/contract.rs": `pub struct Contract {
    admin: Address,
    token: Address,
    total_supply: i64,
    balances: Map<Address, i64>,
}

impl Contract {
    pub fn new(env: &Env, admin: Address, token: Address) -> Self {
        Self {
            admin,
            token,
            total_supply: 0,
            balances: Map::new(env),
        }
    }

    // CRITICAL: Missing access control - vulnerability found here
    pub fn mint(&mut self, env: &Env, to: Address, amount: i64) {
        // No access control check - anyone can mint!
        self.total_supply += amount;
        let current_balance = self.balances.get(to).unwrap_or(0);
        self.balances.set(to, current_balance + amount);
    }

    // Partial coverage - only success path tested
    pub fn transfer(&mut self, env: &Env, from: Address, to: Address, amount: i64) -> Result<(), Error> {
        if from != self.admin {
            return Err(Error::from_contract_error(1));
        }
        
        let from_balance = self.balances.get(from).unwrap_or(0);
        if from_balance < amount {
            return Err(Error::from_contract_error(2));
        }
        
        self.balances.set(from, from_balance - amount);
        let to_balance = self.balances.get(to).unwrap_or(0);
        self.balances.set(to, to_balance + amount);
        
        Ok(())
    }

    // CRITICAL: No coverage - untested function
    pub fn get_balance(&self, env: &Env, address: Address) -> i64 {
        self.balances.get(address).unwrap_or(0)
    }

    // Partial coverage
    pub fn approve(&mut self, env: &Env, owner: Address, spender: Address, amount: i64) -> Result<(), Error> {
        if owner != self.admin {
            return Err(Error::from_contract_error(3));
        }
        
        // Implementation would go here
        Ok(())
    }
}`,
  "src/token.rs": `pub struct Token {
    name: String,
    symbol: String,
    decimals: u8,
}

impl Token {
    pub fn new(name: String, symbol: String, decimals: u8) -> Self {
        Self {
            name,
            symbol,
            decimals,
        }
    }
    
    // Fully tested function
    pub fn get_info(&self) -> (String, String, u8) {
        (self.name.clone(), self.symbol.clone(), self.decimals)
    }
}`,
};

// Mock vulnerabilities
const mockVulnerabilities = [
  {
    id: "vuln-1",
    title: "Missing Access Control in Mint Function",
    severity: "critical" as const,
    location: {
      file: "src/contract.rs",
      line: 15,
    },
    found_at_line: 15,
    coverage_at_discovery: "covered" as const,
  },
  {
    id: "vuln-2", 
    title: "Integer Overflow in Transfer",
    severity: "high" as const,
    location: {
      file: "src/contract.rs", 
      line: 25,
    },
    found_at_line: 25,
    coverage_at_discovery: "partial" as const,
  },
  {
    id: "vuln-3",
    title: "Untested Get Balance Function",
    severity: "medium" as const,
    location: {
      file: "src/contract.rs",
      line: 35,
    },
    found_at_line: 35,
    coverage_at_discovery: "not-hit" as const,
  },
];

export default function CoverageDemoPage() {
  const [showViewer, setShowViewer] = useState(false);
  const [scanId] = useState('demo-scan-coverage-123');

  return (
    <div className="min-h-screen bg-gray-50">
      <div className="container mx-auto px-4 py-8">
        <div className="text-center mb-8">
          <h1 className="text-4xl font-bold text-gray-900 mb-4">
            Code Coverage Heatmap Demo
          </h1>
          <p className="text-lg text-gray-600 max-w-3xl mx-auto">
            Visualize which parts of your smart contract were exercised by the invariant fuzzer.
            See coverage data, branch analysis, and correlation with found vulnerabilities.
          </p>
        </div>

        <div className="bg-white rounded-xl shadow-lg p-8 max-w-4xl mx-auto">
          <h2 className="text-2xl font-semibold text-gray-900 mb-6">Features Demonstrated</h2>
          
          <div className="grid md:grid-cols-2 gap-6 mb-8">
            <div className="space-y-4">
              <h3 className="text-lg font-medium text-gray-800">🔍 Coverage Analysis</h3>
              <ul className="space-y-2 text-gray-600">
                <li className="flex items-start">
                  <span className="text-green-500 mr-2">✓</span>
                  Line coverage visualization with color coding
                </li>
                <li className="flex items-start">
                  <span className="text-green-500 mr-2">✓</span>
                  Branch coverage percentage calculation
                </li>
                <li className="flex items-start">
                  <span className="text-green-500 mr-2">✓</span>
                  Function hit tracking and reporting
                </li>
                <li className="flex items-start">
                  <span className="text-green-500 mr-2">✓</span>
                  Interactive Monaco Editor integration
                </li>
              </ul>
            </div>

            <div className="space-y-4">
              <h3 className="text-lg font-medium text-gray-800">📊 Smart Insights</h3>
              <ul className="space-y-2 text-gray-600">
                <li className="flex items-start">
                  <span className="text-green-500 mr-2">✓</span>
                  Click lines to see fuzzer inputs
                </li>
                <li className="flex items-start">
                  <span className="text-green-500 mr-2">✓</span>
                  Low coverage alerts for critical code
                </li>
                <li className="flex items-start">
                  <span className="text-green-500 mr-2">✓</span>
                  Vulnerability-coverage correlation
                </li>
                <li className="flex items-start">
                  <span className="text-green-500 mr-2">✓</span>
                  Exportable coverage reports
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
                <strong>Files Analyzed:</strong> src/contract.rs, src/token.rs
              </p>
              <p className="text-sm text-gray-600">
                <strong>Line Coverage:</strong> 65% (65/100 lines)
              </p>
              <p className="text-sm text-gray-600">
                <strong>Branch Coverage:</strong> 50% (2/4 branches fully covered)
              </p>
              <p className="text-sm text-gray-600">
                <strong>Vulnerabilities Found:</strong> 3 (1 Critical, 1 High, 1 Medium)
              </p>
              <p className="text-sm text-gray-600">
                <strong>Scan Date:</strong> {new Date().toLocaleDateString()}
              </p>
            </div>
          </div>

          <div className="mt-8 text-center">
            <button
              onClick={() => setShowViewer(true)}
              className="inline-flex items-center px-6 py-3 bg-green-600 hover:bg-green-700 text-white font-medium rounded-lg transition-colors"
            >
              Launch Coverage Heatmap Demo
            </button>
          </div>
        </div>

        {/* Additional Information */}
        <div className="mt-12 grid md:grid-cols-3 gap-6">
          <div className="bg-white rounded-lg shadow p-6">
            <div className="text-green-600 text-2xl mb-3">🎯</div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2">Precision Testing</h3>
            <p className="text-gray-600 text-sm">
              See exactly which lines were executed and how many times during fuzzing.
            </p>
          </div>

          <div className="bg-white rounded-lg shadow p-6">
            <div className="text-yellow-600 text-2xl mb-3">⚠️</div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2">Risk Identification</h3>
            <p className="text-gray-600 text-sm">
              Identify untested critical paths that might contain hidden vulnerabilities.
            </p>
          </div>

          <div className="bg-white rounded-lg shadow p-6">
            <div className="text-blue-600 text-2xl mb-3">📈</div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2">Continuous Improvement</h3>
            <p className="text-gray-600 text-sm">
              Track coverage improvements over time and optimize test strategies.
            </p>
          </div>
        </div>
      </div>

      {/* Coverage Report Viewer Modal */}
      {showViewer && (
        <CoverageReportViewer
          scanId={scanId}
          coverageData={mockCoverageData}
          vulnerabilities={mockVulnerabilities}
          fileContents={mockFileContents}
          onClose={() => setShowViewer(false)}
        />
      )}
    </div>
  );
}
