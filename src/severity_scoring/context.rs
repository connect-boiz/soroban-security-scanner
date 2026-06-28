//! Contextual risk factors layered on top of the CVSS base score.
//!
//! CVSS captures the intrinsic characteristics of a vulnerability, but the
//! *actual* risk to a deployment depends on context: how much value the
//! contract secures, what kind of asset it holds, where it is deployed, how
//! exposed its permissions are, and how mature any exploit is. Each factor
//! contributes a multiplier centered on `1.0`; the aggregate multiplier scales
//! the base score up or down.

use serde::{Deserialize, Serialize};

/// A single named contributing factor and the multiplier it applied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactorContribution {
    pub name: String,
    pub value: String,
    pub multiplier: f64,
}

/// Value secured by the contract, bucketed by total value locked (TVL).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractValueTier {
    Negligible,
    Low,
    Medium,
    High,
    Critical,
}

impl ContractValueTier {
    /// Bucket a USD TVL figure into a tier.
    pub fn from_tvl_usd(tvl_usd: u64) -> Self {
        match tvl_usd {
            0..=9_999 => ContractValueTier::Negligible,
            10_000..=99_999 => ContractValueTier::Low,
            100_000..=999_999 => ContractValueTier::Medium,
            1_000_000..=9_999_999 => ContractValueTier::High,
            _ => ContractValueTier::Critical,
        }
    }

    fn multiplier(&self) -> f64 {
        match self {
            ContractValueTier::Negligible => 0.80,
            ContractValueTier::Low => 0.90,
            ContractValueTier::Medium => 1.00,
            ContractValueTier::High => 1.15,
            ContractValueTier::Critical => 1.30,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            ContractValueTier::Negligible => "negligible",
            ContractValueTier::Low => "low",
            ContractValueTier::Medium => "medium",
            ContractValueTier::High => "high",
            ContractValueTier::Critical => "critical",
        }
    }
}

/// Type of asset the contract custodies. Higher-trust monetary assets raise the
/// stakes of any compromise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetType {
    TestToken,
    Nft,
    Utility,
    GovernanceToken,
    WrappedAsset,
    Stablecoin,
    NativeAsset,
}

impl AssetType {
    fn multiplier(&self) -> f64 {
        match self {
            AssetType::TestToken => 0.50,
            AssetType::Nft => 0.95,
            AssetType::Utility => 1.00,
            AssetType::GovernanceToken => 1.15,
            AssetType::WrappedAsset => 1.15,
            AssetType::Stablecoin => 1.25,
            AssetType::NativeAsset => 1.20,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            AssetType::TestToken => "test_token",
            AssetType::Nft => "nft",
            AssetType::Utility => "utility",
            AssetType::GovernanceToken => "governance_token",
            AssetType::WrappedAsset => "wrapped_asset",
            AssetType::Stablecoin => "stablecoin",
            AssetType::NativeAsset => "native_asset",
        }
    }
}

/// Where the contract is deployed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeploymentEnvironment {
    Local,
    Testnet,
    Staging,
    Mainnet,
}

impl DeploymentEnvironment {
    fn multiplier(&self) -> f64 {
        match self {
            DeploymentEnvironment::Local => 0.40,
            DeploymentEnvironment::Testnet => 0.60,
            DeploymentEnvironment::Staging => 0.80,
            DeploymentEnvironment::Mainnet => 1.20,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            DeploymentEnvironment::Local => "local",
            DeploymentEnvironment::Testnet => "testnet",
            DeploymentEnvironment::Staging => "staging",
            DeploymentEnvironment::Mainnet => "mainnet",
        }
    }
}

/// How exposed the vulnerable entry point is, in terms of who may reach it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionExposure {
    AdminOnly,
    Permissioned,
    PublicAuthenticated,
    PublicPermissionless,
}

impl PermissionExposure {
    fn multiplier(&self) -> f64 {
        match self {
            PermissionExposure::AdminOnly => 0.70,
            PermissionExposure::Permissioned => 0.85,
            PermissionExposure::PublicAuthenticated => 1.05,
            PermissionExposure::PublicPermissionless => 1.25,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            PermissionExposure::AdminOnly => "admin_only",
            PermissionExposure::Permissioned => "permissioned",
            PermissionExposure::PublicAuthenticated => "public_authenticated",
            PermissionExposure::PublicPermissionless => "public_permissionless",
        }
    }
}

