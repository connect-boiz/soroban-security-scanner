# SMT-Based Formal Verification of Smart Contract Invariants Using Z3

## Overview

The `formal_verification` engine provides sound, mathematical invariant proofs for Stellar Soroban smart contracts. Unlike heuristic regex scans or lint rules, this engine translates smart contract logic, state transitions, and arithmetic expressions into first-order SMT-LIB2 formulas evaluated by the Z3 theorem prover.

An invariant $I$ is formally proven by asserting its negation $\neg I$ alongside the transition relation and environment axioms. 
- **UNSAT (Verified ✓):** No reachable execution path can violate the invariant. The invariant is proven as a mathematical theorem across all valid inputs.
- **SAT (Violated ✗):** Z3 derives a concrete counterexample (variable bindings and simulated ledger execution trace) demonstrating how the invariant can be violated.
- **UNKNOWN:** Invariant verification was inconclusive (e.g. timeout or unbounded loops requiring manual inductive annotations).

---

## Architecture & Verification Phases

```
┌─────────────────────────────────┐
│ Rust Contract / WASM Bytecode   │
│ Invariant Specs (Doc Comments)  │
└────────────────┬────────────────┘
                 │
                 ▼
┌─────────────────────────────────┐
│ AST & Invariant Parser          │
│ `spec_parser.rs`                │
└────────────────┬────────────────┘
                 │
                 ▼
┌─────────────────────────────────┐
│ Host Environment Axioms         │
│ (Storage, Auth, Timestamps)     │
│ `host_environment.rs`           │
└────────────────┬────────────────┘
                 │
                 ▼
┌─────────────────────────────────┐
│ SMT-LIB2 Bitvector Translator   │
│ `smt_lib.rs` (QF_ABV Logic)     │
└────────────────┬────────────────┘
                 │
                 ▼
┌─────────────────────────────────┐
│ Z3 Theorem Prover Engine        │
│ `z3_solver.rs`                  │
└────────┬───────────────┬────────┘
         │               │
      [UNSAT]          [SAT]
         │               │
         ▼               ▼
   Verified (✓)    Counterexample Synthesizer
                   `counterexample.rs`
```

---

## 1. Invariant Specification Language

Invariants are declared via structured doc comments directly above function implementations:

```rust
/// #[invariant: transfer_conservation]
/// #[invariant: old(a) + old(b) == new(a) + new(b)]
pub fn transfer(env: Env, from: Address, to: Address, amount: i128) { ... }
```

### Supported Syntax
- **Arithmetic Operators:** `+`, `-`, `*`, `/`
- **Relational Comparisons:** `==`, `!=`, `<`, `<=`, `>`, `>=`
- **Logical Connectives:** `&&`, `||`, `!`, `==>` (implication)
- **State Temporal Operators:**
  - `old(expr)`: Evaluates expression in contract state prior to function invocation.
  - `new(expr)`: Evaluates expression in state immediately after function invocation.
- **Quantifiers:**
  - `forall <var>: <sort>, <expr>`
  - `exists <var>: <sort>, <expr>`

---

## 2. SMT Encoding Details & Bitvector Arithmetic

Soroban primitive numeric types are encoded directly into fixed-width bitvectors:
- `u32` / `i32` $\to$ `(_ BitVec 32)`
- `u64` / `i64` $\to$ `(_ BitVec 64)`
- `u128` / `i128` $\to$ `(_ BitVec 128)`
- `Address` / `BytesN<32>` $\to$ `(_ BitVec 256)`
- Contract persistent storage $\to$ `(Array (_ BitVec 256) (_ BitVec 1024))`

### Bitvector Operation Mapping
| Soroban WASM Operation | SMT-LIB2 Equivalent |
|---|---|
| `i128.add` | `bvadd` |
| `i128.sub` | `bvsub` |
| `i128.mul` | `bvmul` |
| `i128.div_u` | `bvudiv` |
| `i128.ge_u` | `bvuge` |
| `i128.eq` | `=` |
| `i128.ite` | `ite` |

Overflow flags are modeled as explicit assertions:
```smt2
(assert (not (= (bvadd a b) (concat (_ bv0 1) (bvadd a b)))))
```

---

## 3. Host Environment Theory Axioms

Soroban host functions are formalized as uninterpreted functions and theory axioms:
1. **Storage Operations:**
   - `storage_get(key)`: Function mapping `(_ BitVec 256) -> (_ BitVec 1024)`.
   - `storage_set(key, val)`: SMT array theory store `(store storage_map key val)`.
2. **Authentication Verification:**
   - `require_auth(caller)`: Constrains path condition: `(assert (= caller authenticated_address))`.
3. **Ledger Timestamps:**
   - `ledger().timestamp()`: Fresh symbolic bitvector constrained by `(assert (bvuge timestamp deployment_timestamp))`.

---

## 4. Modular Verification & Function Summaries

To scale verification across large multi-contract protocols:
- Each function is verified in isolation.
- Preconditions, postconditions, and frame conditions (`modifies`) are extracted into a `FunctionSummary`.
- Cross-contract calls replace function bodies with their corresponding summaries, eliminating exponential path explosion.

---

## 5. Loop Handling: Bounded Unrolling & Inductive Invariants

- **Bounded Unrolling:** Loops with constant upper bounds or induction counters are unrolled up to `max_unroll` (default: 20 iterations).
- **Inductive Invariant Annotations:** For unbounded loops, the contract author supplies inductive invariants:
  ```rust
  /// #[invariant: inductive, processed_count <= total_items]
  ```
- If a loop cannot be bounded and lacks an inductive invariant, the engine returns `Unknown` rather than a false positive.

---

## 6. Counterexample Generation & Reproduction

When an invariant fails (Z3 returns `SAT`), the model values are parsed and structured into a reproduction test case:
```json
{
  "contract": "VaultContract",
  "invariant": "balance_non_negative",
  "status": "violated",
  "counterexample": {
    "inputs": {
      "balance": "0",
      "withdraw_amount": "1"
    },
    "reproduction_trace": "Invoke withdraw(withdraw_amount = 1) when balance = 0 causes underflow."
  }
}
```

---

## 7. Benchmark Suite

The engine includes a 15-contract verification benchmark suite (`benchmarks.rs`):
- **5 Provably Correct Invariants:** Token conservation, mint supply increase, admin identity, burn conservation, and multiplication commutativity (all proven UNSAT).
- **5 Known Invariant Violations:** Unchecked underflow, unauthorized state modification, unbacked minting, improper balance tracking, and reentrancy divergence (all successfully derived counterexamples).
- **5 Bounded Loop / Summary Invariants:** Inductive step verification and unrolled loop invariant checks.
