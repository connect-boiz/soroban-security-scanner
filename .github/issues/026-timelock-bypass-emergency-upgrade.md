# Issue 26: [Contract Upgrade] Timelock Bypass in Emergency Upgrade Allows Admin to Skip Waiting Period Without Justification

## Description

The contract upgrade mechanism in `docs/UPGRADE_MECHANISM.md` and the associated Rust code allows an admin to perform an "emergency upgrade" that bypasses the standard 7-day timelock delay. This is necessary for critical security patches. However, the current implementation does not require any justification or proof that the upgrade is genuinely an emergency. An admin can call `emergency_upgrade()` with any reason string, including an arbitrary or misleading reason, and the upgrade proceeds immediately. There is no on-chain audit trail that distinguishes genuine emergency upgrades from malicious ones, and no mechanism for other stakeholders (multi-sig signers, community members) to challenge or halt a suspicious emergency upgrade. The contract also does not enforce a maximum frequency of emergency upgrades — a malicious admin could perform emergency upgrades repeatedly, effectively disabling the timelock entirely.

## Acceptance Criteria

- [ ] Add a `MAX_EMERGENCY_UPGRADES_PER_MONTH` constant (default 2) to limit emergency upgrade frequency
- [ ] Require the emergency upgrade reason to be at least 50 characters and include specifics about the vulnerability being patched
- [ ] Add a cooling-off period of 24 hours between emergency upgrades (even with admin privileges)
- [ ] Publish a forced `EMERGENCY_UPGRADE_NOTIFICATION` event with full details (reason, code diff hash, affected functions) that cannot be suppressed
- [ ] Implement a "Challenge Period" (6 hours) during which multi-sig signers can vote to halt the upgrade
- [ ] Write tests verifying: frequency cap enforcement, insufficient reason rejection, cooling-off period, and challenge period resolution

## Additional Context

Key files: `docs/UPGRADE_MECHANISM.md`, `src/time_travel_debugger/contract_upgrade.rs`.
