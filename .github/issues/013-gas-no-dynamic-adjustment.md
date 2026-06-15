# Issue 13: [Gas Limits] No Dynamic Gas Adjustment Based on Historical Usage Patterns

## Description

The `GasLimitManager` in `src/gas_limits.rs` uses static operation profiles and a fixed safety margin (10%) to estimate gas consumption. It does not learn from actual execution results to improve its estimation accuracy over time. If the base profile for `escrow_release` assumes 5,000 gas per transfer but on Stellar mainnet each transfer consistently costs 6,200 gas, the estimator will always under-report, causing users to set insufficient gas limits. Conversely, if a profile is too conservative, users may overpay in gas fees. Without adaptive gas estimation, the platform cannot optimize gas costs for users, and the `InsufficientGasLimitConsiderations` vulnerability detection produces stale recommendations.

## Acceptance Criteria

- [ ] Add a `GasUsageHistory` storage that records actual gas consumption per operation type per execution
- [ ] Implement a moving-average gas estimator that uses the last 100 executions to adjust profile base costs
- [ ] Add a `learning_rate` parameter (default 0.3) to control how quickly estimates adapt to new data
- [ ] Expose historical gas trends via a new API endpoint: `GET /api/v1/gas/trends?operation=escrow_release`
- [ ] Add a frontend visualization in `AnalyticsDashboard.tsx` showing gas estimation accuracy over time
- [ ] Write tests verifying that the adaptive estimator converges to within +/-5% of actual costs after 50+ data points

## Additional Context

Key files: `src/gas_limits.rs`, `src/analysis.rs`, `frontend/components/AnalyticsDashboard.tsx`.
