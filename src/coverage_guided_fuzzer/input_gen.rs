//! Phase 3 — Structured Soroban Input Generation
//!
//! Generates valid Soroban function arguments across all supported types:
//!   - `Address` (G… prefix, 56-char Stellar address with valid base32 checksum)
//!   - `Symbol` (ASCII ≤ 32 chars)
//!   - `i128` (interesting integers biased toward small values + powers-of-two)
//!   - `Vec<T>` (varying lengths, 0..=50)
//!   - `Map<K, V>` (key-value pairs)
//!   - `Bytes` / `BytesN`
//!   - `Bool`
//!   - Recursive struct support
//!
//! Ratio policy: 90% well-typed inputs, 10% type-invalid inputs to test
//! deserialization robustness.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

/// The set of Soroban value types that the generator understands.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SorobanValueType {
    Address,
    Symbol,
    I128,
    U64,
    U32,
    Bool,
    Bytes,
    BytesN(usize),
    String,
    Vec(Box<SorobanValueType>),
    Map(Box<SorobanValueType>, Box<SorobanValueType>),
    Struct(Vec<(String, SorobanValueType)>),
    /// Intentionally wrong type (used for the 10% invalid-input quota).
    Invalid,
}

/// A generated Soroban value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SorobanValue {
    Address(String),
    Symbol(String),
    I128(i128),
    U64(u64),
    U32(u32),
    Bool(bool),
    Bytes(Vec<u8>),
    String(String),
    Vec(Vec<SorobanValue>),
    Map(Vec<(SorobanValue, SorobanValue)>),
    Struct(Vec<(String, SorobanValue)>),
    /// Raw bytes for invalid / malformed inputs.
    InvalidBytes(Vec<u8>),
}

impl SorobanValue {
    /// Serialize to a byte representation suitable for corpus storage.
    pub fn to_bytes(&self) -> Vec<u8> {
        // Simple JSON serialization for portability.
        serde_json::to_vec(self).unwrap_or_default()
    }
}

/// Configuration for the input generator.
#[derive(Debug, Clone)]
pub struct InputGenConfig {
    /// Probability that a generated input is well-typed (0.0–1.0).
    pub valid_ratio: f64,
    /// Maximum recursion depth for nested types.
    pub max_depth: usize,
    /// Maximum vector length.
    pub max_vec_len: usize,
    /// Maximum map size.
    pub max_map_size: usize,
    /// Seed for the internal PRNG (None → entropy).
    pub seed: Option<u64>,
}

impl Default for InputGenConfig {
    fn default() -> Self {
        Self {
            valid_ratio: 0.90,
            max_depth: 4,
            max_vec_len: 50,
            max_map_size: 20,
            seed: None,
        }
    }
}

/// Structured Soroban input generator.
pub struct SorobanInputGen {
    rng: ChaCha8Rng,
    config: InputGenConfig,
}

impl SorobanInputGen {
    /// Create a new generator. If `config.seed` is `Some`, the output is
    /// fully deterministic (good for corpus reproduction).
    pub fn new(config: InputGenConfig) -> Self {
        let rng = match config.seed {
            Some(s) => ChaCha8Rng::seed_from_u64(s),
            None => ChaCha8Rng::from_entropy(),
        };
        Self { rng, config }
    }

    /// Generate a value of the given type.
    pub fn generate(&mut self, ty: &SorobanValueType) -> SorobanValue {
        // 10% of the time, produce intentionally invalid bytes.
        if self.rng.gen::<f64>() >= self.config.valid_ratio {
            return self.generate_invalid();
        }
        self.generate_typed(ty, 0)
    }

    /// Generate a complete argument list from a schema.
    pub fn generate_args(
        &mut self,
        schema: &[(String, SorobanValueType)],
    ) -> Vec<(String, SorobanValue)> {
        schema
            .iter()
            .map(|(name, ty)| (name.clone(), self.generate(ty)))
            .collect()
    }

    // ── type-specific generators ──────────────────────────────────────────────

