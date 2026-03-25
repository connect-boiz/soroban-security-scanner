# Differential Fuzzing Module Documentation

## Overview

The Differential Fuzzing module provides comprehensive testing capabilities for Soroban smart contracts by comparing execution results across multiple SDK versions. This module helps identify discrepancies, security vulnerabilities, and non-deterministic behavior that might not be apparent through traditional testing methods.

## Key Features

### 1. Multi-Version Testing
- Execute the same test inputs across different Soroban SDK versions
- Compare gas consumption, state changes, and return values
- Identify version-specific behaviors and compatibility issues

### 2. Edge Case Generation
- Automatically generate edge case inputs (max/min values, empty vectors, etc.)
- Support for custom edge case types
- Intelligent complexity scoring for test prioritization

### 3. Execution Tracing
- Detailed execution trace logging for each SDK version
- Trace similarity analysis to detect logic divergences
- Memory usage and gas consumption tracking

### 4. Discrepancy Detection
- Gas consumption variance analysis
- State change comparison across versions
- Logic divergence detection through trace analysis
- Return value and error difference identification

### 5. Cross-Contract Simulation
- Reentrancy vulnerability detection
- Call graph analysis and cycle detection
- State consistency issue identification
- Security scoring for cross-contract interactions

### 6. Ledger Snapshot Integration
- Pull real network state for realistic testing
- Generate test inputs based on actual contract states
- Validate execution results against network baselines

### 7. Deterministic Behavior Analysis
- Flag non-deterministic behavior as high-priority vulnerabilities
- Multiple execution comparison for consistency
- Time dependency and external state analysis

## Architecture

The module is organized into several key components:

### Core Module (`differential_fuzzing.rs`)
- Main orchestrator and configuration
- Common data structures and types
- Integration point for all sub-modules

### Test Runner (`test_runner.rs`)
- Manages execution across multiple SDK versions
- Handles environment setup and cleanup
- Parallel execution support

### Input Generator (`input_generator.rs`)
- Edge case input generation
- Random and deterministic test data
- Complexity-based input selection

### Execution Tracer (`execution_tracer.rs`)
- Detailed execution trace capture
- Event logging and analysis
- Memory and gas tracking

### Discrepancy Detector (`discrepancy_detector.rs`)
- Multi-version result comparison
- Various discrepancy type detection
- Severity assessment and reporting

### Cross-Contract Simulator (`cross_contract_simulator.rs`)
- Reentrancy pattern detection
- Call graph construction and analysis
- Security vulnerability identification

### Ledger Snapshot Integration (`ledger_snapshot_integration.rs`)
- Real network state fetching
- Cache management for performance
- Network-based test generation

### Deterministic Detector (`deterministic_detector.rs`)
- Non-deterministic behavior detection
- Multiple execution analysis
- Impact assessment

## Usage Examples

### Basic Differential Fuzzing

```bash
# Run differential fuzzing with default settings
stellar-scanner differential-fuzzing run --contract-path ./contract.wasm

# Run with custom SDK versions
stellar-scanner differential-fuzzing run \
  --contract-path ./contract.wasm \
  --sdk-versions "25.3.0,25.2.0,25.1.0" \
  --test-count 5000

# Enable all advanced features
stellar-scanner differential-fuzzing run \
  --contract-path ./contract.wasm \
  --enable-cross-contract \
  --enable-ledger-snapshot \
  --enable-deterministic-detection \
  --gas-threshold 5.0
```

### Edge Case Input Generation

```bash
# Generate edge case inputs
stellar-scanner differential-fuzzing generate-inputs \
  --count 1000 \
  --output ./test_inputs.json \
  --edge-cases "MaxI128,MinI128,EmptyVector,LargeVector"

# Target specific functions
stellar-scanner differential-fuzzing generate-inputs \
  --count 500 \
  --output ./transfer_inputs.json \
  --functions "transfer,approve,balance"
```

### Network State Testing

```bash
# Test with real network state
stellar-scanner differential-fuzzing test-with-network-state \
  --contract-path ./contract.wasm \
  --ledger-sequence 1000000 \
  --test-count 100 \
  --rpc-url "https://mainnet.stellar.rpc"
```

### Reentrancy Analysis

```bash
# Analyze specific function for reentrancy
stellar-scanner differential-fuzzing analyze-reentrancy \
  --contract-path ./contract.wasm \
  --function "transfer" \
  --max-depth 15 \
  --output ./reentrancy_report.json
```

### Version Comparison

```bash
# Compare two specific SDK versions
stellar-scanner differential-fuzzing compare-versions \
  --contract-path ./contract.wasm \
  --version1 "25.3.0" \
  --version2 "25.2.0" \
  --input-file ./custom_inputs.json
```

