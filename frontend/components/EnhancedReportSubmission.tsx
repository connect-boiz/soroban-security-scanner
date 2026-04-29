'use client';

import { useState } from 'react';
import { Bounty, BountySubmission } from '@/types/bounty';
import { EncryptionService } from '@/utils/encryption';
import { ValidationRules, FormConfig } from '@/utils/validation';
import { Form, FormField, FormErrorSummary, FormProgress } from '@/components/form';
import { 
  FileText, 
  Shield, 
  Send, 
  AlertCircle, 
  CheckCircle, 
  Lock,
  Upload,
  Eye,
  EyeOff,
  Loader2
} from 'lucide-react';

interface ReportSubmissionProps {
  bounty: Bounty;
  onSubmit: (submission: BountySubmission) => void;
  onCancel: () => void;
}

interface FormData {
  findings: string;
  severity: 'Critical' | 'High' | 'Medium' | 'Low';
  ownerPublicKey: string;
  proofOfConcept: string;
  affectedFiles: string;
  reproductionSteps: string;
}

const formConfig: FormConfig = {
  findings: {
    rules: [
      ValidationRules.required('Security findings are required'),
      ValidationRules.minLength(100, 'Findings must be at least 100 characters to provide sufficient detail'),
      ValidationRules.maxLength(5000, 'Findings must be less than 5000 characters'),
      ValidationRules.custom(
        (value) => {
          if (!value || typeof value !== 'string') return true;
          const hasRequiredSections = 
            value.toLowerCase().includes('vulnerability') &&
            value.toLowerCase().includes('impact') &&
            value.toLowerCase().includes('mitigation');
          return hasRequiredSections || 'Please include vulnerability details, impact assessment, and mitigation recommendations';
        },
        'Comprehensive analysis required'
      )
    ],
    validateOnChange: true,
    validateOnBlur: true
  },
  severity: {
    rules: [
      ValidationRules.required('Severity level is required'),
      ValidationRules.custom(
        (value) => ['Critical', 'High', 'Medium', 'Low'].includes(value),
        'Please select a valid severity level'
      )
    ],
    validateOnChange: true
  },
  ownerPublicKey: {
    rules: [
      ValidationRules.required('Owner public key is required for encryption'),
      ValidationRules.stellarPublicKey('Please enter a valid Stellar public key (starts with G, 56 characters)'),
      ValidationRules.async(
        async (value) => {
          // Simulate async validation to check if key exists on network
          await new Promise(resolve => setTimeout(resolve, 500));
          return Math.random() > 0.1 || 'This public key appears to be invalid or not found on the network';
        },
        'Public key validation'
      )
    ],
    validateOnChange: true,
    validateOnBlur: true
  },
  proofOfConcept: {
    rules: [
      ValidationRules.maxLength(2000, 'Proof of concept must be less than 2000 characters'),
      ValidationRules.custom(
        (value) => {
          if (!value) return true; // Optional field
          const hasCodeBlocks = value.includes('```') || value.includes('code');
          return hasCodeBlocks || 'Please format your proof of concept with code blocks for clarity';
        },
        'Code formatting recommended'
      )
    ],
    validateOnChange: true
  },
  affectedFiles: {
    rules: [
      ValidationRules.maxLength(1000, 'Affected files list must be less than 1000 characters'),
      ValidationRules.custom(
        (value) => {
          if (!value) return true; // Optional field
          const filePattern = /^[\w\-/.\\,\s]+$/;
          return filePattern.test(value) || 'Please list valid file paths separated by commas';
        },
        'Valid file paths required'
      )
    ],
    validateOnChange: true
  },
  reproductionSteps: {
    rules: [
      ValidationRules.minLength(50, 'Reproduction steps must be at least 50 characters'),
      ValidationRules.maxLength(3000, 'Reproduction steps must be less than 3000 characters'),
      ValidationRules.custom(
        (value) => {
          if (!value) return true; // Optional field
          const hasSteps = /\d+\.|step\s*\d+/i.test(value);
          return hasSteps || 'Please provide numbered steps for reproduction';
        },
        'Numbered steps required'
      )
    ],
    validateOnChange: true
  }
};

