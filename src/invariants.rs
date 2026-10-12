//! Invariant checking rules for Stellar smart contracts

use crate::Severity;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InvariantRule {
    // Token Invariants
    TotalSupplyConsistency,
    BalanceNonNegative,
    TransferConservation,
    MintSupplyIncrease,
    BurnSupplyDecrease,
    SufficientBalanceCheck,
    BalanceBoundsCheck,
    TransferAtomicity,
    BalanceIntegrity,

    // Access Control Invariants
    AdminAuthorization,
    OwnershipConsistency,
    PermissionIntegrity,

    // Mathematical Invariants
    SumOfBalancesEqualsSupply,
    NoNegativeBalances,
    OverflowProtection,

    // State Consistency Invariants
    StateTransitionValidity,
    EventStateConsistency,
    TimestampMonotonicity,

    // Economic Invariants
    NoFreeMoney,
    ConservationOfValue,
    FairDistribution,

    // Stellar Specific Invariants
    StellarAssetIntegrity,
    AccountStateConsistency,
    SequenceNumberIntegrity,
    FeeConservation,
}

impl InvariantRule {
    pub fn description(&self) -> &'static str {
        match self {
            InvariantRule::TotalSupplyConsistency => {
                "Total token supply must always equal sum of all balances"
            }
            InvariantRule::BalanceNonNegative => "Account balances must never be negative",
            InvariantRule::TransferConservation => {
                "Transfers must conserve total value (sender decrease = receiver increase)"
            }
            InvariantRule::MintSupplyIncrease => {
                "Minting must increase total supply by minted amount"
            }
            InvariantRule::BurnSupplyDecrease => {
                "Burning must decrease total supply by burned amount"
            }
            InvariantRule::SufficientBalanceCheck => {
                "All transfers must verify sufficient balance before execution"
            }
            InvariantRule::BalanceBoundsCheck => "Account balances must stay within defined bounds",
            InvariantRule::TransferAtomicity => {
                "Transfer operations must be atomic and reversible on failure"
            }
            InvariantRule::BalanceIntegrity => "Balance operations must maintain data integrity",

            InvariantRule::AdminAuthorization => "Admin operations must be properly authorized",
            InvariantRule::OwnershipConsistency => {
                "Ownership must be consistent and transferable only by owner"
            }
            InvariantRule::PermissionIntegrity => {
                "Permission checks must be consistent across all functions"
            }

            InvariantRule::SumOfBalancesEqualsSupply => {
                "Sum of all user balances must equal total supply"
            }
            InvariantRule::NoNegativeBalances => "No account can have negative balance",
            InvariantRule::OverflowProtection => {
                "All arithmetic operations must be protected from overflow/underflow"
            }

            InvariantRule::StateTransitionValidity => {
                "All state transitions must be valid and authorized"
            }
            InvariantRule::EventStateConsistency => {
                "Emitted events must accurately reflect state changes"
            }
            InvariantRule::TimestampMonotonicity => "Timestamps must always increase",

            InvariantRule::NoFreeMoney => {
                "No mechanism should allow creation of value without corresponding input"
            }
            InvariantRule::ConservationOfValue => {
                "Total system value must be conserved in closed operations"
            }
            InvariantRule::FairDistribution => {
                "Distribution mechanisms must be fair and predictable"
            }

            InvariantRule::StellarAssetIntegrity => {
                "Stellar asset operations must maintain integrity"
            }
            InvariantRule::AccountStateConsistency => {
                "Account state must be consistent across operations"
            }
            InvariantRule::SequenceNumberIntegrity => "Sequence numbers must be properly managed",
            InvariantRule::FeeConservation => "Fees must be properly accounted for and conserved",
        }
    }

    pub fn check_pattern(&self) -> &'static str {
        match self {
            InvariantRule::TotalSupplyConsistency => r"total_supply.*=.*\+.*balance",
            InvariantRule::BalanceNonNegative => r"balance.*<.*0",
            InvariantRule::TransferConservation => r"transfer.*from.*to.*amount",
            InvariantRule::MintSupplyIncrease => r"mint.*total_supply.*\+=",
            InvariantRule::BurnSupplyDecrease => r"burn.*total_supply.*-=",
            InvariantRule::SufficientBalanceCheck => r"require.*balance.*>=|balance.*>=.*amount",
            InvariantRule::BalanceBoundsCheck => {
                r"balance.*<=.*max_balance|max_balance.*>=.*balance"
            }
            InvariantRule::TransferAtomicity => r"transfer.*\{[^}]*?balance.*-=.*balance.*\+=",
            InvariantRule::BalanceIntegrity => r"balance.*checked.*|checked.*balance",

            InvariantRule::AdminAuthorization => r"require_auth.*admin|admin.*require_auth",
            InvariantRule::OwnershipConsistency => r"owner.*transfer|transfer.*owner",
            InvariantRule::PermissionIntegrity => r"has_auth.*permission|permission.*has_auth",

            InvariantRule::SumOfBalancesEqualsSupply => r"sum.*balances.*total_supply",
            InvariantRule::NoNegativeBalances => r"balance.*>=.*0",
            InvariantRule::OverflowProtection => r"checked_add|checked_sub|checked_mul",

            InvariantRule::StateTransitionValidity => r"require.*state|state.*require",
            InvariantRule::EventStateConsistency => r"event.*emit.*state|state.*event.*emit",
            InvariantRule::TimestampMonotonicity => r"timestamp.*>.*prev_timestamp",

            InvariantRule::NoFreeMoney => r"mint.*require_auth|burn.*require_auth",
            InvariantRule::ConservationOfValue => r"value.*conservation|conservation.*value",
            InvariantRule::FairDistribution => r"distribute.*fair|fair.*distribute",

            InvariantRule::StellarAssetIntegrity => {
                r"stellar_asset.*validate|validate.*stellar_asset"
            }
            InvariantRule::AccountStateConsistency => r"account.*state.*consistency",
            InvariantRule::SequenceNumberIntegrity => r"sequence.*number.*integrity",
            InvariantRule::FeeConservation => r"fee.*accounting|accounting.*fee",
        }
    }

    pub fn severity(&self) -> Severity {
        match self {
            InvariantRule::TotalSupplyConsistency => Severity::Critical,
            InvariantRule::BalanceNonNegative => Severity::Critical,
            InvariantRule::TransferConservation => Severity::Critical,
            InvariantRule::NoFreeMoney => Severity::Critical,

            InvariantRule::MintSupplyIncrease => Severity::High,
            InvariantRule::BurnSupplyDecrease => Severity::High,
            InvariantRule::SufficientBalanceCheck => Severity::Critical,
            InvariantRule::BalanceBoundsCheck => Severity::High,
            InvariantRule::TransferAtomicity => Severity::Critical,
            InvariantRule::BalanceIntegrity => Severity::High,
            InvariantRule::AdminAuthorization => Severity::High,
            InvariantRule::OwnershipConsistency => Severity::High,
            InvariantRule::SumOfBalancesEqualsSupply => Severity::High,
            InvariantRule::NoNegativeBalances => Severity::High,

            InvariantRule::PermissionIntegrity => Severity::Medium,
            InvariantRule::OverflowProtection => Severity::Medium,
            InvariantRule::StateTransitionValidity => Severity::Medium,
            InvariantRule::EventStateConsistency => Severity::Medium,
            InvariantRule::ConservationOfValue => Severity::Medium,
            InvariantRule::StellarAssetIntegrity => Severity::Medium,
            InvariantRule::AccountStateConsistency => Severity::Medium,

            InvariantRule::TimestampMonotonicity => Severity::Low,
            InvariantRule::FairDistribution => Severity::Low,
            InvariantRule::SequenceNumberIntegrity => Severity::Low,
            InvariantRule::FeeConservation => Severity::Low,
        }
    }

    pub fn recommendation(&self) -> &'static str {
        match self {
            InvariantRule::TotalSupplyConsistency => {
                "Implement checks to ensure total_supply equals sum of all balances"
            }
            InvariantRule::BalanceNonNegative => {
                "Add checks to prevent negative balances in all operations"
            }
            InvariantRule::TransferConservation => "Validate that transfers conserve total value",
            InvariantRule::MintSupplyIncrease => {
                "Ensure minting operations properly increase total supply"
            }
            InvariantRule::BurnSupplyDecrease => {
                "Ensure burning operations properly decrease total supply"
            }
            InvariantRule::SufficientBalanceCheck => {
                "Add comprehensive balance verification before all transfers"
            }
            InvariantRule::BalanceBoundsCheck => {
                "Implement balance bounds validation to prevent overflow"
            }
            InvariantRule::TransferAtomicity => {
                "Ensure transfers are atomic with proper rollback on failure"
            }
            InvariantRule::BalanceIntegrity => "Use checked arithmetic for all balance operations",

            InvariantRule::AdminAuthorization => {
                "Implement proper admin authorization for all admin functions"
            }
            InvariantRule::OwnershipConsistency => {
                "Maintain ownership consistency across all operations"
            }
            InvariantRule::PermissionIntegrity => {
                "Ensure permission checks are consistent and comprehensive"
            }

            InvariantRule::SumOfBalancesEqualsSupply => {
                "Add invariant checks for supply vs balance consistency"
            }
            InvariantRule::NoNegativeBalances => "Implement negative balance protection",
            InvariantRule::OverflowProtection => {
                "Use safe arithmetic operations throughout the contract"
            }

            InvariantRule::StateTransitionValidity => {
                "Validate all state transitions before execution"
            }
            InvariantRule::EventStateConsistency => {
                "Ensure events accurately reflect state changes"
            }
            InvariantRule::TimestampMonotonicity => "Implement proper timestamp validation",

            InvariantRule::NoFreeMoney => "Ensure all value creation has corresponding input",
            InvariantRule::ConservationOfValue => "Implement value conservation checks",
            InvariantRule::FairDistribution => "Add fairness checks to distribution mechanisms",

            InvariantRule::StellarAssetIntegrity => "Validate all Stellar asset operations",
            InvariantRule::AccountStateConsistency => "Maintain account state consistency",
            InvariantRule::SequenceNumberIntegrity => "Properly manage sequence numbers",
            InvariantRule::FeeConservation => "Implement proper fee accounting",
        }
    }

    /// Returns the temporal logic modality associated with this invariant rule
    pub fn temporal_modality(&self) -> TemporalInvariant {
        match self {
            InvariantRule::TotalSupplyConsistency => TemporalInvariant::AlwaysHolds,
            InvariantRule::BalanceNonNegative => TemporalInvariant::AlwaysHolds,
            InvariantRule::TransferConservation => TemporalInvariant::AlwaysHolds,
            InvariantRule::MintSupplyIncrease => TemporalInvariant::MonotonicIncrease,
            InvariantRule::BurnSupplyDecrease => TemporalInvariant::MonotonicDecrease,
            InvariantRule::SufficientBalanceCheck => TemporalInvariant::AlwaysHolds,
            InvariantRule::BalanceBoundsCheck => TemporalInvariant::AlwaysHolds,
            InvariantRule::TransferAtomicity => TemporalInvariant::AlwaysHolds,
            InvariantRule::BalanceIntegrity => TemporalInvariant::AlwaysHolds,

            InvariantRule::AdminAuthorization => TemporalInvariant::AlwaysHolds,
            InvariantRule::OwnershipConsistency => TemporalInvariant::AlwaysHolds,
            InvariantRule::PermissionIntegrity => TemporalInvariant::AlwaysHolds,

            InvariantRule::SumOfBalancesEqualsSupply => TemporalInvariant::AlwaysHolds,
            InvariantRule::NoNegativeBalances => TemporalInvariant::NeverViolated,
            InvariantRule::OverflowProtection => TemporalInvariant::NeverViolated,

            InvariantRule::StateTransitionValidity => TemporalInvariant::AlwaysHolds,
            InvariantRule::EventStateConsistency => TemporalInvariant::EventuallyHolds,
            InvariantRule::TimestampMonotonicity => TemporalInvariant::MonotonicIncrease,

            InvariantRule::NoFreeMoney => TemporalInvariant::NeverViolated,
            InvariantRule::ConservationOfValue => TemporalInvariant::AlwaysHolds,
            InvariantRule::FairDistribution => TemporalInvariant::AlwaysHolds,

            InvariantRule::StellarAssetIntegrity => TemporalInvariant::AlwaysHolds,
            InvariantRule::AccountStateConsistency => TemporalInvariant::AlwaysHolds,
            InvariantRule::SequenceNumberIntegrity => TemporalInvariant::MonotonicIncrease,
            InvariantRule::FeeConservation => TemporalInvariant::AlwaysHolds,
        }
    }

    /// Verifies that this invariant rule holds temporally across all paths up to `max_depth`
    pub fn temporal_check(
        &self,
        diagram: &SequenceDiagram,
        max_depth: usize,
    ) -> Result<(), TemporalViolation> {
        let violations = self.find_temporal_violations(diagram, max_depth);
        if let Some(first) = violations.into_iter().next() {
            Err(first)
        } else {
            Ok(())
        }
    }

    /// Finds all temporal invariant violations across execution paths up to `max_depth`
    pub fn find_temporal_violations(
        &self,
        diagram: &SequenceDiagram,
        max_depth: usize,
    ) -> Vec<TemporalViolation> {
        let paths = diagram.explore_paths(max_depth);
        let mut violations = Vec::new();

        for path in &paths {
            if let Some(violation) = self.check_path(path) {
                violations.push(violation);
                if violations.len() >= 50 {
                    break;
                }
            }
        }

        violations
    }

    /// Evaluates whether this invariant holds across a specific execution transition path
    pub fn check_path(&self, path: &[StateTransition]) -> Option<TemporalViolation> {
        if path.is_empty() {
            return None;
        }

        let fn_sequence: Vec<String> = path.iter().map(|t| t.function.clone()).collect();

        match self {
            InvariantRule::ConservationOfValue => {
                // Total system value must be conserved in closed operations.
                // Any sequence where net token supply/value changes (mint or burn)
                // without external backing violates conservation of closed system value.
                for (idx, step) in path.iter().enumerate() {
                    let step_delta: i64 = step
                        .state_deltas
                        .iter()
                        .filter(|d| d.target == "total_supply" || d.target == "value")
                        .map(|d| d.value_effect)
                        .sum();

                    if step_delta != 0 {
                        return Some(TemporalViolation {
                            rule: InvariantRule::ConservationOfValue,
                            temporal_type: TemporalInvariant::AlwaysHolds,
                            path: fn_sequence,
                            description: format!(
                                "ConservationOfValue violated at operation '{}': value delta was {}, breaking closed value conservation in execution sequence",
                                step.function, step_delta
                            ),
                            step_at_fault: idx + 1,
                        });
                    }
                }
                None
            }

            InvariantRule::TimestampMonotonicity => {
                // Timestamps must monotonically increase along execution transitions
                for (idx, step) in path.iter().enumerate() {
                    for delta in &step.state_deltas {
                        if delta.target == "timestamp" && delta.value_effect < 0 {
                            return Some(TemporalViolation {
                                rule: InvariantRule::TimestampMonotonicity,
                                temporal_type: TemporalInvariant::MonotonicIncrease,
                                path: fn_sequence,
                                description: format!(
                                    "Timestamp monotonicity violated: step '{}' decreases timestamp",
                                    step.function
                                ),
                                step_at_fault: idx + 1,
                            });
                        }
                    }
                }
                None
            }

            InvariantRule::MintSupplyIncrease => {
                // Minting must monotonically increase total supply
                for (idx, step) in path.iter().enumerate() {
                    if step.function.contains("mint") {
                        let has_increase = step
                            .state_deltas
                            .iter()
                            .any(|d| d.target == "total_supply" && d.value_effect > 0);
                        if !has_increase {
                            return Some(TemporalViolation {
                                rule: InvariantRule::MintSupplyIncrease,
                                temporal_type: TemporalInvariant::MonotonicIncrease,
                                path: fn_sequence,
                                description: format!(
                                    "MintSupplyIncrease violated: mint operation '{}' does not increase total supply",
                                    step.function
                                ),
                                step_at_fault: idx + 1,
                            });
                        }
                    }
                }
                None
            }

            InvariantRule::BurnSupplyDecrease => {
                // Burning must monotonically decrease total supply
                for (idx, step) in path.iter().enumerate() {
                    if step.function.contains("burn") {
                        let has_decrease = step
                            .state_deltas
                            .iter()
                            .any(|d| d.target == "total_supply" && d.value_effect < 0);
                        if !has_decrease {
                            return Some(TemporalViolation {
                                rule: InvariantRule::BurnSupplyDecrease,
                                temporal_type: TemporalInvariant::MonotonicDecrease,
                                path: fn_sequence,
                                description: format!(
                                    "BurnSupplyDecrease violated: burn operation '{}' does not decrease total supply",
                                    step.function
                                ),
                                step_at_fault: idx + 1,
                            });
                        }
                    }
                }
                None
            }

            InvariantRule::TotalSupplyConsistency | InvariantRule::SumOfBalancesEqualsSupply => {
                for (idx, step) in path.iter().enumerate() {
                    let has_supply = step.state_deltas.iter().any(|d| d.target == "total_supply");
                    let has_balance = step.state_deltas.iter().any(|d| d.target == "balance");
                    if (has_supply && !has_balance)
                        || (!has_supply
                            && has_balance
                            && (step.function.contains("mint") || step.function.contains("burn")))
                    {
                        return Some(TemporalViolation {
                            rule: self.clone(),
                            temporal_type: TemporalInvariant::AlwaysHolds,
                            path: fn_sequence,
                            description: format!(
                                "Total supply and user balance state desynchronized at step '{}'",
                                step.function
                            ),
                            step_at_fault: idx + 1,
                        });
                    }
                }
                None
            }

            InvariantRule::NoFreeMoney => {
                for (idx, step) in path.iter().enumerate() {
                    let creates_value = step
                        .state_deltas
                        .iter()
                        .any(|d| (d.target == "balance" || d.target == "total_supply") && d.value_effect > 0);
                    if creates_value && !step.requires_auth {
                        return Some(TemporalViolation {
                            rule: InvariantRule::NoFreeMoney,
                            temporal_type: TemporalInvariant::NeverViolated,
                            path: fn_sequence,
                            description: format!(
                                "NoFreeMoney violated: unauthenticated value creation in operation '{}'",
                                step.function
                            ),
                            step_at_fault: idx + 1,
                        });
                    }
                }
                None
            }

            InvariantRule::AdminAuthorization => {
                for (idx, step) in path.iter().enumerate() {
                    if (step.function.contains("admin") || step.function.contains("owner"))
                        && !step.requires_auth
                    {
                        return Some(TemporalViolation {
                            rule: InvariantRule::AdminAuthorization,
                            temporal_type: TemporalInvariant::AlwaysHolds,
                            path: fn_sequence,
                            description: format!(
                                "AdminAuthorization violated: admin function '{}' lacks required authorization",
                                step.function
                            ),
                            step_at_fault: idx + 1,
                        });
                    }
                }
                None
            }

            _ => None,
        }
    }
}

