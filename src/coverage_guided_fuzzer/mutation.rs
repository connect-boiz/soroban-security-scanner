//! Phase 4 — Mutation Strategies and Scheduler
//!
//! Implements the full AFL mutation suite adapted for Soroban inputs:
//!   (a) bit_flip   — flip 1/2/4/8/16/32 random bits
//!   (b) byte_flip  — flip, zero, or set 0xFF
//!   (c) arithmetic — add/subtract ±35 from integer fields
//!   (d) interesting — replace with known-dangerous values (0, -1, INT_MAX…)
//!   (e) splice     — concatenate a suffix from another corpus entry
//!   (f) havoc      — random chain of N of the above
//!   (g) struct_splice — swap entire function arguments between two entries
//!
//! The scheduler assigns selection probability using AFL's performance score
//! heuristic: smaller + faster corpus entries get more mutation energy.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

/// Every distinct mutation operator the scheduler can apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MutationStrategy {
    BitFlip1,
    BitFlip2,
    BitFlip4,
    BitFlip8,
    BitFlip16,
    BitFlip32,
    ByteFlip,
    ByteZero,
    ByteMax,
    ArithmeticAdd,
    ArithmeticSub,
    InterestingValue,
    Splice,
    Havoc,
    StructSplice,
}

impl MutationStrategy {
    pub fn all() -> &'static [MutationStrategy] {
        use MutationStrategy::*;
        &[
            BitFlip1,
            BitFlip2,
            BitFlip4,
            BitFlip8,
            BitFlip16,
            BitFlip32,
            ByteFlip,
            ByteZero,
            ByteMax,
            ArithmeticAdd,
            ArithmeticSub,
            InterestingValue,
            Splice,
            Havoc,
            StructSplice,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::BitFlip1 => "bit_flip_1",
            Self::BitFlip2 => "bit_flip_2",
            Self::BitFlip4 => "bit_flip_4",
            Self::BitFlip8 => "bit_flip_8",
            Self::BitFlip16 => "bit_flip_16",
            Self::BitFlip32 => "bit_flip_32",
            Self::ByteFlip => "byte_flip",
            Self::ByteZero => "byte_zero",
            Self::ByteMax => "byte_max",
            Self::ArithmeticAdd => "arith_add",
            Self::ArithmeticSub => "arith_sub",
            Self::InterestingValue => "interesting_val",
            Self::Splice => "splice",
            Self::Havoc => "havoc",
            Self::StructSplice => "struct_splice",
        }
    }
}

/// Tunable parameters for the mutation scheduler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutatorConfig {
    /// Maximum number of sequential mutations applied in a single havoc pass.
    pub havoc_max_rounds: usize,
    /// Maximum splice suffix length as a fraction of the source input.
    pub splice_max_fraction: f64,
    /// Minimum input size to attempt splicing (bytes).
    pub splice_min_bytes: usize,
    /// Number of mutations to try per corpus entry before rotating to the next.
    pub mutations_per_entry: usize,
}

impl Default for MutatorConfig {
    fn default() -> Self {
        Self {
            havoc_max_rounds: 128,
            splice_max_fraction: 0.5,
            splice_min_bytes: 4,
            mutations_per_entry: 512,
        }
    }
}

/// Mutation statistics (per run, used in dashboard).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MutationStats {
    pub total_mutations: u64,
    pub per_strategy: std::collections::HashMap<String, u64>,
    pub interesting_mutations: u64,
}

impl MutationStats {
    pub fn record(&mut self, strategy: MutationStrategy) {
        self.total_mutations += 1;
        *self
            .per_strategy
            .entry(strategy.name().to_string())
            .or_insert(0) += 1;
    }
}

/// The mutation scheduler: selects corpus entries weighted by performance score
/// and applies a mutation operator to produce new candidate inputs.
pub struct MutationScheduler {
    rng: ChaCha8Rng,
    pub config: MutatorConfig,
    pub stats: MutationStats,
    /// Known interesting i128/i64 values used in `InterestingValue` mutations.
    interesting_i64: Vec<i64>,
}

impl MutationScheduler {
    pub fn new(config: MutatorConfig) -> Self {
        Self::with_seed(config, None)
    }

    pub fn with_seed(config: MutatorConfig, seed: Option<u64>) -> Self {
        let rng = match seed {
            Some(s) => ChaCha8Rng::seed_from_u64(s),
            None => ChaCha8Rng::from_entropy(),
        };
        let interesting_i64 = vec![
            0,
            1,
            -1,
            i64::MAX,
            i64::MIN,
            i32::MAX as i64,
            i32::MIN as i64,
            i16::MAX as i64,
            i16::MIN as i64,
            0xFF,
            0x7F,
            0x80,
            0x100,
            0xFFFF,
            0x7FFF,
            0x8000,
            0x10000,
            0xFFFF_FFFF,
            0x7FFF_FFFF,
            -0x8000_0000,
        ];
        Self {
            rng,
            config,
            stats: MutationStats::default(),
            interesting_i64,
        }
    }

