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
  {
    symbol: 'ETH',
    name: 'Ethereum',
    balance: '0.75',
    decimals: 18,
    usdValue: 1500.00,
    change24h: -1.2,
    contractAddress: 'CA3D5KRYM6CB7OWQ6TWYRR3Z4T7VN6LARJ7K36QKUQ5Q5Y6R7H5W7Q',
  },
  {
    symbol: 'YXLM',
    name: 'Yield Lumens',
    balance: '2500.00',
    decimals: 7,
    usdValue: 375.00,
    change24h: 5.8,
    contractAddress: 'CBVN5L2LQKZQ6K7V2N5Q4R2M7K3L9N8P5Q2R7T6W3X2Y1Z4A5B6C7',
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
  { from: 'ETH', to: 'USD', rate: 2000.00, timestamp: Date.now() },
  { from: 'YXLM', to: 'USD', rate: 0.15, timestamp: Date.now() },
];

// Date formatting utility
const formatDate = (timestamp: number, format: string): string => {
  const date = new Date(timestamp);
  if (format === 'MMM dd, yyyy HH:mm:ss') {
    return date.toLocaleDateString('en-US', { 
      month: 'short', 
      day: 'numeric', 
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit'
    });
  }
  if (format === 'MMM dd') {
    return date.toLocaleDateString('en-US', { 
      month: 'short', 
      day: 'numeric'
    });
  }
  return date.toLocaleDateString();
};

// Balance Card Component
const BalanceCard: React.FC<{ token: TokenBalance; onClick?: () => void }> = ({ token, onClick }) => {
  const changeColor = token.change24h >= 0 ? 'text-green-600' : 'text-red-600';
  const changeIcon = token.change24h >= 0 ? '↗' : '↘';

  return (
    <div 
      className="bg-white rounded-lg shadow-md p-6 hover:shadow-lg transition-shadow cursor-pointer border border-gray-200"
      onClick={onClick}
    >
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center space-x-3">
          <div className="w-12 h-12 bg-blue-100 rounded-full flex items-center justify-center">
            <span className="text-blue-600 font-bold text-sm">{token.symbol.slice(0, 2)}</span>
          </div>
          <div>
            <h3 className="font-semibold text-gray-900">{token.symbol}</h3>
            <p className="text-sm text-gray-500">{token.name}</p>
          </div>
        </div>
        <div className={`flex items-center space-x-1 ${changeColor}`}>
          <span>{changeIcon}</span>
          <span className="text-sm font-medium">{Math.abs(token.change24h)}%</span>
        </div>
      </div>
      
      <div className="space-y-2">
        <div>
          <p className="text-sm text-gray-500">Balance</p>
          <p className="text-xl font-bold text-gray-900">
            {parseFloat(token.balance).toLocaleString()}
          </p>
        </div>
        <div>
          <p className="text-sm text-gray-500">USD Value</p>
          <p className="text-lg font-semibold text-gray-900">
            ${token.usdValue.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
          </p>
        </div>
      </div>
      
      <div className="mt-4 pt-4 border-t border-gray-100">
        <p className="text-xs text-gray-400 truncate">
          {token.contractAddress.slice(0, 10)}...{token.contractAddress.slice(-10)}
        </p>
      </div>
    </div>
  );
};

// Mini Chart Component
const MiniChart: React.FC<{ data: HistoricalData[] }> = ({ data }) => {
  const maxValue = Math.max(...data.map(d => d.usdValue));
  const minValue = Math.min(...data.map(d => d.usdValue));
  const range = maxValue - minValue;

  return (
    <div className="h-16 flex items-end space-x-1">
      {data.slice(-14).map((point, index) => {
        const height = range > 0 ? ((point.usdValue - minValue) / range) * 100 : 50;
        return (
          <div
            key={index}
            className="flex-1 bg-blue-200 hover:bg-blue-300 transition-colors"
            style={{ height: `${height}%` }}
            title={`${formatDate(point.timestamp, 'MMM dd')}: $${point.usdValue.toFixed(2)}`}
          />
        );
      })}
    </div>
  );
};

// Conversion Panel Component
const ConversionPanel: React.FC<{ 
  tokens: TokenBalance[]; 
  conversionRates: ConversionRate[];
}> = ({ tokens, conversionRates }) => {
  const [fromToken, setFromToken] = useState(tokens[0]?.symbol || '');
  const [toToken, setToToken] = useState('USD');
  const [amount, setAmount] = useState('1');

  const getConversionRate = (from: string, to: string): number => {
    if (from === to) return 1;
    const rate = conversionRates.find(r => r.from === from && r.to === to);
    return rate?.rate || 0;
  };

  const convertedAmount = parseFloat(amount || '0') * getConversionRate(fromToken, toToken);

  return (
    <div className="bg-white rounded-lg shadow-md p-6 border border-gray-200">
      <h3 className="text-lg font-semibold text-gray-900 mb-4">Token Converter</h3>
      <div className="space-y-4">
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">From</label>
          <div className="flex space-x-2">
            <input
              type="number"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              className="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="Amount"
            />
            <select
              value={fromToken}
              onChange={(e) => setFromToken(e.target.value)}
              className="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              {tokens.map(token => (
                <option key={token.symbol} value={token.symbol}>{token.symbol}</option>
              ))}
            </select>
          </div>
        </div>
        
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">To</label>
          <select
            value={toToken}
            onChange={(e) => setToToken(e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="USD">USD</option>
            {tokens.map(token => (
              <option key={token.symbol} value={token.symbol}>{token.symbol}</option>
            ))}
          </select>
        </div>
        
        <div className="pt-4 border-t border-gray-200">
          <p className="text-sm text-gray-500">Result</p>
          <p className="text-2xl font-bold text-gray-900">
            {convertedAmount.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 6 })} {toToken}
          </p>
          <p className="text-xs text-gray-400 mt-1">
            Rate: 1 {fromToken} = {getConversionRate(fromToken, toToken).toFixed(6)} {toToken}
          </p>
        </div>
      </div>
    </div>
  );
};

