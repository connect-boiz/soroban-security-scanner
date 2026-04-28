'use client';

import { useState, useCallback, useMemo } from 'react';

// Import types and utilities - these will be provided by the consuming application
interface SignerInfo {
  id: string;
  name: string;
  publicKey: string;
  weight: number;
  signatureScheme: 'ed25519' | 'secp256k1' | 'p256';
}

interface MultiSigConfig {
  name: string;
  description: string;
  signers: SignerInfo[];
  threshold: number;
  timeLock: number;
  network: 'mainnet' | 'testnet' | 'futurenet';
}

interface ValidationResult {
  isValid: boolean;
  errors: string[];
  warnings: string[];
}

export type MultiSigWizardProps = {
  onConfigCreate?: (config: MultiSigConfig) => void;
  initialConfig?: Partial<MultiSigConfig>;
  className?: string;
};

// Utility functions (simplified versions for the component library)
function calculateTotalWeight(signers: SignerInfo[]): number {
  return signers.reduce((sum, signer) => sum + signer.weight, 0);
}

function calculateSignersNeeded(threshold: number, signers: SignerInfo[]): number {
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

function getThresholdRecommendations(signers: SignerInfo[]): {
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

function analyzeSecurity(config: MultiSigConfig): {
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

  return {
    score: Math.max(0, score),
    risks,
    recommendations
  };
}

function formatDuration(seconds: number): string {
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

function truncatePublicKey(publicKey: string, startChars: number = 8, endChars: number = 4): string {
  if (publicKey.length <= startChars + endChars) {
    return publicKey;
  }
  
  return `${publicKey.substring(0, startChars)}...${publicKey.substring(publicKey.length - endChars)}`;
}

function generateSignerId(): string {
  return Date.now().toString() + Math.random().toString(36).substr(2, 9);
}

function isValidPublicKey(publicKey: string, scheme: string): boolean {
  if (!publicKey) return false;
  
  switch (scheme) {
    case 'ed25519':
      return /^G[A-Z0-9]{55}$/.test(publicKey);
    case 'secp256k1':
      return /^0[2-3][0-9a-fA-F]{64}$/.test(publicKey);
    case 'p256':
      return /^0[2-3][0-9a-fA-F]{64}$/.test(publicKey);
    default:
      return false;
  }
}

function validateMultiSigConfig(config: MultiSigConfig): ValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  // Basic validation
  if (!config.name.trim()) {
    errors.push('Wallet name is required');
  }

  if (config.signers.length === 0) {
    errors.push('At least one signer is required');
  }

  const totalWeight = calculateTotalWeight(config.signers);
  if (config.threshold > totalWeight) {
    errors.push('Threshold cannot exceed total signer weight');
  }

  return {
    isValid: errors.length === 0,
    errors,
    warnings
  };
}

type WizardStep = 'basic-info' | 'signers' | 'threshold' | 'advanced' | 'preview';

const STEP_TITLES: Record<WizardStep, string> = {
  'basic-info': 'Basic Information',
  'signers': 'Configure Signers',
  'threshold': 'Set Threshold',
  'advanced': 'Advanced Settings',
  'preview': 'Preview & Create'
};

const STEP_DESCRIPTIONS: Record<WizardStep, string> = {
  'basic-info': 'Provide basic information about your multi-signature wallet',
  'signers': 'Add and configure the signers for your multi-signature wallet',
  'threshold': 'Define the signature threshold required for transactions',
  'advanced': 'Configure advanced security settings',
  'preview': 'Review your configuration before creating the wallet'
};

export function MultiSigWizard({ 
  onConfigCreate, 
  initialConfig = {},
  className = ''
}: MultiSigWizardProps) {
  const [currentStep, setCurrentStep] = useState<WizardStep>('basic-info');
  const [config, setConfig] = useState<MultiSigConfig>({
    name: '',
    description: '',
    signers: [],
    threshold: 1,
    timeLock: 0,
    network: 'testnet',
    ...initialConfig
  });

  const [validationErrors, setValidationErrors] = useState<ValidationResult>({
    isValid: false,
    errors: [],
    warnings: []
  });

  const steps: WizardStep[] = ['basic-info', 'signers', 'threshold', 'advanced', 'preview'];
  const currentStepIndex = steps.indexOf(currentStep);

  // Validation functions
  const validateStep = useCallback((): ValidationResult => {
    switch (currentStep) {
      case 'basic-info':
        const basicErrors: string[] = [];
        const basicWarnings: string[] = [];

        if (!config.name.trim()) {
          basicErrors.push('Wallet name is required');
        } else if (config.name.length < 3) {
          basicErrors.push('Wallet name must be at least 3 characters');
        } else if (config.name.length > 50) {
          basicErrors.push('Wallet name must be less than 50 characters');
        }

        if (config.description && config.description.length > 200) {
          basicWarnings.push('Description is quite long, consider keeping it concise');
        }

        return {
          isValid: basicErrors.length === 0,
          errors: basicErrors,
          warnings: basicWarnings
        };
      case 'signers':
      case 'threshold':
      case 'advanced':
        return validateMultiSigConfig(config);
      case 'preview':
        return validateMultiSigConfig(config);
      default:
        return { isValid: true, errors: [], warnings: [] };
    }
  }, [currentStep, config]);

  // Update validation errors when step or config changes
  useMemo(() => {
    setValidationErrors(validateStep());
  }, [validateStep]);

  const canProceed = validationErrors.isValid;
  const canGoBack = currentStepIndex > 0;

  const handleNext = () => {
    if (canProceed && currentStepIndex < steps.length - 1) {
      setCurrentStep(steps[currentStepIndex + 1]);
    }
  };

  const handleBack = () => {
    if (canGoBack) {
      setCurrentStep(steps[currentStepIndex - 1]);
    }
  };

  const handleStepClick = (step: WizardStep) => {
    const stepIndex = steps.indexOf(step);
    // Only allow navigation to completed steps or current step
    if (stepIndex <= currentStepIndex || validateStep().isValid) {
      setCurrentStep(step);
    }
  };

  // Signer management
  const addSigner = () => {
    const newSigner: SignerInfo = {
      id: generateSignerId(),
      name: '',
      publicKey: '',
      weight: 1,
      signatureScheme: 'ed25519'
    };
    setConfig(prev => ({
      ...prev,
      signers: [...prev.signers, newSigner]
    }));
  };

  const updateSigner = (id: string, updates: Partial<SignerInfo>) => {
    setConfig(prev => ({
      ...prev,
      signers: prev.signers.map(signer =>
        signer.id === id ? { ...signer, ...updates } : signer
      )
    }));
  };

  const removeSigner = (id: string) => {
    setConfig(prev => ({
      ...prev,
      signers: prev.signers.filter(signer => signer.id !== id)
    }));
  };

  const handleConfigCreate = () => {
    if (onConfigCreate && canProceed) {
      onConfigCreate(config);
    }
  };

  const renderStepContent = () => {
    switch (currentStep) {
      case 'basic-info':
        return <BasicInfoStep config={config} setConfig={setConfig} />;
      case 'signers':
        return (
          <SignersStep
            signers={config.signers}
            onAddSigner={addSigner}
            onUpdateSigner={updateSigner}
            onRemoveSigner={removeSigner}
          />
        );
      case 'threshold':
        return (
          <ThresholdStep
            threshold={config.threshold}
            signers={config.signers}
            onThresholdChange={(threshold) => setConfig(prev => ({ ...prev, threshold }))}
          />
        );
      case 'advanced':
        return (
          <AdvancedStep
            config={config}
            setConfig={setConfig}
          />
        );
      case 'preview':
        return <PreviewStep config={config} />;
      default:
        return null;
    }
  };

  return (
    <div className={`bg-white rounded-lg shadow-md p-6 space-y-6 ${className}`}>
      <div className="flex items-center space-x-4">
        <div className="w-12 h-12 bg-blue-100 rounded-full flex items-center justify-center">
          <span className="text-blue-600 font-bold">🔐</span>
        </div>
        <div>
          <h2 className="text-xl font-semibold text-gray-900">
            Multi-Signature Wallet Creator
          </h2>
          <p className="text-sm text-gray-600">
            Create a secure multi-signature wallet with customizable thresholds
          </p>
        </div>
      </div>

      {/* Progress Steps */}
      <div className="border-b">
        <nav className="flex space-x-8">
          {steps.map((step, index) => (
            <button
              key={step}
              onClick={() => handleStepClick(step)}
              className={`py-4 px-1 border-b-2 font-medium text-sm transition-optimized ${
                currentStep === step
                  ? 'border-blue-500 text-blue-600'
                  : index < currentStepIndex
                  ? 'border-green-500 text-green-600 hover:text-green-700'
                  : 'border-transparent text-gray-500 hover:text-gray-700'
              }`}
              disabled={index > currentStepIndex && !validateStep().isValid}
            >
              <div className="flex items-center space-x-2">
                <div className={`w-6 h-6 rounded-full flex items-center justify-center text-xs ${
                  currentStep === step
                    ? 'bg-blue-500 text-white'
                    : index < currentStepIndex
                    ? 'bg-green-500 text-white'
                    : 'bg-gray-300 text-gray-600'
                }`}>
                  {index < currentStepIndex ? '✓' : index + 1}
                </div>
                <span>{STEP_TITLES[step]}</span>
              </div>
            </button>
          ))}
        </nav>
      </div>

      {/* Step Description */}
      <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
        <h3 className="text-sm font-medium text-blue-900">
          {STEP_TITLES[currentStep]}
        </h3>
        <p className="text-sm text-blue-700 mt-1">
          {STEP_DESCRIPTIONS[currentStep]}
        </p>
      </div>

      {/* Step Content */}
      <div className="min-h-[400px]">
        {renderStepContent()}
      </div>

      {/* Validation Errors/Warnings */}
      {(validationErrors.errors.length > 0 || validationErrors.warnings.length > 0) && (
        <div className="space-y-2">
          {validationErrors.errors.map((error, index) => (
            <div key={index} className="bg-red-50 border border-red-200 rounded-md p-3">
              <p className="text-sm text-red-800">⚠️ {error}</p>
            </div>
          ))}
          {validationErrors.warnings.map((warning, index) => (
            <div key={index} className="bg-yellow-50 border border-yellow-200 rounded-md p-3">
              <p className="text-sm text-yellow-800">⚡ {warning}</p>
            </div>
          ))}
        </div>
      )}

      {/* Navigation */}
      <div className="flex justify-between pt-6 border-t">
        <button
          onClick={handleBack}
          disabled={!canGoBack}
          className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed transition-optimized"
        >
          ← Previous
        </button>

        <div className="flex space-x-2">
          {currentStep === 'preview' ? (
            <button
              onClick={handleCreate}
              disabled={!canProceed}
              className="px-6 py-2 text-sm font-medium text-white bg-green-600 border border-transparent rounded-md hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed transition-optimized"
            >
              Create Wallet
            </button>
          ) : (
            <button
              onClick={handleNext}
              disabled={!canProceed}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-600 border border-transparent rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-optimized"
            >
              Next →
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

// Step Components
function BasicInfoStep({ config, setConfig }: { config: MultiSigConfig; setConfig: (config: MultiSigConfig) => void }) {
  return (
    <div className="space-y-6">
      <div>
        <label htmlFor="wallet-name" className="block text-sm font-medium text-gray-700 mb-2">
          Wallet Name *
        </label>
        <input
          id="wallet-name"
          type="text"
          value={config.name}
          onChange={(e) => setConfig({ ...config, name: e.target.value })}
          className="w-full p-3 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500 transition-optimized"
          placeholder="My Multi-Sig Wallet"
        />
        <p className="text-xs text-gray-500 mt-1">
          A descriptive name for your multi-signature wallet
        </p>
      </div>

      <div>
        <label htmlFor="wallet-description" className="block text-sm font-medium text-gray-700 mb-2">
          Description
        </label>
        <textarea
          id="wallet-description"
          value={config.description}
          onChange={(e) => setConfig({ ...config, description: e.target.value })}
          rows={3}
          className="w-full p-3 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500 transition-optimized"
          placeholder="Optional description of the wallet's purpose"
        />
        <p className="text-xs text-gray-500 mt-1">
          Describe the purpose of this wallet (optional)
        </p>
      </div>

      <div>
        <label htmlFor="network" className="block text-sm font-medium text-gray-700 mb-2">
          Network
        </label>
        <select
          id="network"
          value={config.network}
          onChange={(e) => setConfig({ ...config, network: e.target.value as 'mainnet' | 'testnet' | 'futurenet' })}
          className="w-full p-3 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500 transition-optimized"
        >
          <option value="testnet">Testnet (Recommended for testing)</option>
          <option value="futurenet">Futurenet</option>
          <option value="mainnet">Mainnet (Production)</option>
        </select>
        <p className="text-xs text-gray-500 mt-1">
          Choose the network where this wallet will be created
        </p>
      </div>
    </div>
  );
}

function SignersStep({
  signers,
  onAddSigner,
  onUpdateSigner,
  onRemoveSigner
}: {
  signers: SignerInfo[];
  onAddSigner: () => void;
  onUpdateSigner: (id: string, updates: Partial<SignerInfo>) => void;
  onRemoveSigner: (id: string) => void;
}) {
  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h3 className="text-lg font-medium text-gray-900">Signers ({signers.length})</h3>
        <button
          onClick={onAddSigner}
          className="px-4 py-2 text-sm font-medium text-blue-600 bg-blue-50 border border-blue-200 rounded-md hover:bg-blue-100 transition-optimized"
        >
          + Add Signer
        </button>
      </div>

      {signers.length === 0 ? (
        <div className="text-center py-12 bg-gray-50 rounded-lg border-2 border-dashed border-gray-300">
          <p className="text-gray-500">No signers added yet</p>
          <p className="text-sm text-gray-400 mt-1">Add at least one signer to continue</p>
        </div>
      ) : (
        <div className="space-y-4">
          {signers.map((signer, index) => (
            <SignerCard
              key={signer.id}
              signer={signer}
              index={index}
              onUpdate={(updates) => onUpdateSigner(signer.id, updates)}
              onRemove={() => onRemoveSigner(signer.id)}
              canRemove={signers.length > 1}
            />
          ))}
        </div>
      )}

      <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
        <h4 className="text-sm font-medium text-blue-900 mb-2">💡 Tips</h4>
        <ul className="text-sm text-blue-700 space-y-1">
          <li>• Each signer has a weight that contributes to the threshold</li>
          <li>• Ed25519 is the standard for Stellar public keys</li>
          <li>• Consider using different signature schemes for added security</li>
        </ul>
      </div>
    </div>
  );
}

function SignerCard({
  signer,
  index,
  onUpdate,
  onRemove,
  canRemove
}: {
  signer: SignerInfo;
  index: number;
  onUpdate: (updates: Partial<SignerInfo>) => void;
  onRemove: () => void;
  canRemove: boolean;
}) {
  const [isExpanded, setIsExpanded] = useState(false);

  return (
    <div className="border border-gray-200 rounded-lg p-4 space-y-4">
      <div className="flex justify-between items-center">
        <h4 className="text-sm font-medium text-gray-900">Signer {index + 1}</h4>
        <div className="flex items-center space-x-2">
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            className="text-blue-600 hover:text-blue-800 text-sm font-medium transition-optimized"
          >
            {isExpanded ? 'Collapse' : 'Expand'}
          </button>
          {canRemove && (
            <button
              onClick={onRemove}
              className="text-red-600 hover:text-red-800 text-sm font-medium transition-optimized"
            >
              Remove
            </button>
          )}
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Name *
          </label>
          <input
            type="text"
            value={signer.name}
            onChange={(e) => onUpdate({ name: e.target.value })}
            className="w-full p-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500 transition-optimized"
            placeholder="Signer name"
          />
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Weight *
          </label>
          <input
            type="number"
            min="1"
            max="100"
            value={signer.weight}
            onChange={(e) => onUpdate({ weight: parseInt(e.target.value) || 1 })}
            className="w-full p-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500 transition-optimized"
          />
        </div>
      </div>

      {isExpanded && (
        <>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Signature Scheme
            </label>
            <select
              value={signer.signatureScheme}
              onChange={(e) => onUpdate({ signatureScheme: e.target.value as 'ed25519' | 'secp256k1' | 'p256' })}
              className="w-full p-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500 transition-optimized"
            >
              <option value="ed25519">Ed25519 (Stellar)</option>
              <option value="secp256k1">Secp256k1</option>
              <option value="p256">P256</option>
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Public Key *
            </label>
            <input
              type="text"
              value={signer.publicKey}
              onChange={(e) => onUpdate({ publicKey: e.target.value })}
              className="w-full p-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500 transition-optimized font-mono text-sm"
              placeholder={signer.signatureScheme === 'ed25519' ? 'G...' : 'Enter public key'}
            />
            <p className="text-xs text-gray-500 mt-1">
              {signer.signatureScheme === 'ed25519' 
                ? 'Stellar public keys start with "G" and are 56 characters'
                : 'Enter the public key in hex format'
              }
            </p>
          </div>
        </>
      )}
    </div>
  );
}

function ThresholdStep({
  threshold,
  signers,
  onThresholdChange
}: {
  threshold: number;
  signers: SignerInfo[];
  onThresholdChange: (threshold: number) => void;
}) {
  const totalWeight = calculateTotalWeight(signers);
  const maxThreshold = totalWeight;
  const recommendations = getThresholdRecommendations(signers);

  return (
    <div className="space-y-6">
      <div>
        <label htmlFor="threshold" className="block text-sm font-medium text-gray-700 mb-2">
          Signature Threshold
        </label>
        <div className="flex items-center space-x-4">
          <input
            id="threshold"
            type="range"
            min="1"
            max={maxThreshold}
            value={threshold}
            onChange={(e) => onThresholdChange(parseInt(e.target.value))}
            className="flex-1"
          />
          <div className="w-20 text-center">
            <span className="text-2xl font-bold text-blue-600">{threshold}</span>
            <span className="text-sm text-gray-500"> / {maxThreshold}</span>
          </div>
        </div>
        <p className="text-sm text-gray-600 mt-2">
          Minimum weight required to approve transactions
        </p>
      </div>

      <div className="bg-gray-50 rounded-lg p-4">
        <h4 className="text-sm font-medium text-gray-900 mb-3">Threshold Analysis</h4>
        <div className="space-y-2">
          <div className="flex justify-between text-sm">
            <span>Total Signer Weight:</span>
            <span className="font-medium">{totalWeight}</span>
          </div>
          <div className="flex justify-between text-sm">
            <span>Required Threshold:</span>
            <span className="font-medium text-blue-600">{threshold}</span>
          </div>
          <div className="flex justify-between text-sm">
            <span>Signers Needed:</span>
            <span className="font-medium">
              {calculateSignersNeeded(threshold, signers)} of {signers.length}
            </span>
          </div>
        </div>
      </div>

      <div className="space-y-3">
        <h4 className="text-sm font-medium text-gray-900">Quick Presets</h4>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-2">
          <button
            onClick={() => onThresholdChange(recommendations.conservative)}
            className="px-3 py-2 text-sm bg-gray-100 hover:bg-gray-200 rounded-md transition-optimized"
          >
            Conservative (80%)
          </button>
          <button
            onClick={() => onThresholdChange(recommendations.standard)}
            className="px-3 py-2 text-sm bg-gray-100 hover:bg-gray-200 rounded-md transition-optimized"
          >
            Standard (67%)
          </button>
          <button
            onClick={() => onThresholdChange(recommendations.flexible)}
            className="px-3 py-2 text-sm bg-gray-100 hover:bg-gray-200 rounded-md transition-optimized"
          >
            Flexible (51%)
          </button>
          <button
            onClick={() => onThresholdChange(totalWeight)}
            className="px-3 py-2 text-sm bg-gray-100 hover:bg-gray-200 rounded-md transition-optimized"
          >
            Unanimous (100%)
          </button>
        </div>
      </div>

      <ThresholdVisualization threshold={threshold} signers={signers} />
    </div>
  );
}

function AdvancedStep({
  config,
  setConfig
}: {
  config: MultiSigConfig;
  setConfig: (config: MultiSigConfig) => void;
}) {
  return (
    <div className="space-y-6">
      <div>
        <label htmlFor="timelock" className="block text-sm font-medium text-gray-700 mb-2">
          Time Lock (seconds)
        </label>
        <input
          id="timelock"
          type="number"
          min="0"
          value={config.timeLock}
          onChange={(e) => setConfig({ ...config, timeLock: parseInt(e.target.value) || 0 })}
          className="w-full p-3 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500 transition-optimized"
        />
        <p className="text-sm text-gray-600 mt-1">
          Delay before transactions can be executed (0 = no delay)
        </p>
        <div className="flex space-x-2 mt-2">
          <button
            onClick={() => setConfig({ ...config, timeLock: 0 })}
            className="px-3 py-1 text-xs bg-gray-100 hover:bg-gray-200 rounded transition-optimized"
          >
            No delay
          </button>
          <button
            onClick={() => setConfig({ ...config, timeLock: 3600 })}
            className="px-3 py-1 text-xs bg-gray-100 hover:bg-gray-200 rounded transition-optimized"
          >
            1 hour
          </button>
          <button
            onClick={() => setConfig({ ...config, timeLock: 86400 })}
            className="px-3 py-1 text-xs bg-gray-100 hover:bg-gray-200 rounded transition-optimized"
          >
            1 day
          </button>
          <button
            onClick={() => setConfig({ ...config, timeLock: 604800 })}
            className="px-3 py-1 text-xs bg-gray-100 hover:bg-gray-200 rounded transition-optimized"
          >
            1 week
          </button>
        </div>
      </div>

      <div className="bg-yellow-50 border border-yellow-200 rounded-lg p-4">
        <h4 className="text-sm font-medium text-yellow-900 mb-2">⚠️ Security Considerations</h4>
        <ul className="text-sm text-yellow-700 space-y-1">
          <li>• Time locks add security but delay transactions</li>
          <li>• Consider your use case when setting delays</li>
          <li>• Longer time locks protect against rapid attacks</li>
          <li>• Test thoroughly on testnet before mainnet deployment</li>
        </ul>
      </div>
    </div>
  );
}

function PreviewStep({ config }: { config: MultiSigConfig }) {
  const totalWeight = calculateTotalWeight(config.signers);
  const securityAnalysis = analyzeSecurity(config);

  return (
    <div className="space-y-6">
      <div className="bg-green-50 border border-green-200 rounded-lg p-4">
        <h3 className="text-lg font-medium text-green-900 mb-2">🎉 Ready to Create</h3>
        <p className="text-sm text-green-700">
          Review your multi-signature wallet configuration below
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div className="space-y-4">
          <div>
            <h4 className="text-sm font-medium text-gray-900 mb-2">Basic Information</h4>
            <dl className="space-y-2">
              <div className="flex justify-between text-sm">
                <dt className="text-gray-600">Name:</dt>
                <dd className="font-medium">{config.name}</dd>
              </div>
              <div className="flex justify-between text-sm">
                <dt className="text-gray-600">Network:</dt>
                <dd className="font-medium capitalize">{config.network}</dd>
              </div>
              {config.description && (
                <div className="text-sm">
                  <dt className="text-gray-600">Description:</dt>
                  <dd className="font-medium mt-1">{config.description}</dd>
                </div>
              )}
            </dl>
          </div>

          <div>
            <h4 className="text-sm font-medium text-gray-900 mb-2">Security Settings</h4>
            <dl className="space-y-2">
              <div className="flex justify-between text-sm">
                <dt className="text-gray-600">Threshold:</dt>
                <dd className="font-medium">{config.threshold} / {totalWeight}</dd>
              </div>
              <div className="flex justify-between text-sm">
                <dt className="text-gray-600">Time Lock:</dt>
                <dd className="font-medium">
                  {formatDuration(config.timeLock)}
                </dd>
              </div>
              <div className="flex justify-between text-sm">
                <dt className="text-gray-600">Signers:</dt>
                <dd className="font-medium">{config.signers.length}</dd>
              </div>
            </dl>
          </div>

          <div>
            <h4 className="text-sm font-medium text-gray-900 mb-2">Security Analysis</h4>
            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <dt className="text-gray-600">Security Score:</dt>
                <dd className={`font-medium ${
                  securityAnalysis.score >= 85 ? 'text-green-600' :
                  securityAnalysis.score >= 70 ? 'text-yellow-600' : 'text-red-600'
                }`}>
                  {securityAnalysis.score}/100
                </dd>
              </div>
            </div>
          </div>
        </div>

        <div>
          <h4 className="text-sm font-medium text-gray-900 mb-2">Signers</h4>
          <div className="space-y-2 max-h-64 overflow-y-auto">
            {config.signers.map((signer, index) => (
              <div key={signer.id} className="bg-gray-50 rounded p-3 text-sm">
                <div className="flex justify-between items-start">
                  <div>
                    <div className="font-medium">{signer.name || `Signer ${index + 1}`}</div>
                    <div className="text-xs text-gray-500 font-mono mt-1">
                      {truncatePublicKey(signer.publicKey)}
                    </div>
                    <div className="text-xs text-gray-500 mt-1">
                      {signer.signatureScheme.toUpperCase()}
                    </div>
                  </div>
                  <div className="text-right">
                    <div className="font-medium text-blue-600">Weight: {signer.weight}</div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {securityAnalysis.risks.length > 0 && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4">
          <h4 className="text-sm font-medium text-red-900 mb-2">⚠️ Security Risks</h4>
          <ul className="text-sm text-red-700 space-y-1">
            {securityAnalysis.risks.map((risk, index) => (
              <li key={index}>• {risk}</li>
            ))}
          </ul>
        </div>
      )}

      {securityAnalysis.recommendations.length > 0 && (
        <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
          <h4 className="text-sm font-medium text-blue-900 mb-2">💡 Recommendations</h4>
          <ul className="text-sm text-blue-700 space-y-1">
            {securityAnalysis.recommendations.map((rec, index) => (
              <li key={index}>• {rec}</li>
            ))}
          </ul>
        </div>
      )}

      <div className="border-t pt-4">
        <div className="flex items-center space-x-2 text-sm text-gray-600">
          <span>💡</span>
          <span>
            This configuration will be deployed to {config.network}. 
            {config.network === 'mainnet' && ' Please double-check all details before proceeding.'}
          </span>
        </div>
      </div>
    </div>
  );
}

// Helper components
function ThresholdVisualization({ threshold, signers }: { threshold: number; signers: SignerInfo[] }) {
  const totalWeight = calculateTotalWeight(signers);
  const thresholdPercentage = (threshold / totalWeight) * 100;

  return (
    <div className="bg-gray-50 rounded-lg p-4">
      <h4 className="text-sm font-medium text-gray-900 mb-3">Visual Threshold</h4>
      <div className="relative">
        <div className="w-full bg-gray-200 rounded-full h-8">
          <div
            className="bg-blue-500 h-8 rounded-full flex items-center justify-center text-white text-xs font-medium"
            style={{ width: `${thresholdPercentage}%` }}
          >
            {thresholdPercentage.toFixed(0)}%
          </div>
        </div>
        <div className="absolute -top-1 left-0 w-full flex justify-between text-xs text-gray-600 px-1">
          <span>0</span>
          <span>{totalWeight}</span>
        </div>
      </div>
      <p className="text-xs text-gray-600 mt-2 text-center">
        Threshold: {threshold} of {totalWeight} total weight
      </p>
    </div>
  );
}

export default MultiSigWizard;
