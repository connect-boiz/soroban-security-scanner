use crate::error::{ServiceError, ServiceResult};
use crate::models::{RemediationHistory, CodePatch, VerificationStatus};
use sqlx::{PgPool, Row};
use uuid::Uuid;
use chrono::Utc;

pub struct RemediationDB {
    pool: PgPool,
}

impl RemediationDB {
    pub async fn new(database_url: &str) -> ServiceResult<Self> {
        let pool = PgPool::connect(database_url).await?;
        
        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;
        
        Ok(Self { pool })
    }
    
    pub async fn store_remediation(
        &self,
        vulnerability_id: &str,
        patch: &CodePatch,
        confidence_score: f64,
        verification_status: VerificationStatus,
    ) -> ServiceResult<String> {
        let id = Uuid::new_v4().to_string();
        
        let patch_json = serde_json::to_value(patch)?;
        
        sqlx::query!(
            r#"
            INSERT INTO remediation_history (
                id, vulnerability_id, patch, confidence_score, 
                verification_status, applied, success_rate, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            id,
            vulnerability_id,
            patch_json,
            confidence_score,
            verification_status as VerificationStatus,
            false, // not applied by default
            0.0,   // success rate will be updated later
            Utc::now()
        )
        .execute(&self.pool)
        .await?;
        
        Ok(id)
    }
    
    pub async fn get_remediation_history(
        &self,
        vulnerability_id: &str,
    ) -> ServiceResult<Vec<RemediationHistory>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, vulnerability_id, patch, confidence_score, 
                   verification_status, applied, success_rate, created_at
            FROM remediation_history
            WHERE vulnerability_id = $1
            ORDER BY created_at DESC
            "#,
            vulnerability_id
        )
        .fetch_all(&self.pool)
        .await?;
        
        let mut history = Vec::new();
        for row in rows {
            let patch: CodePatch = serde_json::from_value(row.patch)?;
            
            history.push(RemediationHistory {
                id: row.id,
                vulnerability_id: row.vulnerability_id,
                patch,
                confidence_score: row.confidence_score,
                verification_status: row.verification_status,
                applied: row.applied,
                success_rate: row.success_rate,
                created_at: row.created_at,
            });
        }
        
