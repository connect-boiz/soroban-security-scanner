pub mod service;
pub mod types;

pub use service::{InMemoryMultiSigStore, MultiSigService, MultiSigStore};
pub use types::{
    AggregatedSignatures, CreateProposalRequest, MultiSigError, MultiSigProposal, MultiSigSigner,
    ProposalStatus, SignatureEntry, SignerDecision, SignerSpec, SubmitSignatureRequest,
};
