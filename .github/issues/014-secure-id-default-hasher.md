# Issue 14: [Secure ID Generation] DefaultHasher Used Instead of Cryptographic Hash, Breaking Security Guarantees

## Description

The `SecureIdGenerator` in `src/secure_id_generation.rs` is designed to generate cryptographically secure IDs for bounties, sessions, transactions, and nonces. However, the actual hashing implementation uses Rust's `std::collections::hash_map::DefaultHasher` in multiple critical methods (`hash_string`, `hash_entropy_to_id`, `hash_entropy_to_bytes`, `get_random_entropy`). `DefaultHasher` is explicitly documented as not cryptographically secure — it uses SipHash-1-3 with a fixed key (not the keyed variant), making it vulnerable to hash collision attacks and preimage attacks. An attacker who observes a generated ID can determine the internal hasher state, predict future IDs, and forge session tokens or nonces. This defeats the entire purpose of the secure ID generation module, which was created specifically to address issue #114 (predictable ledger sequence IDs).

## Acceptance Criteria

- [ ] Replace all `DefaultHasher` usages with a proper cryptographic hash function from the `ring` or `sha2` crate (already in `Cargo.toml` dependencies)
- [ ] Use `ring::digest::SHA256` for ID generation and `ring::hkdf` for key derivation where applicable
- [ ] Use `ring::rand::SecureRandom` for all random number generation instead of the PRNG approach
- [ ] Remove the `get_random_entropy` function that uses `DefaultHasher` and replace with `ring::rand::SystemRandom`
- [ ] Conduct a security review of the rewritten module to confirm no cryptographic shortcuts remain
- [ ] Write cryptographic tests that verify generated IDs are indistinguishable from random (e.g., chi-squared test on bit distribution)

## Additional Context

Key files: `src/secure_id_generation.rs`, `src/secure_id_generation_tests.rs`, `Cargo.toml`.
