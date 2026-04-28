// Multi-signature wallet utilities and helpers

export interface SignerInfo {
  id: string;
  name: string;
  publicKey: string;
  weight: number;
  signatureScheme: 'ed25519' | 'secp256k1' | 'p256';
}

export interface MultiSigConfig {
  name: string;
  description: string;
  signers: SignerInfo[];
  threshold: number;
  timeLock: number;
  network: 'mainnet' | 'testnet' | 'futurenet';
}

export interface ValidationResult {
  isValid: boolean;
  errors: string[];
  warnings: string[];
}

// Public key validation functions
export function isValidPublicKey(publicKey: string, scheme: string): boolean {
  if (!publicKey) return false;
  
  switch (scheme) {
    case 'ed25519':
      // Stellar Ed25519 public keys start with 'G' and are 56 characters
      return /^G[A-Z0-9]{55}$/.test(publicKey);
    case 'secp256k1':
      // Secp256k1 compressed public keys are 66 hex characters (33 bytes)
      return /^0[2-3][0-9a-fA-F]{64}$/.test(publicKey);
    case 'p256':
      // P256 compressed public keys are 66 hex characters (33 bytes)
      return /^0[2-3][0-9a-fA-F]{64}$/.test(publicKey);
    default:
      return false;
  }
}

// Stellar address validation
export function isValidStellarAddress(address: string): boolean {
  // Stellar addresses start with 'G' and are 56 characters long
  return /^G[A-Z0-9]{55}$/.test(address);
}

// Threshold calculation utilities
export function calculateTotalWeight(signers: SignerInfo[]): number {
  return signers.reduce((sum, signer) => sum + signer.weight, 0);
}

export function calculateSignersNeeded(threshold: number, signers: SignerInfo[]): number {
  const sortedSigners = [...signers].sort((a, b) => b.weight - a.weight);
  let accumulated = 0;
  let count = 0;

  for (const signer of sortedSigners) {
    accumulated += signer.weight;
    count++;
    if (accumulated >= threshold) break;
  }

  return count;
}

export function getThresholdRecommendations(signers: SignerInfo[]): {
  conservative: number;
  standard: number;
  flexible: number;
} {
  const totalWeight = calculateTotalWeight(signers);
  
  return {
    conservative: Math.ceil(totalWeight * 0.8), // 80% - high security
    standard: Math.ceil(totalWeight * 0.67), // 67% - supermajority
    flexible: Math.ceil(totalWeight * 0.51) // 51% - simple majority
  };
}

// Multi-sig configuration validation
export function validateMultiSigConfig(config: MultiSigConfig): ValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  // Basic info validation
  if (!config.name.trim()) {
    errors.push('Wallet name is required');
  } else if (config.name.length < 3) {
    errors.push('Wallet name must be at least 3 characters');
  } else if (config.name.length > 50) {
    errors.push('Wallet name must be less than 50 characters');
  }

  if (config.description && config.description.length > 200) {
    warnings.push('Description is quite long, consider keeping it concise');
  }

  // Signers validation
  if (config.signers.length === 0) {
    errors.push('At least one signer is required');
  }

  if (config.signers.length < 2) {
    warnings.push('Consider adding multiple signers for better security');
  }

  if (config.signers.length > 20) {
    errors.push('Maximum 20 signers allowed');
  }

  // Check for duplicate names and public keys
  const names = new Set<string>();
  const publicKeys = new Set<string>();

  config.signers.forEach((signer, index) => {
    if (!signer.name.trim()) {
      errors.push(`Signer ${index + 1} name is required`);
    }

    if (names.has(signer.name)) {
      errors.push(`Duplicate signer name: ${signer.name}`);
    } else {
      names.add(signer.name);
    }

    if (!signer.publicKey.trim()) {
      errors.push(`Signer ${index + 1} public key is required`);
    } else if (!isValidPublicKey(signer.publicKey, signer.signatureScheme)) {
      errors.push(`Invalid public key format for ${signer.name}`);
    }

    if (publicKeys.has(signer.publicKey)) {
      errors.push(`Duplicate public key: ${signer.publicKey}`);
    } else {
      publicKeys.add(signer.publicKey);
    }

    if (signer.weight < 1 || signer.weight > 100) {
      errors.push(`Signer ${signer.name} weight must be between 1 and 100`);
    }
  });

  // Threshold validation
  const totalWeight = calculateTotalWeight(config.signers);

  if (config.threshold < 1) {
    errors.push('Threshold must be at least 1');
  }

  if (config.threshold > totalWeight) {
    errors.push('Threshold cannot exceed total signer weight');
  }

  if (config.threshold === totalWeight) {
    warnings.push('Threshold equals total weight - all signers must approve');
  }

  if (config.threshold === 1 && config.signers.length > 1) {
    warnings.push('Threshold of 1 with multiple signers - any single signer can approve');
  }

  // Time lock validation
  if (config.timeLock < 0) {
    errors.push('Time lock cannot be negative');
  }

  if (config.timeLock > 86400 * 30) { // 30 days
    warnings.push('Time lock is very long (over 30 days)');
  }

  return {
    isValid: errors.length === 0,
    errors,
    warnings
  };
}