impl fmt::Display for InvariantRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Temporal invariant modality classifications based on Linear Temporal Logic (LTL)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TemporalInvariant {
    /// Invariant predicate must hold at all states across all execution paths (□ P / Globally)
    AlwaysHolds,
    /// Invariant predicate must eventually hold or be restored along all paths (◇ P / Finally)
    EventuallyHolds,
    /// Invariant predicate must never be violated / bad state is unreachable (□ ¬P / Safety)
    NeverViolated,
    /// Invariant state value must monotonically increase or stay non-decreasing over transitions
    MonotonicIncrease,
    /// Invariant state value must monotonically decrease or stay non-increasing over transitions
    MonotonicDecrease,
}

impl fmt::Display for TemporalInvariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TemporalInvariant::AlwaysHolds => write!(f, "AlwaysHolds (□ P)"),
            TemporalInvariant::EventuallyHolds => write!(f, "EventuallyHolds (◇ P)"),
            TemporalInvariant::NeverViolated => write!(f, "NeverViolated (□ ¬P)"),
            TemporalInvariant::MonotonicIncrease => write!(f, "MonotonicIncrease (Δ >= 0)"),
            TemporalInvariant::MonotonicDecrease => write!(f, "MonotonicDecrease (Δ <= 0)"),
        }
    }
}