export const EnhancedReportSubmission: React.FC<ReportSubmissionProps> = ({ 
  bounty, 
  onSubmit, 
  onCancel 
}) => {
  const [showPreview, setShowPreview] = useState(false);
  const [encryptedData, setEncryptedData] = useState<{ encrypted: string; salt: string } | null>(null);
  const [currentStep, setCurrentStep] = useState(0);

  const formSteps = [
    { name: 'severity', title: 'Severity' },
    { name: 'findings', title: 'Findings' },
    { name: 'details', title: 'Details' },
    { name: 'encryption', title: 'Encryption' }
  ];

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case 'Critical': return 'text-red-600 bg-red-50 border-red-200';
      case 'High': return 'text-orange-600 bg-orange-50 border-orange-200';
      case 'Medium': return 'text-yellow-600 bg-yellow-50 border-yellow-200';
      case 'Low': return 'text-green-600 bg-green-50 border-green-200';
      default: return 'text-gray-600 bg-gray-50 border-gray-200';
    }
  };

  const getRewardPercentage = (severity: string): number => {
    switch (severity) {
      case 'Critical': return 100;
      case 'High': return 100;
      case 'Medium': return 60;
      case 'Low': return 30;
      default: return 0;
    }
  };

  const handleEncrypt = async (formData: FormData) => {
    try {
      const encrypted = EncryptionService.encrypt(formData.findings, formData.ownerPublicKey);
      setEncryptedData(encrypted);
      setShowPreview(true);
      setCurrentStep(3);
    } catch (error) {
      throw new Error('Failed to encrypt findings');
    }
  };

  const handleSubmit = async (formData: FormData) => {
    if (!encryptedData) {
      throw new Error('Please encrypt your findings first');
    }

    const submission: BountySubmission = {
      id: `sub_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      bountyId: bounty.id,
      researcher: 'CURRENT_USER_ADDRESS',
      findings: formData.findings,
      encryptedFindings: encryptedData.encrypted,
      severity: formData.severity,
      submittedAt: new Date(),
      status: 'Pending'
    };

    await onSubmit(submission);
  };

  const initialData: FormData = {
    findings: '',
    severity: 'Medium',
    ownerPublicKey: '',
    proofOfConcept: '',
    affectedFiles: '',
    reproductionSteps: ''
  };

  return (
    <div className="max-w-4xl mx-auto p-6">
      <div className="card">
        {/* Header */}
        <div className="mb-6">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-2xl font-bold text-gray-900 flex items-center">
              <Shield className="h-6 w-6 mr-2 text-primary-600" />
              Submit Security Report
            </h2>
            <button
              onClick={onCancel}
              className="text-gray-500 hover:text-gray-700"
            >
              ×
            </button>
          </div>

          {/* Bounty Info */}
          <div className="bg-gray-50 rounded-lg p-4 mb-4">
            <h3 className="font-semibold text-gray-900 mb-2">{bounty.title}</h3>
            <div className="flex items-center space-x-4 text-sm text-gray-600">
              <span>Reward: <span className="font-semibold text-green-600">{bounty.rewardAmount} XLM</span></span>
              <span>â¢</span>
              <span>Difficulty: <span className={`px-2 py-1 rounded-full text-xs border ${getSeverityColor(bounty.difficulty)}`}>{bounty.difficulty}</span></span>
              {bounty.firstToFind && (
                <>
                  <span>â¢</span>
                  <span className="text-yellow-600 font-medium">ð First-to-Find Bonus</span>
                </>
              )}
            </div>
          </div>

          {/* Progress */}
          <FormProgress 
            steps={formSteps} 
            currentStep={currentStep}
            className="mb-6"
          />
        </div>

        <Form
          config={formConfig}
          onSubmit={handleSubmit}
          options={{
            validateOnChange: true,
            validateOnBlur: true,
            validateOnSubmit: true,
            initialData
          }}
        >
          {({
            formData,
            errors,
            touched,
            isValidating,
            isValid,
            isSubmitting,
            setFieldValue,
            setFieldTouched,
            validateField,
            resetForm,
            getFieldError,
            isFieldValid,
            isFieldInvalid,
            isFieldTouched,
            isFieldValidating
          }) => {
            const estimatedReward = Math.floor(bounty.rewardAmount * getRewardPercentage(formData.severity) / 100);

            return (
              <div className="space-y-6">
                {/* Error Summary */}
                <FormErrorSummary
                  errors={errors}
                  touched={touched}
                  showOnlyTouched={true}
                />

                {/* Step 1: Severity Selection */}
                <div className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-3">
                      Vulnerability Severity
                    </label>
                    <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
                      {(['Critical', 'High', 'Medium', 'Low'] as const).map((level) => (
                        <button
                          key={level}
                          type="button"
                          onClick={() => {
                            setFieldValue('severity', level);
                            setCurrentStep(1);
                          }}
                          className={`p-3 rounded-lg border-2 transition-all ${
                            formData.severity === level
                              ? getSeverityColor(level)
                              : 'border-gray-200 hover:border-gray-300'
                          }`}
                        >
                          <div className="font-medium">{level}</div>
                          <div className="text-xs mt-1">
                            {getRewardPercentage(level)}% reward
                          </div>
                        </button>
                      ))}
                    </div>
                    <div className="mt-2 text-sm text-gray-600">
                      Estimated reward: <span className="font-semibold text-green-600">{estimatedReward} XLM</span>
                    </div>
                  </div>
                </div>

                {/* Step 2: Findings */}
                <div>
                  <FormField
                    name="findings"
                    label="Security Findings"
                    type="textarea"
                    placeholder="Describe the vulnerability in detail including:
â¢ Vulnerability type and location
â¢ Step-by-step reproduction
â¢ Potential impact
â¢ Recommended mitigation"
                    rows={8}
                    required
                    error={getFieldError('findings')}
                    isValid={isFieldValid('findings')}
                    isInvalid={isFieldInvalid('findings')}
                    isValidating={isFieldValidating('findings')}
                    value={formData.findings}
                    onChange={(value) => setFieldValue('findings', value)}
                    onBlur={() => setFieldTouched('findings')}
                    helperText={`${formData.findings.length}/5000 characters (minimum: 100)`}
                  />
                </div>

                {/* Step 3: Additional Details */}
                <div className="space-y-4">
                  <FormField
                    name="proofOfConcept"
                    label="Proof of Concept (Optional)"
                    type="textarea"
                    placeholder="Provide code examples or proof of concept..."
                    rows={4}
                    error={getFieldError('proofOfConcept')}
                    isValid={isFieldValid('proofOfConcept')}
                    isInvalid={isFieldInvalid('proofOfConcept')}
                    value={formData.proofOfConcept}
                    onChange={(value) => setFieldValue('proofOfConcept', value)}
                    onBlur={() => setFieldTouched('proofOfConcept')}
                    helperText="Include code blocks for better readability"
                  />

                  <FormField
                    name="affectedFiles"
                    label="Affected Files (Optional)"
                    type="text"
                    placeholder="e.g., src/auth.js, lib/crypto.ts, config/security.json"
                    error={getFieldError('affectedFiles')}
                    isValid={isFieldValid('affectedFiles')}
                    isInvalid={isFieldInvalid('affectedFiles')}
                    value={formData.affectedFiles}
                    onChange={(value) => setFieldValue('affectedFiles', value)}
                    onBlur={() => setFieldTouched('affectedFiles')}
                    helperText="List file paths separated by commas"
                  />

                  <FormField
                    name="reproductionSteps"
                    label="Reproduction Steps (Optional)"
                    type="textarea"
                    placeholder="1. First step to reproduce...
2. Second step...
3. Expected vs actual behavior..."
                    rows={4}
                    error={getFieldError('reproductionSteps')}
                    isValid={isFieldValid('reproductionSteps')}
                    isInvalid={isFieldInvalid('reproductionSteps')}
                    value={formData.reproductionSteps}
                    onChange={(value) => setFieldValue('reproductionSteps', value)}
                    onBlur={() => setFieldTouched('reproductionSteps')}
                    helperText="Provide numbered steps for clear reproduction"
                  />
                </div>

                {/* Step 4: Encryption */}
                <div>
                  <FormField
                    name="ownerPublicKey"
                    label="Owner's Public Key (for encryption)"
                    type="text"
                    placeholder="Enter the bounty owner's public key"
                    required
                    error={getFieldError('ownerPublicKey')}
                    isValid={isFieldValid('ownerPublicKey')}
                    isInvalid={isFieldInvalid('ownerPublicKey')}
                    isValidating={isFieldValidating('ownerPublicKey')}
                    value={formData.ownerPublicKey}
                    onChange={(value) => {
                      setFieldValue('ownerPublicKey', value);
                      setCurrentStep(3);
                    }}
                    onBlur={() => setFieldTouched('ownerPublicKey')}
                    helperText="Your findings will be encrypted with the owner's public key to ensure confidentiality"
                  />
                </div>

                {/* Encryption Preview */}
                {showPreview && encryptedData && (
                  <div className="bg-green-50 border border-green-200 rounded-lg p-4">
                    <div className="flex items-center mb-2">
                      <CheckCircle className="h-5 w-5 text-green-600 mr-2" />
                      <span className="font-medium text-green-800">Findings Encrypted Successfully</span>
                    </div>
                    <div className="text-sm text-green-700">
                      <p>Your findings have been encrypted and are ready for submission.</p>
                      <button
                        type="button"
                        onClick={() => setShowPreview(!showPreview)}
                        className="mt-2 text-green-600 hover:text-green-800 flex items-center"
                      >
                        {showPreview ? <EyeOff className="h-4 w-4 mr-1" /> : <Eye className="h-4 w-4 mr-1" />}
                        {showPreview ? 'Hide' : 'Show'} encrypted data
                      </button>
                      {showPreview && (
                        <div className="mt-2 p-2 bg-green-100 rounded text-xs font-mono break-all">
                          {encryptedData.encrypted.substring(0, 100)}...
                        </div>
                      )}
                    </div>
                  </div>
                )}

                {/* Action Buttons */}
                <div className="flex justify-between pt-6 border-t border-gray-200">
                  <button
                    onClick={onCancel}
                    className="btn-secondary"
                    disabled={isSubmitting}
                  >
                    Cancel
                  </button>
                  
                  <div className="space-x-3">
                    {!encryptedData ? (
                      <button
                        type="button"
                        onClick={() => handleEncrypt(formData)}
                        disabled={!isValid || isValidating.ownerPublicKey}
                        className="btn-primary flex items-center"
                      >
                        {isValidating.ownerPublicKey ? (
                          <>
                            <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                            Validating...
                          </>
                        ) : (
                          <>
                            <Lock className="h-4 w-4 mr-2" />
                            Encrypt & Preview
                          </>
                        )}
                      </button>
                    ) : (
                      <button
                        type="submit"
                        disabled={isSubmitting}
                        className="btn-primary flex items-center"
                      >
                        {isSubmitting ? (
                          <>
                            <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                            Submitting...
                          </>
                        ) : (
                          <>
                            <Send className="h-4 w-4 mr-2" />
                            Submit Report
                          </>
                        )}
                      </button>
                    )}
                  </div>
                </div>
              </div>
            );
          }}
        </Form>
      </div>
    </div>
  );
};
