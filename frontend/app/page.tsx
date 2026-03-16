import Header from '@/components/Header';
import ScannerInterface from '@/components/ScannerInterface';
import VulnerabilityList from '@/components/VulnerabilityList';

export default function Home() {
  return (
    <div className="min-h-screen bg-gray-50">
      <Header />
      
      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-gray-900 mb-2">
            Soroban Security Scanner
          </h1>
          <p className="text-gray-600">
            Comprehensive security analysis for Soroban smart contracts
          </p>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
          <ScannerInterface />
          <VulnerabilityList />
        </div>
      </main>
    </div>
  );
}
