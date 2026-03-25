//! Test Runner for Differential Fuzzing
//! 
//! Executes the same test input across multiple Soroban SDK versions and collects results.

use crate::differential_fuzzing::{
    SdkVersion, TestInput, ExecutionResult, DifferentialFuzzingConfig,
    StateChange, StateChangeType, ArgumentValue
};
use soroban_sdk::{BytesN, Address, Vec as SorobanVec};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};
use anyhow::{Result, anyhow};
use tokio::task::JoinSet;
use serde::{Serialize, Deserialize};

/// Configuration for individual test execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub timeout: Duration,
    pub max_memory: u64,
    pub enable_tracing: bool,
    pub capture_state_changes: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_memory: 1024 * 1024 * 1024, // 1GB
            enable_tracing: true,
            capture_state_changes: true,
        }
    }
}

/// Test execution environment for a specific SDK version
#[derive(Debug)]
pub struct TestEnvironment {
    pub sdk_version: SdkVersion,
    pub wasm_path: String,
    pub contract_id: [u8; 32],
    pub sandbox_path: String,
}

impl TestEnvironment {
    pub fn new(sdk_version: SdkVersion, wasm_path: &str, contract_id: [u8; 32]) -> Result<Self> {
        let sandbox_path = format!("/tmp/soroban_sandbox_{}", contract_id.iter().map(|b| format!("{:02x}", b)).collect::<String>());
        
        Ok(Self {
            sdk_version,
            wasm_path: wasm_path.to_string(),
            contract_id,
            sandbox_path,
        })
    }

    /// Setup the test environment with the specific SDK version
    pub async fn setup(&self) -> Result<()> {
        // Create sandbox directory
        std::fs::create_dir_all(&self.sandbox_path)?;
        
        // Install specific SDK version if needed
        self.install_sdk_version().await?;
        
        // Initialize contract in sandbox
        self.initialize_contract().await?;
        
        Ok(())
    }

    async fn install_sdk_version(&self) -> Result<()> {
        // In a real implementation, this would install the specific SDK version
        // For now, we'll assume the SDK is already available
        println!("Setting up SDK version: {}", self.sdk_version.version);
        Ok(())
    }

    async fn initialize_contract(&self) -> Result<()> {
        // Initialize the contract in the sandbox environment
        let output = Command::new("soroban")
            .args(&[
                "contract",
                "deploy",
                "--wasm", &self.wasm_path,
                "--source", &format!("{:x}", self.contract_id.iter().fold(String::new(), |acc, &b| format!("{}{:02x}", acc, b))),
            ])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to initialize contract: {}", String::from_utf8_lossy(&output.stderr)));
        }

        Ok(())
    }

    /// Clean up the test environment
    pub async fn cleanup(&self) -> Result<()> {
        if Path::new(&self.sandbox_path).exists() {
            std::fs::remove_dir_all(&self.sandbox_path)?;
        }
        Ok(())
    }
}

/// Differential test runner that executes tests across multiple SDK versions
pub struct DifferentialTestRunner {
    environments: HashMap<SdkVersion, TestEnvironment>,
    config: TestConfig,
}

impl DifferentialTestRunner {
    pub fn new(sdk_versions: Vec<SdkVersion>) -> Result<Self> {
        let mut environments = HashMap::new();
        
        for version in sdk_versions {
            // In a real implementation, we would locate the appropriate WASM file for each version
            let wasm_path = "./target/wasm32-unknown-unknown/release/contract.wasm".to_string();
            let contract_id = generate_contract_id(&version);
            
            let env = TestEnvironment::new(version, &wasm_path, contract_id)?;
            environments.insert(version, env);
        }

        Ok(Self {
            environments,
            config: TestConfig::default(),
        })
    }