### Deterministic Behavior Validation

```bash
# Validate deterministic behavior
stellar-scanner differential-fuzzing validate-deterministic \
  --contract-path ./contract.wasm \
  --retries 10 \
  --threshold 0.05
```

## Configuration

### DifferentialFuzzingConfig

```rust
pub struct DifferentialFuzzingConfig {
    pub sdk_versions: Vec<SdkVersion>,
    pub contract_path: String,
    pub test_count: usize,
    pub max_execution_time: Duration,
    pub enable_cross_contract_simulation: bool,
    pub enable_ledger_snapshot_integration: bool,
    pub enable_deterministic_detection: bool,
    pub edge_case_types: Vec<EdgeCaseType>,
    pub gas_threshold_percentage: f64,
}
```

### SDK Version Configuration

```rust
let sdk_version = SdkVersion::new("25.3.0")
    .with_git_hash("abc123")
    .with_release_date("2024-01-15");
```

## Discrepancy Types

The module can detect various types of discrepancies:

### Gas Consumption Differences
- Percentage-based gas variance detection
- Configurable threshold for significance
- Performance regression identification

### State Change Differences
- Storage entry comparison
- Missing or extra state changes
- Value inconsistency detection

### Logic Divergence
- Execution trace similarity analysis
- Branch difference detection
- Control flow variation identification

### Return Value Differences
- Type consistency checking
- Value variation analysis
- Success/failure mismatch

### Error Differences
- Error message comparison
- Error type consistency
- Success status variation

## Security Vulnerabilities Detected

### Reentrancy Patterns
- Direct reentrancy detection
- Indirect reentrancy through multiple contracts
- Cross-function reentrancy
- Delegate call reentrancy

### State Consistency Issues
- Race conditions
- Check-then-race patterns
- Atomicity violations
- Inconsistent state updates

### Non-Deterministic Behavior
- Random value generation
- Time-dependent logic
- External state dependencies
- Concurrent execution issues

## Performance Considerations

### Parallel Execution
- Tests run in parallel across SDK versions
- Configurable concurrency limits
- Resource usage monitoring

### Caching
- Ledger snapshot caching
- Execution result caching
- Memory usage optimization

### Resource Management
- Timeout handling for long-running tests
- Memory usage tracking
- Cleanup of temporary resources

## Integration with Other Modules

The differential fuzzing module integrates seamlessly with:

### Time Travel Debugger
- Historical state testing
- Ledger sequence analysis
- State consistency validation

### Security Scanner
- Vulnerability correlation
- Combined reporting
- Priority assessment

### Kubernetes Integration
- Distributed testing
- Resource scaling
- Parallel execution

## Best Practices

### Test Design
1. Use diverse edge cases for comprehensive coverage
2. Include realistic network state when possible
3. Test critical functions thoroughly
4. Monitor resource usage during testing

### Configuration
1. Adjust gas thresholds based on contract complexity
2. Enable cross-contract simulation for complex contracts
3. Use network state integration for production contracts
4. Configure appropriate timeouts for your environment

### Analysis
1. Review high-priority discrepancies first
2. Pay attention to non-deterministic behavior
3. Investigate gas consumption regressions
4. Validate reentrancy findings manually

## Troubleshooting

### Common Issues

1. **SDK Version Not Found**
   - Ensure the specified SDK version is available
   - Check version format (e.g., "25.3.0")

2. **Contract Execution Failures**
   - Verify contract WASM file path
   - Check contract initialization requirements
   - Review function signatures and arguments

3. **Network State Issues**
   - Verify RPC URL connectivity
   - Check ledger sequence availability
   - Ensure proper network configuration

4. **Performance Issues**
   - Reduce test count for faster results
   - Disable unnecessary features
   - Increase timeout values

### Debug Mode

Enable verbose output for detailed debugging:

```bash
stellar-scanner differential-fuzzing run \
  --contract-path ./contract.wasm \
  --verbose
```

## Future Enhancements

Planned improvements include:

1. **Enhanced Pattern Recognition**
   - Machine learning for discrepancy classification
   - Automated vulnerability pattern detection

2. **Performance Optimizations**
   - Incremental testing for large contracts
   - Smart test selection based on code changes

3. **Advanced Analytics**
   - Trend analysis across versions
   - Predictive vulnerability assessment

4. **Integration Improvements**
   - CI/CD pipeline integration
   - Automated reporting systems

## Contributing

When contributing to the differential fuzzing module:

1. Follow existing code patterns and naming conventions
2. Add comprehensive tests for new features
3. Update documentation for API changes
4. Consider performance implications
5. Ensure backward compatibility

## License

This module is part of the Soroban Security Scanner project and is licensed under the MIT License.
