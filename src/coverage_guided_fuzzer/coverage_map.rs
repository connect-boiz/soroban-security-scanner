//! Phase 2 — Coverage Map
//!
//! AFL-style 64-KB coverage bitmap. Each entry tracks whether a specific
//! (block_a XOR block_b) edge has been reached. An input is "interesting" if
//! it sets at least one bit that was previously 0 in the global map.
//!
//! The bitmap is deliberately compact (65536 bytes = 64 KB) so it fits in L2
//! cache and can be compared with a single `memcmp`-like scan.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

/// Number of entries in the edge coverage bitmap (must be a power of 2).
pub const COVERAGE_MAP_SIZE: usize = 1 << 16; // 65536

/// An edge identifier derived from two consecutive block IDs:
///   edge_id = (prev_block_id XOR cur_block_id) % COVERAGE_MAP_SIZE
pub type EdgeId = u32;

/// A single-thread coverage snapshot produced after one execution.
/// Contains the raw counts for each edge slot hit during that run.
///
/// Note: the inner array is stored as a Vec<u16> so that serde can handle
/// it (serde does not implement Serialize/Deserialize for arrays of size
/// > 32 without third-party crates).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCoverage {
    /// Counts of hits per edge slot. Slot indices are `EdgeId % COVERAGE_MAP_SIZE`.
    /// Length is always exactly COVERAGE_MAP_SIZE.
    pub edge_counts: Vec<u16>,
    /// Number of distinct edge slots (non-zero entries).
    pub edges_hit: usize,
}

impl ExecutionCoverage {
    /// Create a zeroed coverage snapshot.
    pub fn new() -> Self {
        Self {
            edge_counts: vec![0u16; COVERAGE_MAP_SIZE],
            edges_hit: 0,
        }
    }

    /// Increment the count for a given edge slot (saturating at u16::MAX).
    #[inline]
    pub fn record_edge(&mut self, edge_id: EdgeId) {
        let slot = (edge_id as usize) % COVERAGE_MAP_SIZE;
        let prev = self.edge_counts[slot];
        if prev == 0 {
            self.edges_hit += 1;
        }
        self.edge_counts[slot] = prev.saturating_add(1);
    }

    /// Reset all counters to zero.
    pub fn reset(&mut self) {
        self.edge_counts.iter_mut().for_each(|v| *v = 0);
        self.edges_hit = 0;
    }
}

impl Default for ExecutionCoverage {
    fn default() -> Self {
        Self::new()
    }
}

/// The global, cumulative coverage map maintained across all corpus executions.
///
/// Thread-safe via `Arc<RwLock<...>>`. The inner bitmap contains a 1 in each
/// slot where any corpus input has hit that edge.
#[derive(Debug, Clone)]
pub struct CoverageMap {
    inner: Arc<RwLock<CoverageMapInner>>,
}

#[derive(Debug)]
struct CoverageMapInner {
    /// Global bitmap: non-zero ⟹ edge has been seen at least once.
    global_bitmap: Box<[u8; COVERAGE_MAP_SIZE]>,
    /// Total number of edges discovered across all corpus inputs.
    total_edges_seen: usize,
    /// Number of inputs that increased coverage.
    interesting_inputs: usize,
}

