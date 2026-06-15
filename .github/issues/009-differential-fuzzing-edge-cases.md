# Issue 9: [Differential Fuzzing] Input Generator Misses Critical Edge Cases for Soroban `i128` Arithmetic

## Description

The `InputGenerator` in `src/differential_fuzzing/input_generator.rs` generates test inputs for contract functions, including edge cases like `MaxI128`, `MinI128`, `ZeroValue`, and `LargeVector`. However, it does not generate composite edge cases that combine multiple extreme values in a single function call — for example, invoking a `transfer(from, to, amount)` function with both `from` and `to` set to the same address, or combining `MaxI128` for amount with an empty `from` address, or passing a `LargeVector` as the `to` parameter. Many real-world Soroban vulnerabilities arise precisely from these combinations of edge conditions (e.g., self-transfer with maximum amount, or overflow when summing multiple extreme values). The current generator treats each edge case independently, producing test inputs that exercise only one boundary condition at a time.

## Acceptance Criteria

- [ ] Add a combinatorial edge case generation mode that creates Cartesian products of individual edge case types for multi-parameter functions
- [ ] Include specific composite scenarios: self-transfer, transfer to zero address, simultaneous max input and max output, overflow through multiple accumulative operations
- [ ] Implement a configurable `combinatorial_depth` parameter (default 2) in `DifferentialFuzzingConfig` that limits combinatorial explosion
- [ ] Add a deduplication step to avoid running identical test inputs multiple times
- [ ] Verify in tests that at least 50 distinct composite edge cases are generated from a configuration with 5 basic edge case types
- [ ] Document the composite edge case strategy in `docs/DIFFERENTIAL_FUZZING.md`

## Additional Context

Key files: `src/differential_fuzzing/input_generator.rs`, `src/differential_fuzzing/test_runner.rs`, `src/differential_fuzzing/tests.rs`.