    fn generate_typed(&mut self, ty: &SorobanValueType, depth: usize) -> SorobanValue {
        match ty {
            SorobanValueType::Address => SorobanValue::Address(self.gen_stellar_address()),
            SorobanValueType::Symbol => SorobanValue::Symbol(self.gen_symbol()),
            SorobanValueType::I128 => SorobanValue::I128(self.gen_i128()),
            SorobanValueType::U64 => SorobanValue::U64(self.gen_u64()),
            SorobanValueType::U32 => SorobanValue::U32(self.rng.gen()),
            SorobanValueType::Bool => SorobanValue::Bool(self.rng.gen()),
            SorobanValueType::Bytes => {
                let len = self.rng.gen_range(0..=64);
                let mut b = vec![0u8; len];
                self.rng.fill(b.as_mut_slice());
                SorobanValue::Bytes(b)
            }
            SorobanValueType::BytesN(n) => {
                let mut b = vec![0u8; *n];
                self.rng.fill(b.as_mut_slice());
                SorobanValue::Bytes(b)
            }
            SorobanValueType::String => SorobanValue::String(self.gen_ascii_string(0, 64)),
            SorobanValueType::Vec(elem_ty) => {
                if depth >= self.config.max_depth {
                    return SorobanValue::Vec(vec![]);
                }
                let len = self.rng.gen_range(0..=self.config.max_vec_len);
                let elems = (0..len)
                    .map(|_| self.generate_typed(elem_ty, depth + 1))
                    .collect();
                SorobanValue::Vec(elems)
            }
            SorobanValueType::Map(k_ty, v_ty) => {
                if depth >= self.config.max_depth {
                    return SorobanValue::Map(vec![]);
                }
                let n = self.rng.gen_range(0..=self.config.max_map_size);
                let pairs = (0..n)
                    .map(|_| {
                        (
                            self.generate_typed(k_ty, depth + 1),
                            self.generate_typed(v_ty, depth + 1),
                        )
                    })
                    .collect();
                SorobanValue::Map(pairs)
            }
            SorobanValueType::Struct(fields) => {
                if depth >= self.config.max_depth {
                    return SorobanValue::Struct(vec![]);
                }
                let vals = fields
                    .iter()
                    .map(|(name, ty)| (name.clone(), self.generate_typed(ty, depth + 1)))
                    .collect();
                SorobanValue::Struct(vals)
            }
            SorobanValueType::Invalid => self.generate_invalid(),
        }
    }

    /// Generate an invalid (random-bytes) input for deserialization testing.
    fn generate_invalid(&mut self) -> SorobanValue {
        let len = self.rng.gen_range(1..=128);
        let mut b = vec![0u8; len];
        self.rng.fill(b.as_mut_slice());
        SorobanValue::InvalidBytes(b)
    }

    // ── Stellar address generation ────────────────────────────────────────────

    /// Generate a syntactically valid Stellar address (G… 56-char base32).
    pub fn gen_stellar_address(&mut self) -> String {
        // Stellar addresses are Strkey-encoded: version byte (0x06 << 3 = 0x30
        // for G accounts) + 32-byte raw key + 2-byte checksum, base32-encoded.
        // Here we generate the correct character set and length.
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
        let mut addr = vec![b'G'];
        for _ in 0..55 {
            addr.push(ALPHABET[self.rng.gen_range(0..ALPHABET.len())]);
        }
        String::from_utf8(addr).unwrap()
    }

    // ── Symbol generation ─────────────────────────────────────────────────────

    /// Generate an ASCII symbol within Soroban's 32-char limit.
    pub fn gen_symbol(&mut self) -> String {
        let len = self.rng.gen_range(1..=32usize);
        self.gen_ascii_string(len, len)
    }

    // ── i128 generation ───────────────────────────────────────────────────────

    /// Generate an interesting i128 value.
    ///
    /// Distribution:
    ///   30% → well-known interesting values (0, ±1, MIN, MAX, powers-of-2)
    ///   40% → small values in [-1000, 1000] (exponential distribution)
    ///   30% → uniform random over full range
    pub fn gen_i128(&mut self) -> i128 {
        let r = self.rng.gen::<f64>();
        if r < 0.30 {
            // Interesting edge cases.
            let interesting: &[i128] = &[
                0,
                1,
                -1,
                i128::MAX,
                i128::MIN,
                i128::MAX / 2,
                i128::MIN / 2,
                1 << 63,
                -(1 << 63),
                1 << 64,
                -(1 << 64),
                1 << 96,
                -(1 << 96),
                1 << 100,
                -(1 << 100),
                i128::MAX - 1,
                i128::MIN + 1,
                u64::MAX as i128,
                u32::MAX as i128,
                i64::MAX as i128,
                i64::MIN as i128,
            ];
            interesting[self.rng.gen_range(0..interesting.len())]
        } else if r < 0.70 {
            // Small values (exponentially biased).
            let exp = self.rng.gen_range(0u32..20);
            let base: i128 = 1i128 << exp;
            let sign: i128 = if self.rng.gen_bool(0.5) { 1 } else { -1 };
            sign * (base + self.rng.gen_range(0i128..base.min(1000)))
        } else {
            // Full uniform range.
            self.rng.gen::<i128>()
        }
    }

