# Issue 29: [Bounty Marketplace] Payout Function Does Not Verify Escrow Balance Before Emitting `payout_ready`

## Description

The `BountyMarketplace` contract in `src/bounty_marketplace.rs` has a `claim_reward` function that calculates the bounty payout amount based on severity and emits a `payout_ready` event. However, it does not verify that the contract's escrow balance is sufficient to cover the payout before emitting the event. The `emit_payout_ready` function checks that `amount > 0` (positive), but does not check that the contract holds at least `amount` in XLM. If multiple bounties are approved but the escrow balance is insufficient (e.g., due to a prior payout that drained the contract), `payout_ready` is emitted even though the payout cannot be fulfilled. This creates a "phantom payout" scenario where researchers believe they will receive a reward, attempt to claim it, and fail because the funds do not exist — leading to a poor user experience and potential legal disputes.

## Acceptance Criteria

- [ ] Add a `get_contract_balance(env)` helper that queries the contract's actual XLM balance
- [ ] Before emitting `payout_ready` in `claim_reward()`, verify `contract_balance >= reward_amount`
- [ ] If the escrow balance is insufficient, emit an `insufficient_escrow_funds` error event and provide details about the shortfall
- [ ] Add a `replenish_escrow(amount, funder)` function that allows the admin to top up the escrow balance
- [ ] Expose the current escrow balance via a `GET /api/v1/bounty/escrow-balance` endpoint
- [ ] Write contract tests that simulate an escrow deficit scenario and verify that `payout_ready` is not emitted

## Additional Context

Key files: `src/bounty_marketplace.rs`, `src/escrow.rs`, `src/bounty_marketplace_tests.rs`.
