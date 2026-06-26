//! Comprehensive Audit Trail for Security-Critical Operations (#326)
//!
//! This module provides structured, tamper-evident audit logging for every
//! security-critical operation on the platform: vulnerability create / update /
//! delete, vulnerability verification, bounty payments, and administrative
//! actions.
//!
//! # Design goals
//!
//! * **Structured events** — every event carries a fixed, queryable schema:
//!   timestamp, user id, action, affected resource, IP address, user agent,
//!   request id, and previous / new state values.
//! * **Tamper-evident** — entries are linked into an append-only SHA-256 hash
//!   chain. Any modification to a past entry breaks the chain and is detected
//!   by [`AuditTrail::verify_chain`].
//! * **Write-once, read-many** — the in-memory store only appends; the backing
//!   database table (migration `008_add_audit_trail.sql`) enforces WORM
//!   semantics with triggers.
//! * **Role-based access** — querying audit entries requires an admin-class
//!   role (see [`UserRole::can_read_audit`]).
//! * **Real-time alerting** — [`AuditTrail::detect_suspicious_patterns`]
//!   surfaces anomalies such as a single user performing admin actions from
//!   multiple IP addresses within a short window.
//! * **Retention** — entries are retained for a configurable period (default
//!   7 years) and archived rather than dropped.
//! * **Performance** — recording an event is an in-memory append plus a single
//!   SHA-256 hash, well under the 50ms-per-operation budget.

use anyhow::{anyhow, Result};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Number of seconds in a 365-day year (used for the default retention window).
const SECONDS_PER_YEAR: u64 = 365 * 24 * 60 * 60;

/// The category a security-critical action belongs to.
///
/// Categories let the alerting layer reason about classes of activity (e.g.
/// "all administrative actions") without enumerating every concrete action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuditCategory {
    /// Vulnerability lifecycle operations.
    Vulnerability,
    /// Verification of a reported vulnerability.
    Verification,
    /// Bounty / payment operations involving funds.
    Bounty,
    /// Administrative / privileged operations.
    Admin,
    /// Authentication and session operations.
    Auth,
    /// Anything that does not fit the buckets above.
    General,
}

impl AuditCategory {
    /// Lowercase wire representation, matching the database `category` column.
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditCategory::Vulnerability => "vulnerability",
            AuditCategory::Verification => "verification",
            AuditCategory::Bounty => "bounty",
            AuditCategory::Admin => "admin",
            AuditCategory::Auth => "auth",
            AuditCategory::General => "general",
        }
    }
}

/// A concrete security-critical action.
///
/// Every state-changing operation that must appear in the audit trail has a
/// dedicated variant here so coverage can be reasoned about exhaustively.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuditAction {
    // Vulnerability lifecycle
    VulnerabilityCreate,
    VulnerabilityUpdate,
    VulnerabilityDelete,
    VulnerabilityVerify,
    VulnerabilityReject,
    // Bounty / payments
    BountyCreate,
    BountyUpdate,
    BountyPayment,
    BountyCancel,
    EscrowRelease,
    // Administrative actions
    AdminRoleChange,
    AdminUserSuspend,
    AdminConfigChange,
    AdminAccessGrant,
    AdminAccessRevoke,
    // Authentication
    AuthLogin,
    AuthLogout,
    AuthFailed,
}

impl AuditAction {
    /// The category this action rolls up into.
    pub fn category(&self) -> AuditCategory {
        match self {
            AuditAction::VulnerabilityCreate
            | AuditAction::VulnerabilityUpdate
            | AuditAction::VulnerabilityDelete => AuditCategory::Vulnerability,
            AuditAction::VulnerabilityVerify | AuditAction::VulnerabilityReject => {
                AuditCategory::Verification
            }
            AuditAction::BountyCreate
            | AuditAction::BountyUpdate
            | AuditAction::BountyPayment
            | AuditAction::BountyCancel
            | AuditAction::EscrowRelease => AuditCategory::Bounty,
            AuditAction::AdminRoleChange
            | AuditAction::AdminUserSuspend
            | AuditAction::AdminConfigChange
            | AuditAction::AdminAccessGrant
            | AuditAction::AdminAccessRevoke => AuditCategory::Admin,
            AuditAction::AuthLogin | AuditAction::AuthLogout | AuditAction::AuthFailed => {
                AuditCategory::Auth
            }
        }
    }