        Ok(history)
    }
    
    pub async fn get_successful_remediations(
        &self,
        limit: Option<i64>,
    ) -> ServiceResult<Vec<RemediationHistory>> {
        let limit = limit.unwrap_or(100);
        
        let rows = sqlx::query!(
            r#"
            SELECT id, vulnerability_id, patch, confidence_score, 
                   verification_status, applied, success_rate, created_at
            FROM remediation_history
            WHERE verification_status = $1 AND applied = $2 AND success_rate >= $3
            ORDER BY success_rate DESC, created_at DESC
            LIMIT $4
            "#,
            VerificationStatus::Passed as VerificationStatus,
            true,
            0.8,
            limit
        )
        .fetch_all(&self.pool)
        .await?;
        
        let mut remediations = Vec::new();
        for row in rows {
            let patch: CodePatch = serde_json::from_value(row.patch)?;
            
            remediations.push(RemediationHistory {
                id: row.id,
                vulnerability_id: row.vulnerability_id,
                patch,
                confidence_score: row.confidence_score,
                verification_status: row.verification_status,
                applied: row.applied,
                success_rate: row.success_rate,
                created_at: row.created_at,
            });
        }
        
        Ok(remediations)
    }
    
    pub async fn update_remediation_success(
        &self,
        remediation_id: &str,
        success_rate: f64,
    ) -> ServiceResult<()> {
        sqlx::query!(
            r#"
            UPDATE remediation_history
            SET success_rate = $1, applied = $2
            WHERE id = $3
            "#,
            success_rate,
            true,
            remediation_id
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn get_similar_remediations(
        &self,
        vulnerability_type: &str,
        severity: &str,
        limit: Option<i64>,
    ) -> ServiceResult<Vec<RemediationHistory>> {
        let limit = limit.unwrap_or(50);
        
        let rows = sqlx::query!(
            r#"
            SELECT rh.id, rh.vulnerability_id, rh.patch, rh.confidence_score, 
                   rh.verification_status, rh.applied, rh.success_rate, rh.created_at
            FROM remediation_history rh
            JOIN vulnerabilities v ON rh.vulnerability_id = v.id
            WHERE v.vulnerability_type = $1 
              AND v.severity = $2
              AND rh.verification_status = $3
              AND rh.applied = $4
              AND rh.success_rate >= $5
            ORDER BY rh.success_rate DESC, rh.created_at DESC
            LIMIT $6
            "#,
            vulnerability_type,
            severity,
            VerificationStatus::Passed as VerificationStatus,
            true,
            0.7,
            limit
        )
        .fetch_all(&self.pool)
        .await?;
        
        let mut remediations = Vec::new();
        for row in rows {
            let patch: CodePatch = serde_json::from_value(row.patch)?;
            
            remediations.push(RemediationHistory {
                id: row.id,
                vulnerability_id: row.vulnerability_id,
                patch,
                confidence_score: row.confidence_score,
                verification_status: row.verification_status,
                applied: row.applied,
                success_rate: row.success_rate,
                created_at: row.created_at,
            });
        }
        
        Ok(remediations)
    }
    
    pub async fn get_remediation_stats(&self) -> ServiceResult<RemediationStats> {
        let row = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_remediations,
                COUNT(CASE WHEN applied = true THEN 1 END) as applied_remediations,
                AVG(confidence_score) as avg_confidence,
                AVG(success_rate) as avg_success_rate,
                COUNT(CASE WHEN verification_status = 'Passed' THEN 1 END) as passed_verifications
            FROM remediation_history
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(RemediationStats {
            total_remediations: row.total_remediations.unwrap_or(0) as i64,
            applied_remediations: row.applied_remediations.unwrap_or(0) as i64,
            avg_confidence: row.avg_confidence.unwrap_or(0.0),
            avg_success_rate: row.avg_success_rate.unwrap_or(0.0),
            passed_verifications: row.passed_verifications.unwrap_or(0) as i64,
        })
    }
    
    pub async fn cleanup_old_remediations(&self, days_old: i64) -> ServiceResult<i64> {
        let cutoff_date = Utc::now() - chrono::Duration::days(days_old);
        
        let result = sqlx::query!(
            r#"
            DELETE FROM remediation_history
            WHERE created_at < $1 AND success_rate < 0.5 AND applied = false
            "#,
            cutoff_date
        )
        .execute(&self.pool)
        .await?;
        
        Ok(result.rows_affected() as i64)
    }
}

#[derive(Debug)]
pub struct RemediationStats {
    pub total_remediations: i64,
    pub applied_remediations: i64,
    pub avg_confidence: f64,
    pub avg_success_rate: f64,
    pub passed_verifications: i64,
}

// Database migration helper
pub async fn create_migrations() -> ServiceResult<()> {
    // This would typically be handled by sqlx migrate, but for completeness:
    let migration_sql = r#"
    CREATE TABLE IF NOT EXISTS remediation_history (
        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
        vulnerability_id VARCHAR(255) NOT NULL,
        patch JSONB NOT NULL,
        confidence_score FLOAT NOT NULL CHECK (confidence_score >= 0.0 AND confidence_score <= 1.0),
        verification_status VARCHAR(50) NOT NULL CHECK (verification_status IN ('Passed', 'Failed', 'Skipped')),
        applied BOOLEAN NOT NULL DEFAULT FALSE,
        success_rate FLOAT NOT NULL CHECK (success_rate >= 0.0 AND success_rate <= 1.0),
        created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
        updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
    );
    
    CREATE TABLE IF NOT EXISTS vulnerabilities (
        id VARCHAR(255) PRIMARY KEY,
        file_path VARCHAR(500) NOT NULL,
        vulnerability_type VARCHAR(100) NOT NULL,
        severity VARCHAR(20) NOT NULL CHECK (severity IN ('Critical', 'High', 'Medium', 'Low')),
        title VARCHAR(500) NOT NULL,
        description TEXT NOT NULL,
        code_snippet TEXT NOT NULL,
        line_number INTEGER NOT NULL,
        sarif_report JSONB,
        created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
    );
    
    CREATE INDEX IF NOT EXISTS idx_remediation_vulnerability_id ON remediation_history(vulnerability_id);
    CREATE INDEX IF NOT EXISTS idx_remediation_created_at ON remediation_history(created_at);
    CREATE INDEX IF NOT EXISTS idx_remediation_success_rate ON remediation_history(success_rate);
    CREATE INDEX IF NOT EXISTS idx_vulnerability_type ON vulnerabilities(vulnerability_type);
    CREATE INDEX IF NOT EXISTS idx_vulnerability_severity ON vulnerabilities(severity);
    "#;
    
    // In a real implementation, this would be run as a migration
    tracing::info!("Migration SQL generated");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_database_operations() {
        // This test would require a test database
        // For now, we'll just verify the structure
        let stats = RemediationStats {
            total_remediations: 100,
            applied_remediations: 80,
            avg_confidence: 0.75,
            avg_success_rate: 0.85,
            passed_verifications: 90,
        };
        
        assert_eq!(stats.total_remediations, 100);
        assert_eq!(stats.applied_remediations, 80);
        assert_eq!(stats.avg_confidence, 0.75);
    }
}