// Main BalanceDisplay Component
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
  const [historicalData, setHistoricalData] = useState<HistoricalData[]>(
    initialHistoricalData || generateMockHistoricalData()
  );
  const [conversionRates, setConversionRates] = useState<ConversionRate[]>(
    initialConversionRates || generateMockConversionRates()
  );
  const [selectedToken, setSelectedToken] = useState<TokenBalance | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date>(new Date());
  const [isLoading, setIsLoading] = useState(false);
  const [isInitialLoading, setIsInitialLoading] = useState(false);
  const [isRefreshingData, setIsRefreshingData] = useState(false);
  const [refreshProgress, setRefreshProgress] = useState(0);

  // Calculate total portfolio value
  const totalValue = tokens.reduce((sum, token) => sum + token.usdValue, 0);
  const totalChange24h = tokens.reduce((sum, token) => {
    const tokenValue = parseFloat(token.balance) * (token.usdValue / parseFloat(token.balance));
    return sum + (tokenValue * token.change24h / 100);
  }, 0);
  const totalChangePercentage = totalValue > 0 ? (totalChange24h / totalValue) * 100 : 0;

  // Simulate initial data loading
  useEffect(() => {
    const loadInitialData = async () => {
      setIsInitialLoading(true);
      setRefreshProgress(0);
      
      // Simulate progressive loading
      const stages = [
        { progress: 25, delay: 400 },
        { progress: 50, delay: 300 },
        { progress: 75, delay: 500 },
        { progress: 100, delay: 300 }
      ];
      
      for (const stage of stages) {
        await new Promise(resolve => setTimeout(resolve, stage.delay));
        setRefreshProgress(stage.progress);
      }
      
      setIsInitialLoading(false);
      setRefreshProgress(0);
    };
    
    loadInitialData();
  }, []);

  // Real-time updates simulation
  useEffect(() => {
    if (!realTimeUpdates) return;

    const interval = setInterval(() => {
      setTokens(prevTokens =>
        prevTokens.map(token => ({
          ...token,
          usdValue: token.usdValue * (1 + (Math.random() - 0.5) * 0.002),
          change24h: token.change24h + (Math.random() - 0.5) * 0.1,
        }))
      );
      setLastUpdated(new Date());
    }, 5000);

    return () => clearInterval(interval);
  }, [realTimeUpdates]);

  // Enhanced refresh handler with progress tracking
  const handleRefresh = useCallback(async () => {
    setIsRefreshingData(true);
    setRefreshProgress(0);
    
    try {
      // Simulate progressive refresh
      const stages = [
        { progress: 20, delay: 200, message: 'Fetching token balances...' },
        { progress: 40, delay: 300, message: 'Updating market data...' },
        { progress: 60, delay: 400, message: 'Calculating conversions...' },
        { progress: 80, delay: 200, message: 'Processing historical data...' },
        { progress: 100, delay: 100, message: 'Finalizing...' }
      ];
      
      for (const stage of stages) {
        await new Promise(resolve => setTimeout(resolve, stage.delay));
        setRefreshProgress(stage.progress);
      }
      
      setTokens(generateMockTokenBalances());
      setHistoricalData(generateMockHistoricalData());
      setConversionRates(generateMockConversionRates());
      setLastUpdated(new Date());
      onRefresh?.();
    } finally {
      setIsRefreshingData(false);
      setRefreshProgress(0);
    }
  }, [onRefresh]);

  return (
    <LoadingOverlay isLoading={isInitialLoading} text="Loading portfolio data...">
      <div className={`space-y-6 ${className}`}>
        {/* Progress bar for refresh operations */}
        {(isRefreshingData || refreshProgress > 0) && (
          <div className="bg-white rounded-lg shadow-md p-4 border border-gray-200">
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm font-medium text-gray-700">
                {isRefreshingData ? 'Refreshing data...' : 'Loading...'}
              </span>
              <span className="text-sm text-gray-600">{refreshProgress}%</span>
            </div>
            <ProgressBar 
              value={refreshProgress} 
              color="blue"
              showLabel={false}
              className="w-full"
            />
          </div>
        )}

        {/* Header */}
        <div className="bg-white rounded-lg shadow-md p-6 border border-gray-200">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-2xl font-bold text-gray-900">Portfolio Balance</h2>
              <p className="text-sm text-gray-500">
                Last updated: {formatDate(lastUpdated.getTime(), 'MMM dd, yyyy HH:mm:ss')}
              </p>
            </div>
            <button
              onClick={handleRefresh}
              disabled={isRefreshingData}
              className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              {isRefreshingData ? 'Refreshing...' : 'Refresh'}
            </button>
          </div>
          
          <div className="mt-6 grid grid-cols-1 md:grid-cols-3 gap-4">
            <div>
              <p className="text-sm text-gray-500">Total Value</p>
              <p className="text-3xl font-bold text-gray-900">
                ${totalValue.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
              </p>
            </div>
            <div>
              <p className="text-sm text-gray-500">24h Change</p>
              <p className={`text-2xl font-bold ${totalChange24h >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                {totalChange24h >= 0 ? '+' : ''}{totalChange24h.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                <span className="text-sm ml-2">({totalChangePercentage >= 0 ? '+' : ''}{totalChangePercentage.toFixed(2)}%)</span>
              </p>
            </div>
            <div>
              <p className="text-sm text-gray-500">Assets</p>
              <p className="text-2xl font-bold text-gray-900">{tokens.length}</p>
            </div>
          </div>
        </div>

      {/* Main Content Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Token Balances */}
        <div className="lg:col-span-2 space-y-4">
          <h3 className="text-lg font-semibold text-gray-900">Token Balances</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {isRefreshingData ? (
              <>
                <SkeletonCard lines={4} avatar={true} button={false} height="h-32" />
                <SkeletonCard lines={4} avatar={true} button={false} height="h-32" />
                <SkeletonCard lines={4} avatar={true} button={false} height="h-32" />
                <SkeletonCard lines={4} avatar={true} button={false} height="h-32" />
              </>
            ) : (
              tokens.map((token) => (
                <BalanceCard
                  key={token.contractAddress}
                  token={token}
                  onClick={() => setSelectedToken(token)}
                />
              ))
            )}
          </div>
        </div>

        {/* Side Panel */}
        <div className="space-y-6">
          {/* Mini Chart */}
          {showChart && (
            <div className="bg-white rounded-lg shadow-md p-6 border border-gray-200">
              <h3 className="text-lg font-semibold text-gray-900 mb-4">30-Day Performance</h3>
              {isRefreshingData ? (
                <SkeletonCard lines={6} avatar={false} button={false} height="h-48" />
              ) : (
                <MiniChart data={historicalData} />
              )}
            </div>
          )}

          {/* Conversion Calculator */}
          {showConversion && (
            isRefreshingData ? (
              <SkeletonCard lines={8} avatar={false} button={true} height="h-64" />
            ) : (
              <ConversionCalculator
                tokens={tokens}
                conversionRates={conversionRates}
              />
            )
          )}
        </div>
      </div>
                onClick={() => setSelectedToken(null)}
                className="text-gray-400 hover:text-gray-600"
              >
                ✕
              </button>
            </div>
            <div className="space-y-4">
              <div>
                <p className="text-sm text-gray-500">Contract Address</p>
                <p className="font-mono text-sm bg-gray-100 p-2 rounded">{selectedToken.contractAddress}</p>
              </div>
              <div>
                <p className="text-sm text-gray-500">Decimals</p>
                <p className="font-semibold">{selectedToken.decimals}</p>
              </div>
              <div>
                <p className="text-sm text-gray-500">24h Change</p>
                <p className={`font-semibold ${selectedToken.change24h >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                  {selectedToken.change24h >= 0 ? '+' : ''}{selectedToken.change24h}%
                </p>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default BalanceDisplay;
