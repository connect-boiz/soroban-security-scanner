//! Parser fuzzing to surface crash-inducing inputs.
//!
//! Deterministically mutates seed inputs and runs them through a candidate
//! parser inside `catch_unwind`, recording any input that panics. Determinism
//! (no RNG) makes discovered crashes reproducible. This is the offline harness
//! used to drive the parser toward the "zero parser crashes" goal.

use serde::{Deserialize, Serialize};
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Aggregated result of a fuzzing run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FuzzReport {
    /// Total inputs executed (seeds + mutations).
    pub total: usize,
    /// Inputs the parser accepted.
    pub ok: usize,
    /// Inputs the parser rejected with an error (handled gracefully).
    pub errors: usize,
    /// Inputs that crashed (panicked) the parser — these are the bugs.
    pub crashes: Vec<Vec<u8>>,
}

impl FuzzReport {
    /// Whether the parser survived every input without crashing.
    pub fn is_crash_free(&self) -> bool {
        self.crashes.is_empty()
    }
}

/// Fuzzes `parser` against each seed plus `mutations_per_seed` deterministic
/// mutations of it. The parser should return `Ok` for accepted input and `Err`
/// for gracefully-rejected input; a panic is recorded as a crash.
pub fn fuzz_parser<F>(seeds: &[&[u8]], mutations_per_seed: usize, parser: F) -> FuzzReport
where
    F: Fn(&[u8]) -> Result<(), String>,
{
    let mut report = FuzzReport::default();
    for seed in seeds {
        // Iteration 0 is the unmodified seed; the rest are mutations.
        for i in 0..=mutations_per_seed {
            let candidate = mutate(seed, i);
            report.total += 1;
            match catch_unwind(AssertUnwindSafe(|| parser(&candidate))) {
                Ok(Ok(())) => report.ok += 1,
                Ok(Err(_)) => report.errors += 1,
                Err(_) => report.crashes.push(candidate),
            }
        }
    }
    report
}

/// Produces a deterministic mutation of `seed` for iteration `i`.
/// `i == 0` returns the seed unchanged so the corpus itself is always tested.
fn mutate(seed: &[u8], i: usize) -> Vec<u8> {
    if i == 0 || seed.is_empty() {
        return seed.to_vec();
    }
    let mut out = seed.to_vec();
    match i % 4 {
        // Flip a bit.
        1 => {
            let pos = (i / 4) % out.len();
            out[pos] ^= 1 << (i % 8);
        }
        // Truncate.
        2 => {
            let keep = (i / 4) % out.len();
            out.truncate(keep);
        }
        // Duplicate the buffer (size growth).
        3 => {
            let copy = out.clone();
            out.extend_from_slice(&copy);
        }
        // Insert a byte.
        _ => {
            let pos = (i / 4) % (out.len() + 1);
            out.insert(pos, (i % 256) as u8);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A robust parser that never panics: it returns Err on non-UTF-8/empty.
    fn safe_parser(input: &[u8]) -> Result<(), String> {
        if input.is_empty() {
            return Err("empty".to_string());
        }
        std::str::from_utf8(input)
            .map(|_| ())
            .map_err(|_| "not utf8".to_string())
    }

    #[test]
    fn robust_parser_is_crash_free() {
        let seeds: &[&[u8]] = &[b"pub fn main() {}", b"contract { }", b"\xff\xfe"];
        let report = fuzz_parser(seeds, 50, safe_parser);
        assert!(report.is_crash_free());
        assert_eq!(report.total, report.ok + report.errors);
        assert!(report.total >= 3 * 51);
    }

    #[test]
    fn crashing_parser_is_detected_and_contained() {
        // A parser with a bug: panics on a specific seed input.
        let buggy = |input: &[u8]| -> Result<(), String> {
            if input == b"CRASH" {
                panic!("boom");
            }
            Ok(())
        };
        // Iteration 0 tests the raw seed b"CRASH" → one contained crash.
        let report = fuzz_parser(&[b"CRASH"], 0, buggy);
        assert!(!report.is_crash_free());
        assert_eq!(report.crashes.len(), 1);
        assert_eq!(report.crashes[0], b"CRASH");
    }

    #[test]
    fn mutation_zero_is_identity() {
        assert_eq!(mutate(b"abc", 0), b"abc");
    }

    #[test]
    fn mutations_are_deterministic() {
        // Same (seed, i) always yields the same bytes → reproducible crashes.
        for i in 0..20 {
            assert_eq!(mutate(b"hello world", i), mutate(b"hello world", i));
        }
    }
}
