'use client';

import { useState } from 'react';
import { Bounty, BountySubmission } from '@/types/bounty';
import { EncryptionService } from '@/utils/encryption';
import { 
  FileText, 
  Shield, 
  Send, 
  AlertCircle, 
  CheckCircle, 
  Lock,
  Upload,
  Eye,
  EyeOff
} from 'lucide-react';

interface ReportSubmissionProps {
  bounty: Bounty;
  onSubmit: (submission: BountySubmission) => void;
  onCancel: () => void;
}

export const ReportSubmission: React.FC<ReportSubmissionProps> = ({ 
  bounty, 
  onSubmit, 
  onCancel 
}) => {
  const [findings, setFindings] = useState('');
  const [severity, setSeverity] = useState<'Critical' | 'High' | 'Medium' | 'Low'>('Medium');
  const [ownerPublicKey, setOwnerPublicKey] = useState('');
  const [isEncrypting, setIsEncrypting] = useState(false);
  const [showPreview, setShowPreview] = useState(false);
  const [encryptedData, setEncryptedData] = useState<{ encrypted: string; salt: string } | null>(null);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [isSubmitting, setIsSubmitting] = useState(false);

  const validateForm = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (!findings.trim()) {
      newErrors.findings = 'Findings are required';
    }

    if (findings.length < 100) {
      newErrors.findings = 'Findings must be at least 100 characters';
    }

    if (!ownerPublicKey.trim()) {
      newErrors.ownerPublicKey = 'Owner public key is required for encryption';
    }

    if (ownerPublicKey.length < 32) {
      newErrors.ownerPublicKey = 'Invalid public key format';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleEncrypt = () => {
    if (!validateForm()) return;

    setIsEncrypting(true);
    try {
      const encrypted = EncryptionService.encrypt(findings, ownerPublicKey);
      setEncryptedData(encrypted);
      setShowPreview(true);
    } catch (error) {
      setErrors({ ...errors, encryption: 'Failed to encrypt findings' });
    } finally {
      setIsEncrypting(false);
    }
  };

  const handleSubmit = async () => {
    if (!encryptedData) {
      setErrors({ ...errors, encryption: 'Please encrypt your findings first' });
      return;
    }

    setIsSubmitting(true);

    const submission: BountySubmission = {
      id: `sub_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      bountyId: bounty.id,
      researcher: 'CURRENT_USER_ADDRESS', // This would come from wallet connection
      findings: findings, // Store original for user reference
      encryptedFindings: encryptedData.encrypted,
      severity,
      submittedAt: new Date(),
      status: 'Pending',
      encryptionSalt: encryptedData.salt
    };

    try {
      await onSubmit(submission);
    } catch (error) {
      setErrors({ ...errors, submission: 'Failed to submit report' });
    } finally {
      setIsSubmitting(false);
    }
  };

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

  const estimatedReward = Math.floor(bounty.rewardAmount * getRewardPercentage(severity) / 100);

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
              <span>•</span>
              <span>Difficulty: <span className={`px-2 py-1 rounded-full text-xs border ${getSeverityColor(bounty.difficulty)}`}>{bounty.difficulty}</span></span>
              {bounty.firstToFind && (
                <>
                  <span>•</span>
                  <span className="text-yellow-600 font-medium">🏅 First-to-Find Bonus</span>
                </>
              )}
            </div>
          </div>
        </div>

        {/* Form */}
        <div className="space-y-6">
          {/* Severity Selection */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-3">
              Vulnerability Severity
            </label>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
              {(['Critical', 'High', 'Medium', 'Low'] as const).map((level) => (
                <button
                  key={level}
                  type="button"
                  onClick={() => setSeverity(level)}
                  className={`p-3 rounded-lg border-2 transition-all ${
                    severity === level
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

          {/* Findings */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              <FileText className="inline h-4 w-4 mr-1" />
              Security Findings
            </label>
            <textarea
              value={findings}
              onChange={(e) => setFindings(e.target.value)}
              placeholder="Describe the vulnerability in detail including:
• Vulnerability type and location
• Step-by-step reproduction
• Potential impact
• Recommended mitigation"
              rows={8}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
            <div className="mt-1 text-sm text-gray-500">
              {findings.length}/1000 characters (minimum: 100)
            </div>
            {errors.findings && (
              <p className="mt-1 text-sm text-red-600">{errors.findings}</p>
            )}
          </div>

          {/* Encryption */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              <Lock className="inline h-4 w-4 mr-1" />
              Owner's Public Key (for encryption)
            </label>
            <input
              type="text"
              value={ownerPublicKey}
              onChange={(e) => setOwnerPublicKey(e.target.value)}
              placeholder="Enter the bounty owner's public key"
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
            {errors.ownerPublicKey && (
              <p className="mt-1 text-sm text-red-600">{errors.ownerPublicKey}</p>
            )}
            <p className="mt-2 text-sm text-gray-600">
              Your findings will be encrypted with the owner's public key to ensure confidentiality.
            </p>
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

          {/* Error Display */}
          {errors.encryption && (
            <div className="bg-red-50 border border-red-200 rounded-lg p-4">
              <div className="flex items-center">
                <AlertCircle className="h-5 w-5 text-red-600 mr-2" />
                <span className="text-red-800">{errors.encryption}</span>
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
                  onClick={handleEncrypt}
                  disabled={isEncrypting}
                  className="btn-primary flex items-center"
                >
                  {isEncrypting ? (
                    <>
                      <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white mr-2"></div>
                      Encrypting...
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
                  onClick={handleSubmit}
                  disabled={isSubmitting}
                  className="btn-primary flex items-center"
                >
                  {isSubmitting ? (
                    <>
                      <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white mr-2"></div>
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
      </div>
    </div>
  );
};
