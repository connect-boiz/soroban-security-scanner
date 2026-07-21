//! Phase 6 — Path Exploration Strategy
//!
//! Implements search strategies for exploring execution paths:
//! BFS (complete), DFS (fast), and coverage-guided best-first search.

use serde::{Deserialize, Serialize};
use super::state::{SymbolicState, PathConstraint};
use super::ir::ControlFlowGraph;

/// Search strategy for path exploration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchStrategy {
    /// Breadth-first search (complete but memory-intensive).
    Bfs,
    /// Depth-first search (fast but can get stuck).
    Dfs,
    /// Coverage-guided best-first search (prioritizes unvisited CFG edges).
    CoverageGuided,
}

/// Configuration for path exploration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorationConfig {
    /// Search strategy.
    pub strategy: SearchStrategy,
    /// Maximum path depth (number of instructions).
    pub max_depth: u32,
    /// Maximum number of paths to explore.
    pub max_paths: u32,
    /// Timeout in seconds.
    pub timeout_secs: u64,
}

impl Default for ExplorationConfig {
    fn default() -> Self {
        Self {
            strategy: SearchStrategy::CoverageGuided,
            max_depth: 1000,
            max_paths: 10_000,
            timeout_secs: 300,
        }
    }
}

/// The path explorer that drives symbolic execution.
pub struct PathExplorer {
    config: ExplorationConfig,
    /// Visited CFG edges (for coverage tracking).
    visited_edges: std::collections::HashSet<(u32, u32)>,
    /// Number of paths explored.
    paths_explored: u32,
}

impl PathExplorer {
    /// Create a new path explorer with the given configuration.
    pub fn new(config: ExplorationConfig) -> Self {
        Self {
            config,
            visited_edges: std::collections::HashSet::new(),
            paths_explored: 0,
        }
    }

    /// Check if exploration should continue.
    pub fn should_continue(&self) -> bool {
        self.paths_explored < self.config.max_paths
    }

    /// Record a visited CFG edge.
    pub fn mark_edge_visited(&mut self, from: u32, to: u32) {
        self.visited_edges.insert((from, to));
    }

    /// Get the number of paths explored.
    pub fn paths_explored(&self) -> u32 {
        self.paths_explored
    }

    /// Get coverage percentage (visited edges / total edges).
    pub fn coverage(&self, cfg: &ControlFlowGraph) -> f64 {
        let total: usize = cfg.blocks.iter().map(|b| b.successors.len()).sum();
        if total == 0 {
            return 100.0;
        }
        (self.visited_edges.len() as f64 / total as f64) * 100.0
    }

    /// Get the exploration configuration.
    pub fn config(&self) -> &ExplorationConfig {
        &self.config
    }
}
