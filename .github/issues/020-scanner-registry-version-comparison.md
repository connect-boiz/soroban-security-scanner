# Issue 20: [Scanner Registry] No Version Comparison Logic — Cannot Determine If Scanner Is Up-to-Date

## Description

The `ScannerRegistry` in `src/scanner_registry.rs` manages registered security scanners and their versions with `ScannerVersion`, `VersionStatus` enums, and a registry that maps `(name, version)` pairs. However, the `VersionStatus` uses a simple string comparison (`current == latest`) to determine if a scanner is up to date. There is no semantic version comparison that can handle `>=`, `<=`, `~>` (pessimistic), or range constraints. The `ScannerVersion` struct stores version as a `String` and provides no `compare()` method. This means that when `scanner_registry_usage.py` checks if the installed scanner version meets the minimum requirement, the comparison is lexicographic rather than semantic, leading to incorrect results. For example, version `"9.0.0"` would be considered less than `"10.0.0"` in string comparison (because '9' > '1'), but would be correct in semantic comparison.

## Acceptance Criteria

- [ ] Add a `SemanticVersion` struct (major, minor, patch) that can be parsed from strings
- [ ] Replace the `version: String` in `ScannerVersion` with `version: SemanticVersion`
- [ ] Impl `PartialOrd`, `Ord`, and comparison operators for `SemanticVersion` that follow semver 2.0 rules
- [ ] Add support for pre-release tags (`-alpha.1`, `-beta.2`) and build metadata (`+build.1234`)
- [ ] Add a `satisfies_requirement(version, constraint)` function that supports operators: `>=`, `<=`, `~>`, `^`, `=`
- [ ] Update `VersionStatus` to use semantic comparison instead of string equality
- [ ] Write tests covering all comparison operators, pre-release precedence, and edge cases like `0.0.0` and `999.999.999`

## Additional Context

Key files: `src/scanner_registry.rs`, `src/scanner_registry_tests.rs`, `examples/scanner_registry_usage.py`.
