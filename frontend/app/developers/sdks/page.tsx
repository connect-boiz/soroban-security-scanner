import React from 'react';
import { Code, Terminal, Box } from 'lucide-react';

export default function SdkDocumentation() {
  return (
    <div className="py-12 px-8 max-w-5xl mx-auto min-h-screen">
      <div className="mb-12 border-b border-slate-800 pb-8">
        <h1 className="text-4xl font-extrabold text-white mb-4 flex items-center gap-3">
          <Code className="h-8 w-8 text-cyan-400" />
          SDKs & Libraries
        </h1>
        <p className="text-xl text-slate-400">
          Official client libraries for integrating the Soroban Security Scanner into your applications.
        </p>
      </div>

      <div className="space-y-12">
        {/* JavaScript / TypeScript SDK */}
        <section className="bg-gradient-to-r from-slate-900 to-slate-800/50 border border-slate-800 rounded-2xl p-8 relative overflow-hidden">
          <div className="absolute top-0 right-0 w-32 h-32 bg-yellow-500/5 rounded-full blur-[40px]" />
          <div className="flex items-center gap-4 mb-6 relative z-10">
            <div className="w-12 h-12 bg-[#F7DF1E]/10 rounded-xl flex items-center justify-center text-[#F7DF1E]">
              <span className="font-bold text-xl">JS</span>
            </div>
            <div>
              <h2 className="text-2xl font-bold text-white">JavaScript / TypeScript</h2>
              <p className="text-slate-400 text-sm">@sorobanscanner/client</p>
            </div>
          </div>
          
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-8 relative z-10">
            <div>
              <h3 className="text-white font-semibold mb-3">Installation</h3>
              <pre className="bg-[#0d1117] border border-slate-700 p-4 rounded-lg text-sm text-slate-300">
                <code>npm install @sorobanscanner/client</code>
              </pre>
              
              <h3 className="text-white font-semibold mt-6 mb-3">Features</h3>
              <ul className="list-disc pl-5 text-slate-400 space-y-2">
                <li>Fully typed with TypeScript</li>
                <li>Built-in automatic retries</li>
                <li>Promises and async/await support</li>
                <li>Works in Node.js and Browser</li>
              </ul>
            </div>
            
            <div>
              <h3 className="text-white font-semibold mb-3">Quick Example</h3>
              <pre className="bg-[#0d1117] border border-slate-700 p-4 rounded-lg text-xs text-cyan-300 overflow-x-auto">
                <code>{`import { ScannerClient } from '@sorobanscanner/client';

const client = new ScannerClient({
  token: process.env.SCANNER_API_KEY
});

async function scanContract() {
  const result = await client.transactions.submit({
    transaction_type: "SecurityScan",
    network: "testnet",
    data: "base64..."
  });
  console.log(result.transaction_id);
}`}</code>
              </pre>
            </div>
          </div>
        </section>

        {/* Python SDK */}
        <section className="bg-gradient-to-r from-slate-900 to-slate-800/50 border border-slate-800 rounded-2xl p-8 relative overflow-hidden">
          <div className="absolute top-0 right-0 w-32 h-32 bg-blue-500/5 rounded-full blur-[40px]" />
          <div className="flex items-center gap-4 mb-6 relative z-10">
            <div className="w-12 h-12 bg-[#3776AB]/10 rounded-xl flex items-center justify-center text-[#3776AB]">
              <span className="font-bold text-xl">Py</span>
            </div>
            <div>
              <h2 className="text-2xl font-bold text-white">Python</h2>
              <p className="text-slate-400 text-sm">sorobanscanner-py</p>
            </div>
          </div>
          
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-8 relative z-10">
            <div>
              <h3 className="text-white font-semibold mb-3">Installation</h3>
              <pre className="bg-[#0d1117] border border-slate-700 p-4 rounded-lg text-sm text-slate-300">
                <code>pip install sorobanscanner</code>
              </pre>
            </div>
            <div>
              <h3 className="text-white font-semibold mb-3">Quick Example</h3>
              <pre className="bg-[#0d1117] border border-slate-700 p-4 rounded-lg text-xs text-emerald-300 overflow-x-auto">
                <code>{`from sorobanscanner import Client

client = Client(token="your_token")
stats = client.queue.get_stats()
print(f"Pending: {stats.pending}")`}</code>
              </pre>
            </div>
          </div>
        </section>

        {/* Rust SDK */}
        <section className="bg-gradient-to-r from-slate-900 to-slate-800/50 border border-slate-800 rounded-2xl p-8 relative overflow-hidden">
          <div className="absolute top-0 right-0 w-32 h-32 bg-orange-500/5 rounded-full blur-[40px]" />
          <div className="flex items-center gap-4 mb-6 relative z-10">
            <div className="w-12 h-12 bg-[#DEA584]/10 rounded-xl flex items-center justify-center text-[#DEA584]">
              <Box className="w-6 h-6" />
            </div>
            <div>
              <h2 className="text-2xl font-bold text-white">Rust</h2>
              <p className="text-slate-400 text-sm">soroban-scanner-sdk</p>
            </div>
          </div>
          
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-8 relative z-10">
            <div>
              <h3 className="text-white font-semibold mb-3">Installation (Cargo.toml)</h3>
              <pre className="bg-[#0d1117] border border-slate-700 p-4 rounded-lg text-sm text-slate-300">
                <code>{`[dependencies]
soroban-scanner-sdk = "1.0"`}</code>
              </pre>
            </div>
            <div>
              <h3 className="text-white font-semibold mb-3">Quick Example</h3>
              <pre className="bg-[#0d1117] border border-slate-700 p-4 rounded-lg text-xs text-orange-300 overflow-x-auto">
                <code>{`use soroban_scanner_sdk::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("your_token")?;
    let alerts = client.monitoring().get_alerts().await?;
    Ok(())
}`}</code>
              </pre>
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}
