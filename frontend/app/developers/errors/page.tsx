import React from 'react';
import { ShieldAlert, AlertCircle, HelpCircle } from 'lucide-react';

const errorCodes = [
  { code: 400, title: 'Bad Request', description: 'The request was malformed or missing required parameters. Check your payload schema.', solution: 'Validate your request body against the OpenAPI specification.' },
  { code: 401, title: 'Unauthorized', description: 'Authentication failed. Your token may be missing, invalid, or expired.', solution: 'Ensure you are passing a valid JWT in the Authorization header. Re-login if expired.' },
  { code: 403, title: 'Forbidden', description: 'You lack the necessary permissions to access this endpoint.', solution: 'Check if the endpoint requires admin privileges and verify your role.' },
  { code: 404, title: 'Not Found', description: 'The requested resource (e.g., transaction ID) does not exist.', solution: 'Verify the ID in your request matches an existing resource.' },
  { code: 429, title: 'Too Many Requests', description: 'You have exceeded your API rate limit.', solution: 'Implement exponential backoff and respect the Retry-After header.' },
  { code: 500, title: 'Internal Server Error', description: 'An unexpected error occurred on our end.', solution: 'Check our status page. If the issue persists, contact support.' },
];

export default function ErrorCodesReference() {
  return (
    <div className="py-12 px-8 max-w-5xl mx-auto min-h-screen">
      <div className="mb-12">
        <h1 className="text-4xl font-extrabold text-white mb-4 flex items-center gap-3">
          <ShieldAlert className="h-8 w-8 text-rose-500" />
          Error Codes & Troubleshooting
        </h1>
        <p className="text-xl text-slate-400 max-w-3xl">
          We use conventional HTTP response codes to indicate the success or failure of an API request.
          Below is a reference guide for handling errors gracefully.
        </p>
      </div>

      <div className="bg-[#151e32] border border-slate-700 rounded-2xl overflow-hidden shadow-xl">
        <div className="px-6 py-4 bg-slate-800/50 border-b border-slate-700 flex items-center gap-2">
          <AlertCircle className="h-5 w-5 text-slate-400" />
          <h2 className="text-lg font-semibold text-white">Standard Error Format</h2>
        </div>
        <div className="p-6">
          <p className="text-slate-300 mb-4">
            All error responses follow a standardized JSON format:
          </p>
          <pre className="bg-[#0d1117] border border-slate-800 p-4 rounded-lg overflow-x-auto text-sm text-cyan-300">
            <code>{`{
  "success": false,
  "data": null,
  "error": "Detailed error message here",
  "timestamp": "2023-10-25T14:30:00Z"
}`}</code>
          </pre>
        </div>
      </div>

      <div className="mt-12 space-y-6">
        <h2 className="text-2xl font-bold text-white mb-6">HTTP Status Codes</h2>
        {errorCodes.map((error) => (
          <div key={error.code} className="bg-[#131b2c] border border-slate-800 rounded-xl p-6 hover:border-slate-600 transition-colors">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-4">
              <div className="flex items-center gap-3">
                <span className={`px-3 py-1 rounded text-sm font-bold ${
                  error.code >= 500 ? 'bg-rose-500/20 text-rose-400' :
                  error.code >= 400 ? 'bg-amber-500/20 text-amber-400' :
                  'bg-blue-500/20 text-blue-400'
                }`}>
                  {error.code}
                </span>
                <h3 className="text-xl font-bold text-white">{error.title}</h3>
              </div>
            </div>
            <p className="text-slate-400 mb-4">{error.description}</p>
            <div className="flex items-start gap-2 text-sm text-cyan-400 bg-cyan-950/30 p-3 rounded-lg border border-cyan-900/50">
              <HelpCircle className="h-5 w-5 shrink-0 mt-0.5" />
              <div>
                <strong className="block mb-1 text-cyan-300">How to resolve:</strong>
                {error.solution}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
