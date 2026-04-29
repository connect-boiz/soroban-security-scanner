//! Input Generator for Differential Fuzzing
//! 
//! Generates edge case inputs and test data for comprehensive testing.

use crate::differential_fuzzing::{
    TestInput, TestArgument, ArgumentValue, ArgumentType, TestInputMetadata, EdgeCaseType
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;
use anyhow::Result;

/// Input generator for creating test inputs with edge cases
pub struct InputGenerator {
    rng: ChaCha8Rng,
    edge_case_types: Vec<EdgeCaseType>,
    complexity_weights: HashMap<EdgeCaseType, f64>,
}

impl InputGenerator {
    pub fn new(edge_case_types: Vec<EdgeCaseType>) -> Self {
        let mut complexity_weights = HashMap::new();
        complexity_weights.insert(EdgeCaseType::MaxI128, 0.8);
        complexity_weights.insert(EdgeCaseType::MinI128, 0.8);
        complexity_weights.insert(EdgeCaseType::ZeroValue, 0.1);
        complexity_weights.insert(EdgeCaseType::EmptyVector, 0.3);
        complexity_weights.insert(EdgeCaseType::LargeVector, 0.9);
        complexity_weights.insert(EdgeCaseType::EmptyString, 0.2);
        complexity_weights.insert(EdgeCaseType::LongString, 0.7);
        complexity_weights.insert(EdgeCaseType::NullAddress, 0.4);
        complexity_weights.insert(EdgeCaseType::MaxU64, 0.6);
        complexity_weights.insert(EdgeCaseType::ZeroU64, 0.1);
        complexity_weights.insert(EdgeCaseType::NegativeValue, 0.5);
        complexity_weights.insert(EdgeCaseType::BoundaryValue, 0.6);
        complexity_weights.insert(EdgeCaseType::RandomValue, 0.5);

        Self {
            rng: ChaCha8Rng::from_entropy(),
            edge_case_types,
            complexity_weights,
        }
    }

    /// Generate a specified number of test inputs
    pub fn generate_test_inputs(&mut self, count: usize) -> Result<Vec<TestInput>> {
        let mut inputs = Vec::new();
        
        for i in 0..count {
            let edge_case_type = self.select_edge_case_type(i);
            let input = self.generate_single_input(edge_case_type)?;
            inputs.push(input);
        }
        
        Ok(inputs)
    }

    /// Select an edge case type based on the current index
    fn select_edge_case_type(&mut self, index: usize) -> EdgeCaseType {
        if self.edge_case_types.is_empty() {
            return EdgeCaseType::RandomValue;
        }
        
        // Cycle through edge case types with some randomness
        let base_index = index % self.edge_case_types.len();
        let random_offset = self.rng.gen_range(0..=2);
        let adjusted_index = (base_index + random_offset) % self.edge_case_types.len();
        
        self.edge_case_types[adjusted_index].clone()
    }

    /// Generate a single test input
    fn generate_single_input(&mut self, edge_case_type: EdgeCaseType) -> Result<TestInput> {
        let function_names = vec![
            "transfer", "approve", "mint", "burn", "balance", "allowance",
            "deposit", "withdraw", "stake", "unstake", "claim", "vote",
            "initialize", "upgrade", "pause", "unpause", "set_admin", "get_info"
        ];
        
        let function_name = function_names[self.rng.gen_range(0..function_names.len())].to_string();
        let arguments = self.generate_arguments(&function_name, &edge_case_type)?;
        let salt = Some(self.rng.gen());
        
        let complexity_score = self.complexity_weights.get(&edge_case_type)
            .unwrap_or(&0.5);
        
        let metadata = TestInputMetadata {
            edge_case_type: Some(edge_case_type),
            generation_method: "differential_fuzzing".to_string(),
            complexity_score: *complexity_score,
        };

        Ok(TestInput {
            function_name,
            arguments,
            salt,
            metadata,
        })
    }

    /// Generate arguments for a specific function
    fn generate_arguments(&mut self, function_name: &str, edge_case_type: &EdgeCaseType) -> Result<Vec<TestArgument>> {
        match function_name {
            "transfer" | "approve" => Ok(vec![
                TestArgument {
                    value: self.generate_address(edge_case_type)?,
                    argument_type: ArgumentType::Address,
                },
                TestArgument {
                    value: self.generate_i128(edge_case_type)?,
                    argument_type: ArgumentType::I128,
                },
            ]),
            "mint" | "burn" => Ok(vec![
                TestArgument {
                    value: self.generate_address(edge_case_type)?,
                    argument_type: ArgumentType::Address,
                },
                TestArgument {
                    value: self.generate_i128(edge_case_type)?,
                    argument_type: ArgumentType::I128,
                },
            ]),
            "balance" | "allowance" => Ok(vec![
                TestArgument {
                    value: self.generate_address(edge_case_type)?,
                    argument_type: ArgumentType::Address,
                },
            ]),
            "deposit" | "withdraw" => Ok(vec![
                TestArgument {
                    value: self.generate_i128(edge_case_type)?,
                    argument_type: ArgumentType::I128,
                },
            ]),
            "stake" | "unstake" => Ok(vec![
                TestArgument {
                    value: self.generate_address(edge_case_type)?,
                    argument_type: ArgumentType::Address,
                },
                TestArgument {
                    value: self.generate_i128(edge_case_type)?,
                    argument_type: ArgumentType::I128,
                },
            ]),
            "vote" => Ok(vec![
                TestArgument {
                    value: self.generate_address(edge_case_type)?,
                    argument_type: ArgumentType::Address,
                },
                TestArgument {
                    value: ArgumentValue::Bool(self.rng.gen()),
                    argument_type: ArgumentType::Bool,
                },
            ]),
            "initialize" => Ok(vec![
                TestArgument {
                    value: self.generate_address(edge_case_type)?,
                    argument_type: ArgumentType::Address,
                },
                TestArgument {
                    value: ArgumentValue::String("initial".to_string()),
                    argument_type: ArgumentType::String,
                },
            ]),
            "upgrade" => Ok(vec![
                TestArgument {
                    value: self.generate_bytes(edge_case_type)?,
                    argument_type: ArgumentType::Bytes,
                },
            ]),
            "pause" | "unpause" => Ok(vec![
                TestArgument {
                    value: ArgumentValue::Bool(self.rng.gen()),
                    argument_type: ArgumentType::Bool,
                },
            ]),
            "set_admin" => Ok(vec![
                TestArgument {
                    value: self.generate_address(edge_case_type)?,
                    argument_type: ArgumentType::Address,
                },
            ]),
            "get_info" => Ok(vec![]),
            _ => Ok(vec![]),
        }
    }

    /// Generate an i128 value based on edge case type
    fn generate_i128(&mut self, edge_case_type: &EdgeCaseType) -> Result<ArgumentValue> {
        let value = match edge_case_type {
            EdgeCaseType::MaxI128 => i128::MAX,
            EdgeCaseType::MinI128 => i128::MIN,
            EdgeCaseType::ZeroValue => 0,
            EdgeCaseType::NegativeValue => -1,
            EdgeCaseType::BoundaryValue => {
                // Generate values near boundaries
                let boundaries = vec![i128::MAX - 1, i128::MIN + 1, 1, -1];
                boundaries[self.rng.gen_range(0..boundaries.len())]
            },
            EdgeCaseType::RandomValue => self.rng.gen_range(-1000000..1000000),
            _ => self.rng.gen_range(-1000..1000),
        };
        
        Ok(ArgumentValue::I128(value))
    }

    /// Generate a u64 value based on edge case type
    fn generate_u64(&mut self, edge_case_type: &EdgeCaseType) -> Result<ArgumentValue> {
        let value = match edge_case_type {
            EdgeCaseType::MaxU64 => u64::MAX,
            EdgeCaseType::ZeroU64 => 0,
            EdgeCaseType::ZeroValue => 0,
            EdgeCaseType::BoundaryValue => {
                let boundaries = vec![u64::MAX - 1, 1];
                boundaries[self.rng.gen_range(0..boundaries.len())]
            },
            EdgeCaseType::RandomValue => self.rng.gen_range(0..1000000),
            _ => self.rng.gen_range(0..1000),
        };
        
        Ok(ArgumentValue::U64(value))
    }

    /// Generate an address based on edge case type
    fn generate_address(&mut self, edge_case_type: &EdgeCaseType) -> Result<ArgumentValue> {
        let address = match edge_case_type {
            EdgeCaseType::NullAddress => [0u8; 32],
            EdgeCaseType::ZeroValue => [0u8; 32],
            _ => {
                let mut addr = [0u8; 32];
                self.rng.fill(&mut addr);
                addr
            }
        };
        
        Ok(ArgumentValue::Address(address))
    }

    /// Generate bytes based on edge case type
    fn generate_bytes(&mut self, edge_case_type: &EdgeCaseType) -> Result<ArgumentValue> {
        let bytes = match edge_case_type {
            EdgeCaseType::EmptyVector => vec![],
            EdgeCaseType::LargeVector => {
                let size = self.rng.gen_range(10000..50000);
                let mut bytes = vec![0u8; size];
                self.rng.fill(&mut bytes);
                bytes
            },
            EdgeCaseType::ZeroValue => vec![0u8; 32],
            _ => {
                let size = self.rng.gen_range(32..1024);
                let mut bytes = vec![0u8; size];
                self.rng.fill(&mut bytes);
                bytes
            }
        };
        
        Ok(ArgumentValue::Bytes(bytes))
    }

    /// Generate a string based on edge case type
    fn generate_string(&mut self, edge_case_type: &EdgeCaseType) -> Result<ArgumentValue> {
        let string = match edge_case_type {
            EdgeCaseType::EmptyString => String::new(),
            EdgeCaseType::LongString => {
                let size = self.rng.gen_range(1000..10000);
                self.rng.sample_iter(&rand::distributions::Alphanumeric)
                    .take(size)
                    .map(char::from)
                    .collect()
            },
            EdgeCaseType::ZeroValue => String::new(),
            _ => {
                let size = self.rng.gen_range(5..100);
                self.rng.sample_iter(&rand::distributions::Alphanumeric)
                    .take(size)
                    .map(char::from)
                    .collect()
            }
        };
        
        Ok(ArgumentValue::String(string))
    }

    /// Generate a vector based on edge case type
    fn generate_vector(&mut self, edge_case_type: &EdgeCaseType, element_type: &ArgumentType) -> Result<ArgumentValue> {
        let elements = match edge_case_type {
            EdgeCaseType::EmptyVector => vec![],
            EdgeCaseType::LargeVector => {
                let size = self.rng.gen_range(1000..5000);
                (0..size).map(|_| self.generate_argument_value(element_type, edge_case_type))
                    .collect::<Result<Vec<_>>>()?
            },
            _ => {
                let size = self.rng.gen_range(1..10);
                (0..size).map(|_| self.generate_argument_value(element_type, edge_case_type))
                    .collect::<Result<Vec<_>>>()?
            }
        };
        
        Ok(ArgumentValue::Vector(elements))
    }

    /// Generate a single argument value
    fn generate_argument_value(&mut self, arg_type: &ArgumentType, edge_case_type: &EdgeCaseType) -> Result<ArgumentValue> {
        match arg_type {
            ArgumentType::I128 => self.generate_i128(edge_case_type),
            ArgumentType::U64 => self.generate_u64(edge_case_type),
            ArgumentType::U32 => Ok(ArgumentValue::U32(self.rng.gen())),
            ArgumentType::Bool => Ok(ArgumentValue::Bool(self.rng.gen())),
            ArgumentType::Bytes => self.generate_bytes(edge_case_type),
            ArgumentType::String => self.generate_string(edge_case_type),
            ArgumentType::Address => self.generate_address(edge_case_type),
            ArgumentType::Vector(element_type) => self.generate_vector(edge_case_type, element_type),
            ArgumentType::Map => self.generate_map(edge_case_type),
            ArgumentType::Option(inner_type) => {
                if self.rng.gen_bool(0.3) {
                    Ok(ArgumentValue::None)
                } else {
                    self.generate_argument_value(inner_type, edge_case_type)
                }
            },
        }
    }

    /// Generate a map based on edge case type
    fn generate_map(&mut self, edge_case_type: &EdgeCaseType) -> Result<ArgumentValue> {
        let size = match edge_case_type {
            EdgeCaseType::EmptyVector => 0,
            EdgeCaseType::LargeVector => self.rng.gen_range(100..500),
            _ => self.rng.gen_range(1..10),
        };
        
        let mut map = HashMap::new();
        for i in 0..size {
            let key = format!("key_{}", i);
            let value = self.generate_argument_value(&ArgumentType::String, edge_case_type)?;
            map.insert(key, value);
        }
        
        Ok(ArgumentValue::Map(map))
    }

    /// Generate inputs specifically for cross-contract call testing
    pub fn generate_cross_contract_inputs(&mut self, count: usize) -> Result<Vec<TestInput>> {
        let mut inputs = Vec::new();
        
        for _ in 0..count {
            let input = self.generate_cross_contract_input()?;
            inputs.push(input);
        }
        
        Ok(inputs)
    }

    /// Generate a single input for cross-contract testing
    fn generate_cross_contract_input(&mut self) -> Result<TestInput> {
        let function_names = vec![
            "transfer", "approve", "call_external", "delegate_call", "forward_call"
        ];
        
        let function_name = function_names[self.rng.gen_range(0..function_names.len())].to_string();
        let arguments = self.generate_cross_contract_arguments(&function_name)?;
        
        let metadata = TestInputMetadata {
            edge_case_type: Some(EdgeCaseType::Custom("cross_contract".to_string())),
            generation_method: "cross_contract_testing".to_string(),
            complexity_score: 0.8,
        };

        Ok(TestInput {
            function_name,
            arguments,
            salt: Some(self.rng.gen()),
            metadata,
        })
    }

    /// Generate arguments for cross-contract function calls
    fn generate_cross_contract_arguments(&mut self, function_name: &str) -> Result<Vec<TestArgument>> {
        match function_name {
            "call_external" | "delegate_call" | "forward_call" => Ok(vec![
                TestArgument {
                    value: self.generate_address(&EdgeCaseType::RandomValue)?,
                    argument_type: ArgumentType::Address,
                },
                TestArgument {
                    value: ArgumentValue::String("external_function".to_string()),
                    argument_type: ArgumentType::String,
                },
                TestArgument {
                    value: self.generate_i128(&EdgeCaseType::RandomValue)?,
                    argument_type: ArgumentType::I128,
                },
            ]),
            _ => self.generate_arguments(function_name, &EdgeCaseType::RandomValue),
        }
    }

    /// Generate boundary value inputs for comprehensive testing
    pub fn generate_boundary_inputs(&mut self) -> Result<Vec<TestInput>> {
        let mut inputs = Vec::new();
        
        let boundary_values = vec![
            EdgeCaseType::MaxI128,
            EdgeCaseType::MinI128,
            EdgeCaseType::MaxU64,
            EdgeCaseType::ZeroValue,
            EdgeCaseType::EmptyVector,
            EdgeCaseType::NullAddress,
        ];
        
        for edge_case in boundary_values {
            for _ in 0..10 { // Generate 10 inputs per boundary case
                let input = self.generate_single_input(edge_case.clone())?;
                inputs.push(input);
            }
        }
        
        Ok(inputs)
    }
}