impl CoverageMap {
    /// Create a new, empty global coverage map.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(CoverageMapInner {
                global_bitmap: Box::new([0u8; COVERAGE_MAP_SIZE]),
                total_edges_seen: 0,
                interesting_inputs: 0,
            })),
        }
    }

    /// Test whether `exec` contains any edge not yet in the global map.
    ///
    /// Returns the set of new edge IDs discovered (empty ⟹ not interesting).
    pub fn new_edges(&self, exec: &ExecutionCoverage) -> Vec<EdgeId> {
        let inner = self.inner.read().unwrap();
        let mut new = Vec::new();
        for (slot, &count) in exec.edge_counts.iter().enumerate() {
            if count > 0 && inner.global_bitmap[slot] == 0 {
                new.push(slot as EdgeId);
            }
        }
        new
    }

    /// Merge `exec` into the global map. Returns the new edges discovered
    /// (same semantics as `new_edges`).
    pub fn merge(&self, exec: &ExecutionCoverage) -> Vec<EdgeId> {
        let mut inner = self.inner.write().unwrap();
        let mut new = Vec::new();
        for (slot, &count) in exec.edge_counts.iter().enumerate() {
            if count > 0 {
                if inner.global_bitmap[slot] == 0 {
                    inner.global_bitmap[slot] = 1;
                    inner.total_edges_seen += 1;
                    new.push(slot as EdgeId);
                }
            }
        }
        if !new.is_empty() {
            inner.interesting_inputs += 1;
        }
        new
    }

    /// Number of inputs that increased the global coverage map.
    pub fn interesting_input_count(&self) -> usize {
        self.inner.read().unwrap().interesting_inputs
    }

    /// Returns coverage density in [0.0, 1.0].
    pub fn density(&self) -> f64 {
        self.total_edges() as f64 / COVERAGE_MAP_SIZE as f64
    }

    /// Export the raw bitmap for heatmap visualization (Phase 7).
    pub fn bitmap_snapshot(&self) -> Vec<u8> {
        self.inner.read().unwrap().global_bitmap.to_vec()
    }

    /// Number of distinct edges seen across all corpus inputs.
    pub fn total_edges(&self) -> usize {
        self.inner.read().unwrap().total_edges_seen
    }

    /// Reset the global map (used between isolated fuzzing sessions).
    pub fn reset(&self) {
        let mut inner = self.inner.write().unwrap();
        inner.global_bitmap.iter_mut().for_each(|v| *v = 0);
        inner.total_edges_seen = 0;
        inner.interesting_inputs = 0;
    }
}

impl Default for CoverageMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Lightweight stats snapshot for dashboard / reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageStats {
    pub total_edges: usize,
    pub density: f64,
    pub interesting_inputs: usize,
}

impl CoverageMap {
    pub fn stats(&self) -> CoverageStats {
        CoverageStats {
            total_edges: self.total_edges(),
            density: self.density(),
            interesting_inputs: self.interesting_input_count(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_edge_detected() {
        let map = CoverageMap::new();
        let mut exec = ExecutionCoverage::new();
        exec.record_edge(42);

        let new = map.new_edges(&exec);
        assert_eq!(new, vec![42]);
    }

    #[test]
    fn merge_adds_to_global() {
        let map = CoverageMap::new();
        let mut exec = ExecutionCoverage::new();
        exec.record_edge(10);
        exec.record_edge(20);

        let new = map.merge(&exec);
        assert_eq!(new.len(), 2);
        assert_eq!(map.total_edges(), 2);

        // Second merge of same data → no new edges.
        let new2 = map.merge(&exec);
        assert!(new2.is_empty());
        assert_eq!(map.total_edges(), 2);
    }

    #[test]
    fn density_computation() {
        let map = CoverageMap::new();
        let mut exec = ExecutionCoverage::new();
        for i in 0..100 {
            exec.record_edge(i);
        }
        map.merge(&exec);
        let d = map.density();
        assert!(d > 0.0 && d < 1.0);
    }

    #[test]
    fn edge_modulo_wraps() {
        let map = CoverageMap::new();
        // edge_id >= COVERAGE_MAP_SIZE should wrap.
        let overflow_id = COVERAGE_MAP_SIZE as EdgeId + 5;
        let mut exec = ExecutionCoverage::new();
        exec.record_edge(overflow_id);
        let new = map.merge(&exec);
        assert_eq!(new.len(), 1);
        assert_eq!(new[0], 5); // 5 is the wrapped slot
    }

    #[test]
    fn reset_clears_state() {
        let map = CoverageMap::new();
        let mut exec = ExecutionCoverage::new();
        exec.record_edge(1);
        map.merge(&exec);
        assert_eq!(map.total_edges(), 1);
        map.reset();
        assert_eq!(map.total_edges(), 0);
    }
}
