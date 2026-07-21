# Symbolic Execution Engine

## Overview

The symbolic execution engine provides path-aware vulnerability detection for Soroban smart contracts. Unlike the existing pattern-matching scanner, it explores all feasible execution paths by representing contract state and inputs as symbolic variables and solving path constraints with an SMT solver.

## Architecture

### Phase 1 — IR Lifting (`src/symbolic/ir.rs`)
Translates Soroban WASM bytecode into a three-address-code intermediate representation with explicit control flow graphs (CFG).

### Phase 2 — Symbolic State Model (`src/symbolic/state.rs`)
Defines `SymbolicState` containing symbolic ledger entries, call stack, and path constraints.

### Phase 3 — Stellar Semantics
Models Stellar-specific operations (require_auth, trustline changes, account merges) as symbolic constraints. (TODO)

### Phase 4 — Constraint Solver Integration
Integrates with Z3 SMT solver for 128-bit integer arithmetic. (TODO)

### Phase 5 — Vulnerability Checkers (`src/symbolic/checkers.rs`)
Each checker examines the symbolic state at path endpoints:
- `MissingAuthChecker` — detects missing require_auth calls
- `IntegerOverflowChecker` — detects i128 overflow conditions (TODO: Z3 integration)
- `ReentrancyChecker` — detects checks-effects-interactions violations (TODO)

### Phase 6 — Path Exploration (`src/symbolic/explorer.rs`)
Search strategies: BFS, DFS, and coverage-guided best-first search with configurable depth and path limits.

### Phase 7 — Concolic Mode (TODO)
Hybrid mode combining concrete fuzzer inputs with symbolic exploration.

### Phase 8 — Benchmark Suite (TODO)
20 Soroban contracts with known vulnerabilities for validation.

### Phase 9 — CLI Integration (TODO)
`stellar-scanner scan --symbolic [--max-depth N] [--max-paths M] [--timeout T]`

## Known Limitations

- Loop unrolling is bounded by `max_depth`
- Unmodeled host functions are treated as uninterpreted functions
- Z3 SMT solver integration is not yet implemented
- Only `MissingAuthChecker` is fully implemented; other checkers are scaffolds

## When to Use Symbolic vs. Concrete Scanning

- **Use concrete scanning** for: quick checks, known vulnerability patterns, CI/CD pipelines
- **Use symbolic scanning** for: deep analysis, multi-condition vulnerabilities, security audits
