use sqlx::{PgPool, migrate::MigrateDatabase, Postgres};
use anyhow::Result;
use tracing::{info, error, warn};

pub struct MigrationManager {
    pool: PgPool,
}

impl MigrationManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn ensure_database_exists(config: &super::DatabaseConfig) -> Result<()> {
        let database_url = format!(
            "postgresql://{}:{}@{}:{}/postgres",
            config.username, config.password, config.host, config.port
        );

        if !Postgres::database_exists(&database_url).await? {
            warn!("Database {} does not exist, creating it", config.database);
            Postgres::create_database(&database_url).await?;
            info!("Database {} created successfully", config.database);
        }

        Ok(())
    }

    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations...");
        
        match sqlx::migrate!("./migrations").run(&self.pool).await {
            Ok(_) => {
                info!("Database migrations completed successfully");
                Ok(())
            }
            Err(e) => {
                error!("Database migration failed: {}", e);
                Err(e.into())
            }
        }
    }

    pub async fn rollback_migration(&self, version: i64) -> Result<()> {
        info!("Rolling back to migration version {}", version);
        
        // This would require implementing custom rollback logic
        // For now, we'll use sqlx's built-in functionality
        sqlx::migrate!("./migrations")
            .undo(&self.pool, version)
            .await?;
        
        info!("Rollback to version {} completed", version);
        Ok(())
    }

    pub async fn get_migration_status(&self) -> Result<Vec<MigrationInfo>> {
        let migrations = sqlx::query!(
            r#"
            SELECT 
                version,
                description,
                installed_on,
                success
            FROM _sqlx_migrations
            ORDER BY version
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let migration_infos: Vec<MigrationInfo> = migrations
            .into_iter()
            .map(|m| MigrationInfo {
                version: m.version,
                description: m.description,
                installed_on: m.installed_on,
                success: m.success,
            })
            .collect();

        Ok(migration_infos)
    }

    pub async fn validate_schema(&self) -> Result<SchemaValidation> {
        let mut validation = SchemaValidation::default();
        
        // Check if all required tables exist
        let required_tables = vec![
            "users", "wallets", "transactions", "multi_signature_operations",
            "multi_signature_signers", "transaction_signatures", "user_sessions",
            "audit_logs", "security_alerts", "rate_limits", "user_devices",
            "access_patterns", "projects", "bounties", "bounty_applications",
            "bounty_submissions", "escrow_accounts", "bounty_reviews",
            "bounty_payments", "bounty_activity_log", "bounty_tags",
            "bounty_tag_relations"
        ];

        for table in required_tables {
            let exists = sqlx::query_scalar!(
                "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = $1)",
                table
            )
            .fetch_one(&self.pool)
            .await?;

            if exists.unwrap_or(false) {
                validation.existing_tables.push(table.to_string());
            } else {
                validation.missing_tables.push(table.to_string());
            }
        }

        // Check if required indexes exist
        let required_indexes = vec![
            "idx_users_email", "idx_users_username", "idx_wallets_user_id",
            "idx_wallets_stellar_address", "idx_transactions_transaction_hash",
            "idx_transactions_user_id", "idx_multi_sig_user_id"
        ];

        for index in required_indexes {
            let exists = sqlx::query_scalar!(
                "SELECT EXISTS (SELECT FROM pg_indexes WHERE indexname = $1)",
                index
            )
            .fetch_one(&self.pool)
            .await?;

            if exists.unwrap_or(false) {
                validation.existing_indexes.push(index.to_string());
            } else {
                validation.missing_indexes.push(index.to_string());
            }
        }

        validation.is_valid = validation.missing_tables.is_empty() && validation.missing_indexes.is_empty();

        Ok(validation)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationInfo {
    pub version: i64,
    pub description: String,
    pub installed_on: Option<chrono::DateTime<chrono::Utc>>,
    pub success: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SchemaValidation {
    pub is_valid: bool,
    pub existing_tables: Vec<String>,
    pub missing_tables: Vec<String>,
    pub existing_indexes: Vec<String>,
    pub missing_indexes: Vec<String>,
}

// Migration utilities
pub async fn backup_database(config: &super::DatabaseConfig, backup_path: &str) -> Result<()> {
    info!("Creating database backup to {}", backup_path);
    
    // This would typically use pg_dump or similar tool
    // For now, we'll just log the intention
    warn!("Database backup functionality not implemented yet");
    
    Ok(())
}

pub async fn restore_database(config: &super::DatabaseConfig, backup_path: &str) -> Result<()> {
    info!("Restoring database from {}", backup_path);
    
    // This would typically use psql or similar tool
    // For now, we'll just log the intention
    warn!("Database restore functionality not implemented yet");
    
    Ok(())
}

// Seed data for development/testing
pub async fn seed_development_data(pool: &PgPool) -> Result<()> {
    info!("Seeding development data...");
    
    // Create test users
    let test_user_id = sqlx::query_scalar!(
        r#"
        INSERT INTO users (email, username, password_hash, role, status, email_verified)
        VALUES ('test@example.com', 'testuser', '$2b$12$hashedpassword', 'user', 'active', true)
        RETURNING id
        "#
    )
    .fetch_one(pool)
    .await?;

    // Create test wallet
    sqlx::query!(
        r#"
        INSERT INTO wallets (user_id, stellar_address, wallet_name, wallet_type, status)
        VALUES ($1, 'GTEST1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'Test Wallet', 'standard', 'active')
        "#,
        test_user_id
    )
    .execute(pool)
    .await?;

    info!("Development data seeded successfully");
    Ok(())
}

// Database health checks
pub async fn run_health_checks(pool: &PgPool) -> Result<HealthCheckResults> {
    let mut results = HealthCheckResults::default();
    
    // Check database connectivity
    let start_time = std::time::Instant::now();
    match sqlx::query("SELECT 1").fetch_one(pool).await {
        Ok(_) => {
            results.connectivity = HealthStatus::Healthy;
            results.response_time_ms = start_time.elapsed().as_millis() as u64;
        }
        Err(e) => {
            results.connectivity = HealthStatus::Unhealthy;
            results.errors.push(format!("Database connectivity failed: {}", e));
        }
    }

    // Check connection pool
    let pool_size = pool.size();
    let idle_connections = pool.num_idle();
    
    if idle_connections > 0 {
        results.connection_pool = HealthStatus::Healthy;
    } else {
        results.connection_pool = HealthStatus::Warning;
        results.errors.push("No idle connections in pool".to_string());
    }
    
    results.pool_size = pool_size;
    results.idle_connections = idle_connections;

    // Check table counts
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    
    let wallet_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM wallets")
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    
    let transaction_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM transactions")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    results.table_counts.insert("users".to_string(), user_count);
    results.table_counts.insert("wallets".to_string(), wallet_count);
    results.table_counts.insert("transactions".to_string(), transaction_count);

    // Overall health
    results.overall_health = match (results.connectivity, results.connection_pool) {
        (HealthStatus::Healthy, HealthStatus::Healthy) => HealthStatus::Healthy,
        (HealthStatus::Healthy, HealthStatus::Warning) => HealthStatus::Warning,
        _ => HealthStatus::Unhealthy,
    };

    Ok(results)
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct HealthCheckResults {
    pub overall_health: HealthStatus,
    pub connectivity: HealthStatus,
    pub connection_pool: HealthStatus,
    pub response_time_ms: u64,
    pub pool_size: u32,
    pub idle_connections: u32,
    pub table_counts: std::collections::HashMap<String, i64>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Warning,
    Unhealthy,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::Unhealthy
    }
}
