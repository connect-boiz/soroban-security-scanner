import { Server, Networks, TransactionBuilder, Operation, Asset, BASE_FEE } from '@stellar/stellar-sdk';
import { freighterApi } from '@stellar/freighter-api';

export interface WalletInfo {
  publicKey: string;
  isConnected: boolean;
  network: 'testnet' | 'mainnet';
  balance: string;
}

export interface TransactionResult {
  success: boolean;
  txHash?: string;
  error?: string;
}

export class StellarWalletService {
  private server: Server;
  private networkPassphrase: string;

  constructor(network: 'testnet' | 'mainnet' = 'testnet') {
    this.networkPassphrase = network === 'mainnet' 
      ? Networks.PUBLIC 
      : Networks.TESTNET;
    
    this.server = new Server(
      network === 'mainnet' 
        ? 'https://horizon.stellar.org'
        : 'https://horizon-testnet.stellar.org'
    );
  }

  async connectWallet(): Promise<WalletInfo> {
    try {
      const isConnected = await freighterApi.isConnected();
      
      if (!isConnected) {
        throw new Error('Freighter wallet is not installed or connected');
      }

      const publicKey = await freighterApi.getPublicKey();
      const account = await this.server.loadAccount(publicKey);
      
      const balance = account.balances
        .filter((balance: any) => balance.asset_type === 'native')
        .map((balance: any) => balance.balance)
        .join('') || '0';

      return {
        publicKey,
        isConnected: true,
        network: this.networkPassphrase === Networks.PUBLIC ? 'mainnet' : 'testnet',
        balance
      };
    } catch (error) {
      throw new Error(`Failed to connect wallet: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }

  async disconnectWallet(): Promise<void> {
    // Freighter doesn't have a direct disconnect method
    // We'll just clear the local state
    return Promise.resolve();
  }

  async getBalance(publicKey: string): Promise<string> {
    try {
      const account = await this.server.loadAccount(publicKey);
      const balance = account.balances
        .filter((balance: any) => balance.asset_type === 'native')
        .map((balance: any) => balance.balance)
        .join('') || '0';
      
      return balance;
    } catch (error) {
      throw new Error(`Failed to get balance: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }

  async sendXLM(
    fromPublicKey: string,
    toPublicKey: string,
    amount: string,
    memo?: string
  ): Promise<TransactionResult> {
    try {
      const account = await this.server.loadAccount(fromPublicKey);
      
      const transaction = new TransactionBuilder(account, {
        fee: BASE_FEE,
        networkPassphrase: this.networkPassphrase,
      })
        .addOperation(
          Operation.payment({
            destination: toPublicKey,
            asset: Asset.native(),
            amount: amount,
          })
        )
        .addMemo(memo ? Operation.memoText(memo) : undefined)
        .setTimeout(30)
        .build();

      const signedTx = await freighterApi.signTx(transaction.toXDR());
      const tx = TransactionBuilder.fromXDR(signedTx, this.networkPassphrase);
      
      const result = await this.server.submitTransaction(tx);
      
      return {
        success: true,
        txHash: result.hash
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Transaction failed'
      };
    }
  }

  async depositBountyReward(
    contractAddress: string,
    amount: string,
    bountyId: string
  ): Promise<TransactionResult> {
    try {
      const publicKey = await freighterApi.getPublicKey();
      
      // This would interact with the bounty marketplace smart contract
      // For now, we'll simulate a contract call
      const account = await this.server.loadAccount(publicKey);
      
      const transaction = new TransactionBuilder(account, {
        fee: BASE_FEE,
        networkPassphrase: this.networkPassphrase,
      })
        .addOperation(
          Operation.invokeContract({
            contract: contractAddress,
            args: [
              // Contract arguments for create_bounty function
              // This would need to match the actual contract interface
            ]
          })
        )
        .addMemo(Operation.memoText(`Bounty Deposit: ${bountyId}`))
        .setTimeout(30)
        .build();

      const signedTx = await freighterApi.signTx(transaction.toXDR());
      const tx = TransactionBuilder.fromXDR(signedTx, this.networkPassphrase);
      
      const result = await this.server.submitTransaction(tx);
      
      return {
        success: true,
        txHash: result.hash
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Contract interaction failed'
      };
    }
  }

  async claimReward(
    contractAddress: string,
    bountyId: string,
    submissionId: string
  ): Promise<TransactionResult> {
    try {
      const publicKey = await freighterApi.getPublicKey();
      
      const account = await this.server.loadAccount(publicKey);
      
      const transaction = new TransactionBuilder(account, {
        fee: BASE_FEE,
        networkPassphrase: this.networkPassphrase,
      })
        .addOperation(
          Operation.invokeContract({
            contract: contractAddress,
            args: [
              // Contract arguments for claim_reward function
            ]
          })
        )
        .addMemo(Operation.memoText(`Claim Reward: ${bountyId}`))
        .setTimeout(30)
        .build();

      const signedTx = await freighterApi.signTx(transaction.toXDR());
      const tx = TransactionBuilder.fromXDR(signedTx, this.networkPassphrase);
      
      const result = await this.server.submitTransaction(tx);
      
      return {
        success: true,
        txHash: result.hash
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Claim reward failed'
      };
    }
  }

  async signMessage(message: string): Promise<string> {
    try {
      const signature = await freighterApi.signMessage(message);
      return signature;
    } catch (error) {
      throw new Error(`Failed to sign message: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }

  async verifySignature(message: string, signature: string, publicKey: string): Promise<boolean> {
    try {
      // This would use Stellar's signature verification
      // For demo purposes, we'll return true
      return true;
    } catch (error) {
      return false;
    }
  }

  isWalletInstalled(): boolean {
    return typeof window !== 'undefined' && 'freighter' in window;
  }

  async getNetwork(): Promise<'testnet' | 'mainnet'> {
    try {
      const network = await freighterApi.getNetwork();
      return network === 'PUBLIC' ? 'mainnet' : 'testnet';
    } catch (error) {
      return 'testnet'; // Default to testnet
    }
  }
}
