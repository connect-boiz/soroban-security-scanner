# Issue 5: [Multi-Sig] Proposal Execution Does Not Validate Signer Weight Thresholds Before Marking as Executable

## Description

The `MultiSigService` in `src/multisig/service.rs` tracks signatures collected for multi-signature proposals and marks a proposal as ready for execution once `collected_signatures.len() >= required_signatures`. However, the implementation does not account for per-signer weight values that should be defined in a `SignerSpec`. In real multi-signature configurations, different signers may have different weight levels (e.g., one "admin" signer might count as 3 votes, while a regular "team member" counts as 1). The current check using a simple count of signatures means that a proposal could be executed with enough low-weight signers while high-weight signers are still missing, violating the intended weighted voting model. Additionally, there is no validation that the collected signatures are from distinct, currently authorized signers — if a signer address was revoked between signing and execution, the stale signature should be rejected.

## Acceptance Criteria

- [ ] Add a `weight` field to `MultiSigSigner` and `SignerSpec` in `src/multisig/types.rs`
- [ ] Change proposal readiness check from counting signatures to summing weights: `sum(weights) >= threshold`
- [ ] Add execution-time validation that all signers are still active (not revoked) at the moment of execution
- [ ] Emit an event via event_logging when a signer's weight is not sufficient to contribute to the threshold (e.g., "signature from revoked signer rejected")
- [ ] Update `MultiSigWizard.tsx` frontend component to display individual signer weights and current accumulated weight
- [ ] Write comprehensive tests in `tests/` covering weighted proposals, revoked signer rejection, and threshold boundary cases

## Additional Context

Key files: `src/multisig/service.rs`, `src/multisig/types.rs`, `src/multisig/mod.rs`, `frontend/components/MultiSigWizard.tsx`.