    /// Stable wire representation, matching the database `action` column.
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditAction::VulnerabilityCreate => "vulnerability.create",
            AuditAction::VulnerabilityUpdate => "vulnerability.update",
            AuditAction::VulnerabilityDelete => "vulnerability.delete",
            AuditAction::VulnerabilityVerify => "vulnerability.verify",
            AuditAction::VulnerabilityReject => "vulnerability.reject",
            AuditAction::BountyCreate => "bounty.create",
            AuditAction::BountyUpdate => "bounty.update",
            AuditAction::BountyPayment => "bounty.payment",
            AuditAction::BountyCancel => "bounty.cancel",
            AuditAction::EscrowRelease => "escrow.release",
            AuditAction::AdminRoleChange => "admin.role_change",
            AuditAction::AdminUserSuspend => "admin.user_suspend",
            AuditAction::AdminConfigChange => "admin.config_change",
            AuditAction::AdminAccessGrant => "admin.access_grant",
            AuditAction::AdminAccessRevoke => "admin.access_revoke",
            AuditAction::AuthLogin => "auth.login",
            AuditAction::AuthLogout => "auth.logout",
            AuditAction::AuthFailed => "auth.failed",
        }
    }
}

/// Severity of an audit event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuditSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl AuditSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditSeverity::Low => "low",
            AuditSeverity::Medium => "medium",
            AuditSeverity::High => "high",
            AuditSeverity::Critical => "critical",
        }
    }
}

/// Outcome of the audited operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuditOutcome {
    Success,
    Failure,
    Denied,
    Error,
}

impl AuditOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditOutcome::Success => "success",
            AuditOutcome::Failure => "failure",
            AuditOutcome::Denied => "denied",
            AuditOutcome::Error => "error",
        }
    }
}

/// The role of the actor performing the operation. Used both to label events
/// and to gate read access to the audit trail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UserRole {
    Admin,
    SecurityAdmin,
    Auditor,
    Researcher,
    User,
    Unknown,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::Admin => "admin",
            UserRole::SecurityAdmin => "security_admin",
            UserRole::Auditor => "auditor",
            UserRole::Researcher => "researcher",
            UserRole::User => "user",
            UserRole::Unknown => "unknown",
        }
    }

    /// Whether this role is permitted to read audit entries.
    ///
    /// Only admin-class roles may query the audit trail; this enforces the
    /// role-based access control requirement at the application layer.
    pub fn can_read_audit(&self) -> bool {
        matches!(
            self,
            UserRole::Admin | UserRole::SecurityAdmin | UserRole::Auditor
        )
    }
}

/// Request-scoped context describing who performed an action and from where.
///
/// Populated by the web/middleware layer from the incoming request so that the
/// IP address, user agent, and request id are captured consistently.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActorContext {
    pub user_id: String,
    pub user_role: Option<UserRole>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    pub session_id: Option<String>,
}

impl ActorContext {
    /// Convenience constructor for a known user id.
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            ..Default::default()
        }
    }

    pub fn with_role(mut self, role: UserRole) -> Self {
        self.user_role = Some(role);
        self
    }

    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }

    pub fn with_user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}

