# Issue 2: [Core Scanner] Weak Access Control Detection Produces False Positives for Internal Helper Functions

## Description

The `SecurityScanner` in `src/scanners.rs` uses regex-based pattern matching (`require_auth|has_auth`) to detect missing access control on public functions. However, this approach produces a significant number of false positives for public helper functions that are only called internally or are explicitly marked as `pub(crate)` — the regex only checks the `pub fn` visibility modifier at the text level, not the actual Rust visibility from the parsed AST. Functions with `pub(crate)` visibility, utility getters, and read-only query methods are incorrectly flagged as `MissingAccessControl`. This noise reduces the signal-to-noise ratio of scan reports and undermines developer trust in the scanning tool. The AST analysis in `analyze_function` does check for `syn::Visibility::Public`, but the regex patterns in `initialize_patterns` scan the raw file content and don't distinguish between different levels of visibility or function semantics.

## Acceptance Criteria

- [ ] Refactor `MissingAccessControl` detection to rely primarily on AST-level visibility analysis rather than raw regex matching
- [ ] Exclude `pub(crate)` and read-only query functions (no state mutations) from access control checks
- [ ] Add a configuration option `ignore_internal_helpers` to `VulnerabilityConfig` that allows users to suppress false positives for internal utility functions
- [ ] Reduce false positive rate for `MissingAccessControl` detection by at least 80% (measured against a benchmark suite of 50 contracts)
- [ ] Update the test suite in `tests/integration_tests.rs` with known false positive cases to ensure they are no longer flagged
- [ ] Document the change in vulnerability detection behavior in `docs/UPGRADE_MECHANISM.md` or a new migration note

## Additional Context

Key files: `src/scanners.rs`, `src/security_analyzer.rs`, `src/config.rs`, `tests/integration_tests.rs`.
