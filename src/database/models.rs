use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::collections::HashMap;

// User-related models

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub stellar_address: Option<String>,
    pub role: UserRole,
    pub status: UserStatus,
    pub email_verified: bool,
    pub two_factor_enabled: bool,
    pub two_factor_secret: Option<String>,
    pub profile: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub login_count: i32,
    pub reputation_score: i32,
    pub is_verified: bool,
    pub verification_token: Option<String>,
    pub password_reset_token: Option<String>,
    pub password_reset_expires: Option<DateTime<Utc>>,
    // Security fields
    pub failed_login_attempts: i32,
    pub last_failed_login_at: Option<DateTime<Utc>>,
    pub account_locked_until: Option<DateTime<Utc>>,
    pub security_questions: serde_json::Value,
    pub backup_codes: serde_json::Value,
    pub ip_whitelist: serde_json::Value,
    pub device_fingerprints: serde_json::Value,
    pub risk_score: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "snake_case")]
pub enum UserRole {
    Admin,
    SecurityResearcher,
    Developer,
    Auditor,
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_status", rename_all = "snake_case")]
pub enum UserStatus {
    Active,
    Inactive,
    Suspended,
    PendingVerification,
}

// Wallet-related models

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Wallet {
    pub id: Uuid,
    pub user_id: Uuid,
    pub stellar_address: String,
    pub wallet_name: String,
    pub description: Option<String>,
    pub wallet_type: String,
    pub status: WalletStatus,
    pub balance_lumens: sqlx::types::Decimal,
    pub native_balance: sqlx::types::Decimal,
    pub is_primary: bool,
    pub is_verified: bool,
    pub verification_level: i32,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_transaction_at: Option<DateTime<Utc>>,
    pub transaction_count: i32,
    pub frozen_reason: Option<String>,
    pub security_score: i32,
    // Security fields
    pub last_security_scan_at: Option<DateTime<Utc>>,
    pub security_scan_result: serde_json::Value,
    pub suspicious_activity_count: i32,
    pub last_suspicious_activity_at: Option<DateTime<Utc>>,
    pub transaction_limits: serde_json::Value,
    pub approved_origins: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "wallet_status", rename_all = "snake_case")]
pub enum WalletStatus {
    Active,
    Inactive,
    Frozen,
    Compromised,
}

// Transaction-related models

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Transaction {
    pub id: Uuid,
    pub transaction_hash: String,
    pub from_wallet_id: Option<Uuid>,
    pub to_wallet_id: Option<Uuid>,
    pub user_id: Uuid,
    pub transaction_type: TransactionType,
    pub status: TransactionStatus,
    pub amount_lumens: Option<sqlx::types::Decimal>,
    pub amount_native: Option<sqlx::types::Decimal>,
    pub fee_paid: sqlx::types::Decimal,
    pub memo: Option<String>,
    pub memo_type: Option<String>,
    pub stellar_ledger_sequence: Option<i64>,
    pub stellar_operation_count: i32,
    pub envelope: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub related_scan_id: Option<Uuid>,
    pub related_bounty_id: Option<Uuid>,
    pub batch_transaction_id: Option<Uuid>,
    pub metadata: serde_json::Value,
    // Security fields
    pub risk_level: String,
    pub fraud_score: i32,
    pub ip_address: Option<std::net::IpAddr>,
    pub device_fingerprint: Option<String>,
    pub geolocation: serde_json::Value,
    pub is_suspicious: bool,
    pub requires_review: bool,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub review_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "transaction_type", rename_all = "snake_case")]
pub enum TransactionType {
    ScanPayment,
    BountyPayment,
    EscrowDeposit,
    EscrowRelease,
    MultiSigExecution,
    FeePayment,
    Refund,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "transaction_status", rename_all = "snake_case")]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
    Cancelled,
    Processing,
}

// Multi-signature models

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MultiSignatureOperation {
    pub id: Uuid,
    pub user_id: Uuid,
    pub operation_name: String,
    pub description: Option<String>,
    pub stellar_address: String,
    pub threshold_signers: i32,
    pub total_signers: i32,
    pub status: MultiSigStatus,
    pub transaction_envelope: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub executed_at: Option<DateTime<Utc>>,
    pub executed_transaction_hash: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "multi_sig_status", rename_all = "snake_case")]
pub enum MultiSigStatus {
    Pending,
    Approved,
    Rejected,
    Executed,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MultiSignatureSigner {
    pub id: Uuid,
    pub multi_sig_operation_id: Uuid,
    pub signer_address: String,
    pub signer_wallet_id: Option<Uuid>,
    pub signer_user_id: Option<Uuid>,
    pub weight: i32,
    pub status: SignatureStatus,
    pub signature_data: Option<String>,
    pub signed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub comments: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "signature_status", rename_all = "snake_case")]
pub enum SignatureStatus {
    Pending,
    Signed,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TransactionSignature {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub signer_address: String,
    pub signature_data: String,
    pub signature_type: String,
    pub weight: i32,
    pub created_at: DateTime<Utc>,
}

// Session and authentication models

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub session_token: String,
    pub refresh_token: Option<String>,
    pub ip_address: Option<std::net::IpAddr>,
    pub user_agent: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
}

