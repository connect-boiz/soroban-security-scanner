'use client';

import { useState, useCallback, useMemo } from 'react';
import LazyImage from './LazyImage';

// Types for multi-signature configuration
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
  timeLock: number; // in seconds
  network: 'mainnet' | 'testnet' | 'futurenet';
}

export interface ValidationResult {
  isValid: boolean;
  errors: string[];
  warnings: string[];
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

export default function MultiSigWizard() {
  const [currentStep, setCurrentStep] = useState<WizardStep>('basic-info');
  const [config, setConfig] = useState<MultiSigConfig>({
    name: '',
    description: '',
    signers: [],
    threshold: 1,
    timeLock: 0,
    network: 'testnet'
  });

  const [validationErrors, setValidationErrors] = useState<ValidationResult>({
    isValid: false,
    errors: [],
    warnings: []
  });

  const steps: WizardStep[] = ['basic-info', 'signers', 'threshold', 'advanced', 'preview'];
  const currentStepIndex = steps.indexOf(currentStep);

  // Validation functions
  const validateBasicInfo = useCallback((): ValidationResult => {
    const errors: string[] = [];
    const warnings: string[] = [];

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

    return {
      isValid: errors.length === 0,
      errors,
      warnings
    };
  }, [config.name, config.description]);

  const validateSigners = useCallback((): ValidationResult => {
    const errors: string[] = [];
    const warnings: string[] = [];

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

    return {
      isValid: errors.length === 0,
      errors,
      warnings
    };
  }, [config.signers]);

  const validateThreshold = useCallback((): ValidationResult => {
    const errors: string[] = [];
    const warnings: string[] = [];

    const totalWeight = config.signers.reduce((sum, signer) => sum + signer.weight, 0);

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

    return {
      isValid: errors.length === 0,
      errors,
      warnings
    };
  }, [config.threshold, config.signers]);

  const validateAdvanced = useCallback((): ValidationResult => {
    const errors: string[] = [];
    const warnings: string[] = [];

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
  }, [config.timeLock]);

  const validateStep = useCallback((): ValidationResult => {
    switch (currentStep) {
      case 'basic-info':
        return validateBasicInfo();
      case 'signers':
        return validateSigners();
      case 'threshold':
        return validateThreshold();
      case 'advanced':
        return validateAdvanced();
      case 'preview':
        // Validate all steps
        const basic = validateBasicInfo();
        const signers = validateSigners();
        const threshold = validateThreshold();
        const advanced = validateAdvanced();
        
        return {
          isValid: basic.isValid && signers.isValid && threshold.isValid && advanced.isValid,
          errors: [...basic.errors, ...signers.errors, ...threshold.errors, ...advanced.errors],
          warnings: [...basic.warnings, ...signers.warnings, ...threshold.warnings, ...advanced.warnings]
        };
      default:
        return { isValid: true, errors: [], warnings: [] };
    }
  }, [currentStep, validateBasicInfo, validateSigners, validateThreshold, validateAdvanced]);

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
      id: Date.now().toString(),
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
    <div className="bg-white rounded-lg shadow-md p-6 space-y-6">
      <div className="flex items-center space-x-4">
        <LazyImage
          src="/multisig-icon.svg"
          alt="Multi-Signature Icon"
          className="w-12 h-12"
          width={48}
          height={48}
        />
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
              onClick={() => {
                // Handle wallet creation
                console.log('Creating multi-sig wallet:', config);
              }}
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

// Helper function to validate public keys
function isValidPublicKey(publicKey: string, scheme: string): boolean {
  if (!publicKey) return false;
  
  switch (scheme) {
    case 'ed25519':
      // Stellar Ed25519 public keys start with 'G' and are 56 characters
      return /^G[A-Z0-9]{55}$/.test(publicKey);
    case 'secp256k1':
      // Secp256k1 keys (example format validation)
      return /^[0-9a-fA-F]{66}$/.test(publicKey);
    case 'p256':
      // P256 keys (example format validation)
      return /^[0-9a-fA-F]{66}$/.test(publicKey);
    default:
      return false;
  }
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
  return (
    <div className="border border-gray-200 rounded-lg p-4 space-y-4">
      <div className="flex justify-between items-center">
        <h4 className="text-sm font-medium text-gray-900">Signer {index + 1}</h4>
        {canRemove && (
          <button
            onClick={onRemove}
            className="text-red-600 hover:text-red-800 text-sm font-medium transition-optimized"
          >
            Remove
          </button>
        )}
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
  const totalWeight = signers.reduce((sum, signer) => sum + signer.weight, 0);
  const maxThreshold = totalWeight;

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
              {calculateSignersNeeded(threshold, signers)}
            </span>
          </div>
        </div>
      </div>

      <div className="space-y-3">
        <h4 className="text-sm font-medium text-gray-900">Quick Presets</h4>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-2">
          <button
            onClick={() => onThresholdChange(Math.ceil(totalWeight / 2))}
            className="px-3 py-2 text-sm bg-gray-100 hover:bg-gray-200 rounded-md transition-optimized"
          >
            50% (Majority)
          </button>
          <button
            onClick={() => onThresholdChange(Math.ceil(totalWeight * 0.67))}
            className="px-3 py-2 text-sm bg-gray-100 hover:bg-gray-200 rounded-md transition-optimized"
          >
            67% (Supermajority)
          </button>
          <button
            onClick={() => onThresholdChange(totalWeight)}
            className="px-3 py-2 text-sm bg-gray-100 hover:bg-gray-200 rounded-md transition-optimized"
          >
            100% (Unanimous)
          </button>
          <button
            onClick={() => onThresholdChange(1)}
            className="px-3 py-2 text-sm bg-gray-100 hover:bg-gray-200 rounded-md transition-optimized"
          >
            1 (Any signer)
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
  const totalWeight = config.signers.reduce((sum, signer) => sum + signer.weight, 0);

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
                  {config.timeLock === 0 ? 'None' : formatDuration(config.timeLock)}
                </dd>
              </div>
              <div className="flex justify-between text-sm">
                <dt className="text-gray-600">Signers:</dt>
                <dd className="font-medium">{config.signers.length}</dd>
              </div>
            </dl>
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
                      {signer.publicKey.substring(0, 20)}...
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

// Helper components and functions
function calculateSignersNeeded(threshold: number, signers: SignerInfo[]): string {
  const sortedSigners = [...signers].sort((a, b) => b.weight - a.weight);
  let accumulated = 0;
  let count = 0;

  for (const signer of sortedSigners) {
    accumulated += signer.weight;
    count++;
    if (accumulated >= threshold) break;
  }

  return `${count} of ${signers.length}`;
}

function ThresholdVisualization({ threshold, signers }: { threshold: number; signers: SignerInfo[] }) {
  const totalWeight = signers.reduce((sum, signer) => sum + signer.weight, 0);
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

function formatDuration(seconds: number): string {
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
