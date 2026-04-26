//! Execution Tracer for Differential Fuzzing
//! 
//! Captures detailed execution traces to identify where logic diverges between SDK versions.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::SystemTime;
use anyhow::Result;

/// Execution trace containing all events during contract execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub events: Vec<TraceEvent>,
    pub start_time: SystemTime,
    pub end_time: SystemTime,
    pub total_gas_consumed: u64,
    pub max_stack_depth: usize,
    pub memory_usage: MemoryUsageInfo,
}

impl ExecutionTrace {
    pub fn new() -> Self {
        let now = SystemTime::now();
        Self {
            events: Vec::new(),
            start_time: now,
            end_time: now,
            total_gas_consumed: 0,
            max_stack_depth: 0,
            memory_usage: MemoryUsageInfo::new(),
        }
    }

    pub fn add_event(&mut self, event: TraceEvent) {
        self.events.push(event);
    }

    pub fn finalize(&mut self) {
        self.end_time = SystemTime::now();
        self.max_stack_depth = self.calculate_max_stack_depth();
    }

    fn calculate_max_stack_depth(&self) -> usize {
        let mut max_depth = 0;
        let mut current_depth = 0;

        for event in &self.events {
            match event.event_type {
                TraceEventType::FunctionEntry => {
                    current_depth += 1;
                    max_depth = max_depth.max(current_depth);
                },
                TraceEventType::FunctionExit => {
                    current_depth = current_depth.saturating_sub(1);
                },
                _ => {}
            }
        }

        max_depth
    }

    /// Get events by type
    pub fn get_events_by_type(&self, event_type: TraceEventType) -> Vec<&TraceEvent> {
        self.events.iter()
            .filter(|event| event.event_type == event_type)
            .collect()
    }

    /// Get events by function name
    pub fn get_events_by_function(&self, function_name: &str) -> Vec<&TraceEvent> {
        self.events.iter()
            .filter(|event| event.function_name == function_name)
            .collect()
    }

    /// Find the first divergence point between two traces
    pub fn find_divergence_point(&self, other: &ExecutionTrace) -> Option<usize> {
        let min_len = self.events.len().min(other.events.len());
        
        for i in 0..min_len {
            if self.events[i] != other.events[i] {
                return Some(i);
            }
        }
        
        if self.events.len() != other.events.len() {
            return Some(min_len);
        }
        
        None
    }

    /// Calculate trace similarity score
    pub fn similarity_score(&self, other: &ExecutionTrace) -> f64 {
        if self.events.is_empty() && other.events.is_empty() {
            return 1.0;
        }
        
        let max_len = self.events.len().max(other.events.len());
        let mut matching_events = 0;
        
        for i in 0..max_len {
            let self_event = self.events.get(i);
            let other_event = other.events.get(i);
            
            match (self_event, other_event) {
                (Some(e1), Some(e2)) if e1.equivalent_to(e2) => matching_events += 1,
                (None, None) => matching_events += 1,
                _ => {}
            }
        }
        
        matching_events as f64 / max_len as f64
    }
}

/// Individual trace event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEvent {
    pub event_type: TraceEventType,
    pub function_name: String,
    pub line_number: usize,
    pub description: String,
    pub timestamp: SystemTime,
    pub gas_consumed: u64,
    pub stack_depth: usize,
    pub memory_address: Option<u64>,
    pub value: Option<TraceValue>,
}

impl TraceEvent {
    pub fn new(
        event_type: TraceEventType,
        function_name: String,
        line_number: usize,
        description: String,
    ) -> Self {
        Self {
            event_type,
            function_name,
            line_number,
            description,
            timestamp: SystemTime::now(),
            gas_consumed: 0,
            stack_depth: 0,
            memory_address: None,
            value: None,
        }
    }

    /// Check if this event is equivalent to another for comparison purposes
    pub fn equivalent_to(&self, other: &TraceEvent) -> bool {
        self.event_type == other.event_type &&
        self.function_name == other.function_name &&
        self.line_number == other.line_number
    }
}