/// Represents a violation of a temporal invariant over an execution sequence
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalViolation {
    pub rule: InvariantRule,
    pub temporal_type: TemporalInvariant,
    pub path: Vec<String>,
    pub description: String,
    pub step_at_fault: usize,
}

impl fmt::Display for TemporalViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Temporal violation of {:?} [{}] on sequence [{}]: {} at step {}",
            self.rule,
            self.temporal_type,
            self.path.join(" -> "),
            self.description,
            self.step_at_fault
        )
    }
}

impl std::error::Error for TemporalViolation {}

/// Represents a state mutation delta produced by an action
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StateDelta {
    pub target: String,
    pub operation: String,
    pub value_effect: i64, // -1: decrease/destroy, 0: neutral/conserve, +1: increase/create
}

/// A node in the contract's state transition graph
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateNode {
    pub id: String,
    pub name: String,
    pub function_name: String,
    pub state_summary: String,
    pub reads: Vec<String>,
    pub writes: Vec<String>,
}

/// A directed transition between contract states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: String,
    pub to: String,
    pub action: String,
    pub function: String,
    pub state_deltas: Vec<StateDelta>,
    pub condition: Option<String>,
    pub requires_auth: bool,
}

/// State transition graph and Sequence Diagram generator for a contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceDiagram {
    pub title: String,
    pub nodes: HashMap<String, StateNode>,
    pub transitions: Vec<StateTransition>,
    pub entry_points: Vec<String>,
}

