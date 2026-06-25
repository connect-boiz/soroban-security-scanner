import React from 'react';
import { BookOpen, GitBranch, Terminal } from 'lucide-react';

export default function IntegrationGuides() {
  return (
    <div className="py-12 px-8 max-w-4xl mx-auto min-h-screen">
      <div className="mb-12">
        <h1 className="text-4xl font-extrabold text-white mb-4 flex items-center gap-3">
          <BookOpen className="h-8 w-8 text-cyan-400" />
          Integration Guides
        </h1>
        <p className="text-xl text-slate-400">
          Learn how to integrate the Soroban Security Scanner into your existing workflows and CI/CD pipelines.
        </p>
      </div>

      <div className="space-y-10">
        <section className="bg-[#151e32] border border-slate-700 rounded-2xl p-8">
          <div className="flex items-center gap-4 mb-6">
            <div className="p-3 bg-emerald-500/10 rounded-xl text-emerald-400">
              <GitBranch className="h-6 w-6" />
            </div>
            <h2 className="text-2xl font-bold text-white">GitHub Actions Integration</h2>
          </div>
          <p className="text-slate-300 mb-6 leading-relaxed">
            Automatically scan your Soroban smart contracts on every pull request to catch vulnerabilities 
            before they make it into production.
          </p>
          <div className="bg-[#0d1117] border border-slate-800 rounded-xl overflow-hidden">
            <div className="px-4 py-2 bg-slate-800/50 border-b border-slate-700 text-sm font-mono text-slate-400">
              .github/workflows/security-scan.yml
            </div>
            <div className="p-4 overflow-x-auto">
              <pre className="text-sm text-cyan-300">
                <code>{`name: Security Scan
on: [push, pull_request]

jobs:
  scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run Soroban Scanner
        run: |
          curl -X POST https://api.sorobanscanner.com/v1/transactions \\
            -H "Authorization: Bearer \${{ secrets.SCANNER_API_KEY }}" \\
            -H "Content-Type: application/json" \\
            -d '{
              "transaction_type": "SecurityScan",
              "data": "...",
              "submitter": "github-actions",
              "network": "testnet"
            }'`}</code>
              </pre>
            </div>
          </div>
        </section>

        <section className="bg-[#151e32] border border-slate-700 rounded-2xl p-8">
          <div className="flex items-center gap-4 mb-6">
            <div className="p-3 bg-purple-500/10 rounded-xl text-purple-400">
              <Terminal className="h-6 w-6" />
            </div>
            <h2 className="text-2xl font-bold text-white">CLI Integration</h2>
          </div>
          <p className="text-slate-300 mb-6 leading-relaxed">
            Integrate the scanner API into your local build scripts. We provide a Node.js CLI that wraps our API 
            for easy local execution.
          </p>
          <div className="bg-[#0d1117] border border-slate-800 rounded-xl overflow-hidden p-4">
            <pre className="text-sm text-emerald-300">
              <code>{`# Install CLI globally
npm install -g @sorobanscanner/cli

# Configure API Key
soroban-scanner config set api_key your_token

# Run a scan on a compiled WASM file
soroban-scanner scan ./target/wasm32-unknown-unknown/release/contract.wasm`}</code>
            </pre>
          </div>
        </section>
      </div>
    </div>
  );
}
