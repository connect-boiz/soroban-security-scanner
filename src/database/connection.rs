use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;
use anyhow::Result;
use tracing::{info, error};

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            database: "soroban_security_scanner".to_string(),
            username: "postgres".to_string(),
            password: "password".to_string(),
            max_connections: 20,
            min_connections: 5,
            connect_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            max_lifetime: Duration::from_secs(1800),
        }
    }
}

impl DatabaseConfig {
    pub fn connection_string(&self) -> String {
        format!(
            "postgresql://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database
        )
    }

    pub fn from_env() -> Result<Self> {
        let config = Self {
            host: std::env::var("DATABASE_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: std::env::var("DATABASE_PORT")
                .unwrap_or_else(|_| "5432".to_string())
                .parse()
                .unwrap_or(5432),
            database: std::env::var("DATABASE_NAME")
                .unwrap_or_else(|_| "soroban_security_scanner".to_string()),
            username: std::env::var("DATABASE_USER")
                .unwrap_or_else(|_| "postgres".to_string()),
            password: std::env::var("DATABASE_PASSWORD")
                .unwrap_or_else(|_| "password".to_string()),
            max_connections: std::env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "20".to_string())
                .parse()
                .unwrap_or(20),
            min_connections: std::env::var("DATABASE_MIN_CONNECTIONS")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap_or(5),
            connect_timeout: Duration::from_secs(
                std::env::var("DATABASE_CONNECT_TIMEOUT")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30)
            ),
            idle_timeout: Duration::from_secs(
                std::env::var("DATABASE_IDLE_TIMEOUT")
                    .unwrap_or_else(|_| "600".to_string())
                    .parse()
                    .unwrap_or(600)
            ),
            max_lifetime: Duration::from_secs(
                std::env::var("DATABASE_MAX_LIFETIME")
                    .unwrap_or_else(|_| "1800".to_string())
                    .parse()
                    .unwrap_or(1800)
            ),
        };
        Ok(config)
    }
}

#[derive(Debug, Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(config: DatabaseConfig) -> Result<Self> {
        info!("Connecting to database: {}:{}/{}", config.host, config.port, config.database);
        
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .connect_timeout(config.connect_timeout)
            .idle_timeout(config.idle_timeout)
            .max_lifetime(config.max_lifetime)
            .connect(&config.connection_string())
            .await?;

        // Test the connection
        sqlx::query("SELECT 1")
            .fetch_one(&pool)
            .await
            .map_err(|e| {
                error!("Failed to connect to database: {}", e);
                e
            })?;

        info!("Successfully connected to database");
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn close(&self) {
        info!("Closing database connections");
        self.pool.close().await;
    }

    // Migration methods
    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations");
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await?;
        info!("Database migrations completed successfully");
        Ok(())
    }

    // Transaction helper
    pub async fn begin_transaction(&self) -> Result<sqlx::Transaction<'_, sqlx::Postgres>> {
        Ok(self.pool.begin().await?)
    }

    // Connection pool statistics
    pub async fn pool_stats(&self) -> PoolStats {
        let size = self.pool.size();
        let idle = self.pool.num_idle();
        PoolStats {
            total_connections: size,
            idle_connections: idle,
            active_connections: size - idle,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PoolStats {
    pub total_connections: u32,
    pub idle_connections: u32,
    pub active_connections: u32,
}

// Database connection singleton for use throughout the application
use std::sync::Arc;
use tokio::sync::OnceCell;

static DATABASE: OnceCell<Arc<Database>> = OnceCell::const_new();

pub async fn init_database(config: DatabaseConfig) -> Result<Arc<Database>> {
    let db = Arc::new(Database::new(config).await?);
    DATABASE.set(db.clone()).map_err(|_| anyhow::anyhow!("Database already initialized"))?;
    Ok(db)
}

pub async fn get_database() -> Result<Arc<Database>> {
    DATABASE.get()
        .ok_or_else(|| anyhow::anyhow!("Database not initialized"))
        .cloned()
}

// Test utilities
#[cfg(test)]
pub mod test_utils {
    use super::*;
    use sqlx::PgPool;

    pub async fn create_test_pool() -> PgPool {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5432/soroban_security_scanner_test".to_string());

        PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("Failed to create test database pool")
    }

    pub async fn setup_test_database(pool: &PgPool) -> Result<()> {
        // Run migrations on test database
        sqlx::migrate!("./migrations")
            .run(pool)
            .await?;
        Ok(())
    }

    pub async fn cleanup_test_database(pool: &PgPool) -> Result<()> {
        // Clean up all tables
        let tables = vec![
            "audit_logs",
            "user_sessions",
            "transaction_signatures",
            "multi_signature_signers",
            "multi_signature_operations",
            "transactions",
            "wallets",
            "users",
            "security_alerts",
            "rate_limits",
            "user_devices",
            "access_patterns",
            "bounty_activity_log",
            "bounty_tag_relations",
            "bounty_tags",
            "bounty_payments",
            "bounty_reviews",
            "escrow_accounts",
            "bounty_submissions",
            "bounty_applications",
            "bounties",
            "projects",
        ];

        for table in tables {
            sqlx::query(&format!("TRUNCATE TABLE {} RESTART IDENTITY CASCADE", table))
                .execute(pool)
                .await?;
        }

        Ok(())
    }
}