/// A single, fully-formed audit event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Globally-unique identifier for this entry.
    pub audit_id: String,
    /// Event time (Unix seconds).
    pub event_timestamp: u64,
    /// Time the entry was recorded (Unix seconds).
    pub recorded_at: u64,
    /// The action performed.
    pub action: AuditAction,
    /// Category the action rolls up into.
    pub category: AuditCategory,
    /// Severity of the event.
    pub severity: AuditSeverity,
    /// Outcome of the operation.
    pub outcome: AuditOutcome,
    /// Human-readable description.
    pub description: String,
    /// Identifier of the actor performing the operation.
    pub user_id: String,
    /// Role of the actor.
    pub user_role: UserRole,
    /// Source IP address, if known.
    pub ip_address: Option<String>,
    /// Client user agent, if known.
    pub user_agent: Option<String>,
    /// Correlating request id, if known.
    pub request_id: Option<String>,
    /// Session id, if known.
    pub session_id: Option<String>,
    /// Type of the affected resource (e.g. "vulnerability", "bounty").
    pub resource_type: Option<String>,
    /// Identifier of the affected resource.
    pub resource_id: Option<String>,
    /// Serialized previous state, for change tracking.
    pub previous_state: Option<String>,
    /// Serialized new state, for change tracking.
    pub new_state: Option<String>,
    /// Arbitrary structured metadata.
    pub metadata: HashMap<String, String>,
    /// SHA-256 hash of this entry's canonical content.
    pub entry_hash: String,
    /// SHA-256 hash of the previous entry (empty for the first entry).
    pub previous_entry_hash: String,
}

/// Builder for [`AuditEvent`]. The hash fields are computed by [`AuditTrail`]
/// when the event is recorded, so callers never set them directly.
pub struct AuditEventBuilder {
    event: AuditEvent,
}

impl AuditEventBuilder {
    /// Begin building an event for `action` performed within `actor` context.
    pub fn new(action: AuditAction, actor: ActorContext) -> Self {
        let now = current_unix_time();
        Self {
            event: AuditEvent {
                audit_id: String::new(),
                event_timestamp: now,
                recorded_at: now,
                action,
                category: action.category(),
                severity: AuditSeverity::Medium,
                outcome: AuditOutcome::Success,
                description: String::new(),
                user_id: actor.user_id,
                user_role: actor.user_role.unwrap_or(UserRole::Unknown),
                ip_address: actor.ip_address,
                user_agent: actor.user_agent,
                request_id: actor.request_id,
                session_id: actor.session_id,
                resource_type: None,
                resource_id: None,
                previous_state: None,
                new_state: None,
                metadata: HashMap::new(),
                entry_hash: String::new(),
                previous_entry_hash: String::new(),
            },
        }
    }

    pub fn severity(mut self, severity: AuditSeverity) -> Self {
        self.event.severity = severity;
        self
    }

    pub fn outcome(mut self, outcome: AuditOutcome) -> Self {
        self.event.outcome = outcome;
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.event.description = description.into();
        self
    }

    /// Identify the affected resource by type and id.
    pub fn resource(mut self, resource_type: impl Into<String>, resource_id: impl Into<String>) -> Self {
        self.event.resource_type = Some(resource_type.into());
        self.event.resource_id = Some(resource_id.into());
        self
    }

    pub fn previous_state(mut self, state: impl Into<String>) -> Self {
        self.event.previous_state = Some(state.into());
        self
    }

    pub fn new_state(mut self, state: impl Into<String>) -> Self {
        self.event.new_state = Some(state.into());
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.event.metadata.insert(key.into(), value.into());
        self
    }

    /// Finalize the event. Hash fields remain empty until the event is recorded.
    pub fn build(self) -> AuditEvent {
        self.event
    }
}

/// Configuration for the audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Whether audit logging is enabled.
    pub enabled: bool,
    /// Maximum entries retained in memory before the oldest are evicted.
    pub max_entries_in_memory: usize,
    /// Retention period, in seconds (default 7 years).
    pub retention_period_seconds: u64,
    /// Window, in seconds, used by suspicious-pattern detection.
    pub suspicious_window_seconds: u64,
    /// Minimum distinct IP addresses for a single user within the window that
    /// constitutes a suspicious admin-activity alert.
    pub suspicious_min_distinct_ips: usize,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_entries_in_memory: 100_000,
            retention_period_seconds: 7 * SECONDS_PER_YEAR,
            suspicious_window_seconds: 60 * 60, // 1 hour
            suspicious_min_distinct_ips: 2,
        }
    }
}

