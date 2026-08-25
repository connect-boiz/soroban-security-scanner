use serde::{Deserialize, Serialize};
use std::fmt;

/// Number of stroops in 1 XLM (10^7)
pub const STROOPS_PER_XLM: i64 = 10_000_000;

/// Represents a wallet with its balances stored as integer stroops
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub public_key: String,
    /// Balance in stroops (1 XLM = 10,000,000 stroops)
    pub balance_stroops: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Wallet {
    /// Create a new wallet with zero balance
    pub fn new(user_id: uuid::Uuid, public_key: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4(),
            user_id,
            public_key,
            balance_stroops: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get balance as XLM string for display (e.g., "123.4567890")
    pub fn balance_xlm_string(&self) -> String {
        stroops_to_xlm_string(self.balance_stroops)
    }

    /// Get balance as f64 for legacy compatibility (DEPRECATED: use balance_xlm_string)
    #[deprecated(note = "Use balance_xlm_string() for display or balance_stroops for arithmetic")]
    pub fn balance_lumens(&self) -> f64 {
        self.balance_stroops as f64 / STROOPS_PER_XLM as f64
    }
}

/// Balance response for API - uses string representation to avoid precision loss
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalance {
    pub wallet_id: uuid::Uuid,
    pub public_key: String,
    /// XLM balance as string (e.g., "123.4567890")
    pub xlm_balance: String,
    /// XLM balance in stroops for precise arithmetic
    pub xlm_balance_stroops: i64,
    pub asset_balances: Vec<AssetBalance>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl WalletBalance {
    pub fn new(wallet_id: uuid::Uuid, public_key: String, xlm_balance_stroops: i64, asset_balances: Vec<AssetBalance>) -> Self {
        Self {
            wallet_id,
            public_key,
            xlm_balance: stroops_to_xlm_string(xlm_balance_stroops),
            xlm_balance_stroops,
            asset_balances,
            last_updated: chrono::Utc::now(),
        }
    }
}

/// Individual asset balance - stored as integer stroops
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetBalance {
    pub asset_code: String,
    pub asset_issuer: String,
    /// Balance in stroops (smallest unit)
    pub balance_stroops: i64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl AssetBalance {
    pub fn new(asset_code: String, asset_issuer: String, balance_stroops: i64) -> Self {
        Self {
            asset_code,
            asset_issuer,
            balance_stroops,
            last_updated: chrono::Utc::now(),
        }
    }

    /// Get balance as string for display
    pub fn balance_string(&self) -> String {
        stroops_to_xlm_string(self.balance_stroops)
    }

    /// Get balance as f64 for legacy compatibility (DEPRECATED)
    #[deprecated(note = "Use balance_string() for display or balance_stroops for arithmetic")]
    pub fn balance(&self) -> f64 {
        self.balance_stroops as f64 / STROOPS_PER_XLM as f64
    }
}

/// Convert stroops to XLM string representation (e.g., 1234567890 -> "123.4567890")
pub fn stroops_to_xlm_string(stroops: i64) -> String {
    let xlm = stroops / STROOPS_PER_XLM;
    let remainder = (stroops % STROOPS_PER_XLM).abs();
    format!("{}.{:07}", xlm, remainder)
}

/// Parse XLM string to stroops (e.g., "123.4567890" -> 1234567890)
/// Returns error if more than 7 decimal places or invalid format
pub fn xlm_string_to_stroops(s: &str) -> Result<i64, String> {
    let parts: Vec<&str> = s.split('.').collect();
    match parts.len() {
        1 => {
            // No decimal part
            let whole: i64 = parts[0].parse().map_err(|_| "Invalid whole number")?;
            Ok(whole * STROOPS_PER_XLM)
        }
        2 => {
            let whole: i64 = parts[0].parse().map_err(|_| "Invalid whole number")?;
            let decimal = parts[1];
            if decimal.len() > 7 {
                return Err("Too many decimal places (max 7)".to_string());
            }
            let decimal_padded = format!("{:0<7}", decimal);
            let fractional: i64 = decimal_padded.parse().map_err(|_| "Invalid decimal part")?;
            let sign = if whole < 0 { -1 } else { 1 };
            Ok(whole * STROOPS_PER_XLM + sign * fractional)
        }
        _ => Err("Invalid format".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stroops_to_xlm_string() {
        assert_eq!(stroops_to_xlm_string(0), "0.0000000");
        assert_eq!(stroops_to_xlm_string(1), "0.0000001");
        assert_eq!(stroops_to_xlm_string(STROOPS_PER_XLM), "1.0000000");
        assert_eq!(stroops_to_xlm_string(1234567890), "123.4567890");
        assert_eq!(stroops_to_xlm_string(-1234567890), "-123.4567890");
    }

    #[test]
    fn test_xlm_string_to_stroops() {
        assert_eq!(xlm_string_to_stroops("0").unwrap(), 0);
        assert_eq!(xlm_string_to_stroops("1").unwrap(), STROOPS_PER_XLM);
        assert_eq!(xlm_string_to_stroops("123.4567890").unwrap(), 1234567890);
        assert_eq!(xlm_string_to_stroops("0.0000001").unwrap(), 1);
        assert_eq!(xlm_string_to_stroops("-123.4567890").unwrap(), -1234567890);
        assert!(xlm_string_to_stroops("1.12345678").is_err()); // too many decimals
    }

    #[test]
    fn test_roundtrip() {
        let values = vec![0, 1, 100, STROOPS_PER_XLM, 1234567890, -1234567890];
        for v in values {
            let s = stroops_to_xlm_string(v);
            let parsed = xlm_string_to_stroops(&s).unwrap();
            assert_eq!(v, parsed, "Roundtrip failed for {}", v);
        }
    }
}
