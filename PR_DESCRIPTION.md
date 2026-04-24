## Summary
- Hardened arithmetic paths to prevent overflow/underflow in contract and marketplace reward logic.
- Added guard patterns around payout flows so state changes happen only after payout checks pass.
- Added stricter input validation on critical reporting/funding parameters.
- Replaced predictable ledger-sequence IDs with hybrid counter + hash nonce identifiers.

## Changes
- Added shared checked-math helpers and reused them in pool funding, escrow accounting, reputation updates, bounty counters, timelock math, and reward calculations.
- Added validation helpers for positive amounts and non-empty bounded text/symbol inputs on external-facing mutation paths.
- Migrated report/escrow/emergency alert keying to typed maps with monotonic counters plus SHA-256 nonces derived from caller/context entropy.
- Added payout-ready guard events and ordering to make transfer integration safer and to avoid partial state updates on payout failure paths.
- Preserved public API behavior and event-driven workflow while improving internal safety checks.

## Testing
- [x] `cargo build --manifest-path contracts/Cargo.toml`
- [x] `cargo check --manifest-path contracts/Cargo.toml`
- [x] `ReadLints` on touched files (`contracts/src/lib.rs`, `src/bounty_marketplace.rs`)
- [ ] Full workspace `cargo build` (currently blocked by pre-existing regex literal errors in `src/scanners.rs`)

## Closing
Closes connect-boiz/soroban-security-scanner#107
Closes connect-boiz/soroban-security-scanner#108
Closes connect-boiz/soroban-security-scanner#109
Closes connect-boiz/soroban-security-scanner#110