/// The audit trail: an append-only, tamper-evident store of audit events.
pub struct AuditTrail {
    config: AuditConfig,
    entries: Arc<Mutex<Vec<AuditEvent>>>,
    counter: Arc<AtomicU64>,
}

impl AuditTrail {
    /// Create a new audit trail with the given configuration.
    pub fn new(config: AuditConfig) -> Self {
        Self {
            config,
            entries: Arc::new(Mutex::new(Vec::new())),
            counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Create an audit trail with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(AuditConfig::default())
    }

    /// Record an audit event, assigning it an id and linking it into the hash
    /// chain. Returns the recorded event (including its computed hashes).
    ///
    /// Recording is an in-memory append plus one SHA-256 hash, comfortably
    /// inside the 50ms-per-operation budget.
    pub fn record(&self, mut event: AuditEvent) -> Result<AuditEvent> {
        if !self.config.enabled {
            return Ok(event);
        }

        let mut entries = self
            .entries
            .lock()
            .map_err(|e| anyhow!("failed to acquire audit lock: {}", e))?;

        // Assign a unique id if the caller did not set one.
        if event.audit_id.is_empty() {
            event.audit_id = self.generate_audit_id();
        }
        event.recorded_at = current_unix_time();

        // Link into the hash chain.
        event.previous_entry_hash = entries
            .last()
            .map(|e| e.entry_hash.clone())
            .unwrap_or_default();
        event.entry_hash = Self::compute_entry_hash(&event);

        // Structured log line for downstream collectors.
        info!(
            "audit: action={} outcome={} user={} resource={:?} ip={:?} request_id={:?}",
            event.action.as_str(),
            event.outcome.as_str(),
            event.user_id,
            event.resource_id,
            event.ip_address,
            event.request_id,
        );

        let recorded = event.clone();
        entries.push(event);

        // Enforce the in-memory cap. Eviction rebuilds the chain so the
        // retained slice remains internally consistent.
        if entries.len() > self.config.max_entries_in_memory {
            entries.remove(0);
            Self::rebuild_chain(&mut entries);
        }

        Ok(recorded)
    }

    /// Convenience: build and record an event in one call.
    pub fn record_action(
        &self,
        action: AuditAction,
        actor: ActorContext,
        description: impl Into<String>,
    ) -> Result<AuditEvent> {
        let event = AuditEventBuilder::new(action, actor)
            .description(description)
            .build();
        self.record(event)
    }

    /// Total number of entries currently held.
    pub fn len(&self) -> Result<usize> {
        let entries = self
            .entries
            .lock()
            .map_err(|e| anyhow!("failed to acquire audit lock: {}", e))?;
        Ok(entries.len())
    }

    /// Whether the trail is empty.
    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }

    // ── Role-gated queries ─────────────────────────────────────────────────

    /// Query entries with a filter. Requires an admin-class `requester_role`;
    /// otherwise returns an authorization error so non-admins cannot read the
    /// audit trail.
    pub fn query(
        &self,
        requester_role: UserRole,
        filter: &AuditQuery,
    ) -> Result<Vec<AuditEvent>> {
        if !requester_role.can_read_audit() {
            return Err(anyhow!(
                "access denied: role '{}' is not permitted to read the audit trail",
                requester_role.as_str()
            ));
        }

        let entries = self
            .entries
            .lock()
            .map_err(|e| anyhow!("failed to acquire audit lock: {}", e))?;

        let mut matched: Vec<AuditEvent> = entries
            .iter()
            .filter(|e| filter.matches(e))
            .cloned()
            .collect();

        // Most-recent first.
        matched.sort_by(|a, b| b.event_timestamp.cmp(&a.event_timestamp));

        // Apply offset / limit if requested.
        let start = filter.offset.min(matched.len());
        let mut page = matched.split_off(start);
        if let Some(limit) = filter.limit {
            page.truncate(limit);
        }
        Ok(page)
    }

