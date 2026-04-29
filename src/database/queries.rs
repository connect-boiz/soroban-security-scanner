use super::models::*;
use super::Database;
use sqlx::{PgPool, Row};
use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;

// User queries
impl Database {
    pub async fn create_user(&self, user: CreateUserRequest, password_hash: String) -> Result<User> {
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (
                email, username, password_hash, stellar_address, 
                role, status, email_verified, two_factor_enabled,
                profile, created_at, updated_at, reputation_score,
                is_verified, failed_login_attempts, risk_score
            )
            VALUES ($1, $2, $3, $4, 'user', 'pending_verification', false, false, '{}', NOW(), NOW(), 0, false, 0, 0)
            RETURNING *
            "#,
            user.email,
            user.username,
            password_hash,
            user.stellar_address
        )
        .fetch_one(self.pool())
        .await?;

        Ok(user)
    }

    pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<User>> {
        let user = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE id = $1",
            user_id
        )
        .fetch_optional(self.pool())
        .await?;

        Ok(user)
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE email = $1",
            email
        )
        .fetch_optional(self.pool())
        .await?;

        Ok(user)
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let user = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE username = $1",
            username
        )
        .fetch_optional(self.pool())
        .await?;

        Ok(user)
    }

    pub async fn update_user(&self, user_id: Uuid, updates: PartialUserUpdate) -> Result<User> {
        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users 
            SET 
                email = COALESCE($2, email),
                username = COALESCE($3, username),
                stellar_address = COALESCE($4, stellar_address),
                role = COALESCE($5, role),
                status = COALESCE($6, status),
                email_verified = COALESCE($7, email_verified),
                two_factor_enabled = COALESCE($8, two_factor_enabled),
                profile = COALESCE($9, profile),
                reputation_score = COALESCE($10, reputation_score),
                is_verified = COALESCE($11, is_verified),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
            user_id,
            updates.email,
            updates.username,
            updates.stellar_address,
            updates.role as UserRole,
            updates.status as UserStatus,
            updates.email_verified,
            updates.two_factor_enabled,
            updates.profile,
            updates.reputation_score,
            updates.is_verified
        )
        .fetch_one(self.pool())
        .await?;

        Ok(user)
    }

    pub async fn update_last_login(&self, user_id: Uuid) -> Result<()> {
        sqlx::query!(
            "UPDATE users SET last_login_at = NOW(), login_count = login_count + 1 WHERE id = $1",
            user_id
        )
        .execute(self.pool())
        .await?;

        Ok(())
    }

    pub async fn list_users(&self, limit: i64, offset: i64) -> Result<Vec<User>> {
        let users = sqlx::query_as!(
            User,
            "SELECT * FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            limit,
            offset
        )
        .fetch_all(self.pool())
        .await?;

        Ok(users)
    }
}

// Wallet queries
impl Database {
    pub async fn create_wallet(&self, user_id: Uuid, wallet: CreateWalletRequest) -> Result<Wallet> {
        let wallet = sqlx::query_as!(
            Wallet,
            r#"
            INSERT INTO wallets (
                user_id, stellar_address, wallet_name, description, 
                wallet_type, status, balance_lumens, native_balance,
                is_primary, is_verified, verification_level, 
                security_score, transaction_limits, approved_origins,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, COALESCE($5, 'standard'), 'active', 0, 0, false, false, 0, 0, '{}', '[]', NOW(), NOW())
            RETURNING *
            "#,
            user_id,
            wallet.stellar_address,
            wallet.wallet_name,
            wallet.description,
            wallet.wallet_type
        )
        .fetch_one(self.pool())
        .await?;

        Ok(wallet)
    }

    pub async fn get_wallet_by_id(&self, wallet_id: Uuid) -> Result<Option<Wallet>> {
        let wallet = sqlx::query_as!(
            Wallet,
            "SELECT * FROM wallets WHERE id = $1",
            wallet_id
        )
        .fetch_optional(self.pool())
        .await?;

        Ok(wallet)
    }

    pub async fn get_wallets_by_user(&self, user_id: Uuid) -> Result<Vec<Wallet>> {
        let wallets = sqlx::query_as!(
            Wallet,
            "SELECT * FROM wallets WHERE user_id = $1 ORDER BY created_at DESC",
            user_id
        )
        .fetch_all(self.pool())
        .await?;

        Ok(wallets)
    }

    pub async fn get_wallet_by_address(&self, stellar_address: &str) -> Result<Option<Wallet>> {
        let wallet = sqlx::query_as!(
            Wallet,
            "SELECT * FROM wallets WHERE stellar_address = $1",
            stellar_address
        )
        .fetch_optional(self.pool())
        .await?;

        Ok(wallet)
    }

