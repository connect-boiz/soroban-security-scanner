//! Tests for multi-signature signer revocation and execution-time validation.
//!
//! Covers Issue #5: weighted signer thresholds, revoked signer rejection,
//! and execution-time validation of signer activity.

use std::sync::Arc;
use uuid::Uuid;

use soroban_security_scanner::multisig::*;

fn create_test_proposal_request(signer_count: u32) -> CreateProposalRequest {
    let signers: Vec<SignerSpec> = (0..signer_count)
        .map(|i| SignerSpec {
            signer_address: format!("G{}...test_signer_{}", "A", i),
            signer_wallet_id: None,
            signer_user_id: None,
            weight: 1,
        })
        .collect();

    CreateProposalRequest {
        user_id: Uuid::new_v4(),
        operation_name: "test_operation".to_string(),
        description: Some("Test proposal".to_string()),
        stellar_address: "GABC...TEST_ACCOUNT".to_string(),
        threshold: (signer_count as f64 / 2.0).ceil() as u32, // majority threshold
        transaction_envelope: "AAAAAgAAAAB...test_xdr".to_string(),
        signers,
        expires_in_hours: Some(48),
    }
}

fn create_service() -> MultiSigService {
    let store = Arc::new(InMemoryMultiSigStore::default());
    MultiSigService::new(store)
}

#[tokio::test]
async fn test_create_proposal_with_weighted_signers() {
    let service = create_service();

    let mut req = create_test_proposal_request(3);
    req.signers[0].weight = 3; // Admin weight
    req.signers[1].weight = 1;
    req.signers[2].weight = 1;
    req.threshold = 3; // Need admin or multiple smaller signers

    let proposal = service.create_proposal(req).await.unwrap();
    assert_eq!(proposal.total_weight, 5);
    assert_eq!(proposal.status, ProposalStatus::Pending);
}

#[tokio::test]
async fn test_threshold_exceeds_total_weight_is_rejected() {
    let service = create_service();

    let mut req = create_test_proposal_request(3);
    req.signers.iter_mut().for_each(|s| s.weight = 1);
    req.threshold = 10; // Impossible threshold

    let result = service.create_proposal(req).await;
    assert!(matches!(
        result,
        Err(MultiSigError::InvalidThreshold { threshold: 10, total: 3 })
    ));
}

#[tokio::test]
async fn test_revoke_signer_success() {
    let service = create_service();
    let req = create_test_proposal_request(3);
    let user_id = req.user_id;
    let signer_address = req.signers[0].signer_address.clone();

    let proposal = service.create_proposal(req).await.unwrap();
    let proposal_id = proposal.id;

    // Revoke the first signer
    let updated = service
        .revoke_signer(proposal_id, user_id, signer_address.clone())
        .await
        .unwrap();

    let revoked_signer = updated
        .signers
        .iter()
        .find(|s| s.signer_address == signer_address)
        .unwrap();
    assert!(revoked_signer.revoked_at.is_some());
    assert!(!revoked_signer.is_active());
}

#[tokio::test]
async fn test_revoke_unknown_signer_fails() {
    let service = create_service();
    let req = create_test_proposal_request(2);
    let user_id = req.user_id;

    let proposal = service.create_proposal(req).await.unwrap();

    let result = service
        .revoke_signer(proposal.id, user_id, "GUNKNOWN...signer".to_string())
        .await;
    assert!(matches!(result, Err(MultiSigError::UnknownSigner(_))));
}

#[tokio::test]
async fn test_revoke_already_revoked_signer_fails() {
    let service = create_service();
    let req = create_test_proposal_request(2);
    let user_id = req.user_id;
    let signer_address = req.signers[0].signer_address.clone();

    let proposal = service.create_proposal(req).await.unwrap();

    // First revocation succeeds
    service
        .revoke_signer(proposal.id, user_id, signer_address.clone())
        .await
        .unwrap();

    // Second revocation fails
    let result = service
        .revoke_signer(proposal.id, user_id, signer_address)
        .await;
    assert!(matches!(result, Err(MultiSigError::SignerRevoked(_))));
}

#[tokio::test]
async fn test_revoked_signer_cannot_submit_signature() {
    let service = create_service();
    let req = create_test_proposal_request(3);
    let user_id = req.user_id;
    let signer_address = req.signers[0].signer_address.clone();

    let proposal = service.create_proposal(req).await.unwrap();
    let proposal_id = proposal.id;

    // Revoke the signer
    service
        .revoke_signer(proposal_id, user_id, signer_address.clone())
        .await
        .unwrap();

    // Attempt to submit signature as revoked signer
    let result = service
        .submit_signature(SubmitSignatureRequest {
            proposal_id,
            signer_address: signer_address.clone(),
            decision: SignerDecision::Approved,
            signature_data: Some("base64_sig".to_string()),
            comments: None,
        })
        .await;

    assert!(
        matches!(result, Err(MultiSigError::SignerRevoked(_))),
        "Revoked signer should be rejected: got {:?}",
        result
    );
}

