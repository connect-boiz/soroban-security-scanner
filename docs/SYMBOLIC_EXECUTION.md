# Symbolic Execution Engine

## Issue #442: Build a Symbolic Execution Engine for Path-Aware Vulnerability Detection

## Overview

The symbolic execution engine represents contract state and function inputs as
symbolic variables, then explores all feasible execution paths by solving path
constraints with an SMT solver. At each path endpoint, the engine checks
vulnerability conditions.

## Architecture

### Phase 1 — IR Lifting (`ir.rs`)
Translates Soroban contract source (Rust via `syn` or WASM) into a
three-address-code intermediate representation with explicit control flow
graphs (CFG). Each IR instruction preserves source metadata.

Key types: `IRInstruction`, `BasicBlock`, `ControlFlowGraph`, `IRProgram`, `IRLifter`

### Phase 2 — Symbolic State Model (`symbolic_state.rs`)
`SymbolicState` contains:
- Symbolic Soroban ledger (storage entries, balances, trustlines)
- Symbolic call stack with per-frame local variables
- Path constraint set (`Vec<PathConstraint>`)
- Every storage read produces a fresh symbolic variable; every write updates the state map

### Phase 3 — Stellar Semantics (`stellar_semantics.rs`)
Models 15+ Stellar-specific operations as symbolic constraints:
- `require_auth` → caller address constraint
- `put/get/del_ledger_entry` → storage read/write
- `invoke_contract` → external call (reentrancy vector)
- `account_merge` → balance constraints
- `trustline_authorize/deauthorize` → trustline flag constraints
- `get_ledger_sequence` → monotonic constraint
- `verify_ed25519/secp256k1` → symbolic signature result

### Phase 4 — Constraint Solver (`solver.rs`)
Integrates with an SMT solver for checking path constraint satisfiability.
Uses incremental solving (push/pop) for efficient sibling path exploration.
Supports bit-vectors (128-bit for i128), arrays (storage maps), and
uninterpreted functions (unknown host calls).

### Phase 5 — Vulnerability Checkers (`checkers.rs`)
Refactors vulnerability detectors into path-condition checkers:
- `ReentrancyChecker` — checks-effects-interactions violations
- `IntegerOverflowChecker` — path-dependent i128 overflow
- `AccessControlChecker` — missing require_auth on write paths

Each checker: `check(state: &SymbolicState, path: &[PathConstraint]) -> Option<VulnerabilityReport>`

### Phase 6 — Path Exploration (`engine.rs`)
Search strategies: BFS, DFS, Coverage-guided (best-first using unvisited CFG edges).
Configurable `max_depth` (default 1000) and `max_paths` (default 10,000).

### Phase 7 — Concolic Mode
Hybrid mode where concrete fuzzer inputs guide symbolic exploration.
One branch point is flipped symbolically to explore neighboring paths.

## CLI Usage

```bash
# Run symbolic execution as complement to concrete scanner
stellar-scanner scan --symbolic [--max-depth N] [--max-paths M] [--timeout T]

# Results from both are merged; symbolic-only findings are tagged
# detection_method: "symbolic" in the report.
```

## Known Limitations

- Loop unrolling: loops are bounded by `max_depth`; loop summarization
  is planned for side-effect-free loops with counters
- Unmodeled host functions: unknown host calls return fresh symbolic
  variables (safe over-approximation)
- Path explosion: mitigated by `max_paths` and coverage-guided search
