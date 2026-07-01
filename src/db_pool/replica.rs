//! Read-replica routing for read-heavy workloads.
//!
//! Routes write (and read-your-write) traffic to the primary and load-balances
//! read-only queries across healthy replicas using round-robin selection.
//! Replicas can be marked unhealthy and are then skipped until they recover.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// Whether a query reads or writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryKind {
    /// Read-only; may be served by a replica.
    Read,
    /// Mutating; must go to the primary.
    Write,
}

/// A database endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Endpoint {
    /// Identifier / connection string label.
    pub name: String,
    /// Whether this is the primary (writable) node.
    pub is_primary: bool,
}

/// Routes queries between a primary and a set of read replicas.
pub struct ReplicaRouter {
    primary: Endpoint,
    replicas: Vec<Endpoint>,
    healthy: Mutex<Vec<bool>>,
    cursor: AtomicUsize,
}

impl ReplicaRouter {
    /// Builds a router with a primary and zero or more replicas (all initially
    /// healthy).
    pub fn new(primary_name: impl Into<String>, replica_names: Vec<String>) -> Self {
        let replicas: Vec<Endpoint> = replica_names
            .into_iter()
            .map(|name| Endpoint {
                name,
                is_primary: false,
            })
            .collect();
        let healthy = vec![true; replicas.len()];
        Self {
            primary: Endpoint {
                name: primary_name.into(),
                is_primary: true,
            },
            replicas,
            healthy: Mutex::new(healthy),
            cursor: AtomicUsize::new(0),
        }
    }

    /// Selects an endpoint for the given query kind.
    ///
    /// Writes always go to the primary. Reads round-robin across healthy
    /// replicas, falling back to the primary when none are available.
    pub fn route(&self, kind: QueryKind) -> &Endpoint {
        if kind == QueryKind::Write || self.replicas.is_empty() {
            return &self.primary;
        }
        let healthy = self.healthy.lock().expect("healthy poisoned");
        let healthy_idx: Vec<usize> = (0..self.replicas.len()).filter(|i| healthy[*i]).collect();
        if healthy_idx.is_empty() {
            return &self.primary; // graceful fallback
        }
        let pick = self.cursor.fetch_add(1, Ordering::Relaxed) % healthy_idx.len();
        &self.replicas[healthy_idx[pick]]
    }

    /// Marks a replica healthy/unhealthy by name. Returns false if unknown.
    pub fn set_health(&self, replica_name: &str, healthy: bool) -> bool {
        if let Some(idx) = self.replicas.iter().position(|r| r.name == replica_name) {
            self.healthy.lock().expect("healthy poisoned")[idx] = healthy;
            true
        } else {
            false
        }
    }

    /// Count of currently-healthy replicas.
    pub fn healthy_replica_count(&self) -> usize {
        self.healthy
            .lock()
            .expect("healthy poisoned")
            .iter()
            .filter(|h| **h)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn router() -> ReplicaRouter {
        ReplicaRouter::new("primary", vec!["r1".to_string(), "r2".to_string()])
    }

    #[test]
    fn writes_go_to_primary() {
        let r = router();
        assert!(r.route(QueryKind::Write).is_primary);
    }

    #[test]
    fn reads_round_robin_replicas() {
        let r = router();
        let a = r.route(QueryKind::Read).name.clone();
        let b = r.route(QueryKind::Read).name.clone();
        assert_ne!(a, b); // alternates r1/r2
        assert!(a.starts_with('r') && b.starts_with('r'));
    }

    #[test]
    fn unhealthy_replica_is_skipped() {
        let r = router();
        assert!(r.set_health("r1", false));
        for _ in 0..5 {
            assert_eq!(r.route(QueryKind::Read).name, "r2");
        }
        assert_eq!(r.healthy_replica_count(), 1);
    }

    #[test]
    fn falls_back_to_primary_when_no_healthy_replicas() {
        let r = router();
        r.set_health("r1", false);
        r.set_health("r2", false);
        assert!(r.route(QueryKind::Read).is_primary);
    }

    #[test]
    fn no_replicas_routes_reads_to_primary() {
        let r = ReplicaRouter::new("primary", vec![]);
        assert!(r.route(QueryKind::Read).is_primary);
    }

    #[test]
    fn set_health_unknown_replica_returns_false() {
        let r = router();
        assert!(!r.set_health("nope", false));
    }
}