/// Maturity of any known exploit (analogous to CVSS temporal Exploit Code
/// Maturity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExploitMaturity {
    Unproven,
    ProofOfConcept,
    Functional,
    Weaponized,
}

impl ExploitMaturity {
    fn multiplier(&self) -> f64 {
        match self {
            ExploitMaturity::Unproven => 0.85,
            ExploitMaturity::ProofOfConcept => 0.94,
            ExploitMaturity::Functional => 1.00,
            ExploitMaturity::Weaponized => 1.10,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            ExploitMaturity::Unproven => "unproven",
            ExploitMaturity::ProofOfConcept => "proof_of_concept",
            ExploitMaturity::Functional => "functional",
            ExploitMaturity::Weaponized => "weaponized",
        }
    }
}

/// Degree to which a mitigation is already in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MitigationLevel {
    OfficialFix,
    Workaround,
    None,
}

impl MitigationLevel {
    fn multiplier(&self) -> f64 {
        match self {
            MitigationLevel::OfficialFix => 0.75,
            MitigationLevel::Workaround => 0.90,
            MitigationLevel::None => 1.00,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            MitigationLevel::OfficialFix => "official_fix",
            MitigationLevel::Workaround => "workaround",
            MitigationLevel::None => "none",
        }
    }
}

/// Lower / upper bounds on the aggregate contextual multiplier so context can
/// adjust, but never invert, the intrinsic CVSS score.
pub const MIN_CONTEXT_MULTIPLIER: f64 = 0.40;
pub const MAX_CONTEXT_MULTIPLIER: f64 = 2.00;

/// The full contextual picture for a deployed vulnerability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskContext {
    pub value_tier: ContractValueTier,
    pub asset_type: AssetType,
    pub environment: DeploymentEnvironment,
    pub permission_exposure: PermissionExposure,
    pub exploit_maturity: ExploitMaturity,
    pub mitigation: MitigationLevel,
}

impl RiskContext {
    /// A neutral, mainnet-leaning default suitable when little is known.
    pub fn new(
        value_tier: ContractValueTier,
        asset_type: AssetType,
        environment: DeploymentEnvironment,
        permission_exposure: PermissionExposure,
    ) -> Self {
        Self {
            value_tier,
            asset_type,
            environment,
            permission_exposure,
            exploit_maturity: ExploitMaturity::Functional,
            mitigation: MitigationLevel::None,
        }
    }

    pub fn with_exploit_maturity(mut self, maturity: ExploitMaturity) -> Self {
        self.exploit_maturity = maturity;
        self
    }

    pub fn with_mitigation(mut self, mitigation: MitigationLevel) -> Self {
        self.mitigation = mitigation;
        self
    }

    /// The aggregate multiplier (product of all factors), clamped to
    /// `[MIN_CONTEXT_MULTIPLIER, MAX_CONTEXT_MULTIPLIER]`.
    pub fn aggregate_multiplier(&self) -> f64 {
        let product = self.value_tier.multiplier()
            * self.asset_type.multiplier()
            * self.environment.multiplier()
            * self.permission_exposure.multiplier()
            * self.exploit_maturity.multiplier()
            * self.mitigation.multiplier();
        product.clamp(MIN_CONTEXT_MULTIPLIER, MAX_CONTEXT_MULTIPLIER)
    }

    /// Per-factor breakdown for transparency in API responses.
    pub fn breakdown(&self) -> Vec<FactorContribution> {
        vec![
            FactorContribution {
                name: "contract_value".to_string(),
                value: self.value_tier.label().to_string(),
                multiplier: self.value_tier.multiplier(),
            },
            FactorContribution {
                name: "asset_type".to_string(),
                value: self.asset_type.label().to_string(),
                multiplier: self.asset_type.multiplier(),
            },
            FactorContribution {
                name: "deployment_environment".to_string(),
                value: self.environment.label().to_string(),
                multiplier: self.environment.multiplier(),
            },
            FactorContribution {
                name: "permission_exposure".to_string(),
                value: self.permission_exposure.label().to_string(),
                multiplier: self.permission_exposure.multiplier(),
            },
            FactorContribution {
                name: "exploit_maturity".to_string(),
                value: self.exploit_maturity.label().to_string(),
                multiplier: self.exploit_maturity.multiplier(),
            },
            FactorContribution {
                name: "mitigation".to_string(),
                value: self.mitigation.label().to_string(),
                multiplier: self.mitigation.multiplier(),
            },
        ]
    }
}
