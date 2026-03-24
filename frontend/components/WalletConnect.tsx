'use client';

import { useState, useEffect } from 'react';
import { StellarWalletService, WalletInfo, TransactionResult } from '@/services/stellarWallet';
import { 
  Wallet, 
  DollarSign, 
  AlertCircle, 
  CheckCircle, 
  Copy, 
  ExternalLink,
  Zap,
  Shield
} from 'lucide-react';

interface WalletConnectProps {
  onConnect?: (walletInfo: WalletInfo) => void;
  onDisconnect?: () => void;
}

export const WalletConnect: React.FC<WalletConnectProps> = ({ 
  onConnect, 
  onDisconnect 
}) => {
  const [walletInfo, setWalletInfo] = useState<WalletInfo | null>(null);
  const [isConnecting, setIsConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [stellarService] = useState(() => new StellarWalletService());

  useEffect(() => {
    checkWalletConnection();
  }, []);

  const checkWalletConnection = async () => {
    try {
      if (stellarService.isWalletInstalled()) {
        const info = await stellarService.connectWallet();
        setWalletInfo(info);
        onConnect?.(info);
      }
    } catch (error) {
      console.log('Wallet not connected');
    }
  };

  const handleConnect = async () => {
    if (!stellarService.isWalletInstalled()) {
      setError('Freighter wallet is not installed. Please install it first.');
      return;
    }

    setIsConnecting(true);
    setError(null);

    try {
      const info = await stellarService.connectWallet();
      setWalletInfo(info);
      onConnect?.(info);
    } catch (error) {
      setError(error instanceof Error ? error.message : 'Failed to connect wallet');
    } finally {
      setIsConnecting(false);
    }
  };

  const handleDisconnect = () => {
    setWalletInfo(null);
    onDisconnect?.();
    stellarService.disconnectWallet();
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
  };

  if (!stellarService.isWalletInstalled()) {
    return (
      <div className="card">
        <div className="text-center">
          <Wallet className="h-12 w-12 text-gray-400 mx-auto mb-4" />
          <h3 className="text-lg font-semibold text-gray-900 mb-2">Wallet Required</h3>
          <p className="text-gray-600 mb-4">
            Install Freighter wallet to participate in bounty programs and receive rewards
          </p>
          <a
            href="https://www.freighter.app/"
            target="_blank"
            rel="noopener noreferrer"
            className="btn-primary inline-flex items-center"
          >
            <ExternalLink className="h-4 w-4 mr-2" />
            Install Freighter
          </a>
        </div>
      </div>
    );
  }

  if (!walletInfo) {
    return (
      <div className="card">
        <div className="text-center">
          <Wallet className="h-12 w-12 text-primary-600 mx-auto mb-4" />
          <h3 className="text-lg font-semibold text-gray-900 mb-2">Connect Your Wallet</h3>
          <p className="text-gray-600 mb-4">
            Connect your Freighter wallet to start participating in security bounties
          </p>
          <button
            onClick={handleConnect}
            disabled={isConnecting}
            className="btn-primary"
          >
            {isConnecting ? (
              <>
                <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white mr-2"></div>
                Connecting...
              </>
            ) : (
              <>
                <Wallet className="h-4 w-4 mr-2" />
                Connect Wallet
              </>
            )}
          </button>
          {error && (
            <div className="mt-4 p-3 bg-red-50 border border-red-200 rounded-lg">
              <div className="flex items-center">
                <AlertCircle className="h-5 w-5 text-red-600 mr-2" />
                <span className="text-red-800 text-sm">{error}</span>
              </div>
            </div>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="card">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center">
          <div className="h-10 w-10 bg-green-100 rounded-full flex items-center justify-center mr-3">
            <CheckCircle className="h-6 w-6 text-green-600" />
          </div>
          <div>
            <h3 className="font-semibold text-gray-900">Wallet Connected</h3>
            <p className="text-sm text-gray-600">{walletInfo.network}</p>
          </div>
        </div>
        <button
          onClick={handleDisconnect}
          className="text-gray-500 hover:text-gray-700"
        >
          Disconnect
        </button>
      </div>

      <div className="space-y-3">
        <div className="flex items-center justify-between p-3 bg-gray-50 rounded-lg">
          <span className="text-sm font-medium text-gray-700">Public Key</span>
          <div className="flex items-center">
            <code className="text-sm bg-white px-2 py-1 rounded border">
              {walletInfo.publicKey.slice(0, 8)}...{walletInfo.publicKey.slice(-8)}
            </code>
            <button
              onClick={() => copyToClipboard(walletInfo.publicKey)}
              className="ml-2 text-gray-500 hover:text-gray-700"
            >
              <Copy className="h-4 w-4" />
            </button>
          </div>
        </div>

        <div className="flex items-center justify-between p-3 bg-gray-50 rounded-lg">
          <span className="text-sm font-medium text-gray-700">Balance</span>
          <div className="flex items-center">
            <DollarSign className="h-4 w-4 text-green-600 mr-1" />
            <span className="font-semibold text-green-600">
              {parseFloat(walletInfo.balance).toFixed(2)} XLM
            </span>
          </div>
        </div>
      </div>
    </div>
  );
};

interface BountyDepositProps {
  contractAddress: string;
  bountyId: string;
  onSuccess?: (txHash: string) => void;
}

export const BountyDeposit: React.FC<BountyDepositProps> = ({
  contractAddress,
  bountyId,
  onSuccess
}) => {
  const [amount, setAmount] = useState('');
  const [isDepositing, setIsDepositing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [stellarService] = useState(() => new StellarWalletService());

  const handleDeposit = async () => {
    if (!amount || parseFloat(amount) <= 0) {
      setError('Please enter a valid amount');
      return;
    }

    setIsDepositing(true);
    setError(null);
    setSuccess(null);

    try {
      const result = await stellarService.depositBountyReward(
        contractAddress,
        amount,
        bountyId
      );

      if (result.success) {
        setSuccess(`Successfully deposited ${amount} XLM to bounty ${bountyId}`);
        onSuccess?.(result.txHash!);
        setAmount('');
      } else {
        setError(result.error || 'Deposit failed');
      }
    } catch (error) {
      setError(error instanceof Error ? error.message : 'Deposit failed');
    } finally {
      setIsDepositing(false);
    }
  };

  return (
    <div className="card">
      <div className="flex items-center mb-4">
        <Zap className="h-6 w-6 text-primary-600 mr-2" />
        <h3 className="text-lg font-semibold text-gray-900">Deposit Bounty Reward</h3>
      </div>

      <div className="space-y-4">
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            Reward Amount (XLM)
          </label>
          <input
            type="number"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            placeholder="Enter amount in XLM"
            step="0.0000001"
            min="0.0000001"
            className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent"
          />
        </div>

        {error && (
          <div className="bg-red-50 border border-red-200 rounded-lg p-3">
            <div className="flex items-center">
              <AlertCircle className="h-5 w-5 text-red-600 mr-2" />
              <span className="text-red-800 text-sm">{error}</span>
            </div>
          </div>
        )}

        {success && (
          <div className="bg-green-50 border border-green-200 rounded-lg p-3">
            <div className="flex items-center">
              <CheckCircle className="h-5 w-5 text-green-600 mr-2" />
              <span className="text-green-800 text-sm">{success}</span>
            </div>
          </div>
        )}

        <button
          onClick={handleDeposit}
          disabled={isDepositing || !amount}
          className="w-full btn-primary flex items-center justify-center"
        >
          {isDepositing ? (
            <>
              <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white mr-2"></div>
              Depositing...
            </>
          ) : (
            <>
              <DollarSign className="h-4 w-4 mr-2" />
              Deposit Reward
            </>
          )}
        </button>
      </div>
    </div>
  );
};
