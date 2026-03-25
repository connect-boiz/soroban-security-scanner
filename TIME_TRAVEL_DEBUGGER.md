# Stellar Ledger State "Time Travel" Debugger

## Overview

The Time Travel Debugger enables developers to "fork" the Stellar Mainnet state at specific ledger sequences to test contracts against live historical data while maintaining read-only access.

## Features

- **Historical State Forking**: Fork network state at any ledger sequence
- **Contract State Injection**: Populate local WASM runner with fetched ledger data  
- **Upgrade Simulation**: Test contract upgrade compatibility with existing state
- **Orphaned State Tracking**: Identify storage entries that become inaccessible
- **LRU Caching**: Optimize performance with intelligent caching
- **Read-Only Operation**: Prevents accidental network interference

## CLI Commands

### Fork Network State
```bash
stellar-scanner time-travel fork --ledger-sequence 1000000
```

### Test Contract Against Historical State
```bash
stellar-scanner time-travel test --contract-id CONTRACT_ID --ledger-sequence 1000000
```

### Simulate Contract Upgrade
```bash
stellar-scanner time-travel upgrade --contract-id CONTRACT_ID --wasm-file new.wasm --ledger-sequence 1000000
```

### Find Orphaned State
```bash
stellar-scanner time-travel orphaned --contract-id CONTRACT_ID --wasm-file new.wasm --ledger-sequence 1000000
```

### Cache Management
```bash
stellar-scanner time-travel cache-stats
stellar-scanner time-travel clear-cache
```

## API Usage

```rust
use stellar_security_scanner::time_travel_debugger::*;

let config = TimeTravelConfig::default();
let debugger = TimeTravelDebugger::new(config).await?;

// Fork at specific ledger
let forked_state = debugger.fork_at_ledger(1000000).await?;

// Test contract
let result = forked_state.test_contract("CONTRACT_ID").await?;

// Simulate upgrade
let upgrade_result = debugger.simulate_contract_upgrade(
    "CONTRACT_ID", 
    &new_wasm, 
    1000000
).await?;
```

## Key Components

- **TimeTravelDebugger**: Main interface for time travel operations
- **StateInjector**: Injects ledger data into local WASM environment
- **ContractUpgradeSimulator**: Tests upgrade compatibility
- **OrphanedStateTracker**: Identifies inaccessible storage entries
- **StateCache**: LRU caching for performance optimization

## Security Considerations

- Strictly read-only access to Stellar network
- No transaction submission capabilities
- Local state isolation prevents network interference
- Comprehensive validation of fetched data

## Performance

- LRU cache with configurable size (default: 10,000 contract states)
- Parallel state fetching for multiple contracts
- Intelligent cache eviction policies
- Background cleanup of expired entries

## Integration

The Time Travel Debugger integrates seamlessly with existing scanner functionality and can be used alongside standard security scans to provide comprehensive contract analysis against real-world conditions.
