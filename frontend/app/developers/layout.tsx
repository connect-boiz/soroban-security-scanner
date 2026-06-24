import Link from 'next/link';
import { BookOpen, Key, Activity, Code, List, MessageSquare, Zap, ShieldAlert } from 'lucide-react';

const navItems = [
  { name: 'Quick Start', href: '/developers', icon: Zap },
  { name: 'API Reference', href: '/developers/api-reference', icon: List },
  { name: 'Authentication', href: '/developers/authentication', icon: Key },
  { name: 'SDKs', href: '/developers/sdks', icon: Code },
  { name: 'Integration Guides', href: '/developers/integration', icon: BookOpen },
  { name: 'Error Codes', href: '/developers/errors', icon: ShieldAlert },
  { name: 'Rate Limiting', href: '/developers/rate-limiting', icon: Activity },
  { name: 'Changelog', href: '/developers/changelog', icon: List },
  { name: 'Feedback', href: '/developers/feedback', icon: MessageSquare },
];

export default function DevelopersLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex min-h-screen bg-[#0f172a] text-slate-200 selection:bg-cyan-500/30">
      {/* Sidebar Navigation */}
      <aside className="w-64 border-r border-slate-800 bg-[#0b1121] flex flex-col fixed inset-y-0 left-0 z-10">
        <div className="flex h-16 shrink-0 items-center px-6 border-b border-slate-800">
          <Link href="/" className="flex items-center gap-2 group">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-cyan-400 to-blue-600 flex items-center justify-center text-white font-bold group-hover:shadow-[0_0_15px_rgba(34,211,238,0.5)] transition-all duration-300">
              S
            </div>
            <span className="font-semibold text-lg tracking-tight text-white group-hover:text-cyan-400 transition-colors">
              Soroban Devs
            </span>
          </Link>
        </div>
        <div className="flex-1 overflow-y-auto px-4 py-6 custom-scrollbar">
          <nav className="flex flex-1 flex-col">
            <ul role="list" className="flex flex-1 flex-col gap-y-7">
              <li>
                <div className="text-xs font-semibold leading-6 text-slate-400 uppercase tracking-wider mb-2 px-2">
                  Documentation
                </div>
                <ul role="list" className="-mx-2 space-y-1">
                  {navItems.map(item => (
                    <li key={item.name}>
                      <Link
                        href={item.href}
                        className="group flex gap-x-3 rounded-md p-2 text-sm leading-6 font-medium text-slate-300 hover:bg-slate-800/50 hover:text-cyan-400 transition-all duration-200 relative overflow-hidden"
                      >
                        <span className="absolute inset-y-0 left-0 w-1 bg-cyan-400 rounded-r opacity-0 group-hover:opacity-100 transition-opacity" />
                        <item.icon
                          className="h-5 w-5 shrink-0 text-slate-500 group-hover:text-cyan-400 transition-colors"
                          aria-hidden="true"
                        />
                        {item.name}
                      </Link>
                    </li>
                  ))}
                </ul>
              </li>
              <li className="mt-auto">
                <a
                  href="https://github.com/stellar/soroban-security-scanner"
                  target="_blank"
                  rel="noreferrer"
                  className="group flex items-center justify-between rounded-lg p-3 text-sm font-semibold leading-6 text-white bg-slate-800/50 hover:bg-slate-800 border border-slate-700 hover:border-slate-600 transition-all shadow-sm"
                >
                  <div className="flex items-center gap-2">
                    <Code className="h-4 w-4 text-slate-400 group-hover:text-white transition-colors" />
                    <span>View on GitHub</span>
                  </div>
                  <svg
                    className="h-4 w-4 text-slate-500 group-hover:text-white group-hover:translate-x-0.5 transition-all"
                    fill="none"
                    viewBox="0 0 24 24"
                    strokeWidth="2"
                    stroke="currentColor"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      d="M13.5 4.5L21 12m0 0l-7.5 7.5M21 12H3"
                    />
                  </svg>
                </a>
              </li>
            </ul>
          </nav>
        </div>
      </aside>

      {/* Main Content Area */}
      <main className="flex-1 pl-64">
        <div className="max-w-7xl mx-auto">{children}</div>
      </main>
    </div>
  );
}