/// Types of trace events
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TraceEventType {
    FunctionEntry,
    FunctionExit,
    VariableRead,
    VariableWrite,
    StorageRead,
    StorageWrite,
    ExternalCall,
    ExternalCallReturn,
    Error,
    Panic,
    Assert,
    EmitEvent,
    GasUpdate,
    MemoryAllocation,
    MemoryDeallocation,
    BranchTaken,
    LoopIteration,
}

/// Values captured in trace events
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceValue {
    I128(i128),
    U64(u64),
    Bool(bool),
    Bytes(Vec<u8>),
    String(String),
    Address([u8; 32]),
    Vector(Vec<TraceValue>),
    Map(HashMap<String, TraceValue>),
    None,
}

/// Memory usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsageInfo {
    pub peak_memory: u64,
    pub current_memory: u64,
    pub allocations: Vec<MemoryAllocation>,
}

impl MemoryUsageInfo {
    pub fn new() -> Self {
        Self {
            peak_memory: 0,
            current_memory: 0,
            allocations: Vec::new(),
        }
    }

    pub fn record_allocation(&mut self, address: u64, size: u64) {
        self.allocations.push(MemoryAllocation {
            address,
            size,
            timestamp: SystemTime::now(),
        });
        self.current_memory += size;
        self.peak_memory = self.peak_memory.max(self.current_memory);
    }

    pub fn record_deallocation(&mut self, address: u64, size: u64) {
        if let Some(pos) = self.allocations.iter().position(|alloc| alloc.address == address) {
            self.allocations.remove(pos);
            self.current_memory = self.current_memory.saturating_sub(size);
        }
    }
}

/// Memory allocation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAllocation {
    pub address: u64,
    pub size: u64,
    pub timestamp: SystemTime,
}

/// Execution tracer that captures detailed execution information
pub struct ExecutionTracer {
    enabled: bool,
    capture_memory: bool,
    capture_storage: bool,
    capture_gas: bool,
    current_trace: Option<ExecutionTrace>,
    stack_depth: usize,
}

impl ExecutionTracer {
    pub fn new() -> Self {
        Self {
            enabled: true,
            capture_memory: true,
            capture_storage: true,
            capture_gas: true,
            current_trace: None,
            stack_depth: 0,
        }
    }

    /// Start tracing a new execution
    pub fn start_trace(&mut self) {
        if !self.enabled {
            return;
        }
        
        self.current_trace = Some(ExecutionTrace::new());
        self.stack_depth = 0;
    }

    /// Stop tracing and return the collected trace
    pub fn stop_trace(&mut self) -> Option<ExecutionTrace> {
        if !self.enabled {
            return None;
        }
        
        if let Some(mut trace) = self.current_trace.take() {
            trace.finalize();
            Some(trace)
        } else {
            None
        }
    }

    /// Record a function entry
    pub fn record_function_entry(&mut self, function_name: &str, line_number: usize) {
        if let Some(ref mut trace) = self.current_trace {
            let mut event = TraceEvent::new(
                TraceEventType::FunctionEntry,
                function_name.to_string(),
                line_number,
                format!("Entering function {}", function_name),
            );
            event.stack_depth = self.stack_depth;
            trace.add_event(event);
            
            self.stack_depth += 1;
        }
    }

    /// Record a function exit
    pub fn record_function_exit(&mut self, function_name: &str, line_number: usize, return_value: Option<TraceValue>) {
        self.stack_depth = self.stack_depth.saturating_sub(1);
        
        if let Some(ref mut trace) = self.current_trace {
            let mut event = TraceEvent::new(
                TraceEventType::FunctionExit,
                function_name.to_string(),
                line_number,
                format!("Exiting function {}", function_name),
            );
            event.stack_depth = self.stack_depth;
            event.value = return_value;
            trace.add_event(event);
        }
    }

