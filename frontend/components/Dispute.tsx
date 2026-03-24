'use client';

import { useState } from 'react';
import { BountySubmission } from '@/types/bounty';
import { StellarWalletService } from '@/services/stellarWallet';
import { 
  AlertTriangle, 
  Scale, 
  MessageSquare, 
  Send, 
  CheckCircle, 
  XCircle,
  Clock,
  Users,
  Gavel,
  FileText
} from 'lucide-react';

interface DisputeProps {
  submission: BountySubmission;
  onSubmitDispute: (disputeData: DisputeData) => void;
  onCancel: () => void;
}

export interface DisputeData {
  id: string;
  submissionId: string;
  disputeReason: string;
  detailedExplanation: string;
  evidence: string[];
  requestedAction: 'reconsider' | 'escalate' | 'full_review';
  submittedBy: string;
  submittedAt: Date;
  status: 'pending' | 'under_review' | 'resolved' | 'rejected';
  reviewers?: string[];
  resolution?: string;
}

export const DisputeForm: React.FC<DisputeProps> = ({ 
  submission, 
  onSubmitDispute, 
  onCancel 
}) => {
  const [disputeReason, setDisputeReason] = useState('');
  const [detailedExplanation, setDetailedExplanation] = useState('');
  const [evidence, setEvidence] = useState<string[]>(['']);
  const [requestedAction, setRequestedAction] = useState<'reconsider' | 'escalate' | 'full_review'>('reconsider');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});

  const validateForm = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (!disputeReason.trim()) {
      newErrors.disputeReason = 'Dispute reason is required';
    }

    if (!detailedExplanation.trim()) {
      newErrors.detailedExplanation = 'Detailed explanation is required';
    }

    if (detailedExplanation.length < 50) {
      newErrors.detailedExplanation = 'Explanation must be at least 50 characters';
    }

    if (evidence.filter(e => e.trim()).length === 0) {
      newErrors.evidence = 'At least one piece of evidence is required';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const addEvidenceField = () => {
    setEvidence([...evidence, '']);
  };

  const updateEvidence = (index: number, value: string) => {
    const newEvidence = [...evidence];
    newEvidence[index] = value;
    setEvidence(newEvidence);
  };

  const removeEvidenceField = (index: number) => {
    if (evidence.length > 1) {
      const newEvidence = evidence.filter((_, i) => i !== index);
      setEvidence(newEvidence);
    }
  };

  const handleSubmit = async () => {
    if (!validateForm()) return;

    setIsSubmitting(true);

    const disputeData: DisputeData = {
      id: `dispute_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      submissionId: submission.id,
      disputeReason,
      detailedExplanation,
      evidence: evidence.filter(e => e.trim()),
      requestedAction,
      submittedBy: 'CURRENT_USER_ADDRESS', // This would come from wallet
      submittedAt: new Date(),
      status: 'pending'
    };

    try {
      await onSubmitDispute(disputeData);
    } catch (error) {
      setErrors({ submit: 'Failed to submit dispute' });
    } finally {
      setIsSubmitting(false);
    }
  };

  const getActionDescription = (action: string) => {
    switch (action) {
      case 'reconsider':
        return 'Request reconsideration of the current decision';
      case 'escalate':
        return 'Escalate to senior reviewers';
      case 'full_review':
        return 'Request a complete re-evaluation';
      default:
        return '';
    }
  };

  return (
    <div className="max-w-4xl mx-auto p-6">
      <div className="card">
        {/* Header */}
        <div className="mb-6">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-2xl font-bold text-gray-900 flex items-center">
              <Gavel className="h-6 w-6 mr-2 text-orange-600" />
              Submit Dispute
            </h2>
            <button
              onClick={onCancel}
              className="text-gray-500 hover:text-gray-700"
            >
              ×
            </button>
          </div>

          {/* Submission Info */}
          <div className="bg-orange-50 border border-orange-200 rounded-lg p-4 mb-4">
            <h3 className="font-semibold text-gray-900 mb-2">Submission Being Disputed</h3>
            <div className="grid grid-cols-2 gap-4 text-sm">
              <div>
                <span className="text-gray-600">Submission ID:</span>
                <span className="ml-2 font-mono">{submission.id}</span>
              </div>
              <div>
                <span className="text-gray-600">Status:</span>
                <span className={`ml-2 px-2 py-1 rounded-full text-xs ${
                  submission.status === 'Rejected' ? 'bg-red-100 text-red-800' : 'bg-yellow-100 text-yellow-800'
                }`}>
                  {submission.status}
                </span>
              </div>
              <div>
                <span className="text-gray-600">Severity:</span>
                <span className="ml-2 font-medium">{submission.severity}</span>
              </div>
              <div>
                <span className="text-gray-600">Submitted:</span>
                <span className="ml-2">{submission.submittedAt.toLocaleDateString()}</span>
              </div>
            </div>
          </div>
        </div>

        {/* Form */}
        <div className="space-y-6">
          {/* Dispute Reason */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              <AlertTriangle className="inline h-4 w-4 mr-1" />
              Reason for Dispute
            </label>
            <select
              value={disputeReason}
              onChange={(e) => setDisputeReason(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-orange-500 focus:border-transparent"
            >
              <option value="">Select a reason</option>
              <option value="unfair_rejection">Unfair rejection</option>
              <option value="insufficient_review">Insufficient review</option>
              <option value="misunderstanding">Misunderstanding of findings</option>
              <option value="severity_mismatch">Severity level mismatch</option>
              <option value="duplicate_finding">Incorrect duplicate marking</option>
              <option value="technical_error">Technical error in evaluation</option>
              <option value="other">Other</option>
            </select>
            {errors.disputeReason && (
              <p className="mt-1 text-sm text-red-600">{errors.disputeReason}</p>
            )}
          </div>

          {/* Requested Action */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              <Scale className="inline h-4 w-4 mr-1" />
              Requested Action
            </label>
            <div className="space-y-2">
              {(['reconsider', 'escalate', 'full_review'] as const).map((action) => (
                <label key={action} className="flex items-start">
                  <input
                    type="radio"
                    value={action}
                    checked={requestedAction === action}
                    onChange={(e) => setRequestedAction(e.target.value as any)}
                    className="mt-1 mr-3"
                  />
                  <div>
                    <span className="font-medium capitalize">{action.replace('_', ' ')}</span>
                    <p className="text-sm text-gray-600">{getActionDescription(action)}</p>
                  </div>
                </label>
              ))}
            </div>
          </div>

          {/* Detailed Explanation */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              <MessageSquare className="inline h-4 w-4 mr-1" />
              Detailed Explanation
            </label>
            <textarea
              value={detailedExplanation}
              onChange={(e) => setDetailedExplanation(e.target.value)}
              placeholder="Please provide a detailed explanation of why you believe this decision should be reviewed. Include specific details about why the rejection was unfair or incorrect."
              rows={6}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-orange-500 focus:border-transparent"
            />
            <div className="mt-1 text-sm text-gray-500">
              {detailedExplanation.length}/1000 characters (minimum: 50)
            </div>
            {errors.detailedExplanation && (
              <p className="mt-1 text-sm text-red-600">{errors.detailedExplanation}</p>
            )}
          </div>

          {/* Evidence */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              <FileText className="inline h-4 w-4 mr-1" />
              Supporting Evidence
            </label>
            <p className="text-sm text-gray-600 mb-3">
              Provide links, screenshots, or other evidence to support your dispute claim.
            </p>
            
            {evidence.map((evidenceItem, index) => (
              <div key={index} className="flex items-center space-x-2 mb-2">
                <input
                  type="text"
                  value={evidenceItem}
                  onChange={(e) => updateEvidence(index, e.target.value)}
                  placeholder="Enter evidence URL or description"
                  className="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-orange-500 focus:border-transparent"
                />
                {evidence.length > 1 && (
                  <button
                    type="button"
                    onClick={() => removeEvidenceField(index)}
                    className="p-2 text-red-600 hover:text-red-800"
                  >
                    <XCircle className="h-5 w-5" />
                  </button>
                )}
              </div>
            ))}
            
            <button
              type="button"
              onClick={addEvidenceField}
              className="text-orange-600 hover:text-orange-800 text-sm font-medium"
            >
              + Add Evidence
            </button>
            
            {errors.evidence && (
              <p className="mt-1 text-sm text-red-600">{errors.evidence}</p>
            )}
          </div>

          {/* Error Display */}
          {errors.submit && (
            <div className="bg-red-50 border border-red-200 rounded-lg p-4">
              <div className="flex items-center">
                <AlertTriangle className="h-5 w-5 text-red-600 mr-2" />
                <span className="text-red-800">{errors.submit}</span>
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
                  Submit Dispute
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

interface DisputeStatusProps {
  dispute: DisputeData;
}

export const DisputeStatus: React.FC<DisputeStatusProps> = ({ dispute }) => {
  const getStatusIcon = () => {
    switch (dispute.status) {
      case 'pending':
        return <Clock className="h-5 w-5 text-yellow-600" />;
      case 'under_review':
        return <Users className="h-5 w-5 text-blue-600" />;
      case 'resolved':
        return <CheckCircle className="h-5 w-5 text-green-600" />;
      case 'rejected':
        return <XCircle className="h-5 w-5 text-red-600" />;
      default:
        return <Clock className="h-5 w-5 text-gray-600" />;
    }
  };

  const getStatusColor = () => {
    switch (dispute.status) {
      case 'pending':
        return 'bg-yellow-100 text-yellow-800 border-yellow-200';
      case 'under_review':
        return 'bg-blue-100 text-blue-800 border-blue-200';
      case 'resolved':
        return 'bg-green-100 text-green-800 border-green-200';
      case 'rejected':
        return 'bg-red-100 text-red-800 border-red-200';
      default:
        return 'bg-gray-100 text-gray-800 border-gray-200';
    }
  };

  return (
    <div className="card">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center">
          {getStatusIcon()}
          <h3 className="ml-2 text-lg font-semibold text-gray-900">Dispute Status</h3>
        </div>
        <span className={`px-3 py-1 rounded-full text-sm font-medium border ${getStatusColor()}`}>
          {dispute.status.replace('_', ' ').toUpperCase()}
        </span>
      </div>

      <div className="space-y-4">
        <div>
          <span className="text-sm font-medium text-gray-700">Dispute ID:</span>
          <span className="ml-2 font-mono text-sm">{dispute.id}</span>
        </div>

        <div>
          <span className="text-sm font-medium text-gray-700">Reason:</span>
          <span className="ml-2 text-sm">{dispute.disputeReason.replace('_', ' ')}</span>
        </div>

        <div>
          <span className="text-sm font-medium text-gray-700">Requested Action:</span>
          <span className="ml-2 text-sm capitalize">{dispute.requestedAction.replace('_', ' ')}</span>
        </div>

        <div>
          <span className="text-sm font-medium text-gray-700">Submitted:</span>
          <span className="ml-2 text-sm">{dispute.submittedAt.toLocaleDateString()}</span>
        </div>

        {dispute.reviewers && dispute.reviewers.length > 0 && (
          <div>
            <span className="text-sm font-medium text-gray-700">Reviewers:</span>
            <div className="mt-1 space-y-1">
              {dispute.reviewers.map((reviewer, index) => (
                <div key={index} className="text-sm font-mono text-gray-600">
                  {reviewer.slice(0, 8)}...{reviewer.slice(-8)}
                </div>
              ))}
            </div>
          </div>
        )}

        {dispute.resolution && (
          <div className="bg-gray-50 rounded-lg p-3">
            <span className="text-sm font-medium text-gray-700">Resolution:</span>
            <p className="mt-1 text-sm text-gray-600">{dispute.resolution}</p>
          </div>
        )}
      </div>
    </div>
  );
};
