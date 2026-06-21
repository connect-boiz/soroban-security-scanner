# Issue 30: [Documentation] No API Reference or Usage Examples for the Differential Fuzzing Module

## Description

The differential fuzzing module is one of the most technically sophisticated features of the Soroban Security Scanner, supporting multi-SDK-version execution, cross-contract simulation, ledger snapshot integration, and deterministic behavior detection. However, the module has no user-facing documentation beyond the code comments. The `docs/` directory contains documentation for time-based attacks, upgrade mechanisms, web fonts, balance checks, and accessibility, but no `DIFFERENTIAL_FUZZING.md` file. There are no usage examples showing how to configure and run differential fuzzing for a real contract, how to interpret the `DifferentialFuzzingReport` output, or how to fix discrepancies found by the fuzzer. The `examples/` directory contains example files for vulnerable contracts, secure contracts, Kubernetes scanning, scanner registry usage, and auth server, but no example contract for differential fuzzing. A new user cannot determine what `sdk_versions`, `edge_case_types`, or `gas_threshold_percentage` configurations mean for their use case without reading the source code.

## Acceptance Criteria

- [ ] Create `docs/DIFFERENTIAL_FUZZING.md` with sections: Overview, Quick Start, Configuration Guide, Interpreting Results, Common Issues, and Best Practices
- [ ] Add a commented example contract at `examples/differential_fuzzing_example.rs` with known discrepancies to demonstrate fuzzing output
- [ ] Create a configuration example at `examples/differential_fuzzing_config.toml` with annotated fields
- [ ] Include CLI usage examples for all differential fuzzing subcommands (run, generate-inputs, compare-versions, validate-deterministic, test-with-network-state, analyze-reentrancy)
- [ ] Add a troubleshooting section for common issues: "No discrepancies found when there should be", "Cross-contract simulation times out", "Ledger snapshot integration fails"
- [ ] Link the new documentation from the main `README.md` under a "Differential Fuzzing" heading

## Additional Context

Key files: `src/differential_fuzzing/`, `docs/`, `examples/`, `README.md`.
