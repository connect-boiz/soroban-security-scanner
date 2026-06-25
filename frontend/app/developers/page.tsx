import Link from 'next/link';
import { ArrowRight, Book, Terminal, Shield, Zap, Code2, Cpu } from 'lucide-react';
import React from 'react';

const features = [
  {
    name: 'Secure by Default',
    description: 'Built-in security rules and automatic vulnerability detection for smart contracts.',
    icon: Shield,
    color: 'text-emerald-400',
    bg: 'bg-emerald-400/10',
    border: 'border-emerald-400/20',
  },
  {
    name: 'High Performance',
    description: 'Lightning-fast analysis engine capable of processing thousands of operations per second.',
    icon: Zap,
    color: 'text-amber-400',
    bg: 'bg-amber-400/10',
    border: 'border-amber-400/20',
  },
  {
    name: 'Developer Friendly',
    description: 'Comprehensive SDKs, detailed API documentation, and interactive exploration tools.',
    icon: Code2,
    color: 'text-cyan-400',
    bg: 'bg-cyan-400/10',
    border: 'border-cyan-400/20',
  },
  {
    name: 'Scalable Architecture',
    description: 'Designed for enterprise scale with distributed queue management and monitoring.',
    icon: Cpu,
    color: 'text-purple-400',
    bg: 'bg-purple-400/10',
    border: 'border-purple-400/20',
  },
];

export default function DeveloperPortalHome() {
  return (
    <div className="py-12 px-8 max-w-6xl mx-auto min-h-screen">
      {/* Hero Section */}
      <div className="relative mb-20 pt-10">
        <div className="absolute -top-24 -left-24 w-96 h-96 bg-cyan-500/10 blur-[100px] rounded-full pointer-events-none" />
        <div className="absolute top-10 right-10 w-72 h-72 bg-blue-500/10 blur-[80px] rounded-full pointer-events-none" />
        
        <h1 className="text-5xl font-extrabold tracking-tight text-white sm:text-6xl mb-6 relative z-10">
          Build securely with <br />
          <span className="text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-blue-500">
            Soroban Scanner
          </span>
        </h1>
        <p className="mt-4 text-xl text-slate-400 max-w-2xl relative z-10 leading-relaxed">
          Integrate enterprise-grade smart contract security analysis into your workflow. 
          Our comprehensive APIs and SDKs make it easy to secure your decentralized applications.
        </p>
        
        <div className="mt-10 flex items-center gap-x-6 relative z-10">
          <Link
            href="/developers/api-reference"
            className="group relative inline-flex items-center justify-center px-8 py-3.5 text-base font-semibold text-white transition-all duration-200 bg-cyan-500 border border-transparent rounded-lg hover:bg-cyan-400 hover:shadow-[0_0_20px_rgba(34,211,238,0.4)] focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-cyan-500 focus:ring-offset-slate-900"
          >
            Explore API
            <ArrowRight className="ml-2 h-5 w-5 group-hover:translate-x-1 transition-transform" />
          </Link>
          <Link
            href="/developers/authentication"
            className="text-base font-semibold leading-6 text-slate-300 hover:text-white transition-colors flex items-center gap-2 group"
          >
            Authentication Guide <span aria-hidden="true" className="group-hover:translate-x-1 transition-transform">→</span>
          </Link>
        </div>
      </div>

      {/* Quick Links Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-20">
        <Link href="/developers/sdks" className="group relative rounded-2xl border border-slate-800 bg-[#151e32] p-8 hover:border-slate-700 transition-all duration-300 overflow-hidden">
          <div className="absolute inset-0 bg-gradient-to-br from-blue-500/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity" />
          <div className="flex items-center gap-4 mb-4 relative z-10">
            <div className="p-3 rounded-lg bg-blue-500/10 text-blue-400">
              <Terminal className="h-6 w-6" />
            </div>
            <h3 className="text-xl font-bold text-white">SDKs & Libraries</h3>
          </div>
          <p className="text-slate-400 relative z-10">
            Official client libraries for JavaScript, Python, and Rust. Start building faster with native language support.
          </p>
        </Link>
        
        <Link href="/developers/integration" className="group relative rounded-2xl border border-slate-800 bg-[#151e32] p-8 hover:border-slate-700 transition-all duration-300 overflow-hidden">
          <div className="absolute inset-0 bg-gradient-to-br from-emerald-500/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity" />
          <div className="flex items-center gap-4 mb-4 relative z-10">
            <div className="p-3 rounded-lg bg-emerald-500/10 text-emerald-400">
              <Book className="h-6 w-6" />
            </div>
            <h3 className="text-xl font-bold text-white">Integration Guides</h3>
          </div>
          <p className="text-slate-400 relative z-10">
            Step-by-step tutorials and architectures for integrating the security scanner into your CI/CD pipelines.
          </p>
        </Link>
      </div>

      {/* Features List */}
      <div className="mb-20">
        <h2 className="text-2xl font-bold text-white mb-8">Platform Capabilities</h2>
        <dl className="grid grid-cols-1 gap-x-8 gap-y-12 sm:grid-cols-2 lg:grid-cols-2">
          {features.map((feature) => (
            <div key={feature.name} className={`relative p-6 rounded-2xl border ${feature.border} bg-[#131b2c] hover:bg-[#162032] transition-colors`}>
              <dt className="text-lg font-semibold leading-7 text-white flex items-center gap-3 mb-3">
                <div className={`p-2 rounded-lg ${feature.bg} ${feature.color}`}>
                  <feature.icon className="h-5 w-5" aria-hidden="true" />
                </div>
                {feature.name}
              </dt>
              <dd className="text-base leading-7 text-slate-400">
                {feature.description}
              </dd>
            </div>
          ))}
        </dl>
      </div>

      {/* Community Section */}
      <div className="rounded-2xl border border-slate-800 bg-gradient-to-br from-slate-900 to-[#151e32] p-10 text-center relative overflow-hidden">
        <div className="absolute top-0 right-0 -mt-4 -mr-4 w-32 h-32 bg-cyan-500/10 blur-[40px] rounded-full pointer-events-none" />
        <h2 className="text-2xl font-bold text-white mb-4 relative z-10">Help Us Improve</h2>
        <p className="text-slate-400 mb-8 max-w-xl mx-auto relative z-10">
          We constantly update our APIs and documentation based on developer needs. 
          If you notice missing information or have feature requests, please let us know!
        </p>
        <Link
          href="/developers/feedback"
          className="inline-flex items-center justify-center px-6 py-2.5 text-sm font-semibold text-white transition-all duration-200 bg-slate-800 border border-slate-700 rounded-lg hover:bg-slate-700 hover:text-cyan-400 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-slate-700 focus:ring-offset-[#0f172a] relative z-10"
        >
          Submit Feedback
        </Link>
      </div>
    </div>
  );
}
