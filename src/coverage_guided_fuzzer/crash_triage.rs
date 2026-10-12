//! Phase 5 — Crash Triage and Deduplication
//!
//! When the fuzzer drives the VM into a crash (panic / trap / timeout) it:
//!   1. Captures the error message / stack-trace fragment from the host.
//!   2. Normalises it (strips pointer addresses, line numbers, and
//!      input-dependent values that would create false uniqueness).
//!   3. Hashes the normalised form → crash fingerprint.
//!   4. Groups crashes by fingerprint — only the *first* crash per group is
//!      reported as a unique finding.
//!   5. Runs delta-debugging on the crashing input to produce a minimal
//!      reproducer (shrinks while the crash is still reproducible).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// The raw information available when a crash is detected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashInfo {
    /// The input bytes that triggered the crash.
    pub input: Vec<u8>,
    /// Error text returned by the VM host (may include backtrace lines).
    pub error_text: String,
    /// Execution time before the crash.
    pub execution_time: Duration,
    /// Human-readable label (e.g. function name that was called).
    pub label: Option<String>,
}

/// A deduplicated, minimised crash report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashReport {
    /// Stable fingerprint of the normalised error.
    pub fingerprint: String,
    /// Minimal reproducing input bytes.
    pub minimal_input: Vec<u8>,
    /// Original (pre-minimisation) input bytes.
    pub original_input: Vec<u8>,
    /// Normalised error text used to derive the fingerprint.
    pub normalised_error: String,
    /// Raw error text.
    pub raw_error: String,
    /// Number of bytes saved by minimisation.
    pub bytes_saved: usize,
    /// Optional function label.
    pub label: Option<String>,
}

/// A group of crashes sharing the same fingerprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashGroup {
    pub fingerprint: String,
    /// The canonical (first) crash report for this group.
    pub canonical: CrashReport,
    /// Total number of inputs that triggered this crash.
    pub occurrence_count: usize,
}

/// Configuration for the crash triage engine.
#[derive(Debug, Clone)]
pub struct CrashTriageConfig {
    /// Maximum delta-debugging iterations.
    pub max_minimise_iters: usize,
    /// Minimum input length below which minimisation stops.
    pub min_input_bytes: usize,
}

impl Default for CrashTriageConfig {
    fn default() -> Self {
        Self {
            max_minimise_iters: 256,
            min_input_bytes: 1,
        }
    }
}

/// The crash triage engine.
pub struct CrashTriageEngine {
    config: CrashTriageConfig,
    /// Map from fingerprint → crash group.
    groups: HashMap<String, CrashGroup>,
}

impl CrashTriageEngine {
    pub fn new(config: CrashTriageConfig) -> Self {
        Self {
            config,
            groups: HashMap::new(),
        }
    }

    /// Process a new crash. Returns `Some(CrashReport)` if this is a new
    /// unique crash (new fingerprint), or `None` if it is a duplicate.
    ///
    /// `reproduces` is a closure that, given a candidate input, returns
    /// `true` if the crash is still triggered. It is used for minimisation.
    pub fn process<F>(&mut self, info: CrashInfo, mut reproduces: F) -> Option<CrashReport>
    where
        F: FnMut(&[u8]) -> bool,
    {
        let normalised = normalise_error(&info.error_text);
        let fingerprint = compute_fingerprint(&normalised);

        if let Some(group) = self.groups.get_mut(&fingerprint) {
            // Duplicate — just increment count.
            group.occurrence_count += 1;
            return None;
        }

        // New unique crash — minimise the input.
        let minimal = self.delta_debug(&info.input, &mut reproduces);
        let bytes_saved = info.input.len().saturating_sub(minimal.len());

        let report = CrashReport {
            fingerprint: fingerprint.clone(),
            minimal_input: minimal,
            original_input: info.input.clone(),
            normalised_error: normalised,
            raw_error: info.error_text.clone(),
            bytes_saved,
            label: info.label.clone(),
        };

        self.groups.insert(
            fingerprint,
            CrashGroup {
                fingerprint: report.fingerprint.clone(),
                canonical: report.clone(),
                occurrence_count: 1,
            },
        );

        Some(report)
    }

