//! Multi-Signature Business Logic Service
//!
//! Handles threshold management, signature aggregation, and approval workflows
//! for Stellar multi-signature transactions.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::multisig::types::{
    AggregatedSignatures, CreateProposalRequest, MultiSigError, MultiSigProposal, MultiSigSigner,
    ProposalStatus, SignatureEntry, SignerDecision, SubmitSignatureRequest,
};

// ---------------------------------------------------------------------------
// Storage trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait MultiSigStore: Send + Sync {
    async fn create_proposal(&self, proposal: &MultiSigProposal) -> Result<(), MultiSigError>;
    async fn get_proposal(&self, id: Uuid) -> Result<Option<MultiSigProposal>, MultiSigError>;
    async fn list_proposals_for_user(&self, user_id: Uuid) -> Result<Vec<MultiSigProposal>, MultiSigError>;
    async fn list_proposals_for_signer(&self, signer_address: &str) -> Result<Vec<MultiSigProposal>, MultiSigError>;
    async fn update_proposal(&self, proposal: &MultiSigProposal) -> Result<(), MultiSigError>;
}

// ---------------------------------------------------------------------------
// In-memory store (for testing / development)
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct InMemoryMultiSigStore {
    proposals: RwLock<HashMap<Uuid, MultiSigProposal>>,
}

#[async_trait]
impl MultiSigStore for InMemoryMultiSigStore {
    async fn create_proposal(&self, proposal: &MultiSigProposal) -> Result<(), MultiSigError> {
        self.proposals.write().await.insert(proposal.id, proposal.clone());
        Ok(())
    }

    async fn get_proposal(&self, id: Uuid) -> Result<Option<MultiSigProposal>, MultiSigError> {
        Ok(self.proposals.read().await.get(&id).cloned())
    }