#[tokio::test]
async fn test_execution_blocked_when_approver_was_revoked() {
    let service = create_service();
    let mut req = create_test_proposal_request(3);
    req.threshold = 1; // Single signer can approve
    let user_id = req.user_id;
    let signer_address = req.signers[0].signer_address.clone();

    let proposal = service.create_proposal(req).await.unwrap();
    let proposal_id = proposal.id;

    // First signer approves
    service
        .submit_signature(SubmitSignatureRequest {
            proposal_id,
            signer_address: signer_address.clone(),
            decision: SignerDecision::Approved,
            signature_data: Some("base64_sig".to_string()),
            comments: None,
        })
        .await
        .unwrap();

    // Proposal should be approved (weight threshold met)
    let approved = service.get_proposal(proposal_id).await.unwrap();
    assert_eq!(approved.status, ProposalStatus::Approved);

    // Now revoke the approving signer
    service
        .revoke_signer(proposal_id, user_id, signer_address.clone())
        .await
        .unwrap();

    // Execution should be blocked
    let result = service
        .mark_executed(proposal_id, user_id, "tx_hash_123".to_string())
        .await;

    assert!(
        matches!(result, Err(MultiSigError::RevokedSignerOnExecution(_))),
        "Execution should be blocked when an approving signer was revoked: got {:?}",
        result
    );
}

#[tokio::test]
async fn test_threshold_auto_reduces_after_revocation() {
    let service = create_service();
    let mut req = create_test_proposal_request(3);
    req.signers.iter_mut().for_each(|s| s.weight = 2); // Each signer weight 2
    req.threshold = 4; // Need at least 2 signers
    let user_id = req.user_id;
    let signer_address = req.signers[0].signer_address.clone();

    let proposal = service.create_proposal(req).await.unwrap();
    assert_eq!(proposal.total_weight, 6);
    assert_eq!(proposal.threshold, 4);

    // Revoke one signer → total drops to 4, threshold should auto-reduce to 4
    let updated = service
        .revoke_signer(proposal.id, user_id, signer_address)
        .await
        .unwrap();

    assert_eq!(updated.total_weight, 4, "Total weight should drop to 4");
    assert_eq!(
        updated.threshold, 4,
        "Threshold should auto-reduce from 4 to match new total of 4"
    );
}

#[tokio::test]
async fn test_proposal_rejected_when_threshold_unreachable_after_revocation() {
    let service = create_service();
    let mut req = create_test_proposal_request(3);
    req.signers[0].weight = 3; // Heavy signer
    req.signers[1].weight = 1;
    req.signers[2].weight = 1;
    req.threshold = 4; // Need heavy + one more
    let user_id = req.user_id;
    let heavy_signer = req.signers[0].signer_address.clone();

    let proposal = service.create_proposal(req).await.unwrap();
    assert_eq!(proposal.total_weight, 5);

    // Revoke the heavy signer → remaining weight = 2, threshold = 4 → unreachable
    let updated = service
        .revoke_signer(proposal.id, user_id, heavy_signer)
        .await
        .unwrap();

    assert_eq!(
        updated.status,
        ProposalStatus::Rejected,
        "Proposal should be rejected when threshold becomes unreachable"
    );
    assert_eq!(updated.total_weight, 2);
    // Threshold should auto-reduce to 2 (the new total)
    assert_eq!(updated.threshold, 2);
}

#[tokio::test]
async fn test_active_signers_can_still_sign_after_other_revoked() {
    let service = create_service();
    let req = create_test_proposal_request(3);
    let user_id = req.user_id;
    let signer1 = req.signers[0].signer_address.clone();
    let signer2 = req.signers[1].signer_address.clone();

    let proposal = service.create_proposal(req).await.unwrap();
    let proposal_id = proposal.id;

    // Revoke signer 1
    service
        .revoke_signer(proposal_id, user_id, signer1)
        .await
        .unwrap();

    // Signer 2 should still be able to sign
    let result = service
        .submit_signature(SubmitSignatureRequest {
            proposal_id,
            signer_address: signer2.clone(),
            decision: SignerDecision::Approved,
            signature_data: Some("base64_sig".to_string()),
            comments: None,
        })
        .await;

    assert!(
        result.is_ok(),
        "Active signer should still be able to sign after other is revoked"
    );
}

#[tokio::test]
async fn test_unauthorized_user_cannot_revoke_signer() {
    let service = create_service();
    let req = create_test_proposal_request(2);
    let wrong_user = Uuid::new_v4();
    let signer_address = req.signers[0].signer_address.clone();

    let proposal = service.create_proposal(req).await.unwrap();

    // Different user tries to revoke
    let result = service
        .revoke_signer(proposal.id, wrong_user, signer_address)
        .await;

    assert!(matches!(result, Err(MultiSigError::Unauthorized(_, _))));
}