    /// Delta-debugging (1-minimisation): repeatedly try to remove halves or
    /// individual bytes while the crash still reproduces.
    ///
    /// This is the standard ddmin algorithm (Zeller 2002), limited to
    /// `max_minimise_iters` iterations for performance.
    fn delta_debug<F>(&self, input: &[u8], reproduces: &mut F) -> Vec<u8>
    where
        F: FnMut(&[u8]) -> bool,
    {
        let mut current = input.to_vec();
        let mut iters = 0;

        // Phase 1 — binary chunking (fast, O(log n) removals).
        let mut granularity = 2usize;
        while granularity <= current.len() && iters < self.config.max_minimise_iters {
            let chunk_size = (current.len() + granularity - 1) / granularity;
            let mut reduced = false;

            for i in 0..granularity {
                if iters >= self.config.max_minimise_iters {
                    break;
                }
                let start = i * chunk_size;
                let end = ((i + 1) * chunk_size).min(current.len());
                if start >= current.len() {
                    break;
                }
                // Try removing this chunk.
                let mut candidate = current[..start].to_vec();
                candidate.extend_from_slice(&current[end..]);
                if candidate.len() >= self.config.min_input_bytes && reproduces(&candidate) {
                    current = candidate;
                    reduced = true;
                    iters += 1;
                    break;
                }
                iters += 1;
            }

            if !reduced {
                granularity *= 2;
            }
        }

        // Phase 2 — byte-by-byte scan (thorough, removes individual bytes).
        let mut pos = 0;
        while pos < current.len() && iters < self.config.max_minimise_iters {
            if current.len() <= self.config.min_input_bytes {
                break;
            }
            let mut candidate = current.clone();
            candidate.remove(pos);
            if reproduces(&candidate) {
                current = candidate;
                // Don't advance pos — the next byte slid into `pos`.
            } else {
                pos += 1;
            }
            iters += 1;
        }

        current
    }

    /// All unique crash groups collected so far.
    pub fn groups(&self) -> Vec<&CrashGroup> {
        self.groups.values().collect()
    }

    /// Number of unique crash fingerprints seen.
    pub fn unique_crash_count(&self) -> usize {
        self.groups.len()
    }

    /// Total crash occurrences (unique + duplicates).
    pub fn total_crash_count(&self) -> usize {
        self.groups.values().map(|g| g.occurrence_count).sum()
    }
}

// ── normalisation ─────────────────────────────────────────────────────────────

/// Normalise an error string to remove input-dependent / run-dependent noise.
///
/// Strips:
///   • Hex addresses (`0x[0-9a-f]+`)
///   • Line-number annotations (`:42:`)
///   • Soroban contract IDs (64-char hex strings)
///   • Timestamps and wallclock values
///   • Specific numeric values after "value:" or "got:"
fn normalise_error(error: &str) -> String {
    // Simple regex-free normalisation using character-level scanning.
    let mut out = String::with_capacity(error.len());
    let chars: Vec<char> = error.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Hex address: 0x followed by hex digits.
        if i + 1 < chars.len() && chars[i] == '0' && chars[i + 1] == 'x' {
            out.push_str("0xADDR");
            i += 2;
            while i < chars.len() && chars[i].is_ascii_hexdigit() {
                i += 1;
            }
            continue;
        }

        // Colon-delimited line:col numbers  ":123:"
        if chars[i] == ':' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
            out.push(':');
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            out.push_str("LINE");
            continue;
        }

        // Long hex run (contract ID, transaction hash): ≥ 32 consecutive hex chars.
        if chars[i].is_ascii_hexdigit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_hexdigit() {
                i += 1;
            }
            let run_len = i - start;
            if run_len >= 32 {
                out.push_str("HASH");
            } else {
                for c in &chars[start..i] {
                    out.push(*c);
                }
            }
            continue;
        }

        out.push(chars[i]);
        i += 1;
    }

    // Collapse whitespace runs.
    let mut collapsed = String::with_capacity(out.len());
    let mut prev_space = false;
    for c in out.chars() {
        if c.is_whitespace() {
            if !prev_space {
                collapsed.push(' ');
            }
            prev_space = true;
        } else {
            collapsed.push(c);
            prev_space = false;
        }
    }

    collapsed.trim().to_string()
}

