use wasmtime::*;
use crate::ledger::{ExecutionContext, MockLedger};
use crate::fuzzer::FuzzValue;
use anyhow::{Result, anyhow};
use std::sync::{Arc, Mutex};

pub struct WasmExecutor {
    engine: Engine,
}

impl WasmExecutor {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.wasm_multi_value(true);
        config.wasm_simd(true);
        // Sandboxed configuration
        config.consume_fuel(true);
        config.epoch_interruption(true);
        
        let engine = Engine::new(&config)?;
        Ok(Self { engine })
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
        invariants: Vec<Box<dyn Fn(&MockLedger) -> Result<()>>>
    ) -> Result<()> {
        let module = Module::from_binary(&self.engine, wasm_bytes)?;
        let mut linker = Linker::new(&self.engine);
        
        // Setup state
        let context = Arc::new(Mutex::new(ExecutionContext::new([0; 32])));
        
        // Module 'v' is values in Soroban (manipulation, comparison)
        linker.func_wrap("v", "obj_from_u64", {
            move |mut _caller: Caller<'_, ()>, val: i64| -> i64 {
                // Simplified: treat as a handle or just return it
                val
            }
        })?;

        // Module 's' is storage in Soroban
        linker.func_wrap("s", "put", {
            let context = Arc::clone(&context);
            move |mut _caller: Caller<'_, ()>, key: i64, val: i64| {
                let mut ctx = context.lock().unwrap();
                ctx.ledger.put_storage(key.to_le_bytes().to_vec(), val.to_le_bytes().to_vec());
                println!("Host call: s::put(k:{}, v:{})", key, val);
            }
        })?;

        linker.func_wrap("s", "get", {
            let context = Arc::clone(&context);
            move |mut _caller: Caller<'_, ()>, key: i64| -> i64 {
                let ctx = context.lock().unwrap();
                let key_bytes = key.to_le_bytes();
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
            move |mut _caller: Caller<'_, ()>| -> i32 {
                context.lock().unwrap().ledger.sequence_number as i32
            }
        })?;

        // Module 'a' is auth
        linker.func_wrap("a", "require_auth", {
            move |mut _caller: Caller<'_, ()>, address: i64| {
                // Simplified: always approve for fuzzing unless specified
                println!("Host call: a::require_auth(a:{})", address);
            }
        })?;

        // Module 'c' is contract calls
        linker.func_wrap("c", "call", {
            move |mut _caller: Caller<'_, ()>, contract: i64, _func: i64, _args: i64| -> i64 {
                // Simplified: return success or 0
                println!("Host call: c::call(c:{})", contract);
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
        
        // Execute the function
        let mut results = vec![Val::I32(0); func.ty(&store).results().len()];
        func.call(&mut store, &wasm_args, &mut results)?;
        
        // Check invariants after state mutation
        let ledger = &context.lock().unwrap().ledger;
        for check in invariants {
            check(ledger)?;
        }

        Ok(())
    }
}
