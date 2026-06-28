//! Limits and resource budgets for the analysis engine.
//!
//! Bounds every dimension an adversarial input could push on: source size and
//! structure, AST shape, and the CPU/memory/wall-time an analysis job may
//! consume (with the 5-minute maximum from the acceptance criteria).

use serde::{Deserialize, Serialize};

/// Limits on the raw contract-code input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputLimits {
    /// Maximum source size in bytes.
    pub max_bytes: usize,
    /// Maximum single-line length (guards pathological/minified input).
    pub max_line_len: usize,
    /// Maximum nesting depth of delimiters `()[]{}`.
    pub max_delimiter_depth: usize,
}

impl Default for InputLimits {
    fn default() -> Self {
        Self {
            max_bytes: 5 * 1024 * 1024, // 5 MB
            max_line_len: 10_000,
            max_delimiter_depth: 256,
        }
    }
}

/// Limits on AST structure (schema enforcement).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstLimits {
    /// Maximum tree depth.
    pub max_depth: usize,
    /// Maximum total node count.
    pub max_nodes: usize,
    /// Maximum children of any single node.
    pub max_children: usize,
}

impl Default for AstLimits {
    fn default() -> Self {
        Self {
            max_depth: 512,
            max_nodes: 1_000_000,
            max_children: 65_536,
        }
    }
}

/// Resource budget for one analysis job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBudget {
    /// CPU work budget in abstract units (the analyzer reports consumption).
    pub max_cpu_units: u64,
    /// Peak memory budget in bytes.
    pub max_memory_bytes: u64,
    /// Wall-clock timeout in milliseconds (acceptance: 5-minute maximum).
    pub max_wall_ms: u64,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            max_cpu_units: 10_000_000,
            max_memory_bytes: 1024 * 1024 * 1024, // 1 GB
            max_wall_ms: 5 * 60 * 1000,           // 5 minutes
        }
    }
}

/// Combined limits configuration for the analysis guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GuardLimits {
    /// Input limits.
    pub input: InputLimits,
    /// AST limits.
    pub ast: AstLimits,
    /// Per-job resource budget.
    pub resources: ResourceBudget,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_enforce_five_minute_timeout() {
        let limits = GuardLimits::default();
        assert_eq!(limits.resources.max_wall_ms, 300_000);
        assert_eq!(limits.input.max_bytes, 5 * 1024 * 1024);
        assert!(limits.ast.max_depth > 0);
    }
}