    /// Record a variable read
    pub fn record_variable_read(&mut self, variable_name: &str, value: TraceValue, line_number: usize) {
        if let Some(ref mut trace) = self.current_trace {
            let mut event = TraceEvent::new(
                TraceEventType::VariableRead,
                format!("read_{}", variable_name),
                line_number,
                format!("Reading variable {}", variable_name),
            );
            event.stack_depth = self.stack_depth;
            event.value = Some(value);
            trace.add_event(event);
        }
    }

    /// Record a variable write
    pub fn record_variable_write(&mut self, variable_name: &str, value: TraceValue, line_number: usize) {
        if let Some(ref mut trace) = self.current_trace {
            let mut event = TraceEvent::new(
                TraceEventType::VariableWrite,
                format!("write_{}", variable_name),
                line_number,
                format!("Writing variable {} = {:?}", variable_name, value),
            );
            event.stack_depth = self.stack_depth;
            event.value = Some(value);
            trace.add_event(event);
        }
    }

    /// Record a storage read
    pub fn record_storage_read(&mut self, key: &[u8], value: Option<&[u8]>, line_number: usize) {
        if !self.capture_storage {
            return;
        }
        
        if let Some(ref mut trace) = self.current_trace {
            let mut event = TraceEvent::new(
                TraceEventType::StorageRead,
                "storage_read".to_string(),
                line_number,
                format!("Reading storage key: {}", hex::encode(key)),
            );
            event.stack_depth = self.stack_depth;
            event.memory_address = Some(u64::from_le_bytes(key[..8].try_into().unwrap_or([0; 8])));
            
            if let Some(val) = value {
                event.value = Some(TraceValue::Bytes(val.to_vec()));
            } else {
                event.value = Some(TraceValue::None);
            }
            
            trace.add_event(event);
        }
    }

    /// Record a storage write
    pub fn record_storage_write(&mut self, key: &[u8], value: &[u8], line_number: usize) {
        if !self.capture_storage {
            return;
        }
        
        if let Some(ref mut trace) = self.current_trace {
            let mut event = TraceEvent::new(
                TraceEventType::StorageWrite,
                "storage_write".to_string(),
                line_number,
                format!("Writing storage key: {} = {}", hex::encode(key), hex::encode(value)),
            );
            event.stack_depth = self.stack_depth;
            event.memory_address = Some(u64::from_le_bytes(key[..8].try_into().unwrap_or([0; 8])));
            event.value = Some(TraceValue::Bytes(value.to_vec()));
            trace.add_event(event);
        }
    }

    /// Record an external contract call
    pub fn record_external_call(&mut self, contract_address: &[u8], function_name: &str, args: &[TraceValue], line_number: usize) {
        if let Some(ref mut trace) = self.current_trace {
            let mut event = TraceEvent::new(
                TraceEventType::ExternalCall,
                format!("call_{}", function_name),
                line_number,
                format!("Calling {} on contract {}", function_name, hex::encode(contract_address)),
            );
            event.stack_depth = self.stack_depth;
            event.value = Some(TraceValue::Vector(args.to_vec()));
            trace.add_event(event);
        }
    }

    /// Record an external call return
    pub fn record_external_call_return(&mut self, contract_address: &[u8], function_name: &str, return_value: Option<TraceValue>, line_number: usize) {
        if let Some(ref mut trace) = self.current_trace {
            let mut event = TraceEvent::new(
                TraceEventType::ExternalCallReturn,
                format!("return_{}", function_name),
                line_number,
                format!("Return from {} on contract {}", function_name, hex::encode(contract_address)),
            );
            event.stack_depth = self.stack_depth;
            event.value = return_value;
            trace.add_event(event);
        }
    }