impl SequenceDiagram {
    pub fn new(title: impl Into<String>) -> Self {
        let title = title.into();
        let mut nodes = HashMap::new();
        let init_node = StateNode {
            id: "Init".to_string(),
            name: "InitialState".to_string(),
            function_name: "Init".to_string(),
            state_summary: "Initial contract state before transactions".to_string(),
            reads: Vec::new(),
            writes: Vec::new(),
        };
        nodes.insert("Init".to_string(), init_node);

        Self {
            title,
            nodes,
            transitions: Vec::new(),
            entry_points: vec!["Init".to_string()],
        }
    }

    pub fn add_node(&mut self, node: StateNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_transition(&mut self, transition: StateTransition) {
        self.transitions.push(transition);
    }

    /// Extracts state transitions and call graph from contract source code
    pub fn from_contract_source(source: &str) -> Self {
        let mut diagram = Self::new("Contract State Transition Sequence");

        let fn_regex = Regex::new(r"pub\s+fn\s+([a-zA-Z0-9_]+)").unwrap();
        let mut functions: Vec<(String, String, bool, Vec<StateDelta>, Vec<String>, Vec<String>)> =
            Vec::new();

        for cap in fn_regex.captures_iter(source) {
            let fn_name = cap[1].to_string();
            let search_target = format!("fn {}", fn_name);
            if let Some(match_idx) = source.find(&search_target) {
                let rest = &source[match_idx..];
                if let Some(brace_start) = rest.find('{') {
                    let body_slice = &rest[brace_start..];
                    let mut brace_count = 0;
                    let mut body_end = 0;
                    for (i, c) in body_slice.char_indices() {
                        if c == '{' {
                            brace_count += 1;
                        } else if c == '}' {
                            brace_count -= 1;
                            if brace_count == 0 {
                                body_end = i + 1;
                                break;
                            }
                        }
                    }

                    let body = if body_end > 0 {
                        &body_slice[..body_end]
                    } else {
                        body_slice
                    };

                    let requires_auth = body.contains("require_auth")
                        || body.contains("has_auth")
                        || body.contains("require_auth_for_args");

                    let mut deltas = Vec::new();
                    let mut reads = Vec::new();
                    let mut writes = Vec::new();

                    // Balance / value deltas
                    if fn_name.contains("mint")
                        || body.contains("mint")
                        || (body.contains("total_supply") && body.contains("+="))
                    {
                        deltas.push(StateDelta {
                            target: "total_supply".to_string(),
                            operation: "mint".to_string(),
                            value_effect: 1,
                        });
                        deltas.push(StateDelta {
                            target: "balance".to_string(),
                            operation: "increase".to_string(),
                            value_effect: 1,
                        });
                        writes.push("total_supply".to_string());
                        writes.push("balance".to_string());
                    } else if fn_name.contains("burn")
                        || body.contains("burn")
                        || (body.contains("total_supply") && body.contains("-="))
                    {
                        deltas.push(StateDelta {
                            target: "total_supply".to_string(),
                            operation: "burn".to_string(),
                            value_effect: -1,
                        });
                        deltas.push(StateDelta {
                            target: "balance".to_string(),
                            operation: "decrease".to_string(),
                            value_effect: -1,
                        });
                        writes.push("total_supply".to_string());
                        writes.push("balance".to_string());
                    } else if fn_name.contains("transfer") || body.contains("transfer") {
                        deltas.push(StateDelta {
                            target: "balance".to_string(),
                            operation: "transfer".to_string(),
                            value_effect: 0,
                        });
                        reads.push("balance".to_string());
                        writes.push("balance".to_string());
                    }

                    // Timestamp
                    if fn_name.contains("timestamp") || body.contains("timestamp") {
                        if body.contains("timestamp <") || body.contains("timestamp -=") {
                            deltas.push(StateDelta {
                                target: "timestamp".to_string(),
                                operation: "decrease".to_string(),
                                value_effect: -1,
                            });
                        } else {
                            deltas.push(StateDelta {
                                target: "timestamp".to_string(),
                                operation: "advance".to_string(),
                                value_effect: 1,
                            });
                        }
                    }

                    // Admin / ownership
                    if fn_name.contains("admin")
                        || fn_name.contains("owner")
                        || body.contains("admin")
                    {
                        writes.push("admin".to_string());
                    }

                    functions.push((
                        fn_name,
                        body.to_string(),
                        requires_auth,
                        deltas,
                        reads,
                        writes,
                    ));
                }
            }
        }

        // Add state nodes for each function
        for (name, _, _, _, reads, writes) in &functions {
            let summary =
                format!("Function {} (reads: {:?}, writes: {:?})", name, reads, writes);
            let node = StateNode {
                id: name.clone(),
                name: format!("State_{}", name),
                function_name: name.clone(),
                state_summary: summary,
                reads: reads.clone(),
                writes: writes.clone(),
            };
            diagram.add_node(node);
        }

        // 1. Initial transitions from Init -> public function
        for (name, _, req_auth, deltas, _, _) in &functions {
            diagram.add_transition(StateTransition {
                from: "Init".to_string(),
                to: name.clone(),
                action: format!("call {}", name),
                function: name.clone(),
                state_deltas: deltas.clone(),
                condition: None,
                requires_auth: *req_auth,
            });
        }

        // 2. Inter-function state transitions (any public function call after any function)
        for (f1, _, _, _, _, _) in &functions {
            for (f2, _, req_auth2, deltas2, _, _) in &functions {
                diagram.add_transition(StateTransition {
                    from: f1.clone(),
                    to: f2.clone(),
                    action: format!("call {}", f2),
                    function: f2.clone(),
                    state_deltas: deltas2.clone(),
                    condition: None,
                    requires_auth: *req_auth2,
                });
            }
        }

        diagram
    }

    /// Explores all state transition paths up to `max_depth` transitions.
    pub fn explore_paths(&self, max_depth: usize) -> Vec<Vec<StateTransition>> {
        if max_depth == 0 {
            return Vec::new();
        }

        const MAX_TOTAL_PATHS: usize = 4096;
        let mut all_paths = Vec::new();

        for entry in &self.entry_points {
            let mut current_path = Vec::new();
            self.dfs_paths(
                entry,
                max_depth,
                &mut current_path,
                &mut all_paths,
                MAX_TOTAL_PATHS,
            );
            if all_paths.len() >= MAX_TOTAL_PATHS {
                break;
            }
        }

        all_paths
    }

    fn dfs_paths(
        &self,
        current_state: &str,
        depth_left: usize,
        current_path: &mut Vec<StateTransition>,
        all_paths: &mut Vec<Vec<StateTransition>>,
        max_paths: usize,
    ) {
        if depth_left == 0 || all_paths.len() >= max_paths {
            return;
        }

        for transition in &self.transitions {
            if transition.from == current_state {
                current_path.push(transition.clone());
                all_paths.push(current_path.clone());

                if all_paths.len() < max_paths && depth_left > 1 {
                    self.dfs_paths(
                        &transition.to,
                        depth_left - 1,
                        current_path,
                        all_paths,
                        max_paths,
                    );
                }

                current_path.pop();
            }
        }
    }

    /// Generates Mermaid sequence diagram markdown
    pub fn to_mermaid(&self) -> String {
        let mut out = String::from(
            "sequenceDiagram\n    autonumber\n    actor Caller\n    participant Contract\n",
        );
        let mut seen = HashSet::new();
        for t in &self.transitions {
            if seen.insert((&t.from, &t.to)) {
                out.push_str(&format!(
                    "    Caller->>Contract: {}(...)\n    activate Contract\n    Contract-->>Caller: Ok\n    deactivate Contract\n",
                    t.function
                ));
            }
        }
        out
    }

    /// Generates Mermaid state diagram markdown
    pub fn to_state_diagram(&self) -> String {
        let mut out = String::from("stateDiagram-v2\n    [*] --> Init\n");
        let mut seen = HashSet::new();
        for t in &self.transitions {
            if seen.insert((&t.from, &t.to)) {
                out.push_str(&format!("    {} --> {}: {}\n", t.from, t.to, t.action));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temporal_modalities_defined() {
        assert_eq!(
            InvariantRule::TotalSupplyConsistency.temporal_modality(),
            TemporalInvariant::AlwaysHolds
        );
        assert_eq!(
            InvariantRule::MintSupplyIncrease.temporal_modality(),
            TemporalInvariant::MonotonicIncrease
        );
        assert_eq!(
            InvariantRule::BurnSupplyDecrease.temporal_modality(),
            TemporalInvariant::MonotonicDecrease
        );
        assert_eq!(
            InvariantRule::TimestampMonotonicity.temporal_modality(),
            TemporalInvariant::MonotonicIncrease
        );
        assert_eq!(
            InvariantRule::NoFreeMoney.temporal_modality(),
            TemporalInvariant::NeverViolated
        );
        assert_eq!(
            InvariantRule::ConservationOfValue.temporal_modality(),
            TemporalInvariant::AlwaysHolds
        );
    }

    #[test]
    fn test_conformance_transfer_mint_burn_violates_conservation_of_value() {
        // Contract with transfer, mint, burn
        let contract_code = r#"
            pub fn transfer(from: Address, to: Address, amount: i128) {
                // transfer conserves value
                balance -= amount;
                balance += amount;
            }

            pub fn mint(to: Address, amount: i128) {
                require_auth(&admin);
                total_supply += amount;
                balance += amount;
            }

            pub fn burn(from: Address, amount: i128) {
                require_auth(&from);
                total_supply -= amount;
                balance -= amount;
            }
        "#;

        let diagram = SequenceDiagram::from_contract_source(contract_code);

        // Verify diagram generated expected nodes and transitions
        assert!(diagram.nodes.contains_key("transfer"));
        assert!(diagram.nodes.contains_key("mint"));
        assert!(diagram.nodes.contains_key("burn"));

        // Verify 3-step path transfer -> mint -> burn exists
        let paths = diagram.explore_paths(3);
        let target_path = paths
            .iter()
            .find(|p| {
                p.len() == 3
                    && p[0].function == "transfer"
                    && p[1].function == "mint"
                    && p[2].function == "burn"
            })
            .expect("Path [transfer -> mint -> burn] must exist in explored state graph");

        // Verify temporal check catches violation on the 3-step sequence
        let violation = InvariantRule::ConservationOfValue
            .check_path(target_path)
            .expect("ConservationOfValue must be violated on transfer -> mint -> burn");

        assert_eq!(violation.rule, InvariantRule::ConservationOfValue);
        assert_eq!(violation.temporal_type, TemporalInvariant::AlwaysHolds);
        assert_eq!(
            violation.path,
            vec!["transfer".to_string(), "mint".to_string(), "burn".to_string()]
        );
        assert_eq!(violation.step_at_fault, 2); // 'mint' creates unbacked value
        assert!(violation.description.contains("ConservationOfValue violated"));

        // Full diagram temporal_check also reports the violation
        let full_check = InvariantRule::ConservationOfValue.temporal_check(&diagram, 3);
        assert!(full_check.is_err());
    }

    #[test]
    fn test_pure_transfer_contract_conserves_value() {
        let contract_code = r#"
            pub fn transfer(from: Address, to: Address, amount: i128) {
                balance -= amount;
                balance += amount;
            }
        "#;

        let diagram = SequenceDiagram::from_contract_source(contract_code);
        let check = InvariantRule::ConservationOfValue.temporal_check(&diagram, 5);
        assert!(check.is_ok(), "Pure transfer contract must conserve value temporally");
    }

    #[test]
    fn test_timestamp_monotonicity_temporal_check() {
        let faulty_contract = r#"
            pub fn update_timestamp(new_time: u64) {
                timestamp -= 10;
            }
        "#;

        let diagram = SequenceDiagram::from_contract_source(faulty_contract);
        let check = InvariantRule::TimestampMonotonicity.temporal_check(&diagram, 3);
        assert!(check.is_err());
        let violation = check.unwrap_err();
        assert_eq!(violation.rule, InvariantRule::TimestampMonotonicity);
        assert_eq!(violation.temporal_type, TemporalInvariant::MonotonicIncrease);
    }

    #[test]
    fn test_sequence_diagram_mermaid_output() {
        let contract = r#"
            pub fn transfer() {}
            pub fn mint() {}
        "#;
        let diagram = SequenceDiagram::from_contract_source(contract);
        let mermaid = diagram.to_mermaid();
        assert!(mermaid.starts_with("sequenceDiagram"));
        assert!(mermaid.contains("Caller->>Contract: transfer(...)"));
        assert!(mermaid.contains("Caller->>Contract: mint(...)"));

        let state_diag = diagram.to_state_diagram();
        assert!(state_diag.starts_with("stateDiagram-v2"));
        assert!(state_diag.contains("Init --> transfer"));
    }
}

