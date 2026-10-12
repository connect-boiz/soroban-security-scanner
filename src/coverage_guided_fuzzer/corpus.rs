//! Phase 2 (continued) — Corpus Management
//!
//! Maintains the set of "interesting" inputs (those that discovered new
//! coverage). Implements AFL's performance-score heuristic: smaller inputs
//! that execute faster get higher selection weight.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// A single corpus entry — one fuzzer-generated input that increased coverage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusEntry {
    /// Raw byte representation of the serialized function arguments.
    pub bytes: Vec<u8>,
    /// Wall-time cost of executing this input.
    pub execution_time: Duration,
    /// Number of new edges this input discovered when it was added.
    pub new_edges_discovered: usize,
    /// AFL-style performance score in [0.0, ∞). Higher ⟹ more likely to be
    /// selected for mutation.
    pub performance_score: f64,
    /// Optional human-readable label (e.g. function name).
    pub label: Option<String>,
    /// Ordinal within the corpus (stable, used for splice operations).
    pub id: usize,
    /// Which mutation stage produced this entry ("seed", "bit_flip", "splice", …).
    pub origin: String,
}

impl CorpusEntry {
    /// Create a new corpus entry and compute its initial performance score.
    pub fn new(
        bytes: Vec<u8>,
        execution_time: Duration,
        new_edges_discovered: usize,
        label: Option<String>,
        origin: &str,
        id: usize,
    ) -> Self {
        let score = Self::compute_score(bytes.len(), execution_time, new_edges_discovered);
        Self {
            bytes,
            execution_time,
            new_edges_discovered,
            performance_score: score,
            label,
            id,
            origin: origin.to_string(),
        }
    }

    /// AFL performance heuristic: prefer smaller, faster inputs with more new edges.
    ///
    /// score = new_edges^2 / (len * exec_ms + 1)
    pub fn compute_score(len: usize, exec_time: Duration, new_edges: usize) -> f64 {
        let exec_ms = exec_time.as_millis().max(1) as f64;
        let len_f = len.max(1) as f64;
        let edges_f = new_edges.max(1) as f64;
        (edges_f * edges_f) / (len_f * exec_ms)
    }
}

/// The corpus: an ordered collection of interesting inputs.
#[derive(Debug, Default)]
pub struct Corpus {
    entries: Vec<CorpusEntry>,
    /// Cumulative performance score, used to build a weighted selection.
    total_weight: f64,
    /// Output directory for persisting corpus entries to disk.
    pub corpus_dir: Option<PathBuf>,
}