    async fn list_proposals_for_user(&self, user_id: Uuid) -> Result<Vec<MultiSigProposal>, MultiSigError> {
        Ok(self
            .proposals
            .read()
            .await
            .values()
            .filter(|p| p.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn list_proposals_for_signer(&self, signer_address: &str) -> Result<Vec<MultiSigProposal>, MultiSigError> {
        Ok(self
            .proposals
            .read()
            .await
            .values()
            .filter(|p| p.signers.iter().any(|s| s.signer_address == signer_address))
            .cloned()
            .collect())
    }

    async fn update_proposal(&self, proposal: &MultiSigProposal) -> Result<(), MultiSigError> {
        self.proposals.write().await.insert(proposal.id, proposal.clone());
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Multi-Signature Service
// ---------------------------------------------------------------------------

pub struct MultiSigService {
    store: Arc<dyn MultiSigStore>,
}

impl MultiSigService {
    pub fn new(store: Arc<dyn MultiSigStore>) -> Self {
        Self { store }
    }

    // -----------------------------------------------------------------------
    // Proposal lifecycle
    // -----------------------------------------------------------------------

    /// Create a new multi-sig proposal with an initial set of signers.
    pub async fn create_proposal(
        &self,
        req: CreateProposalRequest,
    ) -> Result<MultiSigProposal, MultiSigError> {
        // Validate signers: no duplicates
        let mut seen = std::collections::HashSet::new();
        for s in &req.signers {
            if !seen.insert(s.signer_address.clone()) {
                return Err(MultiSigError::DuplicateSigner(s.signer_address.clone()));
            }
        }

        let total_weight: u32 = req.signers.iter().map(|s| s.weight).sum();

        // Threshold must be achievable
        if req.threshold > total_weight {
            return Err(MultiSigError::InvalidThreshold {
                threshold: req.threshold,
                total: total_weight,
            });
        }

        let now = Utc::now();
        let expires_at = req
            .expires_in_hours
            .map(|h| now + Duration::hours(h as i64))
            .or_else(|| Some(now + Duration::days(7)));

        let proposal_id = Uuid::new_v4();
        let signers: Vec<MultiSigSigner> = req
            .signers
            .into_iter()
            .map(|spec| MultiSigSigner {
                id: Uuid::new_v4(),
                proposal_id,
                signer_address: spec.signer_address,
                signer_wallet_id: spec.signer_wallet_id,
                signer_user_id: spec.signer_user_id,
                weight: spec.weight,
                decision: SignerDecision::Pending,
                signature_data: None,
                signed_at: None,
                comments: None,
                created_at: now,
                updated_at: now,
                revoked_at: None,
            })
            .collect();

        let proposal = MultiSigProposal {
            id: proposal_id,
            user_id: req.user_id,
            operation_name: req.operation_name,
            description: req.description,
            stellar_address: req.stellar_address,
            threshold: req.threshold,
            total_weight,
            status: ProposalStatus::Pending,
            transaction_envelope: req.transaction_envelope,
            signers,
            created_at: now,
            updated_at: now,
            expires_at,
            executed_at: None,
            executed_transaction_hash: None,
            metadata: HashMap::new(),
        };

        self.store.create_proposal(&proposal).await?;
        Ok(proposal)
    }

    // -----------------------------------------------------------------------
    // Signature submission
    // -----------------------------------------------------------------------

    /// Submit a signer's approval or rejection for a proposal.
    pub async fn submit_signature(
        &self,
        req: SubmitSignatureRequest,
    ) -> Result<MultiSigProposal, MultiSigError> {
        let mut proposal = self.get_active_proposal(req.proposal_id).await?;

        // Find the signer
        let signer_idx = proposal
            .signers
            .iter()
            .position(|s| s.signer_address == req.signer_address)
            .ok_or_else(|| MultiSigError::UnknownSigner(req.signer_address.clone()))?;

        // Reject if the signer has been revoked
        if !proposal.signers[signer_idx].is_active() {
            let revoked_at = proposal.signers[signer_idx].revoked_at;
            warn!(
                "Revoked signer {} attempted to submit signature on proposal {}. Revoked at: {:?}",
                req.signer_address, req.proposal_id, revoked_at
            );
            return Err(MultiSigError::SignerRevoked(req.signer_address.clone()));
        }

        if proposal.signers[signer_idx].decision != SignerDecision::Pending {
            return Err(MultiSigError::AlreadySigned(req.signer_address.clone()));
        }

        // Require signature data when approving
        if req.decision == SignerDecision::Approved && req.signature_data.is_none() {
            return Err(MultiSigError::InvalidSignature(req.signer_address.clone()));
        }

        let now = Utc::now();
        proposal.signers[signer_idx].decision = req.decision;
        proposal.signers[signer_idx].signature_data = req.signature_data;
        proposal.signers[signer_idx].comments = req.comments;
        proposal.signers[signer_idx].signed_at = Some(now);
        proposal.signers[signer_idx].updated_at = now;

        // Recalculate status
        proposal.status = self.compute_status(&proposal);
        proposal.updated_at = now;

        self.store.update_proposal(&proposal).await?;
        Ok(proposal)
    }

    // -----------------------------------------------------------------------
    // Threshold management
    // -----------------------------------------------------------------------

    /// Update the approval threshold on a pending proposal.
    /// Only the proposal owner can do this.
    pub async fn update_threshold(
        &self,
        proposal_id: Uuid,
        user_id: Uuid,
        new_threshold: u32,
    ) -> Result<MultiSigProposal, MultiSigError> {
        let mut proposal = self.get_owned_proposal(proposal_id, user_id).await?;

        if proposal.status != ProposalStatus::Pending {
            return Err(MultiSigError::InvalidState(proposal.status.clone()));
        }

        if new_threshold > proposal.total_weight {
            return Err(MultiSigError::InvalidThreshold {
                threshold: new_threshold,
                total: proposal.total_weight,
            });
        }

        proposal.threshold = new_threshold;
        proposal.status = self.compute_status(&proposal);
        proposal.updated_at = Utc::now();

        self.store.update_proposal(&proposal).await?;
        Ok(proposal)
    }

    // -----------------------------------------------------------------------
    // Signature aggregation
    // -----------------------------------------------------------------------

    /// Aggregate all approved signatures into a bundle ready for Stellar submission.
    pub async fn aggregate_signatures(
        &self,
        proposal_id: Uuid,
    ) -> Result<AggregatedSignatures, MultiSigError> {
        let proposal = self
            .store
            .get_proposal(proposal_id)
            .await?
            .ok_or(MultiSigError::NotFound(proposal_id))?;

        if proposal.status != ProposalStatus::Approved {
            return Err(MultiSigError::ThresholdNotMet {
                threshold: proposal.threshold,
                current: proposal.approved_weight(),
            });
        }

        let signatures: Vec<SignatureEntry> = proposal
            .signers
            .iter()
            .filter(|s| s.decision == SignerDecision::Approved)
            .filter_map(|s| {
                s.signature_data.as_ref().map(|sig| SignatureEntry {
                    signer_address: s.signer_address.clone(),
                    signature_data: sig.clone(),
                    weight: s.weight,
                })
            })
            .collect();

        Ok(AggregatedSignatures {
            proposal_id,
            transaction_envelope: proposal.transaction_envelope.clone(),
            signatures,
            total_weight: proposal.approved_weight(),
            threshold: proposal.threshold,
        })
    }

    // -----------------------------------------------------------------------
    // Execution
    // -----------------------------------------------------------------------

    /// Mark a proposal as executed after successful Stellar submission.
    /// Validates that all approving signers are still active before execution.
    pub async fn mark_executed(
        &self,
        proposal_id: Uuid,
        user_id: Uuid,
        transaction_hash: String,
    ) -> Result<MultiSigProposal, MultiSigError> {
        let mut proposal = self.get_owned_proposal(proposal_id, user_id).await?;

        if proposal.status != ProposalStatus::Approved {
            return Err(MultiSigError::InvalidState(proposal.status.clone()));
        }

        // Validate that all approving signers are still active (not revoked)
        let revoked_approvers: Vec<&str> = proposal
            .signers
            .iter()
            .filter(|s| s.decision == SignerDecision::Approved && !s.is_active())
            .map(|s| s.signer_address.as_str())
            .collect();

        if !revoked_approvers.is_empty() {
            for address in &revoked_approvers {
                warn!(
                    "Execution blocked for proposal {}: approving signer {} was revoked after signing",
                    proposal_id, address
                );
            }
            return Err(MultiSigError::RevokedSignerOnExecution(
                revoked_approvers.join(", "),
            ));
        }

        let now = Utc::now();
        proposal.status = ProposalStatus::Executed;
        proposal.executed_at = Some(now);
        proposal.executed_transaction_hash = Some(transaction_hash);
        proposal.updated_at = now;

        info!(
            "Proposal {} executed with {} approved signers. Transaction hash: {}",
            proposal_id,
            proposal.signers.iter().filter(|s| s.decision == SignerDecision::Approved).count(),
            transaction_hash
        );

        self.store.update_proposal(&proposal).await?;
        Ok(proposal)
    }

    /// Revoke a signer's ability to participate in a proposal.
    /// Only the proposal owner can revoke a signer.
    pub async fn revoke_signer(
        &self,
        proposal_id: Uuid,
        user_id: Uuid,
        signer_address: String,
    ) -> Result<MultiSigProposal, MultiSigError> {
        let mut proposal = self.get_owned_proposal(proposal_id, user_id).await?;

        if proposal.status != ProposalStatus::Pending {
            return Err(MultiSigError::InvalidState(proposal.status.clone()));
        }

        let signer_idx = proposal
            .signers
            .iter()
            .position(|s| s.signer_address == signer_address)
            .ok_or_else(|| MultiSigError::UnknownSigner(signer_address.clone()))?;

        if !proposal.signers[signer_idx].is_active() {
            return Err(MultiSigError::SignerRevoked(signer_address.clone()));
        }

        let now = Utc::now();
        proposal.signers[signer_idx].revoked_at = Some(now);
        proposal.signers[signer_idx].updated_at = now;

        // Recalculate total_weight since the revoked signer can no longer contribute
        proposal.total_weight = proposal
            .signers
            .iter()
            .filter(|s| s.is_active())
            .map(|s| s.weight)
            .sum();

        // If threshold exceeds new total, adjust threshold down
        if proposal.threshold > proposal.total_weight {
            info!(
                "Reducing threshold from {} to {} after signer {} was revoked on proposal {}",
                proposal.threshold, proposal.total_weight, signer_address, proposal_id
            );
            proposal.threshold = proposal.total_weight;
        }

        // Recalculate status (may become rejected if threshold is no longer achievable)
        let previous_status = proposal.status.clone();
        proposal.status = self.compute_status(&proposal);
        proposal.updated_at = now;

        info!(
            "Signer {} revoked from proposal {}. Status changed from {:?} to {:?}",
            signer_address, proposal_id, previous_status, proposal.status
        );

        self.store.update_proposal(&proposal).await?;
        Ok(proposal)
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    pub async fn get_proposal(&self, proposal_id: Uuid) -> Result<MultiSigProposal, MultiSigError> {
        self.store
            .get_proposal(proposal_id)
            .await?
            .ok_or(MultiSigError::NotFound(proposal_id))
    }

    pub async fn list_proposals_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<MultiSigProposal>, MultiSigError> {
        self.store.list_proposals_for_user(user_id).await
    }

    pub async fn list_proposals_for_signer(
        &self,
        signer_address: &str,
    ) -> Result<Vec<MultiSigProposal>, MultiSigError> {
        self.store.list_proposals_for_signer(signer_address).await
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Compute the correct status based on current signer decisions.
    fn compute_status(&self, proposal: &MultiSigProposal) -> ProposalStatus {
        if proposal.status == ProposalStatus::Executed {
            return ProposalStatus::Executed;
        }
        if proposal.is_expired() {
            return ProposalStatus::Expired;
        }
        if proposal.is_threshold_met() {
            return ProposalStatus::Approved;
        }
        // Rejection: compute remaining possible weight from active, pending signers
        // Revoked signers cannot contribute even if they haven't signed yet
        let pending_active_weight: u32 = proposal
            .signers
            .iter()
            .filter(|s| s.decision == SignerDecision::Pending && s.is_active())
            .map(|s| s.weight)
            .sum();
        let max_achievable = proposal.approved_weight() + pending_active_weight;
        if max_achievable < proposal.threshold {
            info!(
                "Proposal {} rejected: max achievable weight {} < threshold {}",
                proposal.id, max_achievable, proposal.threshold
            );
            return ProposalStatus::Rejected;
        }
        ProposalStatus::Pending
    }

    async fn get_active_proposal(&self, id: Uuid) -> Result<MultiSigProposal, MultiSigError> {
        let proposal = self
            .store
            .get_proposal(id)
            .await?
            .ok_or(MultiSigError::NotFound(id))?;

        match proposal.status {
            ProposalStatus::Executed => Err(MultiSigError::AlreadyExecuted),
            ProposalStatus::Expired => Err(MultiSigError::Expired),
            ProposalStatus::Rejected => Err(MultiSigError::InvalidState(ProposalStatus::Rejected)),
            _ => {
                if proposal.is_expired() {
                    Err(MultiSigError::Expired)
                } else {
                    Ok(proposal)
                }
            }
        }
    }

    async fn get_owned_proposal(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<MultiSigProposal, MultiSigError> {
        let proposal = self
            .store
            .get_proposal(id)
            .await?
            .ok_or(MultiSigError::NotFound(id))?;

        if proposal.user_id != user_id {
            return Err(MultiSigError::Unauthorized(user_id, id));
        }
        Ok(proposal)
    }
}