    // ── Tamper-evidence ────────────────────────────────────────────────────

    /// Verify the integrity of the full hash chain. Detects both content
    /// tampering (an entry's recomputed hash no longer matches) and chain
    /// linkage breaks (an entry's `previous_entry_hash` no longer matches the
    /// prior entry).
    pub fn verify_chain(&self) -> Result<ChainVerification> {
        let entries = self
            .entries
            .lock()
            .map_err(|e| anyhow!("failed to acquire audit lock: {}", e))?;

        let mut mismatches = Vec::new();
        let mut prev_hash = String::new();

        for (i, entry) in entries.iter().enumerate() {
            let recomputed = Self::compute_entry_hash(entry);
            if recomputed != entry.entry_hash {
                mismatches.push(ChainMismatch {
                    index: i,
                    audit_id: entry.audit_id.clone(),
                    reason: "entry content hash mismatch — data was tampered with".to_string(),
                });
            }
            if entry.previous_entry_hash != prev_hash {
                mismatches.push(ChainMismatch {
                    index: i,
                    audit_id: entry.audit_id.clone(),
                    reason: "previous-entry hash mismatch — chain linkage broken".to_string(),
                });
            }
            prev_hash = entry.entry_hash.clone();
        }

        Ok(ChainVerification {
            intact: mismatches.is_empty(),
            verified_count: entries.len(),
            mismatches,
        })
    }

    // ── Real-time alerting ─────────────────────────────────────────────────

    /// Detect suspicious patterns over the configured window. Currently flags
    /// any user who performed administrative actions from two or more distinct
    /// IP addresses within the window — a classic credential-compromise signal.
    pub fn detect_suspicious_patterns(&self) -> Result<Vec<SuspiciousActivityAlert>> {
        let entries = self
            .entries
            .lock()
            .map_err(|e| anyhow!("failed to acquire audit lock: {}", e))?;

        let now = current_unix_time();
        let window_start = now.saturating_sub(self.config.suspicious_window_seconds);

        // user_id -> (set of IPs, action count, first_seen, last_seen)
        let mut by_user: HashMap<String, UserActivity> = HashMap::new();

        for entry in entries.iter() {
            if entry.category != AuditCategory::Admin {
                continue;
            }
            if entry.event_timestamp < window_start {
                continue;
            }
            let ip = match &entry.ip_address {
                Some(ip) => ip.clone(),
                None => continue,
            };
            let activity = by_user.entry(entry.user_id.clone()).or_insert(UserActivity {
                ips: Vec::new(),
                action_count: 0,
                first_seen: entry.event_timestamp,
                last_seen: entry.event_timestamp,
            });
            if !activity.ips.contains(&ip) {
                activity.ips.push(ip);
            }
            activity.action_count += 1;
            activity.first_seen = activity.first_seen.min(entry.event_timestamp);
            activity.last_seen = activity.last_seen.max(entry.event_timestamp);
        }

        let mut alerts = Vec::new();
        for (user_id, activity) in by_user {
            if activity.ips.len() >= self.config.suspicious_min_distinct_ips {
                warn!(
                    "audit alert: user '{}' performed {} admin actions from {} distinct IPs",
                    user_id,
                    activity.action_count,
                    activity.ips.len()
                );
                alerts.push(SuspiciousActivityAlert {
                    user_id,
                    distinct_ips: activity.ips.len(),
                    ip_addresses: activity.ips,
                    action_count: activity.action_count,
                    first_seen: activity.first_seen,
                    last_seen: activity.last_seen,
                    reason: "administrative actions from multiple IP addresses".to_string(),
                });
            }
        }

        alerts.sort_by(|a, b| b.distinct_ips.cmp(&a.distinct_ips));
        Ok(alerts)
    }

    // ── Retention / archival ───────────────────────────────────────────────

