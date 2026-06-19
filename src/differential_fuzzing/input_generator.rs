//! Input Generator for Differential Fuzzing
//!
//! Generates edge case inputs and test data for comprehensive testing.

use crate::differential_fuzzing::{
    ArgumentType, ArgumentValue, EdgeCaseType, TestArgument, TestInput, TestInputMetadata,
};
use anyhow::Result;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::{hash_map::DefaultHasher, HashMap, HashSet};
use std::hash::{Hash, Hasher};

/// Predefined composite edge case scenarios that combine multiple edge conditions
/// in a single function call to uncover vulnerabilities that only manifest at the
/// intersection of extreme values.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompositeScenario {
    /// Self-transfer: two Address params set to the same address
    SelfTransfer,
    /// Transfer to null/zero address
    ZeroAddressTransfer,
    /// Transfer from null/zero address
    ZeroAddressSource,
    /// Both sender and recipient are the same null address
    SelfNullTransfer,
    /// MaxI128 on all i128 params simultaneously
    AllMaxI128,
    /// MinI128 on all i128 params simultaneously
    AllMinI128,
    /// One i128 param = MaxI128, another i128 param = MinI128 (max discrepancy)
    MaxMinI128,
    /// MaxI128 on all numeric params + same address on all Address params
    MaxOverload,
    /// Overflow through multiple accumulative operations (MaxI128 + MaxI128)
    OverflowAccumulative,
    /// Empty vector with LongString
    EmptyVectorLongString,
    /// LargeVector with MaxI128
    LargeVectorMaxI128,
    /// MaxU64 with ZeroU64 (max discrepancy for u64)
    MaxMinU64,
    /// NullAddress with MaxI128
    NullAddressMaxAmount,
    /// Custom user-defined composite
    Custom(String),
}

