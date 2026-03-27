use wasmtime::*;
use crate::ledger::{ExecutionContext, MockLedger};
use crate::fuzzer::FuzzValue;
use anyhow::{Result, anyhow};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageData {
    pub lines_hit: HashMap<u32, u32>, // line_number -> hit_count
    pub branches_hit: HashMap<u32, Vec<bool>>, // line_number -> branch_taken
    pub functions_hit: HashMap<String, u32>, // function_name -> hit_count
    pub total_instructions: u32,
    pub executed_instructions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzerInput {
    pub values: Vec<FuzzValue>,
    pub iteration: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct WasmExecutor {
    engine: Engine,
    coverage_data: Arc<Mutex<CoverageData>>,
    fuzzer_inputs: Arc<Mutex<HashMap<u32, Vec<FuzzerInput>>>>, // line_number -> inputs
}

impl WasmExecutor {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.wasm_multi_value(true);
        config.wasm_simd(true);
        config.consume_fuel(true);
        config.epoch_interruption(true);
        
        // Enable coverage profiling
        config.profiler(wasmtime::ProfilingStrategy::PerfMap);
        
        let engine = Engine::new(&config)?;
        Ok(Self { 
            engine,
            coverage_data: Arc::new(Mutex::new(CoverageData {
                lines_hit: HashMap::new(),
                branches_hit: HashMap::new(),
                functions_hit: HashMap::new(),
                total_instructions: 0,
                executed_instructions: 0,
            })),
            fuzzer_inputs: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn get_coverage_data(&self) -> CoverageData {
        self.coverage_data.lock().unwrap().clone()
    }

    pub fn get_fuzzer_inputs_for_line(&self, line: u32) -> Vec<FuzzerInput> {
        self.fuzzer_inputs.lock().unwrap()
            .get(&line)
            .cloned()
            .unwrap_or_default()
    }

    pub fn reset_coverage(&self) {
        let mut coverage = self.coverage_data.lock().unwrap();
        coverage.lines_hit.clear();
        coverage.branches_hit.clear();
        coverage.functions_hit.clear();
        coverage.executed_instructions = 0;
        coverage.total_instructions = 0;
        
        self.fuzzer_inputs.lock().unwrap().clear();
    }

    fn record_line_hit(&self, line: u32, inputs: &FuzzerInput) {
        let mut coverage = self.coverage_data.lock().unwrap();
        *coverage.lines_hit.entry(line).or_insert(0) += 1;
        
        drop(coverage);
        
        let mut fuzzer_inputs = self.fuzzer_inputs.lock().unwrap();
        fuzzer_inputs.entry(line).or_insert_with(Vec::new).push(inputs.clone());
    }

    fn record_branch_hit(&self, line: u32, taken: bool) {
        let mut coverage = self.coverage_data.lock().unwrap();
        let branches = coverage.branches_hit.entry(line).or_insert_with(Vec::new);
        if branches.len() <= 1 {
            branches.resize(2, false);
        }
        if taken {
            branches[0] = true;
        } else {
            branches[1] = true;
        }
    }

    fn record_function_hit(&self, function_name: &str) {
        let mut coverage = self.coverage_data.lock().unwrap();
        *coverage.functions_hit.entry(function_name.to_string()).or_insert(0) += 1;
    }

    fn fuzz_to_val(&self, f: &FuzzValue) -> Val {
        match f {
            FuzzValue::U32(v) => Val::I32(*v as i32),
            FuzzValue::I32(v) => Val::I32(*v),
            FuzzValue::Bool(v) => Val::I32(if *v { 1 } else { 0 }),
            // For complex types, we would normally store them in the host-managed heap
            // and return a handle. For this fuzzer proto, we'll just pass a dummy handle.
            _ => Val::I64(0), 
        }
    }

    pub fn execute_with_invariants(
        &self, 
        wasm_bytes: &[u8], 
        function: &str, 
        args: Vec<FuzzValue>,
        invariants: Vec<Box<dyn Fn(&MockLedger) -> Result<()>>>,
        iteration: usize,
    ) -> Result<()> {
        let module = Module::from_binary(&self.engine, wasm_bytes)?;
        let mut linker = Linker::new(&self.engine);
        
        // Setup state
        let context = Arc::new(Mutex::new(ExecutionContext::new([0; 32])));
        let coverage_data = Arc::clone(&self.coverage_data);
        let fuzzer_inputs = Arc::clone(&self.fuzzer_inputs);
        
        // Create fuzzer input record for this execution
        let current_input = FuzzerInput {
            values: args.clone(),
            iteration,
            timestamp: chrono::Utc::now(),
        };
        
        // Record function hit
        self.record_function_hit(function);
        
        // Module 'v' is values in Soroban (manipulation, comparison)
        linker.func_wrap("v", "obj_from_u64", {
            let coverage_data = Arc::clone(&coverage_data);
            let fuzzer_inputs = Arc::clone(&fuzzer_inputs);
            let input_clone = current_input.clone();
            move |mut _caller: Caller<'_, ()>, val: i64| -> i64 {
                // Record line hit (mock line number for demonstration)
                let line = (val % 1000) as u32 + 1; // Mock line mapping
                let mut coverage = coverage_data.lock().unwrap();
                *coverage.lines_hit.entry(line).or_insert(0) += 1;
                
                let mut inputs = fuzzer_inputs.lock().unwrap();
                inputs.entry(line).or_insert_with(Vec::new).push(input_clone.clone());
                
                val
            }
        })?;

        // Module 's' is storage in Soroban
        linker.func_wrap("s", "put", {
            let context = Arc::clone(&context);
            let coverage_data = Arc::clone(&coverage_data);
            let fuzzer_inputs = Arc::clone(&fuzzer_inputs);
            let input_clone = current_input.clone();
            move |mut _caller: Caller<'_, ()>, key: i64, val: i64| {
                let mut ctx = context.lock().unwrap();
                ctx.ledger.put_storage(key.to_le_bytes().to_vec(), val.to_le_bytes().to_vec());
                
                // Record coverage for storage operations (mock line numbers)
                let line = (key % 1000 + 100) as u32; // Storage ops around line 100-1100
                let mut coverage = coverage_data.lock().unwrap();
                *coverage.lines_hit.entry(line).or_insert(0) += 1;
                
                let mut inputs = fuzzer_inputs.lock().unwrap();
                inputs.entry(line).or_insert_with(Vec::new).push(input_clone.clone());
                
                println!("Host call: s::put(k:{}, v:{}) - hit line: {}", key, val, line);
            }
        })?;

        linker.func_wrap("s", "get", {
            let context = Arc::clone(&context);
            let coverage_data = Arc::clone(&coverage_data);
            let fuzzer_inputs = Arc::clone(&fuzzer_inputs);
            let input_clone = current_input.clone();
            move |mut _caller: Caller<'_, ()>, key: i64| -> i64 {
                let ctx = context.lock().unwrap();
                let key_bytes = key.to_le_bytes();
                
                // Record coverage for storage get operations
                let line = (key % 1000 + 200) as u32; // Storage get ops around line 200-1200
                let mut coverage = coverage_data.lock().unwrap();
                *coverage.lines_hit.entry(line).or_insert(0) += 1;
                
                let mut inputs = fuzzer_inputs.lock().unwrap();
                inputs.entry(line).or_insert_with(Vec::new).push(input_clone.clone());
                
                if let Some(val_bytes) = ctx.ledger.get_storage(&key_bytes) {
                     let mut buf = [0u8; 8];
                     let len = val_bytes.len().min(8);
                     buf[..len].copy_from_slice(&val_bytes[..len]);
                     i64::from_le_bytes(buf)
                } else {
                    0
                }
            }
        })?;

        // Module 'l' is ledger
        linker.func_wrap("l", "get_ledger_seq", {
            let context = Arc::clone(&context);
            let coverage_data = Arc::clone(&coverage_data);
            let fuzzer_inputs = Arc::clone(&fuzzer_inputs);
            let input_clone = current_input.clone();
            move |mut _caller: Caller<'_, ()>| -> i32 {
                // Record coverage for ledger operations
                let line = 300u32; // Ledger ops around line 300
                let mut coverage = coverage_data.lock().unwrap();
                *coverage.lines_hit.entry(line).or_insert(0) += 1;
                
                let mut inputs = fuzzer_inputs.lock().unwrap();
                inputs.entry(line).or_insert_with(Vec::new).push(input_clone.clone());
                
                context.lock().unwrap().ledger.sequence_number as i32
            }
        })?;

        // Module 'a' is auth
        linker.func_wrap("a", "require_auth", {
            let coverage_data = Arc::clone(&coverage_data);
            let fuzzer_inputs = Arc::clone(&fuzzer_inputs);
            let input_clone = current_input.clone();
            move |mut _caller: Caller<'_, ()>, address: i64| {
                // Record coverage for auth operations
                let line = (address % 100 + 400) as u32; // Auth ops around line 400-500
                let mut coverage = coverage_data.lock().unwrap();
                *coverage.lines_hit.entry(line).or_insert(0) += 1;
                
                let mut inputs = fuzzer_inputs.lock().unwrap();
                inputs.entry(line).or_insert_with(Vec::new).push(input_clone.clone());
                
                println!("Host call: a::require_auth(a:{}) - hit line: {}", address, line);
            }
        })?;

        // Module 'c' is contract calls
        linker.func_wrap("c", "call", {
            let coverage_data = Arc::clone(&coverage_data);
            let fuzzer_inputs = Arc::clone(&fuzzer_inputs);
            let input_clone = current_input.clone();
            move |mut _caller: Caller<'_, ()>, contract: i64, _func: i64, _args: i64| -> i64 {
                // Record coverage for contract calls
                let line = (contract % 100 + 500) as u32; // Contract calls around line 500-600
                let mut coverage = coverage_data.lock().unwrap();
                *coverage.lines_hit.entry(line).or_insert(0) += 1;
                
                let mut inputs = fuzzer_inputs.lock().unwrap();
                inputs.entry(line).or_insert_with(Vec::new).push(input_clone.clone());
                
                println!("Host call: c::call(c:{}) - hit line: {}", contract, line);
                0
            }
        })?;

        let mut store = Store::new(&self.engine, ());
        store.set_fuel(2_000_000)?; // Sandbox: Fuel limit (increased)

        let instance = linker.instantiate(&mut store, &module)?;
        let func = instance.get_func(&mut store, function)
            .ok_or_else(|| anyhow!("Function '{}' not found", function))?;
        
        // Convert FuzzValues to Wasmtime Vals
        let wasm_args: Vec<Val> = args.iter().map(|a| self.fuzz_to_val(a)).collect();
        
        // Record instruction count before execution
        let initial_fuel = store.fuel_consumed().unwrap_or(0);
        
        // Execute the function
        let mut results = vec![Val::I32(0); func.ty(&store).results().len()];
        func.call(&mut store, &wasm_args, &mut results)?;
        
        // Record instruction count after execution
        let final_fuel = store.fuel_consumed().unwrap_or(0);
        let instructions_executed = final_fuel - initial_fuel;
        
        // Update coverage data
        {
            let mut coverage = self.coverage_data.lock().unwrap();
            coverage.executed_instructions += instructions_executed;
            coverage.total_instructions += 2_000_000; // Total fuel allocated
        }
        
        // Check invariants after state mutation
        let ledger = &context.lock().unwrap().ledger;
        for check in invariants {
            check(ledger)?;
        }

        Ok(())
    }
}
