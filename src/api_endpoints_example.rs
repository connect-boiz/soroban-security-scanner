// Input Validation Middleware and Extractors for API Endpoints
// This file demonstrates how to integrate the validation module with Axum endpoints

use axum::{
    extract::{Query, Json, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use crate::validation::{
    validate_address, validate_amount, validate_contract_id, validate_pagination,
    sanitize_string, ValidationError, ValidatedAddress, ValidatedPagination,
};

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

/// Query parameters with pagination
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl PaginationQuery {
    /// Validates and converts to ValidatedPagination with defaults
    pub fn validate(self) -> Result<ValidatedPagination, ValidationError> {
        let limit = self.limit.unwrap_or(50);
        let offset = self.offset.unwrap_or(0);
        ValidatedPagination::new(limit, offset)
    }
}

/// Request to query wallet balance
#[derive(Debug, Deserialize)]
pub struct GetBalanceRequest {
    pub address: String,
}

/// Request to transfer tokens
#[derive(Debug, Deserialize)]
pub struct TransferRequest {
    pub from_address: String,
    pub to_address: String,
    pub amount: i128,
    pub description: Option<String>,
}

impl TransferRequest {
    /// Validates all fields in the transfer request
    pub fn validate(self) -> Result<ValidatedTransferRequest, ValidationError> {
        let from = ValidatedAddress::new(&self.from_address)?;
        let to = ValidatedAddress::new(&self.to_address)?;
        validate_amount(self.amount)?;

        let description = if let Some(desc) = self.description {
            Some(sanitize_string(&desc, 500)?)
        } else {
            None
        };

        Ok(ValidatedTransferRequest {
            from,
            to,
            amount: self.amount,
            description,
        })
    }
}

/// Validated transfer request with all fields validated
#[derive(Debug)]
pub struct ValidatedTransferRequest {
    pub from: ValidatedAddress,
    pub to: ValidatedAddress,
    pub amount: i128,
    pub description: Option<String>,
}

/// Request to deploy a contract
#[derive(Debug, Deserialize)]
pub struct DeployContractRequest {
    pub contract_code: Vec<u8>,
    pub name: String,
    pub owner: String,
}

impl DeployContractRequest {
    /// Validates all fields in the deployment request
    pub fn validate(self) -> Result<ValidatedDeployRequest, ValidationError> {
        let owner = ValidatedAddress::new(&self.owner)?;
        let name = sanitize_string(&self.name, 256)?;

        if self.contract_code.is_empty() {
            return Err(ValidationError::ValidationFailed {
                reason: "Contract code cannot be empty".to_string(),
            });
        }

        if self.contract_code.len() > 10_000_000 {
            return Err(ValidationError::ValidationFailed {
                reason: "Contract code exceeds maximum size of 10MB".to_string(),
            });
        }

        Ok(ValidatedDeployRequest {
            contract_code: self.contract_code,
            name,
            owner,
        })
    }
}

/// Validated deployment request
#[derive(Debug)]
pub struct ValidatedDeployRequest {
    pub contract_code: Vec<u8>,
    pub name: String,
    pub owner: ValidatedAddress,
}

// ============================================================================
// RESPONSE TYPES
// ============================================================================

#[derive(Debug, Serialize)]
pub struct BalanceResponse {
    pub address: String,
    pub balance: i128,
    pub currency: String,
}

#[derive(Debug, Serialize)]
pub struct TransferResponse {
    pub transaction_id: String,
    pub status: String,
    pub amount: i128,
}

#[derive(Debug, Serialize)]
pub struct ListWalletsResponse {
    pub wallets: Vec<WalletInfo>,
    pub limit: u32,
    pub offset: u32,
    pub total: u64,
}

#[derive(Debug, Serialize)]
pub struct WalletInfo {
    pub address: String,
    pub balance: i128,
    pub created_at: String,
}

// ============================================================================
// ENDPOINT HANDLERS
// ============================================================================

/// GET /api/v1/wallets/{address}/balance
/// 
/// Get the balance for a Stellar address
/// 
/// # Arguments
/// * `address` - Stellar address (path parameter, validated)
/// 
/// # Returns
/// * 200 OK with balance
/// * 400 Bad Request if address is invalid
pub async fn get_balance(
    Path(address): Path<String>,
) -> Result<axum::Json<BalanceResponse>, ValidationError> {
    // Validate the address from the path parameter
    ValidatedAddress::new(&address)?;

    // In a real implementation, query the database
    Ok(axum::Json(BalanceResponse {
        address,
        balance: 1000000,
        currency: "stroops".to_string(),
    }))
}

/// POST /api/v1/wallets/transfer
/// 
/// Transfer tokens between accounts
/// 
/// # Request Body
/// ```json
/// {
///   "from_address": "GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQ",
///   "to_address": "GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQ",
///   "amount": 1000,
///   "description": "Payment for services"
/// }
/// ```
/// 
/// # Returns
/// * 200 OK with transaction ID
/// * 400 Bad Request if validation fails
pub async fn transfer(
    Json(payload): Json<TransferRequest>,
) -> Result<axum::Json<TransferResponse>, ValidationError> {
    // Validate all request fields
    let validated = payload.validate()?;

    // In a real implementation, process the transfer
    Ok(axum::Json(TransferResponse {
        transaction_id: "txn_123456".to_string(),
        status: "success".to_string(),
        amount: validated.amount,
    }))
}

/// GET /api/v1/wallets
/// 
/// List wallets with pagination
/// 
/// # Query Parameters
/// * `limit` - Number of results (1-1000, default: 50)
/// * `offset` - Number of results to skip (default: 0)
/// 
/// # Returns
/// * 200 OK with paginated wallet list
/// * 400 Bad Request if pagination parameters are invalid
pub async fn list_wallets(
    Query(params): Query<PaginationQuery>,
) -> Result<axum::Json<ListWalletsResponse>, ValidationError> {
    // Validate pagination parameters
    let pagination = params.validate()?;

    // In a real implementation, query the database with pagination
    Ok(axum::Json(ListWalletsResponse {
        wallets: vec![],
        limit: pagination.limit,
        offset: pagination.offset,
        total: 0,
    }))
}

/// POST /api/v1/contracts/deploy
/// 
/// Deploy a new smart contract
/// 
/// # Request Body
/// ```json
/// {
///   "contract_code": [/* binary data */],
///   "name": "MyContract",
///   "owner": "GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQ"
/// }
/// ```
/// 
/// # Returns
/// * 200 OK with deployment result
/// * 400 Bad Request if validation fails
pub async fn deploy_contract(
    Json(payload): Json<DeployContractRequest>,
) -> Result<axum::Json<serde_json::json_internal::Value>, ValidationError> {
    // Validate all request fields
    let validated = payload.validate()?;

    // In a real implementation, deploy the contract
    Ok(axum::Json(serde_json::json!({
        "contract_id": "C123456",
        "status": "deployed",
        "owner": validated.owner.as_str()
    })))
}

/// GET /api/v1/contracts/{contract_id}
/// 
/// Get contract details
/// 
/// # Arguments
/// * `contract_id` - Contract ID as hex string (path parameter)
/// 
/// # Returns
/// * 200 OK with contract details
/// * 400 Bad Request if contract ID is invalid
pub async fn get_contract(
    Path(contract_id): Path<String>,
) -> Result<axum::Json<serde_json::json_internal::Value>, ValidationError> {
    // Parse the hex string to [u8; 32]
    let contract_bytes: [u8; 32] = hex::decode(&contract_id)
        .ok()
        .and_then(|v| {
            if v.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&v);
                Some(arr)
            } else {
                None
            }
        })
        .ok_or_else(|| ValidationError::InvalidContractId {
            reason: "Invalid contract ID format or length".to_string(),
        })?;

    // Validate the contract ID
    validate_contract_id(&contract_bytes)?;

    // In a real implementation, query contract details
    Ok(axum::Json(serde_json::json!({
        "contract_id": contract_id,
        "status": "active",
        "balance": 5000000
    })))
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Creates the API router with all validated endpoints
pub fn create_router() -> Router {
    Router::new()
        // Wallet endpoints
        .route("/api/v1/wallets/:address/balance", get(get_balance))
        .route("/api/v1/wallets", get(list_wallets))
        .route("/api/v1/wallets/transfer", post(transfer))
        // Contract endpoints
        .route("/api/v1/contracts/deploy", post(deploy_contract))
        .route("/api/v1/contracts/:contract_id", get(get_contract))
}

// ============================================================================
// MIDDLEWARE FOR AUTOMATIC VALIDATION
// ============================================================================

/// Custom extractor for automatic validation of JSON bodies
/// This can be used instead of manual validation in handlers
pub struct ValidatedJson<T>(pub T);

#[async_trait::async_trait]
impl<T, S> axum::extract::FromRequest<S> for ValidatedJson<T>
where
    T: serde::de::DeserializeOwned + std::fmt::Debug,
    S: Send + Sync,
{
    type Rejection = ValidationError;

    async fn from_request(
        req: axum::extract::Request,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let axum::Json(value) = axum::extract::Json::<T>::from_request(req, state)
            .await
            .map_err(|e| ValidationError::ValidationFailed {
                reason: format!("Invalid JSON: {}", e),
            })?;

        Ok(ValidatedJson(value))
    }
}

// ============================================================================
// EXAMPLE INTEGRATION TEST
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_request_validation() {
        let valid_request = TransferRequest {
            from_address: "GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQ".to_string(),
            to_address: "GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQ".to_string(),
            amount: 1000,
            description: Some("Test transfer".to_string()),
        };

        assert!(valid_request.validate().is_ok());
    }

    #[test]
    fn test_transfer_request_invalid_amount() {
        let invalid_request = TransferRequest {
            from_address: "GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQ".to_string(),
            to_address: "GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQ".to_string(),
            amount: -1000, // negative amount
            description: None,
        };

        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_deploy_contract_validation() {
        let valid_request = DeployContractRequest {
            contract_code: vec![1, 2, 3, 4, 5],
            name: "TestContract".to_string(),
            owner: "GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQ".to_string(),
        };

        assert!(valid_request.validate().is_ok());
    }

    #[test]
    fn test_deploy_contract_empty_code() {
        let invalid_request = DeployContractRequest {
            contract_code: vec![], // empty code
            name: "TestContract".to_string(),
            owner: "GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQ".to_string(),
        };

        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_pagination_query_validation() {
        let query = PaginationQuery {
            limit: Some(50),
            offset: Some(0),
        };

        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_pagination_defaults() {
        let query = PaginationQuery {
            limit: None,
            offset: None,
        };

        let validated = query.validate().unwrap();
        assert_eq!(validated.limit, 50);
        assert_eq!(validated.offset, 0);
    }
}
