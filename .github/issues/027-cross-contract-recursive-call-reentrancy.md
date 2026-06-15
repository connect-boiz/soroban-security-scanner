# Issue 27: [Differential Fuzzing] Cross-Contract Simulator Does Not Handle Recursive Call Patterns, Missing Reentrancy Bugs

## Description

The `CrossContractSimulator` in `src/differential_fuzzing/cross_contract_simulator.rs` simulates cross-contract calls to detect reentrancy vulnerabilities. It currently supports a single level of call depth (`A → B`) but does not handle recursive or multi-level call patterns (`A → B → A` or `A → B → C → A`). In real Soroban contracts, reentrancy attacks often exploit multi-contract callback chains. The `ReentrancyPattern` enum has variants like `Simple`, `CrossContract`, `CallbackBased`, but the simulation engine only tests `Simple` patterns. This means that sophisticated reentrancy vulnerabilities that involve three or more contracts are not detected by the differential fuzzer, leaving a significant class of reentrancy bugs unaddressed.

## Acceptance Criteria

- [ ] Extend `CrossContractSimulator` to support recursive call patterns up to a configurable `max_depth` (default 5)
- [ ] Add call graph analysis that identifies cyclic dependencies between contract functions
- [ ] Implement state rollback after each simulated execution to ensure test isolation
- [ ] Add a `detect_reentrancy_patterns()` method that classifies detected patterns by type and severity
- [ ] Add a new `ReentrancyDepth` parameter to `DifferentialFuzzingConfig` to control the maximum call chain length
- [ ] Write tests with mock contracts that form 3-level and 4-level recursive call chains and verify detection

## Additional Context

Key files: `src/differential_fuzzing/cross_contract_simulator.rs`, `src/differential_fuzzing/test_runner.rs`, `src/differential_fuzzing/tests.rs`.
