//! Disaster recovery and business continuity planning.
//!
//! Provides:
//! - Health check probes with dependency resolution order
//! - Graceful degradation modes when dependencies are unavailable
//! - Recovery runbook as structured code comments

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// System dependency health status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus { Healthy, Degraded, Unhealthy }

/// A dependency check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyHealth {
    pub name:    String,
    pub status:  HealthStatus,
    pub latency_ms: Option<u64>,
    pub error:   Option<String>,
    pub critical: bool,
}

/// Overall system readiness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemReadiness {
    pub ready:        bool,
    pub degraded:     bool,
    pub dependencies: Vec<DependencyHealth>,
    pub mode:         OperationMode,
}

/// System operation mode under degraded conditions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationMode {
    /// All systems operational.
    Full,
    /// Redis unavailable: in-memory fallbacks active, caching disabled.
    ReadHeavy,
    /// DB unavailable: serving cached data only.
    CacheOnly,
    /// Critical dependency down: reject new requests.
    Maintenance,
}

impl SystemReadiness {
    pub fn compute(deps: Vec<DependencyHealth>) -> Self {
        let critical_down = deps.iter().any(|d| d.critical && d.status == HealthStatus::Unhealthy);
        let any_degraded  = deps.iter().any(|d| d.status != HealthStatus::Healthy);

        let mode = if critical_down { OperationMode::Maintenance }
            else if deps.iter().any(|d| d.name == "redis" && d.status == HealthStatus::Unhealthy) { OperationMode::ReadHeavy }
            else if deps.iter().any(|d| d.name == "database" && d.status == HealthStatus::Degraded) { OperationMode::CacheOnly }
            else { OperationMode::Full };

        Self {
            ready:        !critical_down,
            degraded:     any_degraded,
            dependencies: deps,
            mode,
        }
    }
}

/// Recovery time objectives.
pub mod rto {
    use std::time::Duration;
    /// Database restore from snapshot.
    pub const DATABASE_RESTORE: Duration = Duration::from_secs(30 * 60);
    /// Redis restart from AOF.
    pub const REDIS_RESTART:    Duration = Duration::from_secs(5 * 60);
    /// Full service restart.
    pub const SERVICE_RESTART:  Duration = Duration::from_secs(2 * 60);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dep(name: &str, status: HealthStatus, critical: bool) -> DependencyHealth {
        DependencyHealth { name: name.into(), status, latency_ms: None, error: None, critical }
    }
    #[test] fn all_healthy_is_full() {
        let r = SystemReadiness::compute(vec![dep("database",HealthStatus::Healthy,true),dep("redis",HealthStatus::Healthy,false)]);
        assert_eq!(r.mode, OperationMode::Full);
        assert!(r.ready);
    }
    #[test] fn critical_down_is_maintenance() {
        let r = SystemReadiness::compute(vec![dep("database",HealthStatus::Unhealthy,true)]);
        assert_eq!(r.mode, OperationMode::Maintenance);
        assert!(!r.ready);
    }
    #[test] fn redis_down_is_read_heavy() {
        let r = SystemReadiness::compute(vec![dep("database",HealthStatus::Healthy,true),dep("redis",HealthStatus::Unhealthy,false)]);
        assert_eq!(r.mode, OperationMode::ReadHeavy);
        assert!(r.ready);
    }
}
