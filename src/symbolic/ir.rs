//! Phase 1 — IR Lifting: WASM-to-IR translation
//!
//! Translates Soroban contract WASM bytecode into a three-address-code
//! intermediate representation with explicit control flow graphs (CFG).

use serde::{Deserialize, Serialize};

/// A three-address-code IR instruction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrInstruction {
    /// Operation type (e.g., "add", "call", "store", "load")
    pub op: String,
    /// Destination operand (result register)
    pub dest: Option<u32>,
    /// Source operands (register IDs)
    pub srcs: Vec<u32>,
    /// Immediate value (for constants)
    pub imm: Option<i128>,
    /// Source location (for mapping back to original code)
    pub source_loc: Option<SourceLocation>,
}

/// Source location metadata for vulnerability reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub function: String,
    pub line: u32,
    pub column: u32,
}

/// A basic block in the control flow graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicBlock {
    /// Block ID.
    pub id: u32,
    /// Instructions in this block.
    pub instructions: Vec<IrInstruction>,
    /// Successor block IDs (branches).
    pub successors: Vec<u32>,
    /// Predecessor block IDs.
    pub predecessors: Vec<u32>,
}

/// The control flow graph for a function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlFlowGraph {
    /// Function name.
    pub function_name: String,
    /// All basic blocks, indexed by ID.
    pub blocks: Vec<BasicBlock>,
    /// Entry block ID.
    pub entry: u32,
}

impl ControlFlowGraph {
    /// Create a new empty CFG for a function.
    pub fn new(function_name: String) -> Self {
        Self {
            function_name,
            blocks: Vec::new(),
            entry: 0,
        }
    }

    /// Add a basic block to the CFG.
    pub fn add_block(&mut self, block: BasicBlock) {
        self.blocks.push(block);
    }

    /// Get a block by ID.
    pub fn get_block(&self, id: u32) -> Option<&BasicBlock> {
        self.blocks.iter().find(|b| b.id == id)
    }

    /// Get all unvisited CFG edges (for coverage-guided search).
    pub fn unvisited_edges(&self, visited: &std::collections::HashSet<(u32, u32)>) -> Vec<(u32, u32)> {
        self.blocks
            .iter()
            .flat_map(|b| b.successors.iter().map(|&s| (b.id, s)))
            .filter(|edge| !visited.contains(edge))
            .collect()
    }
}