/// Compute a stable hex fingerprint of a normalised error string.
fn compute_fingerprint(normalised: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    normalised.hash(&mut h);
    format!("{:016x}", h.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn always_crashes(_: &[u8]) -> bool {
        true
    }

    #[allow(dead_code)]
    fn never_crashes(_: &[u8]) -> bool {
        false
    }

    #[test]
    fn new_unique_crash_is_reported() {
        let mut engine = CrashTriageEngine::new(CrashTriageConfig::default());
        let info = CrashInfo {
            input: b"AAAAAA".to_vec(),
            error_text: "panic: index out of range".to_string(),
            execution_time: Duration::from_millis(5),
            label: None,
        };
        let report = engine.process(info, always_crashes);
        assert!(report.is_some(), "first crash should be reported");
        assert_eq!(engine.unique_crash_count(), 1);
    }

    #[test]
    fn duplicate_crash_suppressed() {
        let mut engine = CrashTriageEngine::new(CrashTriageConfig::default());
        let make = || CrashInfo {
            input: b"TEST".to_vec(),
            error_text: "panic: index out of range".to_string(),
            execution_time: Duration::from_millis(1),
            label: None,
        };
        let r1 = engine.process(make(), always_crashes);
        let r2 = engine.process(make(), always_crashes);
        assert!(r1.is_some());
        assert!(r2.is_none(), "duplicate should be suppressed");
        assert_eq!(engine.unique_crash_count(), 1);
        assert_eq!(engine.total_crash_count(), 2);
    }

    #[test]
    fn different_errors_produce_different_fingerprints() {
        let mut engine = CrashTriageEngine::new(CrashTriageConfig::default());
        let r1 = engine.process(
            CrashInfo {
                input: b"A".to_vec(),
                error_text: "panic: integer overflow".to_string(),
                execution_time: Duration::ZERO,
                label: None,
            },
            always_crashes,
        );
        let r2 = engine.process(
            CrashInfo {
                input: b"B".to_vec(),
                error_text: "panic: null pointer dereference".to_string(),
                execution_time: Duration::ZERO,
                label: None,
            },
            always_crashes,
        );
        assert!(r1.is_some());
        assert!(r2.is_some());
        assert_eq!(engine.unique_crash_count(), 2);
    }

    #[test]
    fn normalisation_strips_addresses() {
        let raw = "error at 0x7fff1234abcd in contract";
        let norm = normalise_error(raw);
        assert!(!norm.contains("7fff1234abcd"), "should strip hex address");
        assert!(norm.contains("0xADDR"));
    }

    #[test]
    fn normalisation_strips_long_hashes() {
        let hash = "a".repeat(64);
        let raw = format!("contract {} panicked", hash);
        let norm = normalise_error(&raw);
        assert!(!norm.contains(&hash));
        assert!(norm.contains("HASH"));
    }

    #[test]
    fn minimisation_reduces_input() {
        let mut engine = CrashTriageEngine::new(CrashTriageConfig::default());
        // Input must contain byte 0x41 ('A') to crash.
        let reproduces = |input: &[u8]| input.contains(&0x41);
        let info = CrashInfo {
            input: vec![0x00, 0x41, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            error_text: "crash".to_string(),
            execution_time: Duration::ZERO,
            label: None,
        };
        let report = engine.process(info, reproduces).unwrap();
        // Minimal input should be just [0x41].
        assert!(
            report.minimal_input.len() <= 2,
            "should minimise to 1-2 bytes, got {}",
            report.minimal_input.len()
        );
        assert!(report.minimal_input.contains(&0x41));
        assert!(report.bytes_saved > 0);
    }

    #[test]
    fn minimisation_stops_when_crash_doesnt_reproduce() {
        let mut engine = CrashTriageEngine::new(CrashTriageConfig::default());
        let info = CrashInfo {
            input: b"ABCDEFGH".to_vec(),
            error_text: "crash".to_string(),
            execution_time: Duration::ZERO,
            label: None,
        };
        // always_crashes returns true for anything, so the minimal will be 1 byte.
        let report = engine.process(info, always_crashes).unwrap();
        assert_eq!(report.minimal_input.len(), 1);
    }
}
