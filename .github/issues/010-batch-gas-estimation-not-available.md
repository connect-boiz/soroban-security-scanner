# Issue 10: [Batch Operations] Gas Estimation Not Available Before Batch Execution

## Description

The `BatchOperations` module in `src/batch_operations.rs` allows users to create and execute batch escrow releases and vulnerability verifications. However, there is no mechanism to estimate the gas cost of a batch operation before execution. Users submit batch requests without knowing whether the gas limit they provide is sufficient, leading to frequent "out of gas" transaction failures on Stellar. The `GasLimitManager` in `src/gas_limits.rs` already provides gas estimation infrastructure (`estimate_gas` and `validate_gas_limit` methods), but these are not integrated with the batch operations pipeline. Each item in a batch can have different gas costs depending on its complexity (e.g., verifying a critical vulnerability vs. a low-severity one), and the total gas cost can vary significantly based on the number and types of items batched together.

## Acceptance Criteria

- [ ] Add a `estimate_batch_gas()` method to `BatchOperations` that iterates over each item, calls `GasLimitManager::estimate_gas()` per item, and sums the results with overhead
- [ ] Add a CLI subcommand `stellar-scanner batch estimate-gas --batch-id <id>` that displays estimated, recommended, and maximum gas for the batch
- [ ] Update the frontend `BatchOperations.tsx` component to show a gas estimation summary before the user confirms execution
- [ ] If estimated gas exceeds 90% of the Stellar transaction limit, warn the user and suggest splitting the batch
- [ ] Write unit tests verifying gas estimation accuracy against known operation profiles
- [ ] Log gas estimation data in the `BatchOperationSummary` structure for post-execution analysis

## Additional Context

Key files: `src/batch_operations.rs`, `src/gas_limits.rs`, `frontend/components/batch/BatchOperationPanel.tsx`, `src/batch_operations_tests.rs`.
