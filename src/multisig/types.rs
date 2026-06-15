//! Multi-Signature Business Logic Types
//!
//! Core data structures for threshold management, signature aggregation,
//! and approval workflows.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Status of a multi-sig proposal
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    /// Awaiting signatures
    Pending,
    /// Threshold reached, ready to execute
    Approved,
    /// Explicitly rejected (threshold of rejections reached)
    Rejected,
    /// Successfully submitted to Stellar
    Executed,
    /// Passed the expiry deadline without reaching threshold
    Expired,
}

/// Individual signer status on a proposal
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignerDecision {
    Pending,
    Approved,
    Rejected,
}

/// A signer registered on a multi-sig account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSigSigner {
    pub id: Uuid,
    pub proposal_id: Uuid,
    /// Stellar public key of the signer
    pub signer_address: String,
    /// Optional link to a wallet record
    pub signer_wallet_id: Option<Uuid>,
    /// Optional link to a user record
    pub signer_user_id: Option<Uuid>,
    /// Signing weight (Stellar account weight)
    pub weight: u32,
    pub decision: SignerDecision,
    /// Base64-encoded Ed25519 signature over the transaction hash
    pub signature_data: Option<String>,
    pub signed_at: Option<DateTime<Utc>>,
    pub comments: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// If Some, this signer was revoked and can no longer participate
    #[serde(default)]
    pub revoked_at: Option<DateTime<Utc>>,
}

impl MultiSigSigner {
    /// Whether this signer is currently active (not revoked)
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none()
    }
}

/// A multi-signature proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSigProposal {
    pub id: Uuid,
    pub user_id: Uuid,
    pub operation_name: String,
    pub description: Option<String>,
    /// The Stellar account that requires multiple signatures
    pub stellar_address: String,
    /// Minimum cumulative weight required to approve
    pub threshold: u32,
    /// Total cumulative weight of all registered signers
    pub total_weight: u32,
    pub status: ProposalStatus,
    /// Serialized Stellar transaction envelope (XDR base64)
    pub transaction_envelope: String,
    pub signers: Vec<MultiSigSigner>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub executed_at: Option<DateTime<Utc>>,
    /// Stellar transaction hash after execution
    pub executed_transaction_hash: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl MultiSigProposal {
    /// Sum of weights from signers who approved
    pub fn approved_weight(&self) -> u32 {
        self.signers
            .iter()
            .filter(|s| s.decision == SignerDecision::Approved)
            .map(|s| s.weight)
            .sum()
    }

    /// Sum of weights from signers who rejected
    pub fn rejected_weight(&self) -> u32 {
        self.signers
            .iter()
            .filter(|s| s.decision == SignerDecision::Rejected)
            .map(|s| s.weight)
            .sum()
    }

    /// Whether the approval threshold has been met
    pub fn is_threshold_met(&self) -> bool {
        self.approved_weight() >= self.threshold
    }

    /// Whether the proposal has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| Utc::now() > exp)
            .unwrap_or(false)
    }
}

/// Request to create a new multi-sig proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProposalRequest {
    pub user_id: Uuid,
    pub operation_name: String,
    pub description: Option<String>,
    pub stellar_address: String,
    /// Minimum cumulative weight required to approve
    pub threshold: u32,
    /// Serialized Stellar transaction envelope (XDR base64)
    pub transaction_envelope: String,
    /// Initial list of signers with their weights
    pub signers: Vec<SignerSpec>,
    /// Optional expiry (defaults to 7 days)
    pub expires_in_hours: Option<u64>,
}

/// Signer specification used when creating a proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignerSpec {
    pub signer_address: String,
    pub signer_wallet_id: Option<Uuid>,
    pub signer_user_id: Option<Uuid>,
    pub weight: u32,
}

/// Request to submit a signature (approve or reject)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitSignatureRequest {
    pub proposal_id: Uuid,
    pub signer_address: String,
    pub decision: SignerDecision,
    /// Base64-encoded Ed25519 signature over the transaction hash (required for Approved)
    pub signature_data: Option<String>,
    pub comments: Option<String>,
}

/// Aggregated signature bundle ready for Stellar submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedSignatures {
    pub proposal_id: Uuid,
    pub transaction_envelope: String,
    pub signatures: Vec<SignatureEntry>,
    pub total_weight: u32,
    pub threshold: u32,
}

/// A single signature entry in the aggregated bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureEntry {
    pub signer_address: String,
    pub signature_data: String,
    pub weight: u32,
}

/// Errors specific to multi-sig operations
#[derive(Debug, thiserror::Error)]
pub enum MultiSigError {
    #[error("Proposal not found: {0}")]
    NotFound(Uuid),

    #[error("Proposal already executed")]
    AlreadyExecuted,

    #[error("Proposal has expired")]
    Expired,

    #[error("Proposal is not in a state that allows this action (current: {0:?})")]
    InvalidState(ProposalStatus),

    #[error("Signer {0} is not registered on this proposal")]
    UnknownSigner(String),

    #[error("Signer {0} has already submitted a decision")]
    AlreadySigned(String),

    #[error("Threshold not met: need {threshold} weight, have {current}")]
    ThresholdNotMet { threshold: u32, current: u32 },

    #[error("Invalid signature for signer {0}")]
    InvalidSignature(String),

    #[error("Threshold {threshold} exceeds total signer weight {total}")]
    InvalidThreshold { threshold: u32, total: u32 },

    #[error("Duplicate signer address: {0}")]
    DuplicateSigner(String),

    #[error("Signer {0} has been revoked and can no longer sign")]
    SignerRevoked(String),

    #[error("Execution blocked: signer {0} was revoked after signing")]
    RevokedSignerOnExecution(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Stellar submission error: {0}")]
    SubmissionError(String),

    #[error("Unauthorized: user {0} does not own proposal {1}")]
    Unauthorized(Uuid, Uuid),
}