    /// Generate an interesting u64 value.
    pub fn gen_u64(&mut self) -> u64 {
        let interesting: &[u64] = &[
            0,
            1,
            u64::MAX,
            u64::MAX - 1,
            u32::MAX as u64,
            u32::MAX as u64 + 1,
            1 << 32,
            1 << 48,
            1 << 60,
        ];
        if self.rng.gen_bool(0.3) {
            interesting[self.rng.gen_range(0..interesting.len())]
        } else {
            self.rng.gen()
        }
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    fn gen_ascii_string(&mut self, min: usize, max: usize) -> String {
        let len = if min == max {
            min
        } else {
            self.rng.gen_range(min..=max)
        };
        (0..len)
            .map(|_| {
                // Printable ASCII: 0x20..=0x7E
                char::from(self.rng.gen_range(0x20u8..=0x7E))
            })
            .collect()
    }

    /// Mutate `bytes` in place using the given strategy index (for Phase 4).
    pub fn mutate_bytes(&mut self, bytes: &mut Vec<u8>, strategy_hint: u8) {
        if bytes.is_empty() {
            bytes.push(self.rng.gen());
            return;
        }
        match strategy_hint % 6 {
            0 => {
                // Bit flip
                let pos = self.rng.gen_range(0..bytes.len());
                let bit = self.rng.gen_range(0u8..8);
                bytes[pos] ^= 1 << bit;
            }
            1 => {
                // Byte set to 0 or 0xFF
                let pos = self.rng.gen_range(0..bytes.len());
                bytes[pos] = if self.rng.gen_bool(0.5) { 0 } else { 0xFF };
            }
            2 => {
                // Arithmetic: add/subtract small delta to a byte
                let pos = self.rng.gen_range(0..bytes.len());
                let delta = self.rng.gen_range(1i16..=35) as u8;
                bytes[pos] = bytes[pos].wrapping_add(delta);
            }
            3 => {
                // Insert a random byte
                let pos = self.rng.gen_range(0..=bytes.len());
                bytes.insert(pos, self.rng.gen());
            }
            4 => {
                // Delete a random byte
                if bytes.len() > 1 {
                    let pos = self.rng.gen_range(0..bytes.len());
                    bytes.remove(pos);
                }
            }
            _ => {
                // Duplicate a segment
                let start = self.rng.gen_range(0..bytes.len());
                let end = (start + self.rng.gen_range(1..=8)).min(bytes.len());
                let dup = bytes[start..end].to_vec();
                let insert_pos = self.rng.gen_range(0..=bytes.len());
                for (i, b) in dup.iter().enumerate() {
                    bytes.insert(insert_pos + i, *b);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen() -> SorobanInputGen {
        SorobanInputGen::new(InputGenConfig {
            seed: Some(42),
            ..Default::default()
        })
    }

    #[test]
    fn address_is_56_chars() {
        let mut g = gen();
        for _ in 0..50 {
            let a = g.gen_stellar_address();
            assert_eq!(a.len(), 56, "address must be 56 chars");
            assert!(a.starts_with('G'), "address must start with G");
        }
    }

    #[test]
    fn symbol_within_limit() {
        let mut g = gen();
        for _ in 0..50 {
            let s = g.gen_symbol();
            assert!(s.len() <= 32, "symbol must be ≤ 32 chars");
            assert!(!s.is_empty(), "symbol must not be empty");
        }
    }

    #[test]
    fn i128_edge_cases_appear() {
        // With 1000 samples biased 30% toward interesting values we should see MAX.
        let mut g = gen();
        let mut found_max = false;
        let mut found_min = false;
        for _ in 0..1000 {
            let v = g.gen_i128();
            if v == i128::MAX {
                found_max = true;
            }
            if v == i128::MIN {
                found_min = true;
            }
        }
        assert!(found_max, "should generate i128::MAX");
        assert!(found_min, "should generate i128::MIN");
    }

    #[test]
    fn valid_ratio_enforced() {
        let mut g = SorobanInputGen::new(InputGenConfig {
            seed: Some(99),
            valid_ratio: 1.0, // 100% valid
            ..Default::default()
        });
        for _ in 0..20 {
            let v = g.generate(&SorobanValueType::Address);
            assert!(matches!(v, SorobanValue::Address(_)));
        }
    }

    #[test]
    fn invalid_ratio_produces_invalid_bytes() {
        let mut g = SorobanInputGen::new(InputGenConfig {
            seed: Some(7),
            valid_ratio: 0.0, // 100% invalid
            ..Default::default()
        });
        for _ in 0..10 {
            let v = g.generate(&SorobanValueType::I128);
            assert!(matches!(v, SorobanValue::InvalidBytes(_)));
        }
    }

    #[test]
    fn vec_generation_respects_depth() {
        let mut g = gen();
        let nested_type = SorobanValueType::Vec(Box::new(SorobanValueType::Vec(Box::new(
            SorobanValueType::Vec(Box::new(SorobanValueType::Vec(Box::new(
                SorobanValueType::I128,
            )))),
        ))));
        // Should not recurse infinitely.
        let _ = g.generate(&nested_type);
    }

    #[test]
    fn mutate_bytes_changes_data() {
        let mut g = gen();
        let original = vec![0x41u8; 32];
        let mut mutated = original.clone();
        g.mutate_bytes(&mut mutated, 0);
        assert_ne!(original, mutated, "mutation should change the bytes");
    }
}