// Configuration generation for Stellar SDK
export function generateStellarMultiSigConfig(config: MultiSigConfig): {
  signerKeys: Array<{ publicKey: string; weight: number }>;
  masterKey: string;
  lowThreshold: number;
  medThreshold: number;
  highThreshold: number;
} {
  const signerKeys = config.signers.map(signer => ({
    publicKey: signer.publicKey,
    weight: signer.weight
  }));

  // Use the same threshold for all operations for simplicity
  // In production, you might want different thresholds for different operation types
  const threshold = config.threshold;

  return {
    signerKeys,
    masterKey: config.signers[0]?.publicKey || '', // First signer as master key
    lowThreshold: threshold,
    medThreshold: threshold,
    highThreshold: threshold
  };
}

// Transaction simulation
export function simulateMultiSigTransaction(
  config: MultiSigConfig,
  approvingSigners: string[]
): {
  canExecute: boolean;
  currentWeight: number;
  requiredWeight: number;
  remainingWeight: number;
  signersNeeded: number;
} {
  const totalWeight = calculateTotalWeight(config.signers);
  const currentWeight = config.signers
    .filter(signer => approvingSigners.indexOf(signer.publicKey) !== -1)
    .reduce((sum, signer) => sum + signer.weight, 0);

  const remainingWeight = config.threshold - currentWeight;
  const signersNeeded = calculateSignersNeeded(remainingWeight, 
    config.signers.filter(signer => approvingSigners.indexOf(signer.publicKey) === -1)
  );

  return {
    canExecute: currentWeight >= config.threshold,
    currentWeight,
    requiredWeight: config.threshold,
    remainingWeight: Math.max(0, remainingWeight),
    signersNeeded: Math.max(0, signersNeeded)
  };
}

// Security analysis
export function analyzeSecurity(config: MultiSigConfig): {
  score: number;
  risks: string[];
  recommendations: string[];
} {
  const risks: string[] = [];
  const recommendations: string[] = [];
  let score = 100;

  // Analyze signer count
  if (config.signers.length === 1) {
    risks.push('Single point of failure - only one signer');
    score -= 40;
  } else if (config.signers.length === 2) {
    risks.push('Limited redundancy with only 2 signers');
    score -= 20;
  }

  // Analyze threshold
  const totalWeight = calculateTotalWeight(config.signers);
  const thresholdPercentage = (config.threshold / totalWeight) * 100;

  if (thresholdPercentage < 51) {
    risks.push('Low threshold allows minority control');
    score -= 25;
  } else if (thresholdPercentage > 90) {
    risks.push('Very high threshold may cause operational issues');
    score -= 10;
  }

  // Analyze weight distribution
  const maxWeight = Math.max(...config.signers.map(s => s.weight));
  const maxWeightPercentage = (maxWeight / totalWeight) * 100;

  if (maxWeightPercentage > 50) {
    risks.push('Single signer has majority control');
    score -= 20;
  }

  // Analyze signature schemes
  const ed25519Count = config.signers.filter(s => s.signatureScheme === 'ed25519').length;
  if (ed25519Count === config.signers.length) {
    recommendations.push('Consider using multiple signature schemes for diversity');
  }

  // Analyze time lock
  if (config.timeLock === 0) {
    risks.push('No time delay - vulnerable to rapid attacks');
    score -= 15;
  } else if (config.timeLock < 3600) {
    recommendations.push('Consider longer time lock for better security');
  }

  // Generate recommendations
  if (score < 70) {
    recommendations.push('Review security configuration - multiple risks detected');
  } else if (score < 85) {
    recommendations.push('Consider implementing suggested improvements');
  }

  return {
    score: Math.max(0, score),
    risks,
    recommendations
  };
}

// Export/Import functionality
export function exportConfig(config: MultiSigConfig): string {
  return JSON.stringify(config, null, 2);
}

export function importConfig(jsonString: string): MultiSigConfig | null {
  try {
    const config = JSON.parse(jsonString);
    
    // Basic validation
    if (!config.name || !Array.isArray(config.signers) || typeof config.threshold !== 'number') {
      throw new Error('Invalid configuration format');
    }
    
    return config;
  } catch (error) {
    console.error('Failed to import configuration:', error);
    return null;
  }
}

// Network configuration
export function getNetworkConfig(network: 'mainnet' | 'testnet' | 'futurenet'): {
  horizonUrl: string;
  networkPassphrase: string;
  isTestnet: boolean;
} {
  const configs = {
    mainnet: {
      horizonUrl: 'https://horizon.stellar.org',
      networkPassphrase: 'Public Global Stellar Network ; September 2015',
      isTestnet: false
    },
    testnet: {
      horizonUrl: 'https://horizon-testnet.stellar.org',
      networkPassphrase: 'Test SDF Network ; September 2015',
      isTestnet: true
    },
    futurenet: {
      horizonUrl: 'https://horizon-futurenet.stellar.org',
      networkPassphrase: 'Test SDF Future Network ; October 2022',
      isTestnet: true
    }
  };

  return configs[network];
}

// Utility functions
export function formatDuration(seconds: number): string {
  if (seconds === 0) return 'None';
  
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);

  if (days > 0) {
    return `${days}d ${hours}h ${minutes}m`;
  } else if (hours > 0) {
    return `${hours}h ${minutes}m`;
  } else {
    return `${minutes}m`;
  }
}

export function truncatePublicKey(publicKey: string, startChars: number = 8, endChars: number = 4): string {
  if (publicKey.length <= startChars + endChars) {
    return publicKey;
  }
  
  return `${publicKey.substring(0, startChars)}...${publicKey.substring(publicKey.length - endChars)}`;
}

export function generateSignerId(): string {
  return Date.now().toString() + Math.random().toString(36).substr(2, 9);
}