    pub async fn update_wallet_balance(&self, wallet_id: Uuid, lumen_balance: sqlx::types::Decimal, native_balance: sqlx::types::Decimal) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE wallets 
            SET 
                balance_lumens = $2,
                native_balance = $3,
                last_transaction_at = NOW(),
                transaction_count = transaction_count + 1,
                updated_at = NOW()
            WHERE id = $1
            "#,
            wallet_id,
            lumen_balance,
            native_balance
        )
        .execute(self.pool())
        .await?;

        Ok(())
    }

    pub async fn set_primary_wallet(&self, user_id: Uuid, wallet_id: Uuid) -> Result<()> {
        let mut tx = self.begin_transaction().await?;
        
        // Unset all other primary wallets for this user
        sqlx::query!(
            "UPDATE wallets SET is_primary = false WHERE user_id = $1",
            user_id
        )
        .execute(&mut *tx)
        .await?;
        
        // Set the new primary wallet
        sqlx::query!(
            "UPDATE wallets SET is_primary = true WHERE id = $1 AND user_id = $2",
            wallet_id,
            user_id
        )
        .execute(&mut *tx)
        .await?;
        
        tx.commit().await?;
        Ok(())
    }
}

// Transaction queries
impl Database {
    pub async fn create_transaction(&self, user_id: Uuid, transaction: CreateTransactionRequest) -> Result<Transaction> {
        let transaction = sqlx::query_as!(
            Transaction,
            r#"
            INSERT INTO transactions (
                transaction_hash, from_wallet_id, to_wallet_id, user_id,
                transaction_type, status, amount_lumens, amount_native,
                fee_paid, memo, memo_type, stellar_operation_count,
                envelope, created_at, updated_at, risk_level, fraud_score,
                geolocation, is_suspicious, requires_review
            )
            VALUES ($1, $2, $3, $4, $5, 'pending', $6, $7, 0, $8, $9, 1, $10, NOW(), NOW(), 'low', 0, '{}', false, false)
            RETURNING *
            "#,
            uuid::Uuid::new_v4().to_string(), // Generate placeholder hash
            transaction.from_wallet_id,
            transaction.to_wallet_id,
            user_id,
            transaction.transaction_type as TransactionType,
            transaction.amount_lumens,
            transaction.amount_native,
            transaction.memo,
            transaction.memo_type,
            transaction.envelope
        )
        .fetch_one(self.pool())
        .await?;

        Ok(transaction)
    }

    pub async fn get_transaction_by_id(&self, transaction_id: Uuid) -> Result<Option<Transaction>> {
        let transaction = sqlx::query_as!(
            Transaction,
            "SELECT * FROM transactions WHERE id = $1",
            transaction_id
        )
        .fetch_optional(self.pool())
        .await?;

        Ok(transaction)
    }

    pub async fn get_transaction_by_hash(&self, hash: &str) -> Result<Option<Transaction>> {
        let transaction = sqlx::query_as!(
            Transaction,
            "SELECT * FROM transactions WHERE transaction_hash = $1",
            hash
        )
        .fetch_optional(self.pool())
        .await?;

        Ok(transaction)
    }

    pub async fn update_transaction_status(&self, transaction_id: Uuid, status: TransactionStatus, stellar_hash: Option<String>) -> Result<Transaction> {
        let transaction = sqlx::query_as!(
            Transaction,
            r#"
            UPDATE transactions 
            SET 
                status = $2,
                transaction_hash = COALESCE($3, transaction_hash),
                confirmed_at = CASE WHEN $2 = 'confirmed' THEN NOW() ELSE confirmed_at END,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
            transaction_id,
            status as TransactionStatus,
            stellar_hash
        )
        .fetch_one(self.pool())
        .await?;

        Ok(transaction)
    }

