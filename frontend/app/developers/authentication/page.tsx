import React from 'react';
import { Key, ShieldCheck, Clock } from 'lucide-react';

export default function AuthenticationGuide() {
  return (
    <div className="py-12 px-8 max-w-4xl mx-auto min-h-screen prose prose-invert prose-slate">
      <div className="mb-12">
        <h1 className="text-4xl font-extrabold text-white mb-4 flex items-center gap-3">
          <Key className="h-8 w-8 text-cyan-400" />
          Authentication Guide
        </h1>
        <p className="text-xl text-slate-400">
          Learn how to authenticate your requests to the Soroban Security Scanner API.
        </p>
      </div>

      <section className="mb-12">
        <h2 className="text-2xl font-bold text-white mb-4">Overview</h2>
        <p className="text-slate-300">
          Our API uses <strong>JSON Web Tokens (JWT)</strong> for authentication. You must include this token 
          in the Authorization header of every authenticated request using the Bearer schema.
        </p>
        
        <div className="bg-slate-800/50 border border-slate-700 rounded-lg p-6 mt-6">
          <h3 className="text-white font-semibold mb-2">Authorization Header Format</h3>
          <code className="block bg-slate-900 p-4 rounded text-cyan-400 border border-slate-800 shadow-inner">
            Authorization: Bearer &lt;your_jwt_token&gt;
          </code>
        </div>
      </section>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6 my-12">
        <div className="bg-slate-800/30 border border-slate-700 p-6 rounded-xl">
          <ShieldCheck className="h-8 w-8 text-emerald-400 mb-4" />
          <h3 className="text-white font-bold mb-2">Role-Based Access</h3>
          <p className="text-slate-400 text-sm">
            Tokens contain claims about user roles and permissions. Access to certain endpoints (like `/state/export`) 
            is restricted to admin roles.
          </p>
        </div>
        <div className="bg-slate-800/30 border border-slate-700 p-6 rounded-xl">
          <Clock className="h-8 w-8 text-amber-400 mb-4" />
          <h3 className="text-white font-bold mb-2">Token Expiration</h3>
          <p className="text-slate-400 text-sm">
            Access tokens typically expire after 1 hour. Use your session management logic to refresh tokens 
            before they expire to maintain seamless access.
          </p>
        </div>
      </div>

      <section className="mb-12">
        <h2 className="text-2xl font-bold text-white mb-4">Obtaining a Token</h2>
        <p className="text-slate-300 mb-4">
          To get a token, submit a POST request to the <code>/auth/login</code> endpoint with your credentials.
        </p>
        
        <h3 className="text-white font-semibold mt-6 mb-2">Example: Node.js / Fetch</h3>
        <pre className="bg-[#0d1117] border border-slate-800 p-4 rounded-lg overflow-x-auto text-sm text-slate-300">
          <code>{`const response = await fetch('https://api.sorobanscanner.com/v1/auth/login', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({
    email: 'developer@example.com',
    password: 'your_secure_password'
  })
});

const data = await response.json();
const token = data.token; // Save this for future requests`}</code>
        </pre>
      </section>
      
      <section className="mb-12">
        <h2 className="text-2xl font-bold text-white mb-4">Making an Authenticated Request</h2>
        <p className="text-slate-300 mb-4">
          Once you have your token, include it in your requests.
        </p>
        
        <h3 className="text-white font-semibold mt-6 mb-2">Example: cURL</h3>
        <pre className="bg-[#0d1117] border border-slate-800 p-4 rounded-lg overflow-x-auto text-sm text-slate-300">
          <code>{`curl -X GET "https://api.sorobanscanner.com/v1/queue/stats" \\
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..."`}</code>
        </pre>
      </section>
    </div>
  );
}