impl Corpus {
    /// Create an empty in-memory corpus.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a corpus backed by an on-disk directory.
    pub fn with_dir(dir: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&dir)?;
        Ok(Self {
            corpus_dir: Some(dir),
            ..Default::default()
        })
    }

    /// Add an entry to the corpus. The entry is also written to disk if a
    /// corpus directory has been configured.
    pub fn add(&mut self, entry: CorpusEntry) {
        if let Some(ref dir) = self.corpus_dir {
            let path = dir.join(format!("entry_{:06}.bin", entry.id));
            let _ = std::fs::write(&path, &entry.bytes);
        }
        self.total_weight += entry.performance_score;
        self.entries.push(entry);
    }

    /// Load seeds from a directory on disk. Each file becomes one corpus entry
    /// with a default (high) performance score so it gets tested quickly.
    pub fn load_seeds(&mut self, seed_dir: &std::path::Path) -> std::io::Result<usize> {
        let mut count = 0;
        for entry in std::fs::read_dir(seed_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let bytes = std::fs::read(entry.path())?;
                let id = self.entries.len();
                let ce = CorpusEntry::new(
                    bytes,
                    Duration::from_millis(1),
                    1,
                    entry.file_name().to_str().map(String::from),
                    "seed",
                    id,
                );
                self.add(ce);
                count += 1;
            }
        }
        Ok(count)
    }

    /// Total number of corpus entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Select an entry weighted by performance score.
    ///
    /// Uses a rejection-sampling scheme so the selection is O(n) in the worst
    /// case but fast in practice (most inputs have a score near the mean).
    pub fn select_weighted(&self, rng: &mut impl rand::Rng) -> Option<&CorpusEntry> {
        if self.entries.is_empty() {
            return None;
        }
        if self.total_weight == 0.0 {
            // Fallback: uniform random.
            return self.entries.get(rng.gen_range(0..self.entries.len()));
        }
        let threshold = rng.gen::<f64>() * self.total_weight;
        let mut cumulative = 0.0;
        for entry in &self.entries {
            cumulative += entry.performance_score;
            if cumulative >= threshold {
                return Some(entry);
            }
        }
        self.entries.last()
    }

    /// Select an entry uniformly at random (for splice operations).
    pub fn select_random(&self, rng: &mut impl rand::Rng) -> Option<&CorpusEntry> {
        if self.entries.is_empty() {
            return None;
        }
        self.entries.get(rng.gen_range(0..self.entries.len()))
    }

    /// Return a reference to all entries (for iteration / analysis).
    pub fn entries(&self) -> &[CorpusEntry] {
        &self.entries
    }

    /// Statistics snapshot for the dashboard.
    pub fn stats(&self) -> CorpusStats {
        if self.entries.is_empty() {
            return CorpusStats::default();
        }
        let total_bytes: usize = self.entries.iter().map(|e| e.bytes.len()).sum();
        let avg_bytes = total_bytes / self.entries.len();
        let avg_exec_ms = self
            .entries
            .iter()
            .map(|e| e.execution_time.as_millis() as u64)
            .sum::<u64>()
            / self.entries.len() as u64;
        let max_score = self
            .entries
            .iter()
            .map(|e| e.performance_score)
            .fold(0.0f64, f64::max);
        CorpusStats {
            size: self.entries.len(),
            avg_input_bytes: avg_bytes,
            avg_exec_ms,
            total_weight: self.total_weight,
            max_performance_score: max_score,
        }
    }

    /// Next free entry ID.
    pub fn next_id(&self) -> usize {
        self.entries.len()
    }
}

/// Summary statistics for the corpus (shown in the dashboard).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorpusStats {
    pub size: usize,
    pub avg_input_bytes: usize,
    pub avg_exec_ms: u64,
    pub total_weight: f64,
    pub max_performance_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn make_entry(id: usize, size: usize, exec_ms: u64, edges: usize) -> CorpusEntry {
        CorpusEntry::new(
            vec![0u8; size],
            Duration::from_millis(exec_ms),
            edges,
            None,
            "test",
            id,
        )
    }

    #[test]
    fn add_and_select() {
        let mut corpus = Corpus::new();
        corpus.add(make_entry(0, 10, 5, 3));
        corpus.add(make_entry(1, 20, 50, 1));

        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);
        // Run 100 selections — both entries should appear.
        let mut seen = [0usize; 2];
        for _ in 0..100 {
            let e = corpus.select_weighted(&mut rng).unwrap();
            seen[e.id] += 1;
        }
        // Entry 0 has much higher score — should dominate.
        assert!(seen[0] > seen[1], "faster/smaller entry should win more");
    }

    #[test]
    fn score_heuristic_prefers_small_fast_inputs() {
        let s_small = CorpusEntry::compute_score(10, Duration::from_millis(1), 5);
        let s_large = CorpusEntry::compute_score(1000, Duration::from_millis(100), 5);
        assert!(s_small > s_large);
    }

    #[test]
    fn corpus_stats() {
        let mut corpus = Corpus::new();
        corpus.add(make_entry(0, 10, 5, 2));
        corpus.add(make_entry(1, 20, 10, 3));
        let stats = corpus.stats();
        assert_eq!(stats.size, 2);
        assert_eq!(stats.avg_input_bytes, 15);
    }
}
