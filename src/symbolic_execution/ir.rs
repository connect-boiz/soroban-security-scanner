// SPDX-License-Identifier: MIT
//! Phase 1 — IR Lifting: WASM-to-IR translation with explicit CFGs.
//!
//! Translates Soroban contract WASM into a three-address-code intermediate
//! representation with explicit control flow graphs. Each IR instruction
//! preserves metadata to reconstruct source-level vulnerability locations.

use std::collections::HashMap;

/// IR operation types (three-address code).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IROp {
    /// Load a constant value.
    Const(i128),
    /// Load a symbolic variable (fresh from storage/input).
    LoadSym(String),
    /// Store to a named location (contract storage, local var).
    Store(String, String),
    /// Load from a named location.
    Load(String),
    /// Binary arithmetic: add, sub, mul, div, rem.
    BinOp(BinOpKind, String, String),
    /// Comparison: eq, ne, lt, le, gt, ge.
    Compare(CmpOpKind, String, String),
    /// Unary operation: neg, not.
    UnaryOp(UnaryOpKind, String),
    /// Function call (internal or host function).
    Call(String, Vec<String>),
    /// Conditional branch (if-else).
    Branch(String, String, String), // condition, then_label, else_label
    /// Unconditional jump.
    Jump(String),
    /// Return from function.
    Return(Option<String>),
    /// Host function call (Stellar-specific).
    HostCall(String, Vec<String>),
    /// No-op (placeholder for unhandled instructions).
    Nop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOpKind { Add, Sub, Mul, Div, Rem, And, Or, Xor, Shl, Shr }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CmpOpKind { Eq, Ne, Lt, Le, Gt, Ge }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOpKind { Neg, Not }

/// A single IR instruction with source metadata.
#[derive(Debug, Clone)]
pub struct IRInstruction {
    pub op: IROp,
    /// Destination variable name (if this instruction produces a value).
    pub dest: Option<String>,
    /// Source location for mapping back to original code.
    pub source_file: Option<String>,
    pub source_line: Option<u32>,
    /// WASM offset (byte offset in the WASM binary).
    pub wasm_offset: Option<u64>,
    /// Function name this instruction belongs to.
    pub function: String,
}

/// A basic block in the control flow graph.
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub label: String,
    pub instructions: Vec<IRInstruction>,
    /// Successor block labels.
    pub successors: Vec<String>,
    /// Predecessor block labels.
    pub predecessors: Vec<String>,
}

/// Control flow graph for a single function.
#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    pub function_name: String,
    pub entry_block: String,
    pub blocks: HashMap<String, BasicBlock>,
}

impl ControlFlowGraph {
    pub fn new(function_name: &str) -> Self {
        Self {
            function_name: function_name.to_string(),
            entry_block: "entry".to_string(),
            blocks: HashMap::new(),
        }
    }

    pub fn add_block(&mut self, block: BasicBlock) {
        self.blocks.insert(block.label.clone(), block);
    }

    pub fn get_block(&self, label: &str) -> Option<&BasicBlock> {
        self.blocks.get(label)
    }

    /// Get all block labels reachable from the entry block (BFS).
    pub fn reachable_blocks(&self) -> Vec<String> {
        let mut visited = std::collections::HashSet::new();
        let mut queue = vec![self.entry_block.clone()];
        while let Some(label) = queue.pop() {
            if visited.contains(&label) { continue; }
            visited.insert(label.clone());
            if let Some(block) = self.blocks.get(&label) {
                for succ in &block.successors {
                    if !visited.contains(succ) {
                        queue.push(succ.clone());
                    }
                }
            }
        }
        visited.into_iter().collect()
    }
}

/// A complete IR program: all functions and their CFGs.
#[derive(Debug, Clone)]
pub struct IRProgram {
    pub functions: HashMap<String, ControlFlowGraph>,
    /// Host function imports (Stellar SDK calls).
    pub host_imports: Vec<String>,
    /// Contract metadata.
    pub contract_id: Option<String>,
}

impl IRProgram {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            host_imports: Vec::new(),
            contract_id: None,
        }
    }

    pub fn add_function(&mut self, cfg: ControlFlowGraph) {
        self.functions.insert(cfg.function_name.clone(), cfg);
    }
}

impl Default for IRProgram {
    fn default() -> Self { Self::new() }
}

