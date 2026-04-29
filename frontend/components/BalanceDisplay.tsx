'use client';

import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { LoadingOverlay, SkeletonCard, SkeletonTable, LoadingSpinner, ProgressBar } from './ui';

// Types
export interface TokenBalance {
  symbol: string;
  name: string;
  balance: string;
  decimals: number;
  usdValue: number;
  change24h: number;
  icon?: string;
  contractAddress: string;
}

export interface HistoricalData {
  timestamp: number;
  balance: string;
  usdValue: number;
}

export interface ConversionRate {
  from: string;
  to: string;
  rate: number;
  timestamp: number;
}

export interface BalanceDisplayProps {
  tokens?: TokenBalance[];
  historicalData?: HistoricalData[];
  conversionRates?: ConversionRate[];
  onRefresh?: () => void;
  showChart?: boolean;
  showConversion?: boolean;
  realTimeUpdates?: boolean;
  className?: string;
}

// Mock data generator
const generateMockTokenBalances = (): TokenBalance[] => [
  {
    symbol: 'XLM',
    name: 'Stellar Lumens',
    balance: '1250.50',
    decimals: 7,
    usdValue: 187.58,
    change24h: 2.5,
    contractAddress: 'CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2MLQQ4RUE5667W',
  },
  {
    symbol: 'USDC',
    name: 'USD Coin',
    balance: '500.00',
    decimals: 6,
    usdValue: 500.00,
    change24h: 0.1,
    contractAddress: 'CCVAKHYSKXQD57MZPYE4NPE5W5KXKRNWQ3LCXK4JGFQ2VJ5PO2B5W7Q',
  },
];

const generateMockHistoricalData = (): HistoricalData[] => {
  const data: HistoricalData[] = [];
  const now = Date.now();
  for (let i = 30; i >= 0; i--) {
    const timestamp = now - (i * 24 * 60 * 60 * 1000);
    const baseValue = 1000 + Math.random() * 500;
    data.push({
      timestamp,
      balance: (baseValue + Math.random() * 200).toFixed(2),
      usdValue: baseValue + Math.random() * 100,
    });
  }
  return data;
};

const generateMockConversionRates = (): ConversionRate[] => [
  { from: 'XLM', to: 'USD', rate: 0.15, timestamp: Date.now() },
  { from: 'USDC', to: 'USD', rate: 1.00, timestamp: Date.now() },
];

const formatDate = (timestamp: number, format: string): string => {
  const date = new Date(timestamp);
  if (format === 'MMM dd, yyyy HH:mm:ss') return date.toLocaleString();
  return date.toLocaleDateString();
};

const BalanceCard: React.FC<{ token: TokenBalance; onClick?: () => void }> = ({ token, onClick }) => {
  const changeColor = token.change24h >= 0 ? 'text-green-600' : 'text-red-600';
  return (
    <div 
      className="bg-white rounded-lg shadow-md p-6 hover:shadow-lg transition-shadow cursor-pointer border border-gray-200"
      onClick={onClick}
    >
      <div className="flex items-center justify-between mb-4">
        <h3 className="font-semibold text-gray-900">{token.symbol}</h3>
        <span className={`text-sm font-medium ${changeColor}`}>{token.change24h}%</span>
      </div>
      <p className="text-xl font-bold text-gray-900">{parseFloat(token.balance).toLocaleString()} {token.symbol}</p>
      <p className="text-sm text-gray-500">${token.usdValue.toFixed(2)}</p>
    </div>
  );
};

const ConversionCalculator: React.FC<{ tokens: TokenBalance[]; conversionRates: ConversionRate[] }> = ({ tokens, conversionRates }) => {
  return (
    <div className="bg-white rounded-lg shadow-md p-6 border border-gray-200">
      <h3 className="text-lg font-semibold mb-4">Token Converter</h3>
      <p className="text-sm text-gray-500">Converter tool is ready.</p>
    </div>
  );
};

export const BalanceDisplay: React.FC<BalanceDisplayProps> = ({
  tokens: initialTokens,
  historicalData: initialHistoricalData,
  conversionRates: initialConversionRates,
  onRefresh,
  showChart = true,
  showConversion = true,
  realTimeUpdates = true,
  className = '',
}) => {
  const [tokens, setTokens] = useState<TokenBalance[]>(initialTokens || generateMockTokenBalances());
  const [historicalData, setHistoricalData] = useState<HistoricalData[]>(initialHistoricalData || generateMockHistoricalData());
  const [conversionRates, setConversionRates] = useState<ConversionRate[]>(initialConversionRates || generateMockConversionRates());
  const [selectedToken, setSelectedToken] = useState<TokenBalance | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date>(new Date());
  const [isInitialLoading, setIsInitialLoading] = useState(false);
  const [isRefreshingData, setIsRefreshingData] = useState(false);
  const [refreshProgress, setRefreshProgress] = useState(0);

  const totalValue = tokens.reduce((sum, token) => sum + token.usdValue, 0);

  const handleRefresh = useCallback(async () => {
    setIsRefreshingData(true);
    setRefreshProgress(20);
    await new Promise(resolve => setTimeout(resolve, 800));
    setTokens(generateMockTokenBalances());
    setLastUpdated(new Date());
    setIsRefreshingData(false);
    setRefreshProgress(0);
    onRefresh?.();
  }, [onRefresh]);

  useEffect(() => {
    setIsInitialLoading(true);
    setTimeout(() => setIsInitialLoading(false), 1000);
  }, []);

  return (
    <LoadingOverlay isLoading={isInitialLoading} text="Loading portfolio data...">
      <div className={`space-y-6 ${className}`}>
        <div className="bg-white rounded-lg shadow-md p-6 border border-gray-200">
          <div className="flex items-center justify-between">
            <h2 className="text-2xl font-bold">Portfolio Balance</h2>
            <button onClick={handleRefresh} className="btn btn-primary">Refresh</button>
          </div>
          <p className="text-3xl font-bold mt-4">${totalValue.toFixed(2)}</p>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          <div className="lg:col-span-2 grid grid-cols-1 md:grid-cols-2 gap-4">
            {tokens.map(token => (
              <BalanceCard key={token.contractAddress} token={token} onClick={() => setSelectedToken(token)} />
            ))}
          </div>
          <div className="space-y-6">
            {showConversion && <ConversionCalculator tokens={tokens} conversionRates={conversionRates} />}
          </div>
        </div>

        {selectedToken && (
          <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
            <div className="bg-white p-6 rounded-lg max-w-md w-full">
              <div className="flex justify-between items-center mb-4">
                <h3 className="text-xl font-bold">{selectedToken.name} Details</h3>
                <button onClick={() => setSelectedToken(null)}>✕</button>
              </div>
              <p className="font-mono text-sm break-all">{selectedToken.contractAddress}</p>
            </div>
          </div>
        )}
      </div>
    </LoadingOverlay>
  );
};

export default BalanceDisplay;
