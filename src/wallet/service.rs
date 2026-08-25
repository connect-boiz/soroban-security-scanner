use crate::wallet::types::{AssetBalance, Wallet, WalletBalance, STROOPS_PER_XLM, xlm_string_to_stroops};
use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// Horizon account response structure
#[derive(Debug, Deserialize)]
struct HorizonAccount {
    balances: Vec<HorizonBalance>,
}

#[derive(Debug, Deserialize)]
struct HorizonBalance {
    balance: String,
    asset_type: String,
    #[serde(default)]
    asset_code: Option<String>,
    #[serde(default)]
    asset_issuer: Option<String>,
}

/// Wallet service for managing wallet operations
pub struct WalletService {
    http_client: Client,
    horizon_url: String,
}

impl WalletService {
    /// Create a new wallet service with Horizon endpoint
    pub fn new(horizon_url: String) -> Self {
        Self {
            http_client: Client::new(),
            horizon_url,
        }
    }

    /// Create a new wallet service with custom HTTP client (for testing)
    pub fn with_client(http_client: Client, horizon_url: String) -> Self {
        Self {
            http_client,
            horizon_url,
        }
    }

    /// Fetch and update wallet balance from Horizon
    /// Returns the updated WalletBalance with real-time data
    pub async fn get_balance(&self, wallet: &Wallet) -> Result<WalletBalance> {
        info!(wallet_id = %wallet.id, public_key = %wallet.public_key, "Fetching balance from Horizon");

        let url = format!("{}/accounts/{}", self.horizon_url.trim_end_matches('/'), wallet.public_key);
        
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to Horizon")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!(%status, %error_text, "Horizon request failed");
            anyhow::bail!("Horizon error {}: {}", status, error_text);
        }

        let account: HorizonAccount = response
            .json()
            .await
            .context("Failed to parse Horizon response")?;

        let mut xlm_balance_stroops = 0i64;
        let mut asset_balances = Vec::new();

        for balance in account.balances {
            let balance_stroops = xlm_string_to_stroops(&balance.balance)
                .context(format!("Failed to parse balance: {}", balance.balance))?;

            match balance.asset_type.as_str() {
                "native" => {
                    xlm_balance_stroops = balance_stroops;
                }
                "credit_alphanum4" | "credit_alphanum12" => {
                    if let (Some(code), Some(issuer)) = (balance.asset_code, balance.asset_issuer) {
                        asset_balances.push(AssetBalance::new(code, issuer, balance_stroops));
                    } else {
                        warn!(asset_type = %balance.asset_type, "Missing asset code or issuer");
                    }
                }
                _ => {
                    warn!(asset_type = %balance.asset_type, "Unknown asset type");
                }
            }
        }

        debug!(xlm_stroops = xlm_balance_stroops, asset_count = asset_balances.len(), "Parsed balances");

        Ok(WalletBalance::new(
            wallet.id,
            wallet.public_key.clone(),
            xlm_balance_stroops,
            asset_balances,
        ))
    }

    /// Update local wallet balance from Horizon (for sync operations)
    pub async fn sync_wallet_balance(&self, wallet: &mut Wallet) -> Result<WalletBalance> {
        let balance = self.get_balance(wallet).await?;
        wallet.balance_stroops = balance.xlm_balance_stroops;
        wallet.updated_at = chrono::Utc::now();
        Ok(balance)
    }

    /// Get balance for multiple wallets in batch
    pub async fn get_balances_batch(&self, wallets: &[Wallet]) -> Result<Vec<WalletBalance>> {
        let mut results = Vec::with_capacity(wallets.len());
        
        for wallet in wallets {
            match self.get_balance(wallet).await {
                Ok(balance) => results.push(balance),
                Err(e) => {
                    error!(wallet_id = %wallet.id, error = %e, "Failed to fetch balance");
                    // Return a zero balance with error info instead of failing entire batch
                    results.push(WalletBalance::new(
                        wallet.id,
                        wallet.public_key.clone(),
                        0,
                        vec![],
                    ));
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::types::Wallet;
    use mockito::Server;
    use serde_json::json;

    #[tokio::test]
    async fn test_get_balance_native_only() {
        let mut server = Server::new_async().await;
        let public_key = "GABC1234567890123456789012345678901234567890123456789012";
        
        let mock = server
            .mock("GET", &format!("/accounts/{}", public_key))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "balances": [
                        {
                            "balance": "123.4567890",
                            "asset_type": "native"
                        }
                    ]
                })
                .to_string(),
            )
            .create_async()
            .await;

        let wallet = Wallet::new(uuid::Uuid::new_v4(), public_key.to_string());
        let service = WalletService::new(server.url());
        let balance = service.get_balance(&wallet).await.unwrap();

        assert_eq!(balance.xlm_balance_stroops, 1234567890);
        assert_eq!(balance.xlm_balance, "123.4567890");
        assert!(balance.asset_balances.is_empty());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_balance_with_assets() {
        let mut server = Server::new_async().await;
        let public_key = "GABC1234567890123456789012345678901234567890123456789012";
        
        let mock = server
            .mock("GET", &format!("/accounts/{}", public_key))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "balances": [
                        {
                            "balance": "100.0000000",
                            "asset_type": "native"
                        },
                        {
                            "balance": "50.5000000",
                            "asset_type": "credit_alphanum4",
                            "asset_code": "USDC",
                            "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
                        }
                    ]
                })
                .to_string(),
            )
            .create_async()
            .await;

        let wallet = Wallet::new(uuid::Uuid::new_v4(), public_key.to_string());
        let service = WalletService::new(server.url());
        let balance = service.get_balance(&wallet).await.unwrap();

        assert_eq!(balance.xlm_balance_stroops, 1000000000);
        assert_eq!(balance.asset_balances.len(), 1);
        assert_eq!(balance.asset_balances[0].asset_code, "USDC");
        assert_eq!(balance.asset_balances[0].balance_stroops, 505000000);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_balance_horizon_error() {
        let mut server = Server::new_async().await;
        let public_key = "GABC1234567890123456789012345678901234567890123456789012";
        
        let mock = server
            .mock("GET", &format!("/accounts/{}", public_key))
            .with_status(404)
            .with_header("content-type", "application/json")
            .with_body(json!({"error": "not found"}).to_string())
            .create_async()
            .await;

        let wallet = Wallet::new(uuid::Uuid::new_v4(), public_key.to_string());
        let service = WalletService::new(server.url());
        let result = service.get_balance(&wallet).await;

        assert!(result.is_err());
        mock.assert_async().await;
    }
}