impl CompositeScenario {
    pub fn description(&self) -> &'static str {
        match self {
            CompositeScenario::SelfTransfer => "Same address for both sender and recipient",
            CompositeScenario::ZeroAddressTransfer => "Transfer to null/zero address",
            CompositeScenario::ZeroAddressSource => "Transfer from null/zero address",
            CompositeScenario::SelfNullTransfer => "Both sender and recipient are zero address",
            CompositeScenario::AllMaxI128 => "All i128 parameters at maximum value",
            CompositeScenario::AllMinI128 => "All i128 parameters at minimum value",
            CompositeScenario::MaxMinI128 => "Mix of MaxI128 and MinI128 across i128 params",
            CompositeScenario::MaxOverload => "All numeric params maxed + same address everywhere",
            CompositeScenario::OverflowAccumulative => "MaxI128 + MaxI128 to trigger overflow",
            CompositeScenario::EmptyVectorLongString => "Empty vector combined with long string",
            CompositeScenario::LargeVectorMaxI128 => "Large vector combined with MaxI128",
            CompositeScenario::MaxMinU64 => "Mix of MaxU64 and ZeroU64",
            CompositeScenario::NullAddressMaxAmount => "Null address with maximum amount",
            CompositeScenario::Custom(_) => "Custom composite scenario",
        }
    }
}

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
            "transfer",
            "approve",
            "mint",
            "burn",
            "balance",
            "allowance",
            "deposit",
            "withdraw",
            "stake",
            "unstake",
            "claim",
            "vote",
            "initialize",
            "upgrade",
            "pause",
            "unpause",
            "set_admin",
            "get_info",
        ];

        let function_name = function_names[self.rng.gen_range(0..function_names.len())].to_string();
        let arguments = self.generate_arguments(&function_name, &edge_case_type)?;
        let salt = Some(self.rng.gen());

        let complexity_score = self.complexity_weights.get(&edge_case_type).unwrap_or(&0.5);

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
    fn generate_arguments(
        &mut self,
        function_name: &str,
        edge_case_type: &EdgeCaseType,
    ) -> Result<Vec<TestArgument>> {
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
            "balance" | "allowance" => Ok(vec![TestArgument {
                value: self.generate_address(edge_case_type)?,
                argument_type: ArgumentType::Address,
            }]),
            "deposit" | "withdraw" => Ok(vec![TestArgument {
                value: self.generate_i128(edge_case_type)?,
                argument_type: ArgumentType::I128,
            }]),
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
            "upgrade" => Ok(vec![TestArgument {
                value: self.generate_bytes(edge_case_type)?,
                argument_type: ArgumentType::Bytes,
            }]),
            "pause" | "unpause" => Ok(vec![TestArgument {
                value: ArgumentValue::Bool(self.rng.gen()),
                argument_type: ArgumentType::Bool,
            }]),
            "set_admin" => Ok(vec![TestArgument {
                value: self.generate_address(edge_case_type)?,
                argument_type: ArgumentType::Address,
            }]),
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
            }
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
            }
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
            }
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
                self.rng
                    .sample_iter(&rand::distributions::Alphanumeric)
                    .take(size)
                    .map(char::from)
                    .collect()
            }
            EdgeCaseType::ZeroValue => String::new(),
            _ => {
                let size = self.rng.gen_range(5..100);
                self.rng
                    .sample_iter(&rand::distributions::Alphanumeric)
                    .take(size)
                    .map(char::from)
                    .collect()
            }
        };

        Ok(ArgumentValue::String(string))
    }

    /// Generate a vector based on edge case type
    fn generate_vector(
        &mut self,
        edge_case_type: &EdgeCaseType,
        element_type: &ArgumentType,
    ) -> Result<ArgumentValue> {
        let elements = match edge_case_type {
            EdgeCaseType::EmptyVector => vec![],
            EdgeCaseType::LargeVector => {
                let size = self.rng.gen_range(1000..5000);
                (0..size)
                    .map(|_| self.generate_argument_value(element_type, edge_case_type))
                    .collect::<Result<Vec<_>>>()?
            }
            _ => {
                let size = self.rng.gen_range(1..10);
                (0..size)
                    .map(|_| self.generate_argument_value(element_type, edge_case_type))
                    .collect::<Result<Vec<_>>>()?
            }
        };

        Ok(ArgumentValue::Vector(elements))
    }

    /// Generate a single argument value
    fn generate_argument_value(
        &mut self,
        arg_type: &ArgumentType,
        edge_case_type: &EdgeCaseType,
    ) -> Result<ArgumentValue> {
        match arg_type {
            ArgumentType::I128 => self.generate_i128(edge_case_type),
            ArgumentType::U64 => self.generate_u64(edge_case_type),
            ArgumentType::U32 => Ok(ArgumentValue::U32(self.rng.gen())),
            ArgumentType::Bool => Ok(ArgumentValue::Bool(self.rng.gen())),
            ArgumentType::Bytes => self.generate_bytes(edge_case_type),
            ArgumentType::String => self.generate_string(edge_case_type),
            ArgumentType::Address => self.generate_address(edge_case_type),
            ArgumentType::Vector(element_type) => {
                self.generate_vector(edge_case_type, element_type)
            }
            ArgumentType::Map => self.generate_map(edge_case_type),
            ArgumentType::Option(inner_type) => {
                if self.rng.gen_bool(0.3) {
                    Ok(ArgumentValue::None)
                } else {
                    self.generate_argument_value(inner_type, edge_case_type)
                }
            }
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
            "transfer",
            "approve",
            "call_external",
            "delegate_call",
            "forward_call",
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
    fn generate_cross_contract_arguments(
        &mut self,
        function_name: &str,
    ) -> Result<Vec<TestArgument>> {
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
            for _ in 0..10 {
                // Generate 10 inputs per boundary case
                let input = self.generate_single_input(edge_case.clone())?;
                inputs.push(input);
            }
        }

        Ok(inputs)
    }

    /// Generate composite edge case inputs by combining multiple edge condition types
    /// across all parameters of multi-argument functions. Returns deduplicated inputs
    /// limited by `max_inputs`.
    pub fn generate_composite_inputs(
        &mut self,
        combinatorial_depth: usize,
        max_inputs: usize,
    ) -> Result<Vec<TestInput>> {
        let mut inputs = Vec::new();
        let mut seen = HashSet::new();

        // Define composite scenarios to generate
        let scenarios = self.build_composite_scenario_list(combinatorial_depth);

        for scenario in scenarios {
            if inputs.len() >= max_inputs {
                break;
            }
            let composite_inputs = self.generate_from_scenario(&scenario, combinatorial_depth)?;
            for input in composite_inputs {
                if inputs.len() >= max_inputs {
                    break;
                }
                // Deduplication via fingerprint
                let fingerprint = self.compute_input_fingerprint(&input);
                if seen.insert(fingerprint) {
                    inputs.push(input);
                }
            }
        }

        Ok(inputs)
    }

    /// Build the list of composite scenarios to generate based on combinatorial depth
    fn build_composite_scenario_list(&self, depth: usize) -> Vec<Vec<CompositeScenario>> {
        let base_scenarios = vec![
            CompositeScenario::SelfTransfer,
            CompositeScenario::ZeroAddressTransfer,
            CompositeScenario::ZeroAddressSource,
            CompositeScenario::SelfNullTransfer,
            CompositeScenario::AllMaxI128,
            CompositeScenario::AllMinI128,
            CompositeScenario::MaxMinI128,
            CompositeScenario::MaxOverload,
            CompositeScenario::OverflowAccumulative,
            CompositeScenario::EmptyVectorLongString,
            CompositeScenario::LargeVectorMaxI128,
            CompositeScenario::MaxMinU64,
            CompositeScenario::NullAddressMaxAmount,
        ];

        // Generate up to the specified depth (Cartesian product of scenarios not needed
        // for depth > 1 since each scenario is already composite)
        base_scenarios.into_iter().map(|s| vec![s]).collect()
    }

    /// Generate test inputs for a specific composite scenario
    fn generate_from_scenario(
        &mut self,
        scenarios: &[CompositeScenario],
        _depth: usize,
    ) -> Result<Vec<TestInput>> {
        let function_names: Vec<&str> = match scenarios.as_slice() {
            // Multi-address functions get special treatment for self-transfer etc.
            [CompositeScenario::SelfTransfer]
            | [CompositeScenario::ZeroAddressTransfer]
            | [CompositeScenario::ZeroAddressSource]
            | [CompositeScenario::SelfNullTransfer]
            | [CompositeScenario::NullAddressMaxAmount]
            | [CompositeScenario::MaxOverload] => {
                vec!["transfer", "approve", "mint", "stake", "vote"]
            }
            // Multi-i128 functions get special treatment for max/min combos
            [CompositeScenario::AllMaxI128]
            | [CompositeScenario::AllMinI128]
            | [CompositeScenario::MaxMinI128]
            | [CompositeScenario::OverflowAccumulative] => {
                vec!["transfer", "approve", "mint", "burn", "deposit", "withdraw"]
            }
            // Combined vector + value scenarios
            [CompositeScenario::EmptyVectorLongString]
            | [CompositeScenario::LargeVectorMaxI128] => {
                vec!["upgrade", "call_external"]
            }
            // U64-focused scenarios
            [CompositeScenario::MaxMinU64] => {
                vec!["stake", "unstake"]
            }
            _ => {
                vec!["transfer", "approve", "mint", "burn", "deposit", "withdraw"]
            }
        };

        let mut inputs = Vec::new();
        for &fn_name in &function_names {
            let input = self.generate_composite_input_for_function(fn_name, scenarios)?;
            inputs.push(input);
        }

        Ok(inputs)
    }

    /// Generate a single composite test input for a specific function and scenario
    fn generate_composite_input_for_function(
        &mut self,
        function_name: &str,
        scenarios: &[CompositeScenario],
    ) -> Result<TestInput> {
        let primary_scenario = scenarios
            .first()
            .cloned()
            .unwrap_or(CompositeScenario::SelfTransfer);

        let arguments = self.generate_composite_arguments(function_name, &primary_scenario)?;
        let complexity_score = self.composite_complexity_score(&primary_scenario);
        let _description = primary_scenario.description();

        let metadata = TestInputMetadata {
            edge_case_type: Some(EdgeCaseType::Custom(format!(
                "composite:{:?}",
                primary_scenario
            ))),
            generation_method: "composite_edge_case".to_string(),
            complexity_score,
        };

        Ok(TestInput {
            function_name: function_name.to_string(),
            arguments,
            salt: Some(self.rng.gen()),
            metadata,
        })
    }

    /// Generate arguments for a composite scenario, applying the cross-parameter
    /// edge conditions that define the scenario.
    fn generate_composite_arguments(
        &mut self,
        function_name: &str,
        scenario: &CompositeScenario,
    ) -> Result<Vec<TestArgument>> {
        match scenario {
            CompositeScenario::SelfTransfer => {
                // Both Address params get the same address
                let same_addr = self.generate_same_address();
                match function_name {
                    "transfer" | "approve" | "stake" | "mint" => Ok(vec![
                        TestArgument {
                            value: same_addr.clone(),
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: same_addr,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: ArgumentValue::I128(self.rng.gen_range(1..1000)),
                            argument_type: ArgumentType::I128,
                        },
                    ]),
                    "vote" => Ok(vec![
                        TestArgument {
                            value: same_addr.clone(),
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: same_addr,
                            argument_type: ArgumentType::Address,
                        },
                    ]),
                    _ => Ok(vec![
                        TestArgument {
                            value: same_addr.clone(),
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: same_addr,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: ArgumentValue::I128(1),
                            argument_type: ArgumentType::I128,
                        },
                    ]),
                }
            }
            CompositeScenario::ZeroAddressTransfer => {
                // Recipient address is null/zero
                let sender = self.generate_address(&EdgeCaseType::RandomValue)?;
                let zero_addr = ArgumentValue::Address([0u8; 32]);
                match function_name {
                    "transfer" | "approve" => Ok(vec![
                        TestArgument {
                            value: sender,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: zero_addr,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: ArgumentValue::I128(self.rng.gen_range(1..1000)),
                            argument_type: ArgumentType::I128,
                        },
                    ]),
                    _ => Ok(vec![
                        TestArgument {
                            value: sender,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: zero_addr,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: ArgumentValue::I128(1),
                            argument_type: ArgumentType::I128,
                        },
                    ]),
                }
            }
            CompositeScenario::ZeroAddressSource => {
                // Sender address is null/zero
                let zero_addr = ArgumentValue::Address([0u8; 32]);
                let recipient = self.generate_address(&EdgeCaseType::RandomValue)?;
                match function_name {
                    "transfer" | "approve" => Ok(vec![
                        TestArgument {
                            value: zero_addr,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: recipient,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: ArgumentValue::I128(self.rng.gen_range(1..1000)),
                            argument_type: ArgumentType::I128,
                        },
                    ]),
                    _ => Ok(vec![
                        TestArgument {
                            value: zero_addr,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: recipient,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: ArgumentValue::I128(1),
                            argument_type: ArgumentType::I128,
                        },
                    ]),
                }
            }
            CompositeScenario::SelfNullTransfer => {
                // Both addresses are null/zero
                let zero_addr = ArgumentValue::Address([0u8; 32]);
                Ok(vec![
                    TestArgument {
                        value: zero_addr.clone(),
                        argument_type: ArgumentType::Address,
                    },
                    TestArgument {
                        value: zero_addr,
                        argument_type: ArgumentType::Address,
                    },
                    TestArgument {
                        value: ArgumentValue::I128(0),
                        argument_type: ArgumentType::I128,
                    },
                ])
            }
            CompositeScenario::NullAddressMaxAmount => {
                // Null address with MaxI128 amount
                let zero_addr = ArgumentValue::Address([0u8; 32]);
                match function_name {
                    "mint" | "burn" => Ok(vec![
                        TestArgument {
                            value: zero_addr,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: ArgumentValue::I128(i128::MAX),
                            argument_type: ArgumentType::I128,
                        },
                    ]),
                    "transfer" | "approve" | "stake" => Ok(vec![
                        TestArgument {
                            value: self.generate_address(&EdgeCaseType::RandomValue)?,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: zero_addr,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: ArgumentValue::I128(i128::MAX),
                            argument_type: ArgumentType::I128,
                        },
                    ]),
                    "deposit" | "withdraw" => Ok(vec![TestArgument {
                        value: ArgumentValue::I128(i128::MAX),
                        argument_type: ArgumentType::I128,
                    }]),
                    _ => Ok(vec![
                        TestArgument {
                            value: zero_addr,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: ArgumentValue::I128(i128::MAX),
                            argument_type: ArgumentType::I128,
                        },
                    ]),
                }
            }
            CompositeScenario::AllMaxI128 => {
                // All i128 params get MaxI128
                match function_name {
                    "transfer" | "approve" | "mint" | "burn" | "stake" | "unstake" => Ok(vec![
                        TestArgument {
                            value: self.generate_address(&EdgeCaseType::RandomValue)?,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: self.generate_address(&EdgeCaseType::RandomValue)?,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: ArgumentValue::I128(i128::MAX),
                            argument_type: ArgumentType::I128,
                        },
                    ]),
                    "deposit" | "withdraw" => Ok(vec![TestArgument {
                        value: ArgumentValue::I128(i128::MAX),
                        argument_type: ArgumentType::I128,
                    }]),
                    _ => Ok(vec![
                        TestArgument {
                            value: self.generate_address(&EdgeCaseType::RandomValue)?,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: ArgumentValue::I128(i128::MAX),
                            argument_type: ArgumentType::I128,
                        },
                    ]),
                }
            }
            CompositeScenario::AllMinI128 => {
                // All i128 params get MinI128
                match function_name {
                    "transfer" | "approve" | "mint" | "burn" | "stake" | "unstake" => Ok(vec![
                        TestArgument {
                            value: self.generate_address(&EdgeCaseType::RandomValue)?,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: self.generate_address(&EdgeCaseType::RandomValue)?,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: ArgumentValue::I128(i128::MIN),
                            argument_type: ArgumentType::I128,
                        },
                    ]),
                    "deposit" | "withdraw" => Ok(vec![TestArgument {
                        value: ArgumentValue::I128(i128::MIN),
                        argument_type: ArgumentType::I128,
                    }]),
                    _ => Ok(vec![
                        TestArgument {
                            value: self.generate_address(&EdgeCaseType::RandomValue)?,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: ArgumentValue::I128(i128::MIN),
                            argument_type: ArgumentType::I128,
                        },
                    ]),
                }
            }
            CompositeScenario::MaxMinI128 => {
                // One i128 param = MaxI128, another = MinI128 (if possible) or just max
                match function_name {
                    "transfer" | "approve" | "mint" | "burn" | "stake" | "unstake" => Ok(vec![
                        TestArgument {
                            value: self.generate_address(&EdgeCaseType::RandomValue)?,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: self.generate_address(&EdgeCaseType::RandomValue)?,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: ArgumentValue::I128(i128::MAX),
                            argument_type: ArgumentType::I128,
                        },
                    ]),
                    "deposit" | "withdraw" => {
                        // Only one i128 param, so we combine MaxI128 + MinI128
                        // by generating two separate inputs, one with each
                        let val = if self.rng.gen_bool(0.5) {
                            i128::MAX
                        } else {
                            i128::MIN
                        };
                        Ok(vec![TestArgument {
                            value: ArgumentValue::I128(val),
                            argument_type: ArgumentType::I128,
                        }])
                    }
                    _ => Ok(vec![
                        TestArgument {
                            value: self.generate_address(&EdgeCaseType::RandomValue)?,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: ArgumentValue::I128(i128::MAX),
                            argument_type: ArgumentType::I128,
                        },
                    ]),
                }
            }
            CompositeScenario::MaxOverload => {
                // All i128 params maxed + same address everywhere
                let same_addr = self.generate_same_address();
                Ok(vec![
                    TestArgument {
                        value: same_addr.clone(),
                        argument_type: ArgumentType::Address,
                    },
                    TestArgument {
                        value: same_addr,
                        argument_type: ArgumentType::Address,
                    },
                    TestArgument {
                        value: ArgumentValue::I128(i128::MAX),
                        argument_type: ArgumentType::I128,
                    },
                ])
            }
            CompositeScenario::OverflowAccumulative => {
                // Simulate overflow through multiple accumulative ops:
                // Use MaxI128 as the base, then add more via a large amount
                match function_name {
                    "deposit" | "withdraw" => Ok(vec![TestArgument {
                        value: ArgumentValue::I128(i128::MAX),
                        argument_type: ArgumentType::I128,
                    }]),
                    "mint" | "burn" => {
                        // For mint/burn with same address, MaxI128 on balance + MaxI128 amount = overflow
                        let addr = self.generate_same_address();
                        Ok(vec![
                            TestArgument {
                                value: addr,
                                argument_type: ArgumentType::Address,
                            },
                            TestArgument {
                                value: ArgumentValue::I128(i128::MAX),
                                argument_type: ArgumentType::I128,
                            },
                        ])
                    }
                    _ => Ok(vec![
                        TestArgument {
                            value: self.generate_address(&EdgeCaseType::RandomValue)?,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: self.generate_address(&EdgeCaseType::RandomValue)?,
                            argument_type: ArgumentType::Address,
                        },
                        TestArgument {
                            value: ArgumentValue::I128(i128::MAX),
                            argument_type: ArgumentType::I128,
                        },
                    ]),
                }
            }
            CompositeScenario::EmptyVectorLongString => Ok(vec![
                TestArgument {
                    value: ArgumentValue::Bytes(vec![]),
                    argument_type: ArgumentType::Bytes,
                },
                TestArgument {
                    value: ArgumentValue::String("A".repeat(5000)),
                    argument_type: ArgumentType::String,
                },
            ]),
            CompositeScenario::LargeVectorMaxI128 => Ok(vec![
                TestArgument {
                    value: ArgumentValue::Bytes(vec![0u8; 50000]),
                    argument_type: ArgumentType::Bytes,
                },
                TestArgument {
                    value: ArgumentValue::I128(i128::MAX),
                    argument_type: ArgumentType::I128,
                },
            ]),
            CompositeScenario::MaxMinU64 => match function_name {
                "stake" | "unstake" => Ok(vec![
                    TestArgument {
                        value: self.generate_address(&EdgeCaseType::RandomValue)?,
                        argument_type: ArgumentType::Address,
                    },
                    TestArgument {
                        value: ArgumentValue::U64(u64::MAX),
                        argument_type: ArgumentType::U64,
                    },
                ]),
                _ => Ok(vec![
                    TestArgument {
                        value: self.generate_address(&EdgeCaseType::RandomValue)?,
                        argument_type: ArgumentType::Address,
                    },
                    TestArgument {
                        value: ArgumentValue::U64(u64::MAX),
                        argument_type: ArgumentType::U64,
                    },
                ]),
            },
            CompositeScenario::Custom(_) => {
                // Fall back to basic generation for custom scenarios
                self.generate_arguments(function_name, &EdgeCaseType::BoundaryValue)
            }
        }
    }

    /// Generate a deterministic address used for both sender and recipient
    /// in self-transfer scenarios
    fn generate_same_address(&mut self) -> ArgumentValue {
        let mut addr = [0u8; 32];
        self.rng.fill(&mut addr);
        ArgumentValue::Address(addr)
    }

    /// Compute a deterministic fingerprint for deduplication
    fn compute_input_fingerprint(&self, input: &TestInput) -> String {
        fingerprint_input(input)
    }

    /// Calculate complexity score for a composite scenario
    fn composite_complexity_score(&self, scenario: &CompositeScenario) -> f64 {
        match scenario {
            CompositeScenario::SelfTransfer => 0.9,
            CompositeScenario::ZeroAddressTransfer => 0.7,
            CompositeScenario::ZeroAddressSource => 0.7,
            CompositeScenario::SelfNullTransfer => 0.95,
            CompositeScenario::AllMaxI128 => 0.9,
            CompositeScenario::AllMinI128 => 0.9,
            CompositeScenario::MaxMinI128 => 0.95,
            CompositeScenario::MaxOverload => 1.0,
            CompositeScenario::OverflowAccumulative => 0.95,
            CompositeScenario::EmptyVectorLongString => 0.8,
            CompositeScenario::LargeVectorMaxI128 => 0.95,
            CompositeScenario::MaxMinU64 => 0.7,
            CompositeScenario::NullAddressMaxAmount => 0.85,
            CompositeScenario::Custom(_) => 0.8,
        }
    }
}

/// Input fingerprint for deduplication (hash of function name + argument values)
pub fn fingerprint_input(input: &TestInput) -> String {
    let mut hasher = DefaultHasher::new();
    input.function_name.hash(&mut hasher);
    for arg in &input.arguments {
        format!("{:?}", arg.value).hash(&mut hasher);
    }
    format!("{:x}", hasher.finish())
}