    pub async fn list_transactions(&self, filter: TransactionFilter) -> Result<Vec<Transaction>> {
        let mut query = "SELECT * FROM transactions WHERE 1=1".to_string();
        let mut params = Vec::new();
        let mut param_count = 0;

        if let Some(user_id) = filter.user_id {
            param_count += 1;
            query.push_str(&format!(" AND user_id = ${}", param_count));
            params.push(user_id);
        }

        if let Some(wallet_id) = filter.wallet_id {
            param_count += 1;
            query.push_str(&format!(" AND (from_wallet_id = ${} OR to_wallet_id = ${})", param_count, param_count));
            params.push(wallet_id);
        }

        if let Some(transaction_type) = filter.transaction_type {
            param_count += 1;
            query.push_str(&format!(" AND transaction_type = ${}", param_count));
            params.push(transaction_type as TransactionType);
        }

        if let Some(status) = filter.status {
            param_count += 1;
            query.push_str(&format!(" AND status = ${}", param_count));
            params.push(status as TransactionStatus);
        }

        if let Some(risk_level) = filter.risk_level {
            param_count += 1;
            query.push_str(&format!(" AND risk_level = ${}", param_count));
            params.push(risk_level);
        }

        if let Some(date_from) = filter.date_from {
            param_count += 1;
            query.push_str(&format!(" AND created_at >= ${}", param_count));
            params.push(date_from);
        }

        if let Some(date_to) = filter.date_to {
            param_count += 1;
            query.push_str(&format!(" AND created_at <= ${}", param_count));
            params.push(date_to);
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = filter.limit {
            param_count += 1;
            query.push_str(&format!(" LIMIT ${}", param_count));
            params.push(limit as i32);
        }

        if let Some(offset) = filter.offset {
            param_count += 1;
            query.push_str(&format!(" OFFSET ${}", param_count));
            params.push(offset as i32);
        }

        // This is a simplified version - in production you'd want to use a query builder
        // For now, we'll use a basic approach
        let transactions = sqlx::query_as!(Transaction, &query)
            .fetch_all(self.pool())
            .await?;

        Ok(transactions)
    }
}

// Multi-signature queries
impl Database {
    pub async fn create_multi_signature_operation(&self, user_id: Uuid, operation: CreateMultiSigRequest) -> Result<MultiSignatureOperation> {
        let mut tx = self.begin_transaction().await?;

        let multi_sig = sqlx::query_as!(
            MultiSignatureOperation,
            r#"
            INSERT INTO multi_signature_operations (
                user_id, operation_name, description, stellar_address,
                threshold_signers, total_signers, status,
                transaction_envelope, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'pending', $7, NOW(), NOW())
            RETURNING *
            "#,
            user_id,
            operation.operation_name,
            operation.description,
            operation.stellar_address,
            operation.threshold_signers,
            operation.signers.len() as i32,
            operation.transaction_envelope
        )
        .fetch_one(&mut *tx)
        .await?;

        // Add signers
        for signer in operation.signers {
            sqlx::query!(
                r#"
                INSERT INTO multi_signature_signers (
                    multi_sig_operation_id, signer_address, signer_wallet_id,
                    weight, status, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, 'pending', NOW(), NOW())
                "#,
                multi_sig.id,
                signer.signer_address,
                signer.signer_wallet_id,
                signer.weight
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(multi_sig)
    }

    pub async fn get_multi_signature_operation(&self, operation_id: Uuid) -> Result<Option<MultiSignatureOperation>> {
        let operation = sqlx::query_as!(
            MultiSignatureOperation,
            "SELECT * FROM multi_signature_operations WHERE id = $1",
            operation_id
        )
        .fetch_optional(self.pool())
        .await?;

        Ok(operation)
    }

    pub async fn get_multi_signature_signers(&self, operation_id: Uuid) -> Result<Vec<MultiSignatureSigner>> {
        let signers = sqlx::query_as!(
            MultiSignatureSigner,
            "SELECT * FROM multi_signature_signers WHERE multi_sig_operation_id = $1 ORDER BY created_at",
            operation_id
        )
        .fetch_all(self.pool())
        .await?;

        Ok(signers)
    }

    pub async fn add_signature(&self, operation_id: Uuid, signer_address: &str, signature_data: &str) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE multi_signature_signers 
            SET 
                status = 'signed',
                signature_data = $2,
                signed_at = NOW(),
                updated_at = NOW()
            WHERE multi_sig_operation_id = $1 AND signer_address = $3
            "#,
            operation_id,
            signature_data,
            signer_address
        )
        .execute(self.pool())
        .await?;

        // Check if operation is now complete
        self.check_multi_signature_completion(operation_id).await?;

        Ok(())
    }

