use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::access_control::UserTier;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaConfig {
    pub max_forks_per_day: u32,
    pub max_state_retrievals_per_day: u32,
    pub max_upgrade_simulations_per_day: u32,
    pub max_state_injections_per_day: u32,
    pub max_orphaned_state_analyses_per_day: u32,
    pub max_concurrent_forks: u32,
    pub max_storage_size_mb: u32,
}

impl QuotaConfig {
    pub fn free() -> Self {
        Self {
            max_forks_per_day: 10,
            max_state_retrievals_per_day: 100,
            max_upgrade_simulations_per_day: 5,
            max_state_injections_per_day: 20,
            max_orphaned_state_analyses_per_day: 10,
            max_concurrent_forks: 2,
            max_storage_size_mb: 100,
        }
    }

    pub fn pro() -> Self {
        Self {
            max_forks_per_day: 100,
            max_state_retrievals_per_day: 1000,
            max_upgrade_simulations_per_day: 50,
            max_state_injections_per_day: 200,
            max_orphaned_state_analyses_per_day: 100,
            max_concurrent_forks: 5,
            max_storage_size_mb: 1000,
        }
    }

    pub fn enterprise() -> Self {
        Self {
            max_forks_per_day: 10000,
            max_state_retrievals_per_day: 100000,
            max_upgrade_simulations_per_day: 5000,
            max_state_injections_per_day: 20000,
            max_orphaned_state_analyses_per_day: 10000,
            max_concurrent_forks: 50,
            max_storage_size_mb: 50000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuotaOperation {
    Fork,
    StateRetrieval,
    UpgradeSimulation,
    StateInjection,
    OrphanedStateAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaStatus {
    pub operation: QuotaOperation,
    pub used: u32,
    pub limit: u32,
    pub remaining: u32,
    pub resets_in_seconds: u64,
    pub allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserQuotaState {
    pub forks_today: u32,
    pub retrievals_today: u32,
    pub simulations_today: u32,
    pub injections_today: u32,
    pub orphaned_analyses_today: u32,
    pub active_forks: u32,
    pub storage_used_mb: u32,
    pub last_reset: Instant,
}

pub struct QuotaManager {
    tier_quotas: HashMap<UserTier, QuotaConfig>,
    user_states: Arc<RwLock<HashMap<String, UserQuotaState>>>,
}

impl QuotaManager {
    pub fn new() -> Self {
        let mut tier_quotas = HashMap::new();
        tier_quotas.insert(UserTier::Free, QuotaConfig::free());
        tier_quotas.insert(UserTier::Pro, QuotaConfig::pro());
        tier_quotas.insert(UserTier::Enterprise, QuotaConfig::enterprise());

        Self {
            tier_quotas,
            user_states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn ensure_state(&self, user_id: &str) -> UserQuotaState {
        let mut states = self.user_states.write().await;
        let now = Instant::now();
        states
            .entry(user_id.to_string())
            .or_insert_with(|| UserQuotaState {
                forks_today: 0,
                retrievals_today: 0,
                simulations_today: 0,
                injections_today: 0,
                orphaned_analyses_today: 0,
                active_forks: 0,
                storage_used_mb: 0,
                last_reset: now,
            })
            .clone()
    }

    async fn reset_if_needed(&self, user_id: &str) {
        let mut states = self.user_states.write().await;
        let now = Instant::now();
        if let Some(state) = states.get_mut(user_id) {
            if state.last_reset.elapsed() > Duration::from_secs(86400) {
                state.forks_today = 0;
                state.retrievals_today = 0;
                state.simulations_today = 0;
                state.injections_today = 0;
                state.orphaned_analyses_today = 0;
                state.last_reset = now;
            }
        }
    }

    pub async fn check_quota(
        &self,
        user_id: &str,
        tier: &UserTier,
        operation: &QuotaOperation,
    ) -> QuotaStatus {
        self.reset_if_needed(user_id).await;
        let config = self.tier_quotas.get(tier).cloned().unwrap_or_else(QuotaConfig::free);
        let state = self.ensure_state(user_id).await;

        let (used, limit) = match operation {
            QuotaOperation::Fork => (state.forks_today, config.max_forks_per_day),
            QuotaOperation::StateRetrieval => (state.retrievals_today, config.max_state_retrievals_per_day),
            QuotaOperation::UpgradeSimulation => (state.simulations_today, config.max_upgrade_simulations_per_day),
            QuotaOperation::StateInjection => (state.injections_today, config.max_state_injections_per_day),
            QuotaOperation::OrphanedStateAnalysis => (state.orphaned_analyses_today, config.max_orphaned_state_analyses_per_day),
        };

        let remaining = limit.saturating_sub(used);
        let resets_in = Duration::from_secs(86400)
            .checked_sub(state.last_reset.elapsed())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        QuotaStatus {
            operation: operation.clone(),
            used,
            limit,
            remaining,
            resets_in_seconds: resets_in,
            allowed: remaining > 0,
        }
    }

    pub async fn record_operation(
        &self,
        user_id: &str,
        tier: &UserTier,
        operation: &QuotaOperation,
    ) -> QuotaStatus {
        self.reset_if_needed(user_id).await;
        let config = self.tier_quotas.get(tier).cloned().unwrap_or_else(QuotaConfig::free);

        let mut states = self.user_states.write().await;
        let state = states.entry(user_id.to_string()).or_insert_with(|| UserQuotaState {
            forks_today: 0,
            retrievals_today: 0,
            simulations_today: 0,
            injections_today: 0,
            orphaned_analyses_today: 0,
            active_forks: 0,
            storage_used_mb: 0,
            last_reset: Instant::now(),
        });

        let (used, limit) = match operation {
            QuotaOperation::Fork => {
                state.forks_today += 1;
                (state.forks_today, config.max_forks_per_day)
            }
            QuotaOperation::StateRetrieval => {
                state.retrievals_today += 1;
                (state.retrievals_today, config.max_state_retrievals_per_day)
            }
            QuotaOperation::UpgradeSimulation => {
                state.simulations_today += 1;
                (state.simulations_today, config.max_upgrade_simulations_per_day)
            }
            QuotaOperation::StateInjection => {
                state.injections_today += 1;
                (state.injections_today, config.max_state_injections_per_day)
            }
            QuotaOperation::OrphanedStateAnalysis => {
                state.orphaned_analyses_today += 1;
                (state.orphaned_analyses_today, config.max_orphaned_state_analyses_per_day)
            }
        };

        QuotaStatus {
            operation: operation.clone(),
            used,
            limit,
            remaining: limit.saturating_sub(used),
            resets_in_seconds: Duration::from_secs(86400)
                .checked_sub(state.last_reset.elapsed())
                .map(|d| d.as_secs())
                .unwrap_or(0),
            allowed: used <= limit,
        }
    }

    pub async fn check_concurrent_forks(&self, user_id: &str, tier: &UserTier) -> bool {
        self.reset_if_needed(user_id).await;
        let config = self.tier_quotas.get(tier).cloned().unwrap_or_else(QuotaConfig::free);
        let mut states = self.user_states.write().await;

        let state = states.entry(user_id.to_string()).or_insert_with(|| UserQuotaState {
            forks_today: 0,
            retrievals_today: 0,
            simulations_today: 0,
            injections_today: 0,
            orphaned_analyses_today: 0,
            active_forks: 0,
            storage_used_mb: 0,
            last_reset: Instant::now(),
        });

        state.active_forks < config.max_concurrent_forks
    }

    pub async fn track_active_fork(&self, user_id: &str, start: bool) {
        let mut states = self.user_states.write().await;
        if let Some(state) = states.get_mut(user_id) {
            if start {
                state.active_forks += 1;
            } else {
                state.active_forks = state.active_forks.saturating_sub(1);
            }
        }
    }

    pub async fn track_storage_usage(&self, user_id: &str, bytes: usize) {
        let mb = (bytes / (1024 * 1024)) as u32;
        let mut states = self.user_states.write().await;
        if let Some(state) = states.get_mut(user_id) {
            state.storage_used_mb += mb;
        }
    }

    pub async fn check_storage_quota(
        &self,
        user_id: &str,
        tier: &UserTier,
        additional_bytes: usize,
    ) -> bool {
        self.reset_if_needed(user_id).await;
        let config = self.tier_quotas.get(tier).cloned().unwrap_or_else(QuotaConfig::free);
        let states = self.user_states.read().await;

        let current_mb = states
            .get(user_id)
            .map(|s| s.storage_used_mb)
            .unwrap_or(0);
        let additional_mb = (additional_bytes / (1024 * 1024)) as u32;
        (current_mb + additional_mb) <= config.max_storage_size_mb
    }

    pub async fn get_usage(&self, user_id: &str) -> Option<UserQuotaState> {
        let states = self.user_states.read().await;
        states.get(user_id).cloned()
    }
}

impl Default for QuotaManager {
    fn default() -> Self {
        Self::new()
    }
}