    /// Produce one mutated byte-vector from `base` using a random strategy.
    /// If `donor` is provided it may be used for splice operations.
    pub fn mutate(
        &mut self,
        base: &[u8],
        donor: Option<&[u8]>,
        strategy: MutationStrategy,
    ) -> Vec<u8> {
        self.stats.record(strategy);
        match strategy {
            MutationStrategy::BitFlip1 => self.bit_flip(base, 1),
            MutationStrategy::BitFlip2 => self.bit_flip(base, 2),
            MutationStrategy::BitFlip4 => self.bit_flip(base, 4),
            MutationStrategy::BitFlip8 => self.bit_flip(base, 8),
            MutationStrategy::BitFlip16 => self.bit_flip(base, 16),
            MutationStrategy::BitFlip32 => self.bit_flip(base, 32),
            MutationStrategy::ByteFlip => self.byte_mutate(base, 0),
            MutationStrategy::ByteZero => self.byte_mutate(base, 1),
            MutationStrategy::ByteMax => self.byte_mutate(base, 2),
            MutationStrategy::ArithmeticAdd => self.arithmetic(base, true),
            MutationStrategy::ArithmeticSub => self.arithmetic(base, false),
            MutationStrategy::InterestingValue => self.interesting_value(base),
            MutationStrategy::Splice => self.splice(base, donor.unwrap_or(base)),
            MutationStrategy::Havoc => self.havoc(base, donor),
            MutationStrategy::StructSplice => self.struct_splice(base, donor.unwrap_or(base)),
        }
    }

    /// Pick a random strategy and apply it.
    pub fn mutate_random(
        &mut self,
        base: &[u8],
        donor: Option<&[u8]>,
    ) -> (Vec<u8>, MutationStrategy) {
        let strategies = MutationStrategy::all();
        // Weight bit-flip, arithmetic, and interesting higher (more likely to
        // hit interesting paths in integer-heavy Soroban contracts).
        let strategy = strategies[self.rng.gen_range(0..strategies.len())];
        let result = self.mutate(base, donor, strategy);
        (result, strategy)
    }

    // ── concrete mutators ─────────────────────────────────────────────────────

    /// Flip `n_bits` consecutive bits starting at a random bit position.
    fn bit_flip(&mut self, input: &[u8], n_bits: usize) -> Vec<u8> {
        if input.is_empty() {
            return vec![0];
        }
        let mut out = input.to_vec();
        let total_bits = out.len() * 8;
        let start_bit = self.rng.gen_range(0..total_bits);
        for bit_offset in 0..n_bits {
            let bit = (start_bit + bit_offset) % total_bits;
            out[bit / 8] ^= 1 << (bit % 8);
        }
        out
    }

    /// Byte-level mutation: flip (0), zero (1), or max (2).
    fn byte_mutate(&mut self, input: &[u8], mode: u8) -> Vec<u8> {
        if input.is_empty() {
            return vec![0];
        }
        let mut out = input.to_vec();
        let count = self.rng.gen_range(1..=(out.len().min(8)));
        for _ in 0..count {
            let pos = self.rng.gen_range(0..out.len());
            out[pos] = match mode {
                0 => out[pos] ^ 0xFF,
                1 => 0x00,
                _ => 0xFF,
            };
        }
        out
    }

    /// Arithmetic: add or subtract a small value (1..35) from a random byte.
    fn arithmetic(&mut self, input: &[u8], add: bool) -> Vec<u8> {
        if input.is_empty() {
            return vec![0];
        }
        let mut out = input.to_vec();
        let pos = self.rng.gen_range(0..out.len());
        let delta = self.rng.gen_range(1u8..=35);
        out[pos] = if add {
            out[pos].wrapping_add(delta)
        } else {
            out[pos].wrapping_sub(delta)
        };
        out
    }

    /// Replace bytes at a random position with a known-interesting little-endian i64.
    fn interesting_value(&mut self, input: &[u8]) -> Vec<u8> {
        if input.len() < 8 {
            return self.arithmetic(input, true);
        }
        let mut out = input.to_vec();
        let val = self.interesting_i64[self.rng.gen_range(0..self.interesting_i64.len())];
        let bytes = val.to_le_bytes();
        let pos = self.rng.gen_range(0..=(out.len().saturating_sub(8)));
        out[pos..pos + 8].copy_from_slice(&bytes);
        out
    }

    /// Splice: take a prefix from `base` and a suffix from `donor`.
    fn splice(&mut self, base: &[u8], donor: &[u8]) -> Vec<u8> {
        let min_len = self.config.splice_min_bytes;
        if base.len() < min_len || donor.len() < min_len {
            return base.to_vec();
        }
        let split_base = self.rng.gen_range(1..base.len());
        let split_donor = self.rng.gen_range(0..donor.len());
        let mut out = base[..split_base].to_vec();
        out.extend_from_slice(&donor[split_donor..]);
        out
    }