    async fn check_multi_signature_completion(&self, operation_id: Uuid) -> Result<()> {
        let operation = self.get_multi_signature_operation(operation_id).await?;
        if let Some(op) = operation {
            let signers = self.get_multi_signature_signers(operation_id).await?;
            let signed_weight: i32 = signers.iter()
                .filter(|s| s.status == SignatureStatus::Signed)
                .map(|s| s.weight)
                .sum();

            if signed_weight >= op.threshold_signers {
                // Update operation to approved
                sqlx::query!(
                    "UPDATE multi_signature_operations SET status = 'approved', updated_at = NOW() WHERE id = $1",
                    operation_id
                )
                .execute(self.pool())
                .await?;
            }
        }
        Ok(())
    }
}

// Security queries
impl Database {
    pub async fn create_security_alert(&self, alert: SecurityAlert) -> Result<SecurityAlert> {
        let alert = sqlx::query_as!(
            SecurityAlert,
            r#"
            INSERT INTO security_alerts (
                user_id, wallet_id, transaction_id, alert_type,
                severity, title, description, alert_data,
                status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'open', NOW(), NOW())
            RETURNING *
            "#,
            alert.user_id,
            alert.wallet_id,
            alert.transaction_id,
            alert.alert_type,
            alert.severity,
            alert.title,
            alert.description,
            alert.alert_data
        )
        .fetch_one(self.pool())
        .await?;

        Ok(alert)
    }

    pub async fn get_security_alerts(&self, filter: SecurityAlertFilter) -> Result<Vec<SecurityAlert>> {
        let mut query = "SELECT * FROM security_alerts WHERE 1=1".to_string();
        
        // Similar filtering logic as transactions
        // This is simplified for brevity
        
        let alerts = sqlx::query_as!(SecurityAlert, &query)
            .fetch_all(self.pool())
            .await?;

        Ok(alerts)
    }

    pub async fn update_security_alert_status(&self, alert_id: Uuid, status: &str, resolved_by: Option<Uuid>, notes: Option<String>) -> Result<SecurityAlert> {
        let alert = sqlx::query_as!(
            SecurityAlert,
            r#"
            UPDATE security_alerts 
            SET 
                status = $2,
                resolved_by = $3,
                resolved_at = CASE WHEN $2 IN ('resolved', 'false_positive') THEN NOW() ELSE resolved_at END,
                resolution_notes = $4,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
            alert_id,
            status,
            resolved_by,
            notes
        )
        .fetch_one(self.pool())
        .await?;

        Ok(alert)
    }
}

// Bounty system queries
impl Database {
    pub async fn create_bounty(&self, project_id: Option<Uuid>, user_id: Uuid, bounty: CreateBountyRequest) -> Result<Bounty> {
        let bounty = sqlx::query_as!(
            Bounty,
            r#"
            INSERT INTO bounties (
                project_id, title, description, category, severity,
                status, reward_amount, reward_currency, max_reward_amount,
                deadline, requirements, submission_guidelines,
                evaluation_criteria, tags, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, 'draft', $6, 'XLM', $7, $8, $9, $10, $11, $12, NOW(), NOW())
            RETURNING *
            "#,
            project_id,
            bounty.title,
            bounty.description,
            bounty.category as BountyCategory,
            bounty.severity as BountySeverity,
            bounty.reward_amount,
            bounty.max_reward_amount,
            bounty.deadline,
            bounty.requirements,
            bounty.submission_guidelines,
            bounty.evaluation_criteria,
            serde_json::to_value(bounty.tags).unwrap_or(serde_json::Value::Array(vec![]))
        )
        .fetch_one(self.pool())
        .await?;

        Ok(bounty)
    }

    pub async fn create_escrow_account(&self, bounty_id: Uuid, funder_id: Uuid, amount: sqlx::types::Decimal) -> Result<EscrowAccount> {
        let escrow = sqlx::query_as!(
            EscrowAccount,
            r#"
            INSERT INTO escrow_accounts (
                bounty_id, funder_id, amount, currency, status,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, 'XLM', 'pending', NOW(), NOW())
            RETURNING *
            "#,
            bounty_id,
            funder_id,
            amount
        )
        .fetch_one(self.pool())
        .await?;

        Ok(escrow)
    }

    pub async fn list_bounties(&self, filter: BountyFilter) -> Result<Vec<Bounty>> {
        let mut query = "SELECT * FROM bounties WHERE 1=1".to_string();
        
        // Similar filtering logic as other list methods
        // Simplified for brevity
        
        let bounties = sqlx::query_as!(Bounty, &query)
            .fetch_all(self.pool())
            .await?;

        Ok(bounties)
    }
}

// Partial update structs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialUserUpdate {
    pub email: Option<String>,
    pub username: Option<String>,
    pub stellar_address: Option<String>,
    pub role: Option<UserRole>,
    pub status: Option<UserStatus>,
    pub email_verified: Option<bool>,
    pub two_factor_enabled: Option<bool>,
    pub profile: Option<serde_json::Value>,
    pub reputation_score: Option<i32>,
    pub is_verified: Option<bool>,
}