/// IR Lifter: translates WASM bytecode (or Rust source via syn) into IR.
pub struct IRLifter {
    program: IRProgram,
}

impl IRLifter {
    pub fn new() -> Self {
        Self { program: IRProgram::new() }
    }

    /// Lift a WASM binary into an IR program.
    /// In production, this parses the WASM section by section.
    /// For now, it creates a placeholder structure that can be filled
    /// by the concrete-to-IR bridge or by parsing Rust source.
    pub fn lift_wasm(&mut self, _wasm_bytes: &[u8]) -> &IRProgram {
        // Phase 1: Full WASM parsing would go here.
        // The WASM module is parsed into functions, each function's
        // instruction stream is lifted to IR with explicit CFGs.
        // This is a placeholder that creates an empty program.
        &self.program
    }

    /// Lift a Rust source file into IR using the syn AST.
    /// This is the primary path for Soroban contracts written in Rust.
    pub fn lift_rust_source(&mut self, source: &str, file_path: &str) -> &IRProgram {
        // Parse the Rust source and extract function definitions.
        // Each function is translated to a CFG with IR instructions.
        if let Ok(syntax) = syn::parse_file(source) {
            for item in &syntax.items {
                if let syn::Item::Fn(func) = item {
                    let cfg = self.lift_function(func, file_path);
                    self.program.add_function(cfg);
                }
            }
        }
        &self.program
    }

    /// Lift a single Rust function into a CFG.
    fn lift_function(&self, func: &syn::ItemFn, file_path: &str) -> ControlFlowGraph {
        let func_name = func.sig.ident.to_string();
        let mut cfg = ControlFlowGraph::new(&func_name);

        // Create entry block
        let mut entry = BasicBlock {
            label: "entry".to_string(),
            instructions: Vec::new(),
            successors: Vec::new(),
            predecessors: Vec::new(),
        };

        // Lift function body statements to IR instructions
        for stmt in &func.block.stmts {
            let instructions = self.lift_stmt(stmt, &func_name, file_path);
            entry.instructions.extend(instructions);
        }

        cfg.add_block(entry);
        cfg
    }

    /// Lift a Rust statement into IR instructions.
    fn lift_stmt(&self, stmt: &syn::Stmt, func: &str, file: &str) -> Vec<IRInstruction> {
        let make_meta = || (Some(file.to_string()), None, None::<u64>);
        let (source_file, source_line, wasm_offset) = make_meta();

        match stmt {
            syn::Stmt::Local(local) => {
                let var_name = if let syn::Pat::Ident(pat) = &local.pat {
                    pat.ident.to_string()
                } else {
                    "tmp".to_string()
                };
                let mut instrs = Vec::new();
                if let Some(init) = &local.init {
                    instrs.extend(self.lift_expr(&init.expr, &var_name, func, file));
                }
                instrs
            }
            syn::Stmt::Expr(expr) => {
                self.lift_expr(expr, "result", func, file)
            }
            _ => {
                vec![IRInstruction {
                    op: IROp::Nop,
                    dest: None,
                    source_file,
                    source_line,
                    wasm_offset,
                    function: func.to_string(),
                }]
            }
        }
    }