    /// Identify entries that have aged past the retention window. Returns the
    /// audit ids of entries eligible for cold-storage archival. Entries are not
    /// dropped here — archival is the responsibility of the storage layer — so
    /// the in-memory chain stays intact.
    pub fn entries_eligible_for_archival(&self) -> Result<Vec<String>> {
        let entries = self
            .entries
            .lock()
            .map_err(|e| anyhow!("failed to acquire audit lock: {}", e))?;

        let cutoff = current_unix_time().saturating_sub(self.config.retention_period_seconds);
        Ok(entries
            .iter()
            .filter(|e| e.event_timestamp < cutoff)
            .map(|e| e.audit_id.clone())
            .collect())
    }

    // ── Export ─────────────────────────────────────────────────────────────

    /// Serialize the given entries to pretty JSON.
    pub fn to_json(&self, entries: &[AuditEvent]) -> Result<String> {
        Ok(serde_json::to_string_pretty(entries)?)
    }

    /// Serialize the given entries to CSV.
    pub fn to_csv(&self, entries: &[AuditEvent]) -> String {
        let mut csv = String::from(
            "audit_id,event_timestamp,action,category,severity,outcome,user_id,user_role,ip_address,request_id,resource_type,resource_id,previous_entry_hash,entry_hash\n",
        );
        for e in entries {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                csv_escape(&e.audit_id),
                e.event_timestamp,
                e.action.as_str(),
                e.category.as_str(),
                e.severity.as_str(),
                e.outcome.as_str(),
                csv_escape(&e.user_id),
                e.user_role.as_str(),
                csv_escape(e.ip_address.as_deref().unwrap_or("")),
                csv_escape(e.request_id.as_deref().unwrap_or("")),
                csv_escape(e.resource_type.as_deref().unwrap_or("")),
                csv_escape(e.resource_id.as_deref().unwrap_or("")),
                e.previous_entry_hash,
                e.entry_hash,
            ));
        }
        csv
    }

    // ── Internals ──────────────────────────────────────────────────────────

    /// Compute the canonical SHA-256 hash of an entry's content. The hash
    /// covers every audited field plus the previous entry's hash, so any change
    /// to history is detectable.
    fn compute_entry_hash(event: &AuditEvent) -> String {
        let mut hasher = Sha256::new();
        hasher.update(event.audit_id.as_bytes());
        hasher.update(event.event_timestamp.to_le_bytes());
        hasher.update(event.action.as_str().as_bytes());
        hasher.update(event.category.as_str().as_bytes());
        hasher.update(event.severity.as_str().as_bytes());
        hasher.update(event.outcome.as_str().as_bytes());
        hasher.update(event.description.as_bytes());
        hasher.update(event.user_id.as_bytes());
        hasher.update(event.user_role.as_str().as_bytes());
        hasher.update(event.ip_address.as_deref().unwrap_or("").as_bytes());
        hasher.update(event.user_agent.as_deref().unwrap_or("").as_bytes());
        hasher.update(event.request_id.as_deref().unwrap_or("").as_bytes());
        hasher.update(event.session_id.as_deref().unwrap_or("").as_bytes());
        hasher.update(event.resource_type.as_deref().unwrap_or("").as_bytes());
        hasher.update(event.resource_id.as_deref().unwrap_or("").as_bytes());
        hasher.update(event.previous_state.as_deref().unwrap_or("").as_bytes());
        hasher.update(event.new_state.as_deref().unwrap_or("").as_bytes());
        // Metadata, hashed in a deterministic key order.
        let mut keys: Vec<&String> = event.metadata.keys().collect();
        keys.sort();
        for k in keys {
            hasher.update(k.as_bytes());
            hasher.update(b"=");
            hasher.update(event.metadata[k].as_bytes());
            hasher.update(b";");
        }
        hasher.update(event.previous_entry_hash.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Recompute the entire chain in place (used after an eviction).
    fn rebuild_chain(entries: &mut [AuditEvent]) {
        let mut prev_hash = String::new();
        for entry in entries.iter_mut() {
            entry.previous_entry_hash = prev_hash.clone();
            entry.entry_hash = Self::compute_entry_hash(entry);
            prev_hash = entry.entry_hash.clone();
        }
    }

    /// Generate a unique audit id.
    fn generate_audit_id(&self) -> String {
        let n = self.counter.fetch_add(1, Ordering::SeqCst);
        format!("audit_{}_{}", current_unix_time(), n)
    }
}

/// Filter for querying the audit trail. Every set field narrows the result.
#[derive(Debug, Clone, Default)]
pub struct AuditQuery {
    pub user_id: Option<String>,
    pub action: Option<AuditAction>,
    pub category: Option<AuditCategory>,
    pub severity: Option<AuditSeverity>,
    pub outcome: Option<AuditOutcome>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub ip_address: Option<String>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub offset: usize,
    pub limit: Option<usize>,
}

impl AuditQuery {
    /// An empty query that matches everything.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn action(mut self, action: AuditAction) -> Self {
        self.action = Some(action);
        self
    }

    pub fn category(mut self, category: AuditCategory) -> Self {
        self.category = Some(category);
        self
    }

    pub fn outcome(mut self, outcome: AuditOutcome) -> Self {
        self.outcome = Some(outcome);
        self
    }

    pub fn resource(mut self, resource_type: impl Into<String>, resource_id: impl Into<String>) -> Self {
        self.resource_type = Some(resource_type.into());
        self.resource_id = Some(resource_id.into());
        self
    }

    pub fn time_range(mut self, start: u64, end: u64) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }

    pub fn paginate(mut self, offset: usize, limit: usize) -> Self {
        self.offset = offset;
        self.limit = Some(limit);
        self
    }

    /// Whether `event` satisfies every set predicate.
    fn matches(&self, event: &AuditEvent) -> bool {
        if let Some(ref u) = self.user_id {
            if &event.user_id != u {
                return false;
            }
        }
        if let Some(a) = self.action {
            if event.action != a {
                return false;
            }
        }
        if let Some(c) = self.category {
            if event.category != c {
                return false;
            }
        }
        if let Some(s) = self.severity {
            if event.severity != s {
                return false;
            }
        }
        if let Some(o) = self.outcome {
            if event.outcome != o {
                return false;
            }
        }
        if let Some(ref rt) = self.resource_type {
            if event.resource_type.as_deref() != Some(rt.as_str()) {
                return false;
            }
        }
        if let Some(ref rid) = self.resource_id {
            if event.resource_id.as_deref() != Some(rid.as_str()) {
                return false;
            }
        }
        if let Some(ref ip) = self.ip_address {
            if event.ip_address.as_deref() != Some(ip.as_str()) {
                return false;
            }
        }
        if let Some(start) = self.start_time {
            if event.event_timestamp < start {
                return false;
            }
        }
        if let Some(end) = self.end_time {
            if event.event_timestamp > end {
                return false;
            }
        }
        true
    }
}

/// Result of verifying the audit hash chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerification {
    /// Whether the chain is fully intact.
    pub intact: bool,
    /// Number of entries verified.
    pub verified_count: usize,
    /// Any detected mismatches.
    pub mismatches: Vec<ChainMismatch>,
}

/// A single integrity violation found while verifying the chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainMismatch {
    pub index: usize,
    pub audit_id: String,
    pub reason: String,
}

/// An alert produced by suspicious-pattern detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspiciousActivityAlert {
    pub user_id: String,
    pub distinct_ips: usize,
    pub ip_addresses: Vec<String>,
    pub action_count: usize,
    pub first_seen: u64,
    pub last_seen: u64,
    pub reason: String,
}

/// Internal accumulator for suspicious-pattern detection.
struct UserActivity {
    ips: Vec<String>,
    action_count: usize,
    first_seen: u64,
    last_seen: u64,
}

/// Current Unix time in seconds.
fn current_unix_time() -> u64 {
    chrono::Utc::now().timestamp().max(0) as u64
}

/// Escape a value for inclusion in a CSV field.
fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
#[path = "audit_trail_tests.rs"]
mod audit_trail_tests;
