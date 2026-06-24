import React from 'react';
import { Activity, Zap, AlertTriangle } from 'lucide-react';

export default function RateLimiting() {
  return (
    <div className="py-12 px-8 max-w-4xl mx-auto min-h-screen">
      <div className="mb-12">
        <h1 className="text-4xl font-extrabold text-white mb-4 flex items-center gap-3">
          <Activity className="h-8 w-8 text-cyan-400" />
          Rate Limiting
        </h1>
        <p className="text-xl text-slate-400">
          Learn how to handle rate limits to ensure your integration remains stable and performant.
        </p>
      </div>

      <div className="prose prose-invert prose-slate max-w-none">
        <p className="text-slate-300 text-lg mb-8">
          To protect the infrastructure from abuse and ensure fair usage, we enforce rate limits on all API endpoints. 
          Our rate limiting uses a token bucket algorithm.
        </p>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-12">
          <div className="bg-[#151e32] border border-slate-700 p-6 rounded-xl">
            <Zap className="h-8 w-8 text-amber-400 mb-4" />
            <h3 className="text-white font-bold mb-2">Standard Tier Limits</h3>
            <ul className="text-slate-400 text-sm space-y-2">
              <li>• 100 requests per minute per IP</li>
              <li>• 1000 requests per hour per user account</li>
              <li>• 5 concurrent transactions in processing queue</li>
            </ul>
          </div>
          <div className="bg-[#151e32] border border-slate-700 p-6 rounded-xl">
            <AlertTriangle className="h-8 w-8 text-rose-400 mb-4" />
            <h3 className="text-white font-bold mb-2">When Exceeded</h3>
            <p className="text-slate-400 text-sm">
              If you exceed these limits, the API will return a <strong>429 Too Many Requests</strong> HTTP status code.
            </p>
          </div>
        </div>

        <h2 className="text-2xl font-bold text-white mb-4">HTTP Headers</h2>
        <p className="text-slate-300 mb-6">
          Every API response includes headers detailing your current rate limit status. You should use these headers 
          to proactively manage your request volume.
        </p>

        <div className="bg-[#0d1117] border border-slate-800 rounded-xl overflow-hidden mb-12">
          <table className="w-full text-left text-sm text-slate-300">
            <thead className="bg-slate-800/50 text-slate-200">
              <tr>
                <th className="px-6 py-4 font-semibold">Header Name</th>
                <th className="px-6 py-4 font-semibold">Description</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800">
              <tr>
                <td className="px-6 py-4 font-mono text-cyan-400">X-RateLimit-Limit</td>
                <td className="px-6 py-4">The maximum number of requests you're permitted to make per hour.</td>
              </tr>
              <tr>
                <td className="px-6 py-4 font-mono text-cyan-400">X-RateLimit-Remaining</td>
                <td className="px-6 py-4">The number of requests remaining in the current rate limit window.</td>
              </tr>
              <tr>
                <td className="px-6 py-4 font-mono text-cyan-400">X-RateLimit-Reset</td>
                <td className="px-6 py-4">The time at which the current rate limit window resets in UTC epoch seconds.</td>
              </tr>
              <tr>
                <td className="px-6 py-4 font-mono text-rose-400">Retry-After</td>
                <td className="px-6 py-4">Only sent with 429 responses. Indicates how many seconds to wait before retrying.</td>
              </tr>
            </tbody>
          </table>
        </div>

        <h2 className="text-2xl font-bold text-white mb-4">Best Practices</h2>
        <ul className="space-y-4 text-slate-300">
          <li className="flex gap-3">
            <span className="text-cyan-500 font-bold">1.</span>
            <span><strong>Implement Exponential Backoff:</strong> When you receive a 429, wait the duration specified by <code>Retry-After</code> before retrying. If no header is present, double your wait time on each subsequent failure.</span>
          </li>
          <li className="flex gap-3">
            <span className="text-cyan-500 font-bold">2.</span>
            <span><strong>Monitor Headers:</strong> Read the <code>X-RateLimit-Remaining</code> header on successful requests to slow down your application before hitting the limit.</span>
          </li>
          <li className="flex gap-3">
            <span className="text-cyan-500 font-bold">3.</span>
            <span><strong>Batch Operations:</strong> If you are submitting many small items, consider grouping them using the BatchOperation transaction type to reduce overhead.</span>
          </li>
        </ul>
      </div>
    </div>
  );
}
