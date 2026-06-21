# Issue 8: [Time Travel Debugger] Forked State Not Compatible with Upgraded Contract WASM Versions

## Description

The `TimeTravelDebugger` in `src/time_travel_debugger.rs` creates a `ForkedState` by snapshotting the Stellar ledger at a specific sequence number and replaying contract state. When a user then attempts to test a new contract WASM against this historical state (via `simulate_contract_upgrade`), the forked state often fails to deserialize correctly because the new WASM version may define different storage schema, data structures, or key formats. The current error handling in `orphaned_state.rs` and `contract_upgrade.rs` genericizes these failures as "state incompatibility" without providing developers with specific information about which storage keys could not be migrated, what the expected vs. actual types are, or whether the incompatibility is critical or ignorable. Without detailed migration guidance, developers must manually diff the two contract versions to understand storage changes, which is error-prone and time-consuming.

## Acceptance Criteria

- [ ] Implement per-key storage compatibility analysis that reports for each key: key name, expected type, actual type, and migration status (compatible / incompatible / missing)
- [ ] Add a `StorageDiff` structure to `time_travel_debugger/types.rs` with old vs. new key-value pairs
- [ ] Categorize incompatibilities as `Critical` (data loss risk), `Warning` (type coercion possible), or `Info` (key added/removed)
- [ ] Generate a human-readable "Storage Migration Report" with recommendations for each incompatible storage key
- [ ] Update the time travel CLI command (`stellar-scanner time-travel upgrade`) to display the migration report
- [ ] Write tests that simulate storage schema changes between two contract versions and verify the diagnostic output

## Additional Context

Key files: `src/time_travel_debugger.rs`, `src/time_travel_debugger/contract_upgrade.rs`, `src/time_travel_debugger/orphaned_state.rs`, `src/time_travel_debugger/cache.rs`, `src/time_travel_debugger/state_injection.rs`.
