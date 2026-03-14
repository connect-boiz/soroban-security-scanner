//! Invariant checking rules for Stellar smart contracts

use crate::Severity;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvariantRule {
    // Token Invariants
    TotalSupplyConsistency,
    BalanceNonNegative,
    TransferConservation,
    MintSupplyIncrease,
    BurnSupplyDecrease,
    
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
            InvariantRule::TotalSupplyConsistency => "Total token supply must always equal sum of all balances",
            InvariantRule::BalanceNonNegative => "Account balances must never be negative",
            InvariantRule::TransferConservation => "Transfers must conserve total value (sender decrease = receiver increase)",
            InvariantRule::MintSupplyIncrease => "Minting must increase total supply by minted amount",
            InvariantRule::BurnSupplyDecrease => "Burning must decrease total supply by burned amount",
            
            InvariantRule::AdminAuthorization => "Admin operations must be properly authorized",
            InvariantRule::OwnershipConsistency => "Ownership must be consistent and transferable only by owner",
            InvariantRule::PermissionIntegrity => "Permission checks must be consistent across all functions",
            
            InvariantRule::SumOfBalancesEqualsSupply => "Sum of all user balances must equal total supply",
            InvariantRule::NoNegativeBalances => "No account can have negative balance",
            InvariantRule::OverflowProtection => "All arithmetic operations must be protected from overflow/underflow",
            
            InvariantRule::StateTransitionValidity => "All state transitions must be valid and authorized",
            InvariantRule::EventStateConsistency => "Emitted events must accurately reflect state changes",
            InvariantRule::TimestampMonotonicity => "Timestamps must always increase",
            
            InvariantRule::NoFreeMoney => "No mechanism should allow creation of value without corresponding input",
            InvariantRule::ConservationOfValue => "Total system value must be conserved in closed operations",
            InvariantRule::FairDistribution => "Distribution mechanisms must be fair and predictable",
            
            InvariantRule::StellarAssetIntegrity => "Stellar asset operations must maintain integrity",
            InvariantRule::AccountStateConsistency => "Account state must be consistent across operations",
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
            
            InvariantRule::StellarAssetIntegrity => r"stellar_asset.*validate|validate.*stellar_asset",
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
            InvariantRule::TotalSupplyConsistency => "Implement checks to ensure total_supply equals sum of all balances",
            InvariantRule::BalanceNonNegative => "Add checks to prevent negative balances in all operations",
            InvariantRule::TransferConservation => "Validate that transfers conserve total value",
            InvariantRule::MintSupplyIncrease => "Ensure minting operations properly increase total supply",
            InvariantRule::BurnSupplyDecrease => "Ensure burning operations properly decrease total supply",
            
            InvariantRule::AdminAuthorization => "Implement proper admin authorization for all admin functions",
            InvariantRule::OwnershipConsistency => "Maintain ownership consistency across all operations",
            InvariantRule::PermissionIntegrity => "Ensure permission checks are consistent and comprehensive",
            
            InvariantRule::SumOfBalancesEqualsSupply => "Add invariant checks for supply vs balance consistency",
            InvariantRule::NoNegativeBalances => "Implement negative balance protection",
            InvariantRule::OverflowProtection => "Use safe arithmetic operations throughout the contract",
            
            InvariantRule::StateTransitionValidity => "Validate all state transitions before execution",
            InvariantRule::EventStateConsistency => "Ensure events accurately reflect state changes",
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
}

impl fmt::Display for InvariantRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
