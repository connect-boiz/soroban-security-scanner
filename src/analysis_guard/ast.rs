//! AST structure validation with schema enforcement.
//!
//! Validates a parsed AST against [`AstLimits`] *iteratively* (no recursion, so
//! a deeply-nested tree can't blow the stack) — bounding depth, total node
//! count and per-node fan-out, and rejecting unknown node kinds. This is the
//! guard that stops a malicious AST from exhausting the analyzer.

use crate::analysis_guard::limits::AstLimits;
use serde::{Deserialize, Serialize};

/// A generic AST node. `kind` is validated against an allowlist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstNode {
    /// Node kind (e.g. "Function", "Call", "Literal").
    pub kind: String,
    /// Child nodes.
    pub children: Vec<AstNode>,
}

impl Drop for AstNode {
    /// Dismantles the tree iteratively so dropping an adversarially deep AST
    /// cannot overflow the stack (which would abort the process — a DoS the
    /// analyzer must resist even on the cleanup path).
    fn drop(&mut self) {
        let mut stack: Vec<AstNode> = std::mem::take(&mut self.children);
        while let Some(mut node) = stack.pop() {
            stack.extend(std::mem::take(&mut node.children));
            // `node` now has no children, so its own drop does not recurse.
        }
    }
}

impl AstNode {
    /// Convenience constructor for a leaf.
    pub fn leaf(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            children: Vec::new(),
        }
    }

    /// Convenience constructor with children.
    pub fn node(kind: impl Into<String>, children: Vec<AstNode>) -> Self {
        Self {
            kind: kind.into(),
            children,
        }
    }
}

/// Why AST validation failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AstError {
    /// Depth exceeded the limit.
    TooDeep {
        /// Allowed maximum.
        max: usize,
    },
    /// Total node count exceeded the limit.
    TooManyNodes {
        /// Allowed maximum.
        max: usize,
    },
    /// A node had too many children.
    TooManyChildren {
        /// Allowed maximum.
        max: usize,
    },
    /// A node had an unrecognized kind.
    UnknownKind {
        /// The offending kind.
        kind: String,
    },
}

/// A successful validation summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstSummary {
    /// Total nodes.
    pub nodes: usize,
    /// Maximum depth observed.
    pub depth: usize,
}

/// Validates an AST against `limits`, accepting only `allowed_kinds`.
/// Traversal is iterative to remain safe on adversarially deep trees.
pub fn validate(
    root: &AstNode,
    limits: &AstLimits,
    allowed_kinds: &[&str],
) -> Result<AstSummary, AstError> {
    let mut nodes = 0usize;
    let mut max_depth = 0usize;
    // Explicit stack of (node, depth).
    let mut stack: Vec<(&AstNode, usize)> = vec![(root, 1)];

    while let Some((node, depth)) = stack.pop() {
        nodes += 1;
        if nodes > limits.max_nodes {
            return Err(AstError::TooManyNodes {
                max: limits.max_nodes,
            });
        }
        if depth > limits.max_depth {
            return Err(AstError::TooDeep {
                max: limits.max_depth,
            });
        }
        max_depth = max_depth.max(depth);

        if !allowed_kinds.contains(&node.kind.as_str()) {
            return Err(AstError::UnknownKind {
                kind: node.kind.clone(),
            });
        }
        if node.children.len() > limits.max_children {
            return Err(AstError::TooManyChildren {
                max: limits.max_children,
            });
        }
        for child in &node.children {
            stack.push((child, depth + 1));
        }
    }

    Ok(AstSummary {
        nodes,
        depth: max_depth,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const KINDS: &[&str] = &["Module", "Function", "Call", "Literal"];

    fn limits() -> AstLimits {
        AstLimits::default()
    }

    fn sample() -> AstNode {
        AstNode::node(
            "Module",
            vec![AstNode::node(
                "Function",
                vec![AstNode::node("Call", vec![AstNode::leaf("Literal")])],
            )],
        )
    }

    #[test]
    fn valid_tree_passes() {
        let summary = validate(&sample(), &limits(), KINDS).unwrap();
        assert_eq!(summary.nodes, 4);
        assert_eq!(summary.depth, 4);
    }

    #[test]
    fn unknown_kind_rejected() {
        let tree = AstNode::node("Module", vec![AstNode::leaf("Backdoor")]);
        assert_eq!(
            validate(&tree, &limits(), KINDS).unwrap_err(),
            AstError::UnknownKind {
                kind: "Backdoor".to_string()
            }
        );
    }

    #[test]
    fn too_deep_rejected() {
        let shallow = AstLimits {
            max_depth: 2,
            ..limits()
        };
        assert!(matches!(
            validate(&sample(), &shallow, KINDS),
            Err(AstError::TooDeep { .. })
        ));
    }

    #[test]
    fn too_many_nodes_rejected() {
        let tiny = AstLimits {
            max_nodes: 2,
            ..limits()
        };
        assert!(matches!(
            validate(&sample(), &tiny, KINDS),
            Err(AstError::TooManyNodes { .. })
        ));
    }

    #[test]
    fn too_many_children_rejected() {
        let wide = AstNode::node("Module", vec![AstNode::leaf("Literal"); 10]);
        let narrow = AstLimits {
            max_children: 4,
            ..limits()
        };
        assert!(matches!(
            validate(&wide, &narrow, KINDS),
            Err(AstError::TooManyChildren { .. })
        ));
    }

    #[test]
    fn deeply_nested_tree_does_not_stack_overflow() {
        // Build a 50_000-deep chain; iterative validation must handle it.
        let mut node = AstNode::leaf("Literal");
        for _ in 0..50_000 {
            node = AstNode::node("Call", vec![node]);
        }
        let root = AstNode::node("Module", vec![node]);
        let limits = AstLimits {
            max_depth: 100_000,
            ..limits()
        };
        assert!(validate(&root, &limits, KINDS).is_ok());
    }
}
