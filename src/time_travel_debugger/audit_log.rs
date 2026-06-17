use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: Instant,
    pub user_id: String,
    pub user_roles: Vec<String>,
    pub operation: AuditOperation,
    pub resource: String,
    pub ledger_sequence: Option<u32>,
    pub contract_id: Option<String>,
    pub success: bool,
    pub details: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditOperation {
    ForkAtLedger,
    GetContractState,
    SimulateUpgrade,
    InjectState,
    GetOrphanedState,
    ClearCache,
    ViewCacheStats,
    ViewAuditLogs,
    ManageAccessControl,
    ApproveOperation,
    RejectOperation,
    ManageQuotas,
    ManageRetention,
    ManageEncryption,
    ExportData,
    PermissionDenied,
    RateLimitExceeded,
    QuotaExceeded,
    RetentionCleanup,
    SuspiciousAccessPattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogQuery {
    pub user_id: Option<String>,
    pub operation: Option<AuditOperation>,
    pub contract_id: Option<String>,
    pub ledger_sequence: Option<u32>,
    pub success: Option<bool>,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogSummary {
    pub total_entries: usize,
    pub unique_users: usize,
    pub operations_by_type: std::collections::HashMap<String, usize>,
    pub success_rate: f64,
    pub recent_denials: usize,
    pub suspicious_events: usize,
}

pub struct AuditLogger {
    entries: Arc<RwLock<VecDeque<AuditEntry>>>,
    max_entries: usize,
}

impl AuditLogger {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(VecDeque::with_capacity(max_entries))),
            max_entries,
        }
    }

    pub async fn log(
        &self,
        user_id: &str,
        user_roles: Vec<String>,
        operation: AuditOperation,
        resource: &str,
        ledger_sequence: Option<u32>,
        contract_id: Option<&str>,
        success: bool,
        details: &str,
    ) -> AuditEntry {
        let entry = AuditEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Instant::now(),
            user_id: user_id.to_string(),
            user_roles,
            operation,
            resource: resource.to_string(),
            ledger_sequence,
            contract_id: contract_id.map(|s| s.to_string()),
            success,
            details: details.to_string(),
            ip_address: None,
            user_agent: None,
        };

        let mut entries = self.entries.write().await;
        if entries.len() >= self.max_entries {
            entries.pop_front();
        }
        entries.push_back(entry.clone());

        entry
    }

    pub async fn log_permission_denied(
        &self,
        user_id: &str,
        user_roles: Vec<String>,
        resource: &str,
        reason: &str,
    ) -> AuditEntry {
        self.log(
            user_id,
            user_roles,
            AuditOperation::PermissionDenied,
            resource,
            None,
            None,
            false,
            reason,
        )
        .await
    }

    pub async fn log_suspicious_access(
        &self,
        user_id: &str,
        user_roles: Vec<String>,
        resource: &str,
        details: &str,
    ) -> AuditEntry {
        self.log(
            user_id,
            user_roles,
            AuditOperation::SuspiciousAccessPattern,
            resource,
            None,
            None,
            false,
            details,
        )
        .await
    }

    pub async fn query(&self, query: &AuditLogQuery) -> Vec<AuditEntry> {
        let entries = self.entries.read().await;
        let limit = query.limit.min(entries.len());
        let mut results: Vec<AuditEntry> = entries
            .iter()
            .filter(|e| {
                if let Some(ref uid) = query.user_id {
                    if e.user_id != *uid {
                        return false;
                    }
                }
                if let Some(ref op) = query.operation {
                    if !std::mem::discriminant(&e.operation)
                        == std::mem::discriminant(op)
                    {
                        return false;
                    }
                }
                if let Some(ref cid) = query.contract_id {
                    if e.contract_id.as_deref() != Some(cid) {
                        return false;
                    }
                }
                if let Some(seq) = query.ledger_sequence {
                    if e.ledger_sequence != Some(seq) {
                        return false;
                    }
                }
                if let Some(success) = query.success {
                    if e.success != success {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        results.truncate(limit);
        results
    }

    pub async fn get_summary(&self) -> AuditLogSummary {
        let entries = self.entries.read().await;
        let total = entries.len();
        let unique_users: std::collections::HashSet<&str> =
            entries.iter().map(|e| e.user_id.as_str()).collect();
        let mut ops_by_type = std::collections::HashMap::new();
        let mut successes = 0;
        let mut denials = 0;
        let mut suspicious = 0;

        for entry in entries.iter() {
            let op_name = format!("{:?}", entry.operation);
            *ops_by_type.entry(op_name).or_insert(0) += 1;
            if entry.success {
                successes += 1;
            }
            if matches!(entry.operation, AuditOperation::PermissionDenied) {
                denials += 1;
            }
            if matches!(entry.operation, AuditOperation::SuspiciousAccessPattern) {
                suspicious += 1;
            }
        }

        AuditLogSummary {
            total_entries: total,
            unique_users: unique_users.len(),
            operations_by_type: ops_by_type,
            success_rate: if total > 0 {
                successes as f64 / total as f64
            } else {
                1.0
            },
            recent_denials: denials,
            suspicious_events: suspicious,
        }
    }

    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }

    pub async fn entry_count(&self) -> usize {
        self.entries.read().await.len()
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new(10000)
    }
}