    /// Execute a single test input across all SDK versions
    pub async fn execute_test(
        &mut self,
        input: &TestInput,
        fuzzing_config: &DifferentialFuzzingConfig,
    ) -> Result<Vec<ExecutionResult>> {
        let mut results = Vec::new();
        let mut join_set = JoinSet::new();

        // Setup all environments
        for (version, env) in &self.environments {
            let env = env.clone();
            let input = input.clone();
            let config = self.config.clone();
            let wasm_path = fuzzing_config.contract_path.clone();

            join_set.spawn(async move {
                Self::execute_in_environment(env, input, config, wasm_path).await
            });
        }

        // Collect results
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(execution_result) => results.push(execution_result?),
                Err(e) => return Err(anyhow!("Test execution failed: {}", e)),
            }
        }

        Ok(results)
    }

    /// Execute test in a specific environment
    async fn execute_in_environment(
        mut env: TestEnvironment,
        input: TestInput,
        config: TestConfig,
        wasm_path: String,
    ) -> Result<ExecutionResult> {
        let start_time = Instant::now();
        
        // Setup environment
        env.setup().await?;

        // Convert arguments to Soroban format
        let soroban_args = convert_arguments_to_soroban(&input.arguments)?;

        // Execute the function
        let execution_result = tokio::time::timeout(
            config.timeout,
            Self::execute_function(&env, &input.function_name, &soroban_args, &config)
        ).await;

        let (success, return_value, gas_consumed, state_changes, error) = match execution_result {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => (false, None, 0, Vec::new(), Some(e.to_string())),
            Err(_) => (false, None, 0, Vec::new(), Some("Execution timeout".to_string())),
        };

        let execution_time = start_time.elapsed();

        // Generate execution trace
        let execution_trace = if config.enable_tracing {
            generate_execution_trace(&env, &input.function_name, success).await?
        } else {
            Default::default()
        };

        // Cleanup environment
        env.cleanup().await?;

        Ok(ExecutionResult {
            sdk_version: env.sdk_version,
            success,
            return_value,
            gas_consumed,
            state_changes,
            execution_trace,
            error,
            execution_time,
        })
    }

    /// Execute a function in the contract
    async fn execute_function(
        env: &TestEnvironment,
        function_name: &str,
        arguments: &[String],
        config: &TestConfig,
    ) -> Result<(bool, Option<ArgumentValue>, u64, Vec<StateChange>)> {
        // Build the soroban command
        let mut cmd = Command::new("soroban");
        cmd.args(&[
            "contract",
            "invoke",
            "--id", &format!("{:x}", env.contract_id.iter().fold(String::new(), |acc, &b| format!("{}{:02x}", acc, b))),
            "--function", function_name,
        ]);

        // Add arguments
        for arg in arguments {
            cmd.arg("--arg");
            cmd.arg(arg);
        }

        // Execute the command
        let output = cmd.output()?;

        if !output.status.success() {
            return Err(anyhow!("Function execution failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        // Parse the output
        let stdout = String::from_utf8(output.stdout)?;
        let (return_value, gas_consumed) = parse_execution_output(&stdout)?;

        // Capture state changes if enabled
        let state_changes = if config.capture_state_changes {
            capture_state_changes(env).await?
        } else {
            Vec::new()
        };

        Ok((true, return_value, gas_consumed, state_changes))
    }

    /// Execute multiple tests in parallel
    pub async fn execute_tests(
        &mut self,
        inputs: &[TestInput],
        fuzzing_config: &DifferentialFuzzingConfig,
    ) -> Result<Vec<Vec<ExecutionResult>>> {
        let mut all_results = Vec::new();
        
        for (index, input) in inputs.iter().enumerate() {
            if index % 100 == 0 {
                println!("Executing test {}/{}", index + 1, inputs.len());
            }
            
            let results = self.execute_test(input, fuzzing_config).await?;
            all_results.push(results);
        }

        Ok(all_results)
    }
}

/// Generate a unique contract ID for testing
fn generate_contract_id(version: &SdkVersion) -> [u8; 32] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    version.version.hash(&mut hasher);
    
    let hash = hasher.finish();
    let mut contract_id = [0u8; 32];
    
    for (i, byte) in hash.to_be_bytes().iter().enumerate() {
        if i < 32 {
            contract_id[i] = *byte;
        }
    }
    
    contract_id
}

/// Convert test arguments to Soroban format
fn convert_arguments_to_soroban(arguments: &[crate::differential_fuzzing::TestArgument]) -> Result<Vec<String>> {
    let mut soroban_args = Vec::new();
    
    for arg in arguments {
        let soroban_arg = match &arg.value {
            ArgumentValue::I128(val) => format!("i128:{}", val),
            ArgumentValue::U64(val) => format!("u64:{}", val),
            ArgumentValue::U32(val) => format!("u32:{}", val),
            ArgumentValue::Bool(val) => format!("bool:{}", val),
            ArgumentValue::Bytes(val) => format!("bytes:{}", hex::encode(val)),
            ArgumentValue::String(val) => format!("string:{}", val),
            ArgumentValue::Address(val) => format!("address:{}", hex::encode(val)),
            ArgumentValue::Vector(vals) => {
                let vector_args: Result<Vec<String>> = vals.iter()
                    .map(|v| convert_single_argument_to_soroban(v))
                    .collect();
                format!("vector:[{}]", vector_args?.join(","))
            },
            ArgumentValue::Map(map) => {
                let map_args: Vec<String> = map.iter()
                    .map(|(k, v)| format!("{}:{}", k, convert_single_argument_to_soroban(v).unwrap_or_default()))
                    .collect();
                format!("map:{{{}}}", map_args.join(","))
            },
            ArgumentValue::None => "null".to_string(),
        };
        
        soroban_args.push(soroban_arg);
    }
    
    Ok(soroban_args)
}

fn convert_single_argument_to_soroban(value: &ArgumentValue) -> Result<String> {
    match value {
        ArgumentValue::I128(val) => Ok(format!("i128:{}", val)),
        ArgumentValue::U64(val) => Ok(format!("u64:{}", val)),
        ArgumentValue::U32(val) => Ok(format!("u32:{}", val)),
        ArgumentValue::Bool(val) => Ok(format!("bool:{}", val)),
        ArgumentValue::Bytes(val) => Ok(format!("bytes:{}", hex::encode(val))),
        ArgumentValue::String(val) => Ok(format!("string:{}", val)),
        ArgumentValue::Address(val) => Ok(format!("address:{}", hex::encode(val))),
        _ => Err(anyhow!("Complex types not supported in single argument conversion")),
    }
}

/// Parse execution output to extract return value and gas consumption
fn parse_execution_output(output: &str) -> Result<(Option<ArgumentValue>, u64)> {
    // In a real implementation, this would parse the actual soroban output
    // For now, we'll simulate the parsing
    
    let gas_consumed = extract_gas_from_output(output)?;
    let return_value = extract_return_value_from_output(output)?;
    
    Ok((return_value, gas_consumed))
}

fn extract_gas_from_output(output: &str) -> Result<u64> {
    // Look for gas consumption in the output
    // Example: "Gas consumed: 12345"
    if let Some(line) = output.lines().find(|line| line.contains("Gas consumed")) {
        if let Some(gas_str) = line.split(':').nth(1) {
            return gas_str.trim().parse()
                .map_err(|_| anyhow!("Failed to parse gas consumption"));
        }
    }
    
    // Default gas consumption if not found
    Ok(1000)
}

fn extract_return_value_from_output(output: &str) -> Result<Option<ArgumentValue>> {
    // Look for return value in the output
    // Example: "Return value: i128:42"
    if let Some(line) = output.lines().find(|line| line.contains("Return value")) {
        if let Some(value_str) = line.split(':').nth(1) {
            let value_str = value_str.trim();
            return parse_argument_value(value_str);
        }
    }
    
    Ok(None)
}

fn parse_argument_value(value_str: &str) -> Result<Option<ArgumentValue>> {
    if value_str.starts_with("i128:") {
        let val: i128 = value_str[5..].parse()
            .map_err(|_| anyhow!("Failed to parse i128 value"))?;
        Ok(Some(ArgumentValue::I128(val)))
    } else if value_str.starts_with("u64:") {
        let val: u64 = value_str[4..].parse()
            .map_err(|_| anyhow!("Failed to parse u64 value"))?;
        Ok(Some(ArgumentValue::U64(val)))
    } else if value_str.starts_with("bool:") {
        let val: bool = value_str[5..].parse()
            .map_err(|_| anyhow!("Failed to parse bool value"))?;
        Ok(Some(ArgumentValue::Bool(val)))
    } else if value_str.starts_with("string:") {
        let val = value_str[7..].to_string();
        Ok(Some(ArgumentValue::String(val)))
    } else if value_str == "null" {
        Ok(Some(ArgumentValue::None))
    } else {
        Ok(None)
    }
}

/// Generate execution trace for a function call
async fn generate_execution_trace(
    env: &TestEnvironment,
    function_name: &str,
    success: bool,
) -> Result<crate::differential_fuzzing::ExecutionTrace> {
    // In a real implementation, this would generate a detailed execution trace
    // For now, we'll create a basic trace
    
    use crate::differential_fuzzing::{ExecutionTrace, TraceEvent, TraceEventType};
    
    let mut trace = ExecutionTrace::new();
    
    trace.add_event(TraceEvent {
        event_type: TraceEventType::FunctionEntry,
        function_name: function_name.to_string(),
        line_number: 0,
        description: format!("Entering function {}", function_name),
        timestamp: std::time::SystemTime::now(),
    });
    
    if success {
        trace.add_event(TraceEvent {
            event_type: TraceEventType::FunctionExit,
            function_name: function_name.to_string(),
            line_number: 0,
            description: format!("Successfully exited function {}", function_name),
            timestamp: std::time::SystemTime::now(),
        });
    } else {
        trace.add_event(TraceEvent {
            event_type: TraceEventType::Error,
            function_name: function_name.to_string(),
            line_number: 0,
            description: format!("Function {} failed", function_name),
            timestamp: std::time::SystemTime::now(),
        });
    }
    
    Ok(trace)
}

/// Capture state changes from the contract
async fn capture_state_changes(env: &TestEnvironment) -> Result<Vec<StateChange>> {
    // In a real implementation, this would query the contract state before and after execution
    // For now, we'll return an empty vector
    
    Ok(Vec::new())
}