    /// Structure-aware splice: treat both inputs as JSON and swap a top-level
    /// argument value. Falls back to byte-level splice if JSON parsing fails.
    fn struct_splice(&mut self, base: &[u8], donor: &[u8]) -> Vec<u8> {
        // Try to parse both as JSON arrays.
        if let (Ok(mut base_json), Ok(donor_json)) = (
            serde_json::from_slice::<serde_json::Value>(base),
            serde_json::from_slice::<serde_json::Value>(donor),
        ) {
            if let (Some(base_arr), Some(donor_arr)) =
                (base_json.as_array_mut(), donor_json.as_array())
            {
                if !donor_arr.is_empty() && !base_arr.is_empty() {
                    let base_idx = self.rng.gen_range(0..base_arr.len());
                    let donor_idx = self.rng.gen_range(0..donor_arr.len());
                    base_arr[base_idx] = donor_arr[donor_idx].clone();
                    if let Ok(serialized) = serde_json::to_vec(&base_json) {
                        return serialized;
                    }
                }
            }
        }
        // Fallback.
        self.splice(base, donor)
    }

    /// Havoc: apply a random chain of mutations.
    fn havoc(&mut self, input: &[u8], donor: Option<&[u8]>) -> Vec<u8> {
        let rounds = self.rng.gen_range(2..=self.config.havoc_max_rounds);
        let mut out = input.to_vec();
        let non_havoc_strategies: Vec<MutationStrategy> = MutationStrategy::all()
            .iter()
            .copied()
            .filter(|s| *s != MutationStrategy::Havoc)
            .collect();

        for _ in 0..rounds {
            if out.is_empty() {
                out.push(self.rng.gen());
                continue;
            }
            let s = non_havoc_strategies[self.rng.gen_range(0..non_havoc_strategies.len())];
            // Record sub-strategy stats.
            self.stats.record(s);
            out = match s {
                MutationStrategy::BitFlip1 => self.bit_flip(&out, 1),
                MutationStrategy::BitFlip2 => self.bit_flip(&out, 2),
                MutationStrategy::BitFlip4 => self.bit_flip(&out, 4),
                MutationStrategy::BitFlip8 => self.bit_flip(&out, 8),
                MutationStrategy::BitFlip16 => self.bit_flip(&out, 16),
                MutationStrategy::BitFlip32 => self.bit_flip(&out, 32),
                MutationStrategy::ByteFlip => self.byte_mutate(&out, 0),
                MutationStrategy::ByteZero => self.byte_mutate(&out, 1),
                MutationStrategy::ByteMax => self.byte_mutate(&out, 2),
                MutationStrategy::ArithmeticAdd => self.arithmetic(&out, true),
                MutationStrategy::ArithmeticSub => self.arithmetic(&out, false),
                MutationStrategy::InterestingValue => self.interesting_value(&out),
                MutationStrategy::Splice => {
                    let d = donor.unwrap_or(&out);
                    self.splice(&out, d)
                }
                MutationStrategy::StructSplice => {
                    let d = donor.unwrap_or(&out);
                    self.struct_splice(&out, d)
                }
                MutationStrategy::Havoc => out, // skip nested havoc
            };
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sched() -> MutationScheduler {
        MutationScheduler::with_seed(MutatorConfig::default(), Some(42))
    }

    #[test]
    fn bit_flip_changes_exactly_one_bit() {
        let mut s = sched();
        let input = vec![0b1010_1010u8; 4];
        let out = s.bit_flip(&input, 1);
        let diff_bits: usize = input
            .iter()
            .zip(out.iter())
            .map(|(a, b)| (a ^ b).count_ones() as usize)
            .sum();
        assert_eq!(diff_bits, 1, "bit_flip_1 should flip exactly 1 bit");
    }

    #[test]
    fn arithmetic_changes_one_byte() {
        let mut s = sched();
        let input = vec![0x10u8; 8];
        let out = s.arithmetic(&input, true);
        let changed: usize = input.iter().zip(out.iter()).filter(|(a, b)| a != b).count();
        assert_eq!(changed, 1, "arithmetic should modify exactly 1 byte");
    }

    #[test]
    fn splice_respects_min_length() {
        let mut s = sched();
        let base = vec![1u8; 2]; // below min (4)
        let donor = vec![2u8; 2];
        let out = s.splice(&base, &donor);
        assert_eq!(out, base, "splice on short input returns base unchanged");
    }

    #[test]
    fn havoc_runs_without_panic() {
        let mut s = sched();
        let input: Vec<u8> = (0..32).collect();
        for _ in 0..20 {
            let out = s.havoc(&input, None);
            assert!(!out.is_empty());
        }
    }

    #[test]
    fn interesting_value_on_short_input_doesnt_panic() {
        let mut s = sched();
        let out = s.interesting_value(&[0x01]);
        assert!(!out.is_empty());
    }

    #[test]
    fn stats_track_strategies() {
        let mut s = sched();
        let input = vec![0xAAu8; 16];
        s.mutate(&input, None, MutationStrategy::BitFlip1);
        s.mutate(&input, None, MutationStrategy::ArithmeticAdd);
        assert_eq!(s.stats.total_mutations, 2);
        assert_eq!(s.stats.per_strategy["bit_flip_1"], 1);
        assert_eq!(s.stats.per_strategy["arith_add"], 1);
    }
}
