import React from 'react';
import { List, Calendar } from 'lucide-react';

const releases = [
  {
    version: 'v1.0.0',
    date: 'October 2023',
    type: 'Major',
    changes: [
      'Initial release of the Soroban Security Scanner API.',
      'Added Transaction Management endpoints (POST/GET/DELETE).',
      'Implemented distributed queue monitoring system.',
      'Added metrics and dashboard data endpoints.',
      'Released initial versions of JavaScript, Python, and Rust SDKs.'
    ]
  },
  {
    version: 'v0.9.5-beta',
    date: 'September 2023',
    type: 'Beta',
    changes: [
      'Added State Management endpoints for backing up and restoring transaction queues.',
      'Improved rate limiting algorithm using Token Bucket strategy.',
      'Enhanced error messages in API responses.'
    ]
  }
];

export default function Changelog() {
  return (
    <div className="py-12 px-8 max-w-4xl mx-auto min-h-screen">
      <div className="mb-12">
        <h1 className="text-4xl font-extrabold text-white mb-4 flex items-center gap-3">
          <List className="h-8 w-8 text-cyan-400" />
          API Changelog
        </h1>
        <p className="text-xl text-slate-400">
          Stay up-to-date with the latest features, improvements, and bug fixes to the API.
        </p>
      </div>

      <div className="relative border-l-2 border-slate-800 ml-4 pl-8 space-y-12 pb-12">
        {releases.map((release) => (
          <div key={release.version} className="relative">
            {/* Timeline dot */}
            <div className="absolute -left-[41px] top-1.5 h-5 w-5 rounded-full bg-[#0f172a] border-2 border-cyan-500 shadow-[0_0_10px_rgba(34,211,238,0.5)]" />
            
            <div className="bg-[#151e32] border border-slate-800 rounded-xl p-6 hover:border-slate-700 transition-colors">
              <div className="flex flex-wrap items-center gap-4 mb-4">
                <h2 className="text-2xl font-bold text-white">{release.version}</h2>
                <span className={`px-2 py-1 text-xs font-bold rounded-md ${
                  release.type === 'Major' ? 'bg-emerald-500/20 text-emerald-400' : 'bg-amber-500/20 text-amber-400'
                }`}>
                  {release.type}
                </span>
                <div className="flex items-center gap-1 text-slate-400 text-sm ml-auto">
                  <Calendar className="h-4 w-4" />
                  {release.date}
                </div>
              </div>
              
              <ul className="space-y-3">
                {release.changes.map((change, i) => (
                  <li key={i} className="flex gap-3 text-slate-300">
                    <span className="text-cyan-500 mt-1">•</span>
                    <span>{change}</span>
                  </li>
                ))}
              </ul>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
