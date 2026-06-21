use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UserRole {
    Admin,
    ContractOwner,
    Auditor,
    User,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    ForkAtLedger,
    GetContractState,
    SimulateUpgrade,
    InjectState,
    GetOrphanedState,
    ClearCache,
    ViewCacheStats,
    ViewAuditLogs,
    ManageAccessControl,
    ApproveSensitiveOperation,
    ManageQuotas,
    ManageRetention,
    ManageEncryption,
    ExportData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub user_id: String,
    pub roles: HashSet<UserRole>,
    pub tier: UserTier,
    pub contract_ownerships: HashSet<String>,
    pub issued_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UserTier {
    Free,
    Pro,
    Enterprise,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionCheck {
    pub user: UserContext,
    pub permission: Permission,
    pub resource: Option<String>,
    pub ledger_sequence: Option<u32>,
    pub contract_id: Option<String>,
    pub timestamp: Instant,
    pub allowed: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub operation: String,
    pub requester_id: String,
    pub contract_id: Option<String>,
    pub ledger_sequence: Option<u32>,
    pub reason: String,
    pub status: ApprovalStatus,
    pub requested_at: Instant,
    pub resolved_at: Option<Instant>,
    pub resolved_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

pub struct AccessController {
    role_permissions: Arc<RwLock<HashMap<UserRole, HashSet<Permission>>>>,
    contract_owner_cache: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    pending_approvals: Arc<RwLock<Vec<ApprovalRequest>>>,
    approval_timeout: Duration,
}

impl AccessController {
    pub fn new() -> Self {
        let mut role_permissions = HashMap::new();

        role_permissions.insert(UserRole::Admin, {
            let mut perms = HashSet::new();
            perms.insert(Permission::ForkAtLedger);
            perms.insert(Permission::GetContractState);
            perms.insert(Permission::SimulateUpgrade);
            perms.insert(Permission::InjectState);
            perms.insert(Permission::GetOrphanedState);
            perms.insert(Permission::ClearCache);
            perms.insert(Permission::ViewCacheStats);
            perms.insert(Permission::ViewAuditLogs);
            perms.insert(Permission::ManageAccessControl);
            perms.insert(Permission::ApproveSensitiveOperation);
            perms.insert(Permission::ManageQuotas);
            perms.insert(Permission::ManageRetention);
            perms.insert(Permission::ManageEncryption);
            perms.insert(Permission::ExportData);
            perms
        });

        role_permissions.insert(UserRole::ContractOwner, {
            let mut perms = HashSet::new();
            perms.insert(Permission::ForkAtLedger);
            perms.insert(Permission::GetContractState);
            perms.insert(Permission::SimulateUpgrade);
            perms.insert(Permission::InjectState);
            perms.insert(Permission::GetOrphanedState);
            perms.insert(Permission::ViewCacheStats);
            perms
        });

        role_permissions.insert(UserRole::Auditor, {
            let mut perms = HashSet::new();
            perms.insert(Permission::ForkAtLedger);
            perms.insert(Permission::GetContractState);
            perms.insert(Permission::ViewCacheStats);
            perms.insert(Permission::ViewAuditLogs);
            perms.insert(Permission::ExportData);
            perms
        });

        role_permissions.insert(UserRole::User, {
            let mut perms = HashSet::new();
            perms.insert(Permission::ForkAtLedger);
            perms.insert(Permission::GetContractState);
            perms.insert(Permission::ViewCacheStats);
            perms
        });

        Self {
            role_permissions: Arc::new(RwLock::new(role_permissions)),
            contract_owner_cache: Arc::new(RwLock::new(HashMap::new())),
            pending_approvals: Arc::new(RwLock::new(Vec::new())),
            approval_timeout: Duration::from_secs(86400),
        }
    }

    pub async fn check_permission(
        &self,
        user: &UserContext,
        permission: &Permission,
        contract_id: Option<&str>,
        ledger_sequence: Option<u32>,
    ) -> Result<PermissionCheck> {
        let role_perms = self.role_permissions.read().await;

        for role in &user.roles {
            if let Some(perms) = role_perms.get(role) {
                if perms.contains(permission) {
                    if let Some(cid) = contract_id {
                        if *permission == Permission::SimulateUpgrade
                            || *permission == Permission::InjectState
                            || *permission == Permission::GetOrphanedState
                        {
                            if !user.contract_ownerships.contains(cid)
                                && !user.roles.contains(&UserRole::Admin)
                            {
                                return Ok(PermissionCheck {
                                    user: user.clone(),
                                    permission: permission.clone(),
                                    resource: Some(cid.to_string()),
                                    ledger_sequence,
                                    contract_id: Some(cid.to_string()),
                                    timestamp: Instant::now(),
                                    allowed: false,
                                    reason: Some("User does not own this contract".to_string()),
                                });
                            }
                        }
                    }

                    if let Some(seq) = ledger_sequence {
                        if !self.is_ledger_sequence_accessible(user, seq).await {
                            return Ok(PermissionCheck {
                                user: user.clone(),
                                permission: permission.clone(),
                                resource: None,
                                ledger_sequence: Some(seq),
                                contract_id: contract_id.map(|s| s.to_string()),
                                timestamp: Instant::now(),
                                allowed: false,
                                reason: Some(format!(
                                    "Ledger sequence {} is not accessible for this user tier",
                                    seq
                                )),
                            });
                        }
                    }

                    return Ok(PermissionCheck {
                        user: user.clone(),
                        permission: permission.clone(),
                        resource: None,
                        ledger_sequence,
                        contract_id: contract_id.map(|s| s.to_string()),
                        timestamp: Instant::now(),
                        allowed: true,
                        reason: None,
                    });
                }
            }
        }

        Ok(PermissionCheck {
            user: user.clone(),
            permission: permission.clone(),
            resource: None,
            ledger_sequence,
            contract_id: contract_id.map(|s| s.to_string()),
            timestamp: Instant::now(),
            allowed: false,
            reason: Some("User lacks required role permissions".to_string()),
        })
    }

    pub async fn check_sensitive_contract_access(
        &self,
        user: &UserContext,
        contract_id: &str,
        ledger_sequence: u32,
    ) -> Result<bool> {
        if user.roles.contains(&UserRole::Admin) {
            return Ok(true);
        }

        if user.contract_ownerships.contains(contract_id) {
            return Ok(true);
        }

        if user.roles.contains(&UserRole::Auditor) {
            let owner_cache = self.contract_owner_cache.read().await;
            if let Some(owners) = owner_cache.get(contract_id) {
                if owners.iter().any(|o| o == &user.user_id) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    async fn is_ledger_sequence_accessible(&self, user: &UserContext, seq: u32) -> bool {
        match user.tier {
            UserTier::Enterprise => true,
            UserTier::Pro => seq >= 100000 && seq % 10 == 0,
            UserTier::Free => seq >= 100000 && seq % 100 == 0,
        }
    }

    pub async fn register_contract_owner(&self, contract_id: &str, owner_id: &str) -> Result<()> {
        let mut cache = self.contract_owner_cache.write().await;
        cache
            .entry(contract_id.to_string())
            .or_insert_with(HashSet::new)
            .insert(owner_id.to_string());
        Ok(())
    }

    pub async fn request_approval(
        &self,
        requester_id: &str,
        operation: &str,
        contract_id: Option<&str>,
        ledger_sequence: Option<u32>,
        reason: &str,
    ) -> Result<ApprovalRequest> {
        let id = uuid::Uuid::new_v4().to_string();
        let request = ApprovalRequest {
            id,
            operation: operation.to_string(),
            requester_id: requester_id.to_string(),
            contract_id: contract_id.map(|s| s.to_string()),
            ledger_sequence,
            reason: reason.to_string(),
            status: ApprovalStatus::Pending,
            requested_at: Instant::now(),
            resolved_at: None,
            resolved_by: None,
        };

        {
            let mut approvals = self.pending_approvals.write().await;
            approvals.push(request.clone());
        }

        Ok(request)
    }

    pub async fn resolve_approval(
        &self,
        approval_id: &str,
        resolver_id: &str,
        approved: bool,
    ) -> Result<()> {
        let mut approvals = self.pending_approvals.write().await;
        if let Some(request) = approvals.iter_mut().find(|r| r.id == approval_id) {
            if request.status != ApprovalStatus::Pending {
                return Err(anyhow!("Approval request already resolved"));
            }
            request.status = if approved {
                ApprovalStatus::Approved
            } else {
                ApprovalStatus::Rejected
            };
            request.resolved_at = Some(Instant::now());
            request.resolved_by = Some(resolver_id.to_string());
            Ok(())
        } else {
            Err(anyhow!("Approval request not found"))
        }
    }

    pub async fn check_approval_status(&self, approval_id: &str) -> Option<ApprovalStatus> {
        let approvals = self.pending_approvals.read().await;
        approvals.iter().find(|r| r.id == approval_id).map(|r| {
            if matches!(r.status, ApprovalStatus::Pending)
                && r.requested_at.elapsed() > self.approval_timeout
            {
                ApprovalStatus::Expired
            } else {
                r.status.clone()
            }
        })
    }

    pub async fn get_role_permissions(&self, role: &UserRole) -> HashSet<Permission> {
        let role_perms = self.role_permissions.read().await;
        role_perms.get(role).cloned().unwrap_or_default()
    }

    pub async fn add_role_permission(&self, role: UserRole, permission: Permission) -> Result<()> {
        let mut role_perms = self.role_permissions.write().await;
        role_perms
            .entry(role)
            .or_insert_with(HashSet::new)
            .insert(permission);
        Ok(())
    }

    pub async fn remove_role_permission(
        &self,
        role: UserRole,
        permission: Permission,
    ) -> Result<()> {
        let mut role_perms = self.role_permissions.write().await;
        if let Some(perms) = role_perms.get_mut(&role) {
            perms.remove(&permission);
        }
        Ok(())
    }

    pub async fn get_pending_approvals(&self) -> Vec<ApprovalRequest> {
        let approvals = self.pending_approvals.read().await;
        approvals
            .iter()
            .filter(|r| matches!(r.status, ApprovalStatus::Pending))
            .cloned()
            .collect()
    }
}

impl Default for AccessController {
    fn default() -> Self {
        Self::new()
    }
}