// Security models

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SecurityAlert {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub wallet_id: Option<Uuid>,
    pub transaction_id: Option<Uuid>,
    pub alert_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub alert_data: serde_json::Value,
    pub status: String,
    pub resolved_by: Option<Uuid>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RateLimit {
    pub id: Uuid,
    pub identifier: String,
    pub resource_type: String,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub request_count: i32,
    pub max_allowed: i32,
    pub is_blocked: bool,
    pub block_expires_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserDevice {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_fingerprint: String,
    pub device_name: Option<String>,
    pub device_type: Option<String>,
    pub operating_system: Option<String>,
    pub browser: Option<String>,
    pub ip_address: Option<std::net::IpAddr>,
    pub user_agent: Option<String>,
    pub is_trusted: bool,
    pub last_seen_at: DateTime<Utc>,
    pub first_seen_at: DateTime<Utc>,
    pub usage_count: i32,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AccessPattern {
    pub id: Uuid,
    pub user_id: Uuid,
    pub pattern_type: String,
    pub pattern_data: serde_json::Value,
    pub confidence_score: sqlx::types::Decimal,
    pub is_anomaly: bool,
    pub detected_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

// Audit log model

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    pub ip_address: Option<std::net::IpAddr>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

// Bounty system models

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub repository_url: Option<String>,
    pub contract_address: Option<String>,
    pub owner_id: Uuid,
    pub is_active: bool,
    pub is_public: bool,
    pub total_budget: sqlx::types::Decimal,
    pub budget_currency: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Bounty {
    pub id: Uuid,
    pub project_id: Option<Uuid>,
    pub title: String,
    pub description: String,
    pub category: BountyCategory,
    pub severity: BountySeverity,
    pub status: BountyStatus,
    pub reward_amount: sqlx::types::Decimal,
    pub reward_currency: String,
    pub max_reward_amount: Option<sqlx::types::Decimal>,
    pub assignee_id: Option<Uuid>,
    pub submitter_id: Option<Uuid>,
    pub reviewer_id: Option<Uuid>,
    pub deadline: Option<DateTime<Utc>>,
    pub requirements: serde_json::Value,
    pub submission_guidelines: Option<String>,
    pub evaluation_criteria: serde_json::Value,
    pub tags: serde_json::Value,
    pub view_count: i32,
    pub applicant_count: i32,
    pub submission_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "bounty_category", rename_all = "snake_case")]
pub enum BountyCategory {
    AccessControl,
    TokenEconomics,
    LogicVulnerability,
    GasOptimization,
    EventLogging,
    Randomness,
    StellarSpecific,
    Performance,
    Documentation,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "bounty_severity", rename_all = "snake_case")]
pub enum BountySeverity {
    Critical,
    High,
    Medium,
    Low,
    Informational,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "bounty_status", rename_all = "snake_case")]
pub enum BountyStatus {
    Draft,
    Open,
    Assigned,
    Submitted,
    UnderReview,
    Accepted,
    Rejected,
    Paid,
    Disputed,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BountyApplication {
    pub id: Uuid,
    pub bounty_id: Uuid,
    pub applicant_id: Uuid,
    pub cover_letter: Option<String>,
    pub proposed_solution: Option<String>,
    pub estimated_completion_time: Option<chrono::Duration>,
    pub proposed_budget: Option<sqlx::types::Decimal>,
    pub attachments: serde_json::Value,
    pub status: String,
    pub reviewer_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BountySubmission {
    pub id: Uuid,
    pub bounty_id: Uuid,
    pub submitter_id: Uuid,
    pub title: Option<String>,
    pub description: String,
    pub vulnerability_details: serde_json::Value,
    pub proof_of_concept: Option<String>,
    pub reproduction_steps: Option<String>,
    pub impact_assessment: Option<String>,
    pub recommended_fix: Option<String>,
    pub attachments: serde_json::Value,
    pub code_snippets: serde_json::Value,
    pub test_cases: serde_json::Value,
    pub severity_suggestion: Option<BountySeverity>,
    pub status: String,
    pub review_score: Option<i32>,
    pub review_feedback: Option<String>,
    pub public_visibility: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EscrowAccount {
    pub id: Uuid,
    pub bounty_id: Option<Uuid>,
    pub funder_id: Uuid,
    pub beneficiary_id: Option<Uuid>,
    pub amount: sqlx::types::Decimal,
    pub currency: String,
    pub status: EscrowStatus,
    pub release_conditions: serde_json::Value,
    pub dispute_reason: Option<String>,
    pub dispute_evidence: serde_json::Value,
    pub stellar_transaction_hash: Option<String>,
    pub release_transaction_hash: Option<String>,
    pub refund_transaction_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub funded_at: Option<DateTime<Utc>>,
    pub released_at: Option<DateTime<Utc>>,
    pub refunded_at: Option<DateTime<Utc>>,
    pub disputed_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "escrow_status", rename_all = "snake_case")]
pub enum EscrowStatus {
    Pending,
    Funded,
    Released,
    Refunded,
    Disputed,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BountyReview {
    pub id: Uuid,
    pub bounty_id: Uuid,
    pub submission_id: Uuid,
    pub reviewer_id: Uuid,
    pub overall_score: i32,
    pub severity_rating: Option<BountySeverity>,
    pub quality_score: Option<i32>,
    pub impact_score: Option<i32>,
    pub originality_score: Option<i32>,
    pub review_comments: Option<String>,
    pub recommendation: String,
    pub review_criteria: serde_json::Value,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub finalized_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BountyPayment {
    pub id: Uuid,
    pub bounty_id: Uuid,
    pub submission_id: Uuid,
    pub recipient_id: Uuid,
    pub amount: sqlx::types::Decimal,
    pub currency: String,
    pub payment_method: String,
    pub status: String,
    pub transaction_hash: Option<String>,
    pub payment_details: serde_json::Value,
    pub fee_amount: sqlx::types::Decimal,
    pub fee_currency: String,
    pub net_amount: sqlx::types::Decimal,
    pub due_date: Option<DateTime<Utc>>,
    pub processed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BountyActivityLog {
    pub id: Uuid,
    pub bounty_id: Option<Uuid>,
    pub submission_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub description: String,
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BountyTag {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub usage_count: i32,
    pub created_at: DateTime<Utc>,
}

// DTOs (Data Transfer Objects) for API requests/responses

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub username: String,
    pub password: String,
    pub stellar_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWalletRequest {
    pub stellar_address: String,
    pub wallet_name: String,
    pub description: Option<String>,
    pub wallet_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTransactionRequest {
    pub from_wallet_id: Option<Uuid>,
    pub to_wallet_id: Option<Uuid>,
    pub transaction_type: TransactionType,
    pub amount_lumens: Option<sqlx::types::Decimal>,
    pub amount_native: Option<sqlx::types::Decimal>,
    pub memo: Option<String>,
    pub memo_type: Option<String>,
    pub envelope: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMultiSigRequest {
    pub operation_name: String,
    pub description: Option<String>,
    pub stellar_address: String,
    pub threshold_signers: i32,
    pub transaction_envelope: serde_json::Value,
    pub signers: Vec<MultiSigSignerRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSigSignerRequest {
    pub signer_address: String,
    pub weight: i32,
    pub signer_wallet_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBountyRequest {
    pub project_id: Option<Uuid>,
    pub title: String,
    pub description: String,
    pub category: BountyCategory,
    pub severity: BountySeverity,
    pub reward_amount: sqlx::types::Decimal,
    pub max_reward_amount: Option<sqlx::types::Decimal>,
    pub deadline: Option<DateTime<Utc>>,
    pub requirements: serde_json::Value,
    pub submission_guidelines: Option<String>,
    pub evaluation_criteria: serde_json::Value,
    pub tags: Vec<String>,
}

// Response DTOs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub stellar_address: Option<String>,
    pub role: UserRole,
    pub status: UserStatus,
    pub email_verified: bool,
    pub two_factor_enabled: bool,
    pub reputation_score: i32,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletResponse {
    pub id: Uuid,
    pub stellar_address: String,
    pub wallet_name: String,
    pub description: Option<String>,
    pub wallet_type: String,
    pub status: WalletStatus,
    pub balance_lumens: sqlx::types::Decimal,
    pub native_balance: sqlx::types::Decimal,
    pub is_primary: bool,
    pub is_verified: bool,
    pub verification_level: i32,
    pub security_score: i32,
    pub created_at: DateTime<Utc>,
    pub last_transaction_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResponse {
    pub id: Uuid,
    pub transaction_hash: String,
    pub transaction_type: TransactionType,
    pub status: TransactionStatus,
    pub amount_lumens: Option<sqlx::types::Decimal>,
    pub amount_native: Option<sqlx::types::Decimal>,
    pub fee_paid: sqlx::types::Decimal,
    pub memo: Option<String>,
    pub stellar_ledger_sequence: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub risk_level: String,
    pub is_suspicious: bool,
    pub requires_review: bool,
}

// Utility structs for queries and filtering

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionFilter {
    pub user_id: Option<Uuid>,
    pub wallet_id: Option<Uuid>,
    pub transaction_type: Option<TransactionType>,
    pub status: Option<TransactionStatus>,
    pub risk_level: Option<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub min_amount: Option<sqlx::types::Decimal>,
    pub max_amount: Option<sqlx::types::Decimal>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BountyFilter {
    pub project_id: Option<Uuid>,
    pub category: Option<BountyCategory>,
    pub severity: Option<BountySeverity>,
    pub status: Option<BountyStatus>,
    pub assignee_id: Option<Uuid>,
    pub min_reward: Option<sqlx::types::Decimal>,
    pub max_reward: Option<sqlx::types::Decimal>,
    pub deadline_before: Option<DateTime<Utc>>,
    pub deadline_after: Option<DateTime<Utc>>,
    pub tags: Option<Vec<String>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAlertFilter {
    pub user_id: Option<Uuid>,
    pub wallet_id: Option<Uuid>,
    pub transaction_id: Option<Uuid>,
    pub alert_type: Option<String>,
    pub severity: Option<String>,
    pub status: Option<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