    /// Record an error
    pub fn record_error(&mut self, error_message: &str, line_number: usize) {
        if let Some(ref mut trace) = self.current_trace {
            let mut event = TraceEvent::new(
                TraceEventType::Error,
                "error".to_string(),
                line_number,
                format!("Error: {}", error_message),
            );
            event.stack_depth = self.stack_depth;
            trace.add_event(event);
        }
    }

    /// Record a panic
    pub fn record_panic(&mut self, panic_message: &str, line_number: usize) {
        if let Some(ref mut trace) = self.current_trace {
            let mut event = TraceEvent::new(
                TraceEventType::Panic,
                "panic".to_string(),
                line_number,
                format!("Panic: {}", panic_message),
            );
            event.stack_depth = self.stack_depth;
            trace.add_event(event);
        }
    }

    /// Record an assertion failure
    pub fn record_assert(&mut self, assert_message: &str, line_number: usize) {
        if let Some(ref mut trace) = self.current_trace {
            let mut event = TraceEvent::new(
                TraceEventType::Assert,
                "assert".to_string(),
                line_number,
                format!("Assertion failed: {}", assert_message),
            );
            event.stack_depth = self.stack_depth;
            trace.add_event(event);
        }
    }

    /// Record gas consumption update
    pub fn record_gas_update(&mut self, gas_consumed: u64, line_number: usize) {
        if !self.capture_gas {
            return;
        }
        
        if let Some(ref mut trace) = self.current_trace {
            let mut event = TraceEvent::new(
                TraceEventType::GasUpdate,
                "gas_update".to_string(),
                line_number,
                format!("Gas consumed: {}", gas_consumed),
            );
            event.stack_depth = self.stack_depth;
            event.gas_consumed = gas_consumed;
            trace.add_event(event);
        }
    }

    /// Record memory allocation
    pub fn record_memory_allocation(&mut self, address: u64, size: u64, line_number: usize) {
        if !self.capture_memory {
            return;
        }
        
        if let Some(ref mut trace) = self.current_trace {
            let mut event = TraceEvent::new(
                TraceEventType::MemoryAllocation,
                "memory_alloc".to_string(),
                line_number,
                format!("Allocated {} bytes at address {:#x}", size, address),
            );
            event.stack_depth = self.stack_depth;
            event.memory_address = Some(address);
            trace.add_event(event);
            
            trace.memory_usage.record_allocation(address, size);
        }
    }

    /// Record memory deallocation
    pub fn record_memory_deallocation(&mut self, address: u64, size: u64, line_number: usize) {
        if !self.capture_memory {
            return;
        }
        
        if let Some(ref mut trace) = self.current_trace {
            let mut event = TraceEvent::new(
                TraceEventType::MemoryDeallocation,
                "memory_dealloc".to_string(),
                line_number,
                format!("Deallocated {} bytes at address {:#x}", size, address),
            );
            event.stack_depth = self.stack_depth;
            event.memory_address = Some(address);
            trace.add_event(event);
            
            trace.memory_usage.record_deallocation(address, size);
        }
    }

    /// Configure tracer settings
    pub fn configure(&mut self, enabled: bool, capture_memory: bool, capture_storage: bool, capture_gas: bool) {
        self.enabled = enabled;
        self.capture_memory = capture_memory;
        self.capture_storage = capture_storage;
        self.capture_gas = capture_gas;
    }

    /// Get current trace statistics
    pub fn get_trace_stats(&self) -> Option<TraceStats> {
        self.current_trace.as_ref().map(|trace| TraceStats {
            event_count: trace.events.len(),
            current_stack_depth: self.stack_depth,
            current_memory_usage: trace.memory_usage.current_memory,
            peak_memory_usage: trace.memory_usage.peak_memory,
            total_gas: trace.total_gas_consumed,
        })
    }
}

/// Trace statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStats {
    pub event_count: usize,
    pub current_stack_depth: usize,
    pub current_memory_usage: u64,
    pub peak_memory_usage: u64,
    pub total_gas: u64,
}
