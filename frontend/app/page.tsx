
    import Link from 'next/link';

export default function Home() {
  return (
    <div className="min-h-screen bg-gray-50">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="text-center">
          <h1 className="text-4xl font-bold text-gray-900 mb-4">
            Soroban Security Scanner
          </h1>
          <p className="text-xl text-gray-600 mb-8">
            Comprehensive security scanning platform for Soroban smart contracts
          </p>
          <div className="space-y-4">
            <Link
              href="/dashboard"
              className="inline-flex items-center px-6 py-3 border border-transparent text-base font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700"
            >
              View Security Dashboard
              <svg
                className="ml-2 w-5 h-5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M9 5l7 7-7 7"
                />
              </svg>
            </Link>
            <div className="text-sm text-gray-500">
              Monitor vulnerability trends, scan metrics, and contract security
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