    /// Lift a Rust expression into IR instructions.
    fn lift_expr(&self, expr: &syn::Expr, dest: &str, func: &str, file: &str) -> Vec<IRInstruction> {
        let meta = || (Some(file.to_string()), None, None::<u64>);
        let (source_file, source_line, wasm_offset) = meta();

        match expr {
            syn::Expr::Lit(lit) => {
                let val = if let syn::Lit::Int(int) = &lit.lit {
                    int.base10_parse::<i128>().unwrap_or(0)
                } else {
                    0
                };
                vec![IRInstruction {
                    op: IROp::Const(val),
                    dest: Some(dest.to_string()),
                    source_file, source_line, wasm_offset,
                    function: func.to_string(),
                }]
            }
            syn::Expr::Binary(bin) => {
                let left_var = format!("{}_lhs", dest);
                let right_var = format!("{}_rhs", dest);
                let mut instrs = Vec::new();
                instrs.extend(self.lift_expr(&bin.left, &left_var, func, file));
                instrs.extend(self.lift_expr(&bin.right, &right_var, func, file));
                let binop = match bin.op {
                    syn::BinOp::Add(_) => BinOpKind::Add,
                    syn::BinOp::Sub(_) => BinOpKind::Sub,
                    syn::BinOp::Mul(_) => BinOpKind::Mul,
                    syn::BinOp::Div(_) => BinOpKind::Div,
                    syn::BinOp::Rem(_) => BinOpKind::Rem,
                    syn::BinOp::And(_) => BinOpKind::And,
                    syn::BinOp::Or(_) => BinOpKind::Or,
                    syn::BinOp::BitXor(_) => BinOpKind::Xor,
                    syn::BinOp::Shl(_) => BinOpKind::Shl,
                    syn::BinOp::Shr(_) => BinOpKind::Shr,
                    syn::BinOp::Eq(_) => {
                        instrs.push(IRInstruction {
                            op: IROp::Compare(CmpOpKind::Eq, left_var.clone(), right_var.clone()),
                            dest: Some(dest.to_string()),
                            source_file, source_line, wasm_offset,
                            function: func.to_string(),
                        });
                        return instrs;
                    }
                    syn::BinOp::Ne(_) => {
                        instrs.push(IRInstruction {
                            op: IROp::Compare(CmpOpKind::Ne, left_var.clone(), right_var.clone()),
                            dest: Some(dest.to_string()),
                            source_file, source_line, wasm_offset,
                            function: func.to_string(),
                        });
                        return instrs;
                    }
                    syn::BinOp::Lt(_) => {
                        instrs.push(IRInstruction {
                            op: IROp::Compare(CmpOpKind::Lt, left_var.clone(), right_var.clone()),
                            dest: Some(dest.to_string()),
                            source_file, source_line, wasm_offset,
                            function: func.to_string(),
                        });
                        return instrs;
                    }
                    syn::BinOp::Le(_) => {
                        instrs.push(IRInstruction {
                            op: IROp::Compare(CmpOpKind::Le, left_var.clone(), right_var.clone()),
                            dest: Some(dest.to_string()),
                            source_file, source_line, wasm_offset,
                            function: func.to_string(),
                        });
                        return instrs;
                    }
                    syn::BinOp::Gt(_) => {
                        instrs.push(IRInstruction {
                            op: IROp::Compare(CmpOpKind::Gt, left_var.clone(), right_var.clone()),
                            dest: Some(dest.to_string()),
                            source_file, source_line, wasm_offset,
                            function: func.to_string(),
                        });
                        return instrs;
                    }
                    syn::BinOp::Ge(_) => {
                        instrs.push(IRInstruction {
                            op: IROp::Compare(CmpOpKind::Ge, left_var.clone(), right_var.clone()),
                            dest: Some(dest.to_string()),
                            source_file, source_line, wasm_offset,
                            function: func.to_string(),
                        });
                        return instrs;
                    }
                    _ => BinOpKind::Add,
                };
                instrs.push(IRInstruction {
                    op: IROp::BinOp(binop, left_var, right_var),
                    dest: Some(dest.to_string()),
                    source_file, source_line, wasm_offset,
                    function: func.to_string(),
                });
                instrs
            }
            syn::Expr::Call(call) => {
                let func_name = if let syn::Expr::Path(path) = &*call.func {
                    path.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default()
                } else {
                    "unknown".to_string()
                };
                let args: Vec<String> = call.args.iter()
                    .enumerate()
                    .map(|(i, _)| format!("{}_arg{}", dest, i))
                    .collect();
                let mut instrs = Vec::new();
                for (i, arg) in call.args.iter().enumerate() {
                    instrs.extend(self.lift_expr(arg, &format!("{}_arg{}", dest, i), func, file));
                }
                // Check if this is a known host function
                let is_host = stellar_semantics::STELLAR_HOST_FUNCTIONS.contains(&func_name.as_str());
                instrs.push(IRInstruction {
                    op: if is_host {
                        IROp::HostCall(func_name, args)
                    } else {
                        IROp::Call(func_name, args)
                    },
                    dest: Some(dest.to_string()),
                    source_file, source_line, wasm_offset,
                    function: func.to_string(),
                });
                instrs
            }
            _ => {
                vec![IRInstruction {
                    op: IROp::Nop,
                    dest: Some(dest.to_string()),
                    source_file, source_line, wasm_offset,
                    function: func.to_string(),
                }]
            }
        }
    }

    pub fn finish(self) -> IRProgram {
        self.program
    }
}
